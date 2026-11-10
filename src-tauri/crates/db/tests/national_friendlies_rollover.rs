use chrono::{TimeZone, Utc};
use db::save_manager::SaveManager;
use domain::{
    league::{Fixture, FixtureCompetition, FixtureStatus, League, MatchResult},
    manager::Manager,
    national_team::NationalTeam,
    player::{Player, PlayerAttributes, Position},
    team::Team,
};
use ofm_core::{clock::GameClock, end_of_season::advance_to_next_season, game::Game};

fn spring_career() -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2031, 5, 1, 12, 0, 0).unwrap());
    let manager = Manager::new(
        "manager".into(),
        "A".into(),
        "B".into(),
        "1980-01-01".into(),
        "ENG".into(),
    );
    let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
    game.seed = 57;
    for id in ["club-a", "club-b"] {
        game.teams.push(Team::new(
            id.into(),
            id.into(),
            id.into(),
            "England".into(),
            "City".into(),
            "Stadium".into(),
            1000,
        ));
    }
    let mut league = League::new(
        "league".into(),
        "League".into(),
        2030,
        &["club-a".into(), "club-b".into()],
    );
    league.fixtures.push(Fixture {
        id: "last-league-match".into(),
        competition_id: league.id.clone(),
        date: "2031-04-30".into(),
        home_team_id: "club-a".into(),
        away_team_id: "club-b".into(),
        status: FixtureStatus::Completed,
        result: Some(MatchResult::default()),
        ..Default::default()
    });
    game.competitions.push(league);
    for (id, nation) in [("nt-eng", "ENG"), ("nt-fra", "FR")] {
        let mut player = Player::new(
            format!("player-{id}"),
            id.into(),
            id.into(),
            "2005-01-01".into(),
            nation.into(),
            Position::Midfielder,
            PlayerAttributes {
                pace: 70,
                stamina: 70,
                strength: 70,
                agility: 70,
                passing: 70,
                shooting: 70,
                tackling: 70,
                dribbling: 70,
                defending: 70,
                positioning: 70,
                vision: 70,
                decisions: 70,
                composure: 70,
                aggression: 70,
                teamwork: 70,
                leadership: 70,
                handling: 50,
                reflexes: 50,
                aerial: 60,
            },
        );
        player.team_id = Some(if id == "nt-eng" { "club-a" } else { "club-b" }.into());
        player.condition = 100;
        player.fitness = 65;
        let mut team = NationalTeam::new(id.into(), nation.into(), nation.into(), None);
        team.squad_player_ids.push(player.id.clone());
        game.players.push(player);
        game.national_teams.push(team);
    }
    game.national_teams[0].fixtures.push(Fixture {
        id: "published-june-friendly".into(),
        competition_id: "international-friendlies".into(),
        date: "2031-06-09".into(),
        home_team_id: "nt-eng".into(),
        away_team_id: "nt-fra".into(),
        competition: FixtureCompetition::InternationalNation,
        ..Default::default()
    });
    game.sync_legacy_league();
    game
}

fn friendlies(game: &Game) -> Vec<&Fixture> {
    game.national_teams
        .iter()
        .flat_map(|team| &team.fixtures)
        .collect()
}

fn assert_reload_changes_nothing(game: &Game) {
    let mut probe = game.clone();
    assert_eq!(ofm_core::catchup::repair_stranded_fixtures(&mut probe), 0);
    assert_eq!(
        ofm_core::calendar_repair::repair_loaded_calendar(&mut probe),
        0
    );
    let saves = tempfile::tempdir().unwrap();
    let mut writer = SaveManager::init(saves.path()).unwrap();
    let id = writer.create_save(game, "National calendar").unwrap();
    let mut reader = SaveManager::init(saves.path()).unwrap();
    let loaded = reader.load_game(&id).unwrap();
    let snapshot = |game: &Game| {
        let mut fixtures: Vec<_> = friendlies(game)
            .into_iter()
            .map(|fixture| (fixture.id.clone(), serde_json::to_value(fixture).unwrap()))
            .collect();
        fixtures.sort_by(|a, b| a.0.cmp(&b.0));
        fixtures
    };
    assert_eq!(snapshot(&loaded), snapshot(game));
    for original in &game.players {
        let loaded_player = loaded.players.iter().find(|p| p.id == original.id).unwrap();
        assert_eq!(loaded_player.condition, original.condition);
        assert_eq!(loaded_player.fitness, original.fitness);
    }
}

