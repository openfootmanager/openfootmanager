use chrono::{TimeZone, Utc};
use db::save_manager::SaveManager;
use domain::{
    league::{CompetitionScope, CompetitionType, FixtureStatus},
    manager::Manager,
    national_team::NationalTeam,
};
use ofm_core::{
    clock::GameClock,
    game::Game,
    group_stage::{GroupStageConfig, generate_group_knockout_cup_with},
};
use std::collections::BTreeMap;

fn tournament_game(active: bool) -> Game {
    let start = Utc.with_ymd_and_hms(2033, 2, 1, 12, 0, 0).unwrap();
    let manager = Manager::new(
        "manager".into(),
        "A".into(),
        "B".into(),
        "1980-01-01".into(),
        "ENG".into(),
    );
    let mut game = Game::new(
        GameClock::new(start),
        manager,
        vec![],
        vec![],
        vec![],
        vec![],
    );
    game.seed = 41;
    let mut rng = game.rng_for("test-national-rosters", "2033-02-01");
    for (index, code) in ["ENG", "FR", "DE", "ES", "IT", "PT", "NL", "BE"]
        .iter()
        .enumerate()
    {
        let mut player =
            ofm_core::generator::generate_national_team_player(code, index, 2033, &mut rng);
        player.condition = 100;
        player.fitness = 65;
        let mut nation = NationalTeam::new(
            format!("nt-{code}"),
            code.to_string(),
            code.to_string(),
            Some("europe".into()),
        );
        nation.squad_player_ids.push(player.id.clone());
        game.players.push(player);
        game.national_teams.push(nation);
    }
    let ids: Vec<_> = game
        .national_teams
        .iter()
        .map(|team| team.id.clone())
        .collect();
    let mut cup = generate_group_knockout_cup_with(
        "Continental nations",
        2033,
        &ids,
        start,
        CompetitionType::InternationalNation,
        CompetitionScope::Continental,
        &GroupStageConfig {
            legs: 1,
            matchday_gap_days: 3,
            knockout_round_gap_days: 3,
            knockout_matches_per_day: 2,
            ..Default::default()
        },
    );
    cup.region_id = Some("europe".into());
    if active {
        game.active_competition_ids = vec![cup.id.clone()];
        game.active_region_ids = vec!["europe".into()];
    } else {
        game.active_competition_ids = vec!["unrelated-league".into()];
        game.active_region_ids = vec!["south-america".into()];
    }
    assert_eq!(game.competition_in_active_scope(&cup), active);
    assert!(!ofm_core::world_cup::is_world_cup_competition(&cup));
    game.competitions.push(cup);
    game
}

fn snapshot(game: &Game) -> (BTreeMap<String, serde_json::Value>, serde_json::Value) {
    let cup = &game.competitions[0];
    (
        cup.fixtures
            .iter()
            .map(|f| (f.id.clone(), serde_json::to_value(f).unwrap()))
            .collect(),
        serde_json::to_value((&cup.groups, &cup.knockout_rounds)).unwrap(),
    )
}

fn assert_fresh_load_keeps_tournament(game: &Game) -> Game {
    let mut probe = game.clone();
    assert_eq!(ofm_core::catchup::repair_stranded_fixtures(&mut probe), 0);
    assert_eq!(
        ofm_core::calendar_repair::repair_loaded_calendar(&mut probe),
        0
    );
    let saves = tempfile::tempdir().unwrap();
    let mut writer = SaveManager::init(saves.path()).unwrap();
    let id = writer.create_save(game, "Continental tournament").unwrap();
    let mut reader = SaveManager::init(saves.path()).unwrap();
    let loaded = reader.load_game(&id).unwrap();
    assert_eq!(
        snapshot(&loaded),
        snapshot(game),
        "reload must not play or alter a tournament fixture"
    );
    loaded
}

