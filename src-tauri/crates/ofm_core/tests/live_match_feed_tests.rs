use chrono::{TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::Team;
use engine::EventType;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::live_match_manager::{self, MatchMode};

fn attributes(position: &Position) -> PlayerAttributes {
    let keeper = *position == Position::Goalkeeper;
    PlayerAttributes {
        pace: 65,
        stamina: 40,
        strength: 65,
        agility: 65,
        passing: 65,
        shooting: if keeper { 20 } else { 65 },
        tackling: if keeper { 20 } else { 65 },
        dribbling: 65,
        defending: 65,
        positioning: 65,
        vision: 65,
        decisions: 65,
        composure: 65,
        aggression: 65,
        teamwork: 65,
        leadership: 65,
        handling: if keeper { 65 } else { 20 },
        reflexes: if keeper { 65 } else { 20 },
        aerial: 65,
    }
}

/// Eighteen players with little stamina, so both managers have spent legs to replace.
fn squad(team_id: &str) -> Vec<Player> {
    let mut players = Vec::new();
    for (position, count) in [
        (Position::Goalkeeper, 2),
        (Position::Defender, 6),
        (Position::Midfielder, 6),
        (Position::Forward, 4),
    ] {
        for index in 0..count {
            let id = format!("{team_id}_{position:?}{index}");
            let mut player = Player::new(
                id.clone(),
                id.clone(),
                id,
                "1995-01-01".to_string(),
                "England".to_string(),
                position.clone(),
                attributes(&position),
            );
            player.team_id = Some(team_id.to_string());
            player.condition = 100;
            players.push(player);
        }
    }
    players
}

fn club(id: &str) -> Team {
    Team::new(
        id.to_string(),
        format!("{id} FC"),
        id[..3].to_uppercase(),
        "England".to_string(),
        "London".to_string(),
        "Stadium".to_string(),
        30_000,
    )
}

/// A fixture between two clubs nobody in the game manages, so both dugouts are AI.
fn game_between_two_ai_clubs() -> Game {
    let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    let manager = Manager::new(
        "mgr".to_string(),
        "Probe".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let mut players = squad("alpha");
    players.extend(squad("bravo"));
    let league = League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "fix1".to_string(),
            matchday: 1,
            date: "2025-06-15".to_string(),
            home_team_id: "alpha".to_string(),
            away_team_id: "bravo".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
            ..Default::default()
        }],
        standings: vec![
            StandingEntry::new("alpha".to_string()),
            StandingEntry::new("bravo".to_string()),
        ],
        ..Default::default()
    };
    let mut game = Game::new(
        GameClock::new(date),
        manager,
        vec![club("alpha"), club("bravo")],
        players,
        vec![],
        vec![],
    );
    game.league = Some(league);
    game
}

/// Given a match between two AI-managed clubs with tired legs on both benches' doorsteps,
/// When it is played to the end a minute at a time, as a watched match is,
/// Then every substitution the managers made appears in some minute's results, so the live feed
/// built from those results shows the AI's changes as well as the user's.
#[test]
fn every_substitution_an_ai_manager_makes_reaches_the_minute_results() {
    let game = game_between_two_ai_clubs();
    let mut session =
        live_match_manager::create_live_match(&game, 0, MatchMode::Spectator, false).unwrap();

    let mut reported = 0;
    while !session.is_finished() {
        reported += session
            .step()
            .events
            .iter()
            .filter(|event| event.event_type == EventType::Substitution)
            .count();
    }

    let made = session.snapshot().substitutions.len();
    assert!(made > 0, "the fixture must make the managers substitute");
    assert_eq!(reported, made);
}