/// Given a June friendly already published before a spring rollover,
/// When the career rolls over and reaches its date,
/// Then its identity survives, it plays once with carry-back, and a fresh load repairs nothing.
#[test]
fn a_june_friendly_survives_spring_rollover_and_plays_exactly_once() {
    let mut game = spring_career();
    advance_to_next_season(&mut game).unwrap();
    let kept: Vec<_> = friendlies(&game)
        .into_iter()
        .filter(|f| f.id == "published-june-friendly")
        .collect();
    assert_eq!(
        kept.len(),
        1,
        "spring rollover must retain the published June friendly"
    );
    assert_eq!(kept[0].date, "2031-06-09");
    assert_eq!(kept[0].status, FixtureStatus::Scheduled);
    assert_eq!(
        friendlies(&game)
            .into_iter()
            .filter(|f| f.date == "2031-06-09")
            .count(),
        1,
        "only one national match occupies June 9"
    );
    assert_reload_changes_nothing(&game);
    game.clock.current_date = Utc.with_ymd_and_hms(2031, 6, 9, 12, 0, 0).unwrap();
    let before: Vec<_> = game
        .players
        .iter()
        .filter(|p| p.id.starts_with("player-nt-"))
        .map(|p| (p.id.clone(), p.condition))
        .collect();
    ofm_core::turn::process_day(&mut game);
    let kept: Vec<_> = friendlies(&game)
        .into_iter()
        .filter(|f| f.id == "published-june-friendly")
        .collect();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].date, "2031-06-09");
    assert_eq!(kept[0].status, FixtureStatus::Completed);
    assert!(kept[0].result.is_some());
    assert_eq!(
        friendlies(&game)
            .into_iter()
            .filter(|f| f.date == "2031-06-09" && f.status == FixtureStatus::Completed)
            .count(),
        1
    );
    assert_eq!(before.len(), 2, "both called-up club players are checked");
    for (id, condition) in before {
        assert!(
            game.players.iter().find(|p| p.id == id).unwrap().condition < condition,
            "national match wear reaches {id}"
        );
    }
    let after = serde_json::to_value(&game.players).unwrap();
    let results = serde_json::to_value(friendlies(&game)).unwrap();
    assert_eq!(
        ofm_core::national_team::process_national_team_fixtures_due(&mut game, "2031-06-09"),
        0
    );
    assert_eq!(serde_json::to_value(&game.players).unwrap(), after);
    assert_eq!(serde_json::to_value(friendlies(&game)).unwrap(), results);
    assert_reload_changes_nothing(&game);
}

/// Given a friendly postponed away from its original window,
/// When that window is filled again, then the kept match still fills it and its ID is not reused.
#[test]
fn a_postponed_friendly_still_fills_its_original_window() {
    let mut game = spring_career();
    let windows = vec!["2031-09-09".to_string()];
    game.national_teams[0].fixtures.clear();
    let mut rng = game.rng_for("test-friendly", "2031-09-09");
    ofm_core::national_team::schedule_national_team_friendlies(
        &mut game.national_teams,
        &windows,
        &mut rng,
    );
    assert_eq!(friendlies(&game).len(), 1);
    let fixture = game
        .national_teams
        .iter_mut()
        .flat_map(|t| &mut t.fixtures)
        .next()
        .unwrap();
    fixture.postponement = Some(domain::league::Postponement {
        original_date: fixture.date.clone(),
        reason: domain::league::RescheduleReason::InternationalWindow,
    });
    fixture.date = "2031-09-12".into();
    let identity = fixture.id.clone();
    let mut rng = game.rng_for("test-friendly", "2031-09-09");
    ofm_core::national_team::schedule_national_team_friendlies(
        &mut game.national_teams,
        &windows,
        &mut rng,
    );
    assert_eq!(
        friendlies(&game).len(),
        1,
        "a postponed match still fills its original window"
    );
    assert_eq!(friendlies(&game)[0].id, identity);
}