fn play_tournament_on_its_days(active: bool, live_day: bool) {
    let mut game = tournament_game(active);
    let first_player = game.players[0].id.clone();
    let world_cup_state = serde_json::to_value((
        &game.world_history.world_cup_champions,
        &game.world_history.world_cup_hosts,
    ))
    .unwrap();
    let mut previous_results = BTreeMap::new();
    let mut loaded_after_groups = false;
    for day in 0..60 {
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        let due: Vec<_> = game.competitions[0]
            .fixtures
            .iter()
            .filter(|f| f.date == today && f.status == FixtureStatus::Scheduled)
            .map(|f| f.id.clone())
            .collect();
        if day == 0 {
            assert_eq!(due.len(), 4);
        }
        if live_day {
            ofm_core::turn::finish_live_match_day(&mut game);
        } else {
            ofm_core::turn::process_day(&mut game);
        }
        let completed_day = snapshot(&game);
        let player_and_world_state =
            serde_json::to_value((&game.players, &game.world_history)).unwrap();
        assert_eq!(
            ofm_core::national_tournament::process_fixtures_due(&mut game, &today),
            0,
            "repeating a processed date must not play a completed fixture"
        );
        assert_eq!(snapshot(&game), completed_day);
        assert_eq!(
            serde_json::to_value((&game.players, &game.world_history)).unwrap(),
            player_and_world_state,
            "a repeated date must apply no second wear or ranking change"
        );
        let cup = &game.competitions[0];
        for fixture in &cup.fixtures {
            if fixture.date > today {
                assert_eq!(
                    fixture.status,
                    FixtureStatus::Scheduled,
                    "future matches must wait for their date"
                );
            }
            if fixture.status == FixtureStatus::Completed {
                assert!(
                    fixture.date <= today,
                    "a completed match cannot be dated in the future"
                );
            }
        }
        for id in due {
            let fixture = cup.fixtures.iter().find(|f| f.id == id).unwrap();
            assert_eq!(
                fixture.status,
                FixtureStatus::Completed,
                "{id} must play on {today}"
            );
            assert_eq!(fixture.date, today);
            assert!(fixture.result.is_some());
        }
        for (id, result) in &previous_results {
            let fixture = cup.fixtures.iter().find(|f| &f.id == id).unwrap();
            assert_eq!(
                &serde_json::to_value(&fixture.result).unwrap(),
                result,
                "a completed match cannot be played again"
            );
        }
        previous_results = cup
            .fixtures
            .iter()
            .filter(|f| f.status == FixtureStatus::Completed)
            .map(|f| (f.id.clone(), serde_json::to_value(&f.result).unwrap()))
            .collect();
        if day == 0 {
            assert!(
                game.players
                    .iter()
                    .find(|p| p.id == first_player)
                    .unwrap()
                    .condition
                    < 100,
                "national match wear is carried back"
            );
        }
        let reached_knockouts = !cup.knockout_rounds.is_empty();
        let complete = cup
            .fixtures
            .iter()
            .all(|f| f.status == FixtureStatus::Completed);
        let mut probe = game.clone();
        assert_eq!(ofm_core::catchup::repair_stranded_fixtures(&mut probe), 0);
        if reached_knockouts && !loaded_after_groups {
            game = assert_fresh_load_keeps_tournament(&game);
            loaded_after_groups = true;
        }
        if complete {
            break;
        }
    }
    assert!(
        loaded_after_groups,
        "the groups must seed a bracket and survive a fresh load"
    );
    let cup = &game.competitions[0];
    assert_eq!(
        cup.fixtures.len(),
        15,
        "12 group matches, two semi-finals and a final"
    );
    assert!(
        cup.fixtures
            .iter()
            .all(|f| f.status == FixtureStatus::Completed && f.result.is_some())
    );
    assert_eq!(
        cup.groups
            .iter()
            .flat_map(|g| &g.standings)
            .map(|row| row.played)
            .sum::<u32>(),
        24,
        "every group result is recorded once"
    );
    let group_rows: Vec<_> = cup
        .groups
        .iter()
        .flat_map(|group| &group.standings)
        .collect();
    assert_eq!(group_rows.len(), 8);
    assert!(
        group_rows.iter().all(|row| row.played == 3),
        "each nation must receive its three group results exactly once"
    );
    assert_eq!(cup.knockout_rounds.len(), 2);
    assert!(cup.knockout_rounds.iter().all(|round| round.completed));
    assert!(ofm_core::world_cup::world_cup_champion(cup).is_some());
    assert_eq!(
        serde_json::to_value((
            &game.world_history.world_cup_champions,
            &game.world_history.world_cup_hosts
        ))
        .unwrap(),
        world_cup_state,
        "a continental champion must not trigger World Cup policies"
    );
    assert!(
        !game
            .messages
            .iter()
            .any(|message| message.id.starts_with("world_cup_champion_"))
    );
    assert_fresh_load_keeps_tournament(&game);
}

/// Given an active continental national tournament, when normal days advance through groups and
/// knockouts, then every match plays once on its day and fresh loads repair nothing.
#[test]
fn an_active_continental_tournament_plays_on_normal_days() {
    play_tournament_on_its_days(true, false);
}

/// Given an inactive continental national tournament, when normal days advance through groups and
/// knockouts, then scope cannot strand it and fresh loads repair nothing.
#[test]
fn an_inactive_continental_tournament_plays_on_normal_days() {
    play_tournament_on_its_days(false, false);
}

