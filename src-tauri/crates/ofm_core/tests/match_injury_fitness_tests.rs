//! A club match injures its players by their match fitness, through the path the user's own live
//! match takes: the same step table that drives training and international-duty injuries.

use chrono::{TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::Team;
use engine::{EventType, Side};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::live_match_manager::{self, MatchMode};

const MATCHES: u64 = 300;
const EXHAUSTED: u8 = 20;
const FIT: u8 = 95;

fn attributes() -> PlayerAttributes {
    PlayerAttributes {
        pace: 65,
        stamina: 65,
        strength: 65,
        agility: 65,
        passing: 65,
        shooting: 65,
        tackling: 65,
        dribbling: 65,
        defending: 65,
        positioning: 65,
        vision: 65,
        decisions: 65,
        composure: 65,
        aggression: 65,
        teamwork: 65,
        leadership: 65,
        handling: 65,
        reflexes: 65,
        aerial: 65,
    }
}

/// Eighteen players, every one at match fitness `fitness`.
fn squad(team_id: &str, fitness: u8) -> Vec<Player> {
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
                attributes(),
            );
            player.team_id = Some(team_id.to_string());
            player.condition = 100;
            player.fitness = fitness;
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

/// The user's club alpha, exhausted, at home to bravo, fit, with seed `seed`.
fn game(seed: u64) -> Game {
    let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    let mut manager = Manager::new(
        "mgr".to_string(),
        "Probe".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("alpha".to_string());
    let mut players = squad("alpha", EXHAUSTED);
    players.extend(squad("bravo", FIT));
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
    game.seed = seed;
    game
}

/// Fouls and injuries each side suffered over every match, home first.
#[derive(Default)]
struct Suffered {
    fouls: [u32; 2],
    injuries: [u32; 2],
}

impl Suffered {
    /// Injuries per foul suffered, home over away.
    fn injury_rate_ratio(&self) -> f64 {
        let rate = |side: usize| f64::from(self.injuries[side]) / f64::from(self.fouls[side]);
        rate(0) / rate(1)
    }
}

fn index(side: Side) -> usize {
    match side {
        Side::Home => 0,
        Side::Away => 1,
    }
}

/// Given the user's exhausted club (match fitness 20) at home to a fit one (95), otherwise alike,
/// When their live matches are played out, as the user's own match is,
/// Then each foul the exhausted side suffers injures its player about 3.0 / 0.7 ≈ 4.3 times as
/// often as a foul on the fit side: the fitness table's bottom step against its top.
#[test]
fn an_exhausted_side_is_injured_more_often_in_a_club_match_than_a_fit_one() {
    let mut suffered = Suffered::default();
    for seed in 0..MATCHES {
        let mut session =
            live_match_manager::create_live_match(&game(seed), 0, MatchMode::Live, false)
                .expect("the fixture starts");
        session.run_to_completion();
        for event in session.snapshot().events {
            match event.event_type {
                // A foul is recorded against the side that commits it.
                EventType::Foul => suffered.fouls[1 - index(event.side)] += 1,
                // An injury is recorded for the side whose player is hurt.
                EventType::Injury => suffered.injuries[index(event.side)] += 1,
                _ => {}
            }
        }
    }
    let ratio = suffered.injury_rate_ratio();
    assert!(
        (3.0..=6.0).contains(&ratio),
        "injuries per foul suffered, exhausted over fit, was {ratio:.2} \
         (fouls {:?}, injuries {:?})",
        suffered.fouls,
        suffered.injuries
    );
}
