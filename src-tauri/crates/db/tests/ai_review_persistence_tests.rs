//! Does what the weekly tactical review decides actually reach the save file?
//!
//! `tactics_phase` and `player_roles` are JSON columns that have round-tripped
//! since before either had a writer other than the player, so nothing here is
//! expected to need a migration. But a policy that rewrites a persisted field
//! on a schedule is the shape that has caught this project out before, and the
//! nearest thing to a proof is cheap: advance a world far enough for a review to
//! run, write it, and read the column back out with SQL.
//!
//! Deliberately raw SQL rather than a load-and-inspect. The review is
//! deterministic — same style, same squad, same form, same answer — so a reload
//! would recompute the identical settings whether or not a single byte of them
//! had been stored, and the test would pass with the columns missing entirely.

use chrono::{TimeZone, Utc};
use db::game_database::GameDatabase;
use db::game_persistence::GamePersistenceWriter;
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::{PlayStyle, TacticsPhaseSettings, Team};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::turn;

fn attrs(level: u8) -> PlayerAttributes {
    PlayerAttributes {
        pace: level,
        stamina: level,
        strength: level,
        agility: level,
        passing: level,
        shooting: level,
        tackling: level,
        dribbling: level,
        defending: level,
        positioning: level,
        vision: level,
        decisions: level,
        composure: level,
        aggression: level,
        teamwork: level,
        leadership: level,
        handling: level,
        reflexes: level,
        aerial: level,
    }
}

fn make_player(
    id: &str,
    team_id: &str,
    position: Position,
    attributes: PlayerAttributes,
) -> Player {
    let mut player = Player::new(
        id.to_string(),
        id.to_string(),
        id.to_string(),
        "1998-01-01".to_string(),
        "England".to_string(),
        position,
        attributes,
    );
    player.team_id = Some(team_id.to_string());
    player.condition = 100;
    player.morale = 70;
    player
}

fn make_squad(team_id: &str) -> Vec<Player> {
    let mut squad = vec![make_player(
        &format!("{team_id}_gk"),
        team_id,
        Position::Goalkeeper,
        attrs(60),
    )];
    for i in 0..5 {
        squad.push(make_player(
            &format!("{team_id}_def{i}"),
            team_id,
            Position::Defender,
            attrs(60),
        ));
    }
    for i in 0..5 {
        squad.push(make_player(
            &format!("{team_id}_mid{i}"),
            team_id,
            Position::Midfielder,
            attrs(60),
        ));
    }
    // One player with an unmistakable shape, so the role map has something in it
    // to persist. Shooting, positioning and composure are a poacher's three, and
    // a Counter club asks for poachers.
    let mut poacher = attrs(40);
    poacher.shooting = 95;
    poacher.positioning = 95;
    poacher.composure = 95;
    squad.push(make_player(
        &format!("{team_id}_fwd0"),
        team_id,
        Position::Forward,
        poacher,
    ));
    for i in 1..3 {
        squad.push(make_player(
            &format!("{team_id}_fwd{i}"),
            team_id,
            Position::Forward,
            attrs(60),
        ));
    }
    squad
}

fn make_team(id: &str, play_style: PlayStyle) -> Team {
    let mut team = Team::new(
        id.to_string(),
        format!("Club {id}"),
        id[..3].to_string(),
        "England".to_string(),
        "London".to_string(),
        "Ground".to_string(),
        20_000,
    );
    team.play_style = play_style;
    team.tactics_phase = TacticsPhaseSettings::default();
    team.player_roles.clear();
    team
}

/// Two clubs, neither of which has ever had a tactical setting written to it —
/// the shape every save made before the identity work loads in.
fn make_world() -> Game {
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap());
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("home-fc".to_string());

    let teams = vec![
        make_team("home-fc", PlayStyle::Balanced),
        make_team("away-fc", PlayStyle::Counter),
    ];
    let mut players = make_squad("home-fc");
    players.extend(make_squad("away-fc"));

    let participants = vec!["home-fc".to_string(), "away-fc".to_string()];
    let mut league = League::new(
        "league".to_string(),
        "Test League".to_string(),
        2026,
        &participants,
    );
    league.standings = participants
        .iter()
        .cloned()
        .map(StandingEntry::new)
        .collect();
    league.fixtures.push(Fixture {
        id: "fix1".to_string(),
        competition_id: "league".to_string(),
        matchday: 1,
        date: "2026-08-08".to_string(),
        home_team_id: "home-fc".to_string(),
        away_team_id: "away-fc".to_string(),
        competition: FixtureCompetition::League,
        status: FixtureStatus::Scheduled,
        result: None,
    });

    let mut game = Game::new(clock, manager, teams, players, vec![], vec![]);
    game.league = Some(league.clone());
    game.competitions = vec![league];
    game
}

#[test]
fn what_the_weekly_review_decides_is_written_into_the_save_file() {
    let mut game = make_world();
    // Long enough that every club's review day has come round at least once.
    for _ in 0..10 {
        turn::process_day(&mut game);
    }

    let reviewed = game
        .teams
        .iter()
        .find(|team| team.id == "away-fc")
        .expect("the AI club is still in the world");
    assert_ne!(
        reviewed.tactics_phase,
        TacticsPhaseSettings::default(),
        "the review never ran, so this test cannot say anything about persistence"
    );
    assert!(
        !reviewed.player_roles.is_empty(),
        "the review named nobody, so this test cannot say anything about persistence"
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("save.db");
    let db = GameDatabase::open(&path).unwrap();
    GamePersistenceWriter::write_game(&db, &game, "save-1", "Test Save").unwrap();

    let (tactics_json, roles_json): (String, String) = db
        .conn()
        .query_row(
            "SELECT COALESCE(tactics_phase_json, '{}'), COALESCE(player_roles_json, '{}')
             FROM teams WHERE id = ?1",
            rusqlite::params!["away-fc"],
            |row: &rusqlite::Row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("the AI club has a row in the written save");

    let stored: TacticsPhaseSettings = serde_json::from_str(&tactics_json).unwrap();
    assert_eq!(
        stored, reviewed.tactics_phase,
        "the review's settings did not survive the write: column held {tactics_json}"
    );

    let stored_roles: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(&roles_json).unwrap();
    assert_eq!(
        stored_roles.len(),
        reviewed.player_roles.len(),
        "the review's role assignments did not survive the write: column held {roles_json}"
    );
}