/// Given an active continental national tournament, when the player's live match days finish,
/// then groups and knockouts play once on their days and fresh loads repair nothing.
#[test]
fn an_active_continental_tournament_plays_on_live_match_days() {
    play_tournament_on_its_days(true, true);
}

/// Given an inactive continental national tournament, when the player's live match days finish,
/// then scope cannot strand its groups or knockouts and fresh loads repair nothing.
#[test]
fn an_inactive_continental_tournament_plays_on_live_match_days() {
    play_tournament_on_its_days(false, true);
}

/// Given a legacy save with continental national fixtures stranded yesterday,
/// When a fresh reader repairs it, then the national match path applies player wear and a second
/// save/load needs no repair.
#[test]
fn a_legacy_continental_save_is_repaired_with_national_carry_back() {
    let mut game = tournament_game(false);
    let player_id = game.players[0].id.clone();
    game.clock.advance_days(1);
    let saves = tempfile::tempdir().unwrap();
    let mut writer = SaveManager::init(saves.path()).unwrap();
    let id = writer
        .create_save(&game, "Legacy continental tournament")
        .unwrap();
    let mut reader = SaveManager::init(saves.path()).unwrap();
    let loaded = reader.load_game(&id).unwrap();
    assert_eq!(
        loaded.competitions[0]
            .fixtures
            .iter()
            .filter(|f| f.status == FixtureStatus::Completed)
            .count(),
        4
    );
    assert!(
        loaded
            .players
            .iter()
            .find(|p| p.id == player_id)
            .unwrap()
            .condition
            < 100,
        "continental load repair must apply national-player wear"
    );
    assert_fresh_load_keeps_tournament(&loaded);
}

/// Given an authored continental national tournament using a plain league table,
/// When its match plays, then the same national match path records the result in both table rows.
#[test]
fn a_continental_national_league_table_records_each_result() {
    use domain::league::{Fixture, FixtureCompetition, League};
    let mut game = tournament_game(false);
    let ids: Vec<_> = game
        .national_teams
        .iter()
        .take(2)
        .map(|team| team.id.clone())
        .collect();
    let mut table = League::new(
        "continental-table".into(),
        "Continental table".into(),
        2033,
        &ids,
    );
    table.kind = CompetitionType::InternationalNation;
    table.scope = CompetitionScope::Continental;
    table.fixtures.push(Fixture {
        id: "continental-table-match".into(),
        competition_id: table.id.clone(),
        date: "2033-02-01".into(),
        home_team_id: ids[0].clone(),
        away_team_id: ids[1].clone(),
        competition: FixtureCompetition::InternationalNation,
        ..Default::default()
    });
    game.competitions = vec![table];
    assert_eq!(
        ofm_core::national_tournament::process_fixtures_due(&mut game, "2033-02-01"),
        1
    );
    let table = &game.competitions[0];
    assert_eq!(table.fixtures[0].status, FixtureStatus::Completed);
    assert_eq!(table.standings.len(), 2);
    assert!(
        table.standings.iter().all(|row| row.played == 1),
        "both nations must receive their league-table result"
    );
}

fn competing_tournaments() -> Game {
    use domain::league::{Fixture, FixtureCompetition, League};
    let mut game = tournament_game(false);
    let nations: Vec<_> = game.national_teams.iter().map(|n| n.id.clone()).collect();
    game.competitions[0] = generate_group_knockout_cup_with(
        "Continental nations",
        2033,
        &nations,
        game.clock.current_date,
        CompetitionType::InternationalNation,
        CompetitionScope::Continental,
        &GroupStageConfig {
            group_size: 2,
            legs: 1,
            qualifiers_per_group: 2,
            matchday_gap_days: 3,
            knockout_round_gap_days: 3,
            knockout_matches_per_day: 4,
            ..Default::default()
        },
    );
    game.competitions[0].priority = 9500;
    let mut qualifier = League::new(
        "world-cup-qualifying-2034-europe".into(),
        "World Cup qualifying".into(),
        2034,
        &nations[..2],
    );
    qualifier.kind = CompetitionType::InternationalNation;
    qualifier.scope = CompetitionScope::International;
    qualifier.priority = 9000;
    qualifier.fixtures = vec![Fixture {
        id: "qualifier-eng-fr".into(),
        competition_id: qualifier.id.clone(),
        home_team_id: nations[0].clone(),
        away_team_id: nations[1].clone(),
        date: "2033-02-04".into(),
        competition: FixtureCompetition::InternationalNation,
        ..Default::default()
    }];
    assert!(ofm_core::world_cup::is_world_cup_qualifying(&qualifier));
    game.competitions.push(qualifier);
    game
}

