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
