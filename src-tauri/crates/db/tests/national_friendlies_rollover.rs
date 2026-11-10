use chrono::{TimeZone, Utc};
use db::save_manager::SaveManager;
use domain::{
    league::{Fixture, FixtureCompetition, FixtureStatus, League, MatchResult},
    manager::Manager,
    national_team::NationalTeam,
    player::{Player, PlayerAttributes, Position},
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
    assert_reload_changes_nothing(&game);
    game.clock.current_date = Utc.with_ymd_and_hms(2031, 6, 9, 12, 0, 0).unwrap();
    let before: Vec<_> = game
        .players
        .iter()
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