fn overlaps_qualifier(fixture: &domain::league::Fixture) -> bool {
    [&fixture.home_team_id, &fixture.away_team_id]
        .iter()
        .any(|id| ["nt-ENG", "nt-FR"].contains(&id.as_str()))
}

fn draw_competing_round() -> Game {
    let mut game = competing_tournaments();
    assert_eq!(
        ofm_core::national_tournament::process_fixtures_due(&mut game, "2033-02-01"),
        4
    );
    assert_eq!(game.competitions[0].knockout_rounds.len(), 1);
    assert_eq!(game.competitions[0].knockout_rounds[0].fixture_ids.len(), 4);
    game
}

/// Given a continental round whose authored date clashes with World Cup qualifying,
/// When its last group game draws that round, Then the lower-priority ties already have legal dates.
#[test]
fn a_drawn_continental_round_yields_to_world_cup_qualifying() {
    let game = draw_competing_round();
    let ties: Vec<_> = game.competitions[0]
        .fixtures
        .iter()
        .filter(|f| f.status == FixtureStatus::Scheduled && overlaps_qualifier(f))
        .collect();
    assert!(!ties.is_empty());
    for tie in ties {
        assert!(
            tie.date.as_str() >= "2033-02-07",
            "draw must keep three days from the qualifier: {}",
            tie.date
        );
    }
    assert_eq!(game.competitions[1].fixtures[0].date, "2033-02-04");
}

fn finish_collision_day(live: bool) {
    let mut game = draw_competing_round();
    for fixture in &mut game.competitions[0].fixtures {
        if fixture.status == FixtureStatus::Scheduled && overlaps_qualifier(fixture) {
            fixture.date = "2033-02-04".into();
            fixture.postponement = None;
        }
    }
    game.clock.current_date = Utc.with_ymd_and_hms(2033, 2, 4, 12, 0, 0).unwrap();
    let mut legal = game.clone();
    for fixture in &mut legal.competitions[0].fixtures {
        if fixture.status == FixtureStatus::Scheduled && overlaps_qualifier(fixture) {
            fixture.date = "2033-02-07".into();
        }
    }
    let play = |game: &mut Game| {
        if live {
            ofm_core::turn::finish_live_match_day(game);
        } else {
            ofm_core::turn::process_day(game);
        }
    };
    let before: BTreeMap<_, _> = game
        .players
        .iter()
        .filter(|p| {
            game.national_teams[..2]
                .iter()
                .any(|n| n.squad_player_ids.contains(&p.id))
        })
        .map(|p| (p.id.clone(), p.condition))
        .collect();
    assert_eq!(before.len(), 2);
    play(&mut legal);
    play(&mut game);
    let qualifier = &game.competitions[1].fixtures[0];
    assert_eq!(qualifier.status, FixtureStatus::Completed);
    assert_eq!(qualifier.date, "2033-02-04");
    let national_games_today = game
        .competitions
        .iter()
        .flat_map(|c| &c.fixtures)
        .filter(|f| {
            f.status == FixtureStatus::Completed && f.date == "2033-02-04" && overlaps_qualifier(f)
        })
        .count();
    assert_eq!(
        national_games_today, 1,
        "qualifying nations must play only their qualifier today"
    );
    for (id, condition) in before {
        assert!(
            game.players.iter().find(|p| p.id == id).unwrap().condition < condition,
            "qualifier player {id} must receive one match of wear"
        );
    }
    assert_eq!(
        serde_json::to_value(&game.players).unwrap(),
        serde_json::to_value(&legal.players).unwrap(),
        "the collision must apply exactly the wear of the legal one-match calendar"
    );
    complete_competing_tournaments(game, live);
}

fn assert_national_rest(game: &Game) {
    let mut bookings: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for fixture in game
        .competitions
        .iter()
        .flat_map(|c| &c.fixtures)
        .filter(|f| f.status == FixtureStatus::Completed)
    {
        for nation in [&fixture.home_team_id, &fixture.away_team_id] {
            bookings
                .entry(nation.clone())
                .or_default()
                .push(fixture.date.clone());
        }
    }
    assert!(!bookings.is_empty());
    for dates in bookings.values_mut() {
        dates.sort();
        for pair in dates.windows(2) {
            let date = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
            assert!(
                (date(&pair[1]) - date(&pair[0])).num_days() >= 3,
                "national rest violated: {pair:?}"
            );
        }
    }
}