/// Given a June 9 friendly overdue after its called-up players finished a June 10 club match,
/// When the live day finishes, then the friendly waits until June 13 or later and plays once.
#[test]
fn an_overdue_friendly_moves_to_a_day_the_live_loop_can_still_play() {
    let mut game = spring_career();
    game.competitions[0].fixtures = vec![Fixture {
        id: "completed-club-match".into(),
        competition_id: game.competitions[0].id.clone(),
        date: "2031-06-10".into(),
        home_team_id: "club-a".into(),
        away_team_id: "club-b".into(),
        status: FixtureStatus::Completed,
        result: Some(MatchResult {
            home_goals: 3,
            away_goals: 1,
            ..Default::default()
        }),
        ..Default::default()
    }];
    game.sync_legacy_league();
    game.clock.current_date = Utc.with_ymd_and_hms(2031, 6, 10, 12, 0, 0).unwrap();
    ofm_core::turn::finish_live_match_day(&mut game);
    assert_eq!(friendlies(&game).len(), 1);
    let friendly_day = friendlies(&game)[0].date.clone();
    assert!(
        friendly_day.as_str() >= "2031-06-13",
        "called-up players need three days after their completed club match, got {friendly_day}"
    );
    assert_eq!(game.competitions[0].fixtures[0].date, "2031-06-10");
    assert_eq!(
        game.competitions[0].fixtures[0].status,
        FixtureStatus::Completed
    );
    while game.clock.current_date.format("%Y-%m-%d").to_string() <= friendly_day {
        ofm_core::turn::process_day(&mut game);
    }
    assert_eq!(friendlies(&game)[0].status, FixtureStatus::Completed);
    assert_eq!(ofm_core::catchup::repair_stranded_fixtures(&mut game), 0);
}

/// Given an authored friendly in a snapshot that opens in a World Cup summer,
/// When world foundations stage the tournament, then the published friendly keeps its identity.
#[test]
fn staging_world_cup_foundations_keeps_a_published_friendly() {
    let mut game = spring_career();
    game.clock.current_date = Utc.with_ymd_and_hms(2030, 6, 1, 12, 0, 0).unwrap();
    game.national_teams[0].fixtures[0].date = "2030-06-09".into();
    ofm_core::world::ensure_multi_competition_foundations(&mut game);
    assert_eq!(
        friendlies(&game)
            .iter()
            .filter(|f| f.id == "published-june-friendly")
            .count(),
        1,
        "World Cup foundation staging must retain authored friendlies"
    );
}

/// Given two national squads sharing a player and two other eligible nations,
/// When a window is filled, then no player is assigned to two sides or two matches.
#[test]
fn friendly_pairings_never_double_book_a_shared_player() {
    let mut game = spring_career();
    game.national_teams[0].fixtures.clear();
    for (nation, player) in [("nt-third", "player-nt-eng"), ("nt-fourth", "other-player")] {
        let mut team = NationalTeam::new(nation.into(), nation.into(), nation.into(), None);
        team.squad_player_ids.push(player.into());
        game.national_teams.push(team);
    }
    let mut rng = game.rng_for("test-pairing", "2031-09-09");
    ofm_core::national_team::schedule_national_team_friendlies(
        &mut game.national_teams,
        &["2031-09-09".into()],
        &mut rng,
    );
    assert_eq!(
        friendlies(&game).len(),
        1,
        "three distinct available players can field only one pairing"
    );
    let mut used = std::collections::HashSet::new();
    for fixture in friendlies(&game) {
        for nation in [&fixture.home_team_id, &fixture.away_team_id] {
            let team = game
                .national_teams
                .iter()
                .find(|team| &team.id == nation)
                .unwrap();
            for player in &team.squad_player_ids {
                assert!(used.insert(player), "{player} cannot play twice");
            }
        }
    }
}