fn complete_competing_tournaments(mut game: Game, live: bool) {
    let mut results = BTreeMap::new();
    for _ in 0..30 {
        let date = game.clock.current_date.format("%Y-%m-%d").to_string();
        if live {
            ofm_core::turn::finish_live_match_day(&mut game);
        } else {
            ofm_core::turn::process_day(&mut game);
        }
        for fixture in game.competitions.iter().flat_map(|c| &c.fixtures) {
            if let Some(previous) = results.get(&fixture.id) {
                assert_eq!(
                    previous,
                    &serde_json::to_value(&fixture.result).unwrap(),
                    "a played fixture must retain its result"
                );
            }
            if fixture.status == FixtureStatus::Completed {
                results.insert(
                    fixture.id.clone(),
                    serde_json::to_value(&fixture.result).unwrap(),
                );
            }
        }
        let wear = serde_json::to_value(&game.players).unwrap();
        assert_eq!(
            ofm_core::national_tournament::process_fixtures_due(&mut game, &date),
            0
        );
        assert_eq!(
            serde_json::to_value(&game.players).unwrap(),
            wear,
            "repeated dates apply no wear"
        );
        assert_national_rest(&game);
        if results.len() == 12 {
            break;
        }
    }
    assert_eq!(
        results.len(),
        12,
        "all four group matches, seven knockout ties and the qualifier must play"
    );
    assert_eq!(
        game.competitions
            .iter()
            .map(|c| c.fixtures.len())
            .sum::<usize>(),
        12
    );
    let competitions = serde_json::to_value(&game.competitions).unwrap();
    let players = serde_json::to_value(&game.players).unwrap();
    let loaded = assert_fresh_load_keeps_tournament(&game);
    assert_eq!(
        serde_json::to_value(&loaded.competitions).unwrap(),
        competitions
    );
    assert_eq!(serde_json::to_value(&loaded.players).unwrap(), players);
}

/// Given an existing same-day continental/qualifying collision, When a normal day runs,
/// Then one match per nation plays, wear occurs once and all postponed ties eventually play and persist.
#[test]
fn a_normal_day_settles_competing_national_tournaments() {
    finish_collision_day(false);
}

/// Given the same collision outside active scope, When a live club match day finishes,
/// Then the national fixture and wear invariants match the normal day and survive reload.
#[test]
fn a_live_day_settles_competing_national_tournaments() {
    finish_collision_day(true);
}

/// Given disjoint nations whose squads share a player, When a tournament day runs,
/// Then that player's lower-priority match moves even without a nation-id clash.
#[test]
fn competing_national_squads_do_not_double_book_a_shared_player() {
    let mut game = draw_competing_round();
    let tie = game.competitions[0]
        .fixtures
        .iter()
        .find(|f| f.status == FixtureStatus::Scheduled && !overlaps_qualifier(f))
        .unwrap()
        .clone();
    let player = game.national_teams[0].squad_player_ids[0].clone();
    game.national_teams
        .iter_mut()
        .find(|n| n.id == tie.home_team_id)
        .unwrap()
        .squad_player_ids = vec![player];
    game.clock.current_date = Utc.with_ymd_and_hms(2033, 2, 4, 12, 0, 0).unwrap();
    ofm_core::turn::process_day(&mut game);
    let moved = game.competitions[0]
        .fixtures
        .iter()
        .find(|f| f.id == tie.id)
        .unwrap();
    assert_eq!(
        moved.status,
        FixtureStatus::Scheduled,
        "shared-player tie must wait"
    );
    assert!(moved.date.as_str() >= "2033-02-07");
    assert_eq!(
        game.competitions[1].fixtures[0].status,
        FixtureStatus::Completed
    );
}

/// Given a legacy save with both tournament fixtures still due yesterday, When load repair replays it,
/// Then qualifying keeps its historical date, the other tie moves, and a later fresh load repairs nothing.
#[test]
fn a_legacy_tournament_collision_is_repaired_without_replaying_wear() {
    let mut game = draw_competing_round();
    for fixture in &mut game.competitions[0].fixtures {
        if fixture.status == FixtureStatus::Scheduled && overlaps_qualifier(fixture) {
            fixture.date = "2033-02-04".into();
        }
    }
    game.clock.current_date = Utc.with_ymd_and_hms(2033, 2, 5, 12, 0, 0).unwrap();
    assert!(ofm_core::catchup::repair_stranded_fixtures(&mut game) > 0);
    assert_eq!(game.competitions[1].fixtures[0].date, "2033-02-04");
    assert_national_rest(&game);
    assert_eq!(ofm_core::catchup::repair_stranded_fixtures(&mut game), 0);
    complete_competing_tournaments(game, false);
}
