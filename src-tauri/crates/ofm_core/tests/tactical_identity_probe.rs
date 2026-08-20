//! Identity-vs-identity probe: does a club's tactical blueprint decide matches
//! it should be deciding, and does any blueprint simply win?
//!
//! Run it explicitly — it simulates thousands of matches:
//!
//! ```text
//! cargo test -p ofm_core --test tactical_identity_probe --release -- --ignored --nocapture
//! ```
//!
//! Two clubs, identical in every respect except their play style and the
//! blueprint `ai_tactics::blueprint_for` derives from it. Every pairing is
//! played both ways round so home advantage cancels exactly, and squad condition
//! is pinned at 100 between matches so the result is about tactics and nothing
//! else.
//!
//! # What a bad number looks like
//!
//! Read the win-rate column. Squads are equal, so a blueprint that is merely
//! *different* should land near 50%. A blueprint consistently above ~55% is not
//! a style, it is a free win handed to whichever clubs the world generator
//! happened to roll it for — and since `play_style` is assigned at random and
//! shown to nobody, that advantage would be invisible.
//!
//! This is a live risk rather than a theoretical one: the engine prices four of
//! the nine dials one-sidedly (`ai_tactics` documents which, and what the
//! sweep measured). The blueprints ration those dials rather than budget them,
//! and this probe is what says whether rationing was enough.

use chrono::{TimeZone, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};
use domain::team::{PlayStyle, Team};
use ofm_core::ai_tactics::blueprint_for;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::turn;

/// Matches per ordered pairing. Each unordered pairing therefore gets twice
/// this, half at home and half away. Override with `OFM_PROBE_MATCHES`.
fn matches_per_pairing() -> usize {
    std::env::var("OFM_PROBE_MATCHES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(400)
}

const EVERY_STYLE: [PlayStyle; 6] = [
    PlayStyle::Balanced,
    PlayStyle::Attacking,
    PlayStyle::Defensive,
    PlayStyle::Possession,
    PlayStyle::Counter,
    PlayStyle::HighPress,
];

const XI_QUOTA: [(Position, usize); 4] = [
    (Position::Goalkeeper, 1),
    (Position::Defender, 4),
    (Position::Midfielder, 4),
    (Position::Forward, 2),
];

fn style_name(style: &PlayStyle) -> &'static str {
    match style {
        PlayStyle::Balanced => "Balanced",
        PlayStyle::Attacking => "Attacking",
        PlayStyle::Defensive => "Defensive",
        PlayStyle::Possession => "Possession",
        PlayStyle::Counter => "Counter",
        PlayStyle::HighPress => "HighPress",
    }
}

fn attrs(level: u8, position: &Position) -> PlayerAttributes {
    let gk = *position == Position::Goalkeeper;
    PlayerAttributes {
        pace: level,
        stamina: level,
        strength: level,
        agility: level,
        passing: level,
        shooting: if gk { 20 } else { level },
        tackling: if gk { 20 } else { level },
        dribbling: level,
        defending: level,
        positioning: level,
        vision: level,
        decisions: level,
        composure: level,
        aggression: level,
        teamwork: level,
        leadership: level,
        handling: if gk { level } else { 30 },
        reflexes: if gk { level } else { 30 },
        aerial: level,
    }
}

/// Sixteen players: a starting eleven and five spares, every one of them rated
/// the same so neither club can win on quality.
fn make_squad(team_id: &str) -> Vec<Player> {
    let mut players = Vec::new();
    for (tier, count_mult) in [("first", 1usize), ("sub", 1usize)] {
        for (position, count) in XI_QUOTA {
            for i in 0..(count * count_mult) {
                let id = format!("{team_id}_{tier}_{position:?}{i}");
                let mut player = Player::new(
                    id.clone(),
                    id.clone(),
                    id.clone(),
                    "1998-01-01".to_string(),
                    "England".to_string(),
                    position.clone(),
                    attrs(70, &position),
                );
                player.team_id = Some(team_id.to_string());
                player.morale = 70;
                player.condition = 100;
                player.fitness = 80;
                players.push(player);
            }
        }
    }
    players
}

fn make_club(id: &str, style: &PlayStyle) -> Team {
    let mut team = Team::new(
        id.to_string(),
        format!("{id} FC"),
        id[..3].to_string(),
        "England".to_string(),
        "London".to_string(),
        "Stadium".to_string(),
        30_000,
    );
    team.play_style = style.clone();
    team.tactics_phase = blueprint_for(style);
    // Equal reputation, so the lineup picker treats both managers alike and no
    // rotation difference creeps into the comparison.
    team.reputation = 500;
    team
}

/// One world, one fixture per day, `matches` of them — home always `home_style`.
fn build_world(home_style: &PlayStyle, away_style: &PlayStyle, matches: usize) -> Game {
    let start = Utc.with_ymd_and_hms(2025, 6, 16, 12, 0, 0).unwrap();
    let (home_id, away_id) = ("home", "away");

    let mut players = make_squad(home_id);
    players.extend(make_squad(away_id));

    let fixtures: Vec<Fixture> = (0..matches)
        .map(|day| Fixture {
            id: format!("m{day}"),
            matchday: day as u32 + 1,
            date: (start + chrono::Duration::days(day as i64))
                .format("%Y-%m-%d")
                .to_string(),
            home_team_id: home_id.to_string(),
            away_team_id: away_id.to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
            ..Default::default()
        })
        .collect();

    // Nobody's club: both sides must go through the AI selection branch, or the
    // home side would be picked by the saved-XI branch instead and the two would
    // not be comparable.
    let manager = Manager::new(
        "mgr1".to_string(),
        "Probe".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );

    let league = League {
        id: "league1".to_string(),
        name: "Probe League".to_string(),
        season: 1,
        fixtures,
        standings: [home_id, away_id]
            .iter()
            .map(|id| StandingEntry::new(id.to_string()))
            .collect(),
        transfer_log: vec![],
        transfer_rumours: vec![],
        ..Default::default()
    };

    let mut game = Game::new(
        GameClock::new(start),
        manager,
        vec![
            make_club(home_id, home_style),
            make_club(away_id, away_style),
        ],
        players,
        vec![],
        vec![],
    );
    game.league = Some(league);
    game
}

#[derive(Default, Clone, Copy)]
struct Record {
    played: usize,
    home_wins: usize,
    draws: usize,
    goals_for: usize,
    goals_against: usize,
}

/// Play `matches` fixtures between the two styles, home side fixed.
///
/// Condition is reset to 100 after every match. Without it the probe would be
/// measuring the fatigue economy on top of the tactics, and a blueprint whose
/// dials happen to tire a squad faster would look like a worse *tactic*.
fn play(home_style: &PlayStyle, away_style: &PlayStyle, matches: usize) -> Record {
    let mut game = build_world(home_style, away_style, matches);
    let mut record = Record::default();

    for _ in 0..matches {
        turn::process_day(&mut game);
        for player in game.players.iter_mut() {
            player.condition = 100;
            player.injury = None;
        }
    }

    for fixture in game.league.as_ref().unwrap().fixtures.iter() {
        let Some(result) = &fixture.result else {
            continue;
        };
        record.played += 1;
        record.goals_for += result.home_goals as usize;
        record.goals_against += result.away_goals as usize;
        match result.home_goals.cmp(&result.away_goals) {
            std::cmp::Ordering::Greater => record.home_wins += 1,
            std::cmp::Ordering::Equal => record.draws += 1,
            std::cmp::Ordering::Less => {}
        }
    }

    record
}

#[test]
#[ignore = "simulates thousands of matches; run explicitly"]
fn report_identity_versus_identity() {
    let matches = matches_per_pairing();
    println!();
    println!(
        "Blueprint A/B: identical squads, {matches} matches per ordered pairing, \
         home and away."
    );
    println!("Condition pinned at 100 — this is tactics only.");
    println!();

    // style → (wins, draws, played) accumulated across every pairing, counting
    // each style's own results whether it was at home or away.
    let mut totals: Vec<(usize, usize, usize, usize, usize)> = vec![(0, 0, 0, 0, 0); 6];

    println!(
        "{:<12} {:<12} {:>8} {:>8} {:>8}",
        "home", "away", "H win%", "goals H", "goals A"
    );
    println!("{}", "-".repeat(52));

    for (h, home_style) in EVERY_STYLE.iter().enumerate() {
        for (a, away_style) in EVERY_STYLE.iter().enumerate() {
            if h == a {
                continue;
            }
            let record = play(home_style, away_style, matches);
            let played = record.played.max(1);
            println!(
                "{:<12} {:<12} {:>7.1}% {:>8.2} {:>8.2}",
                style_name(home_style),
                style_name(away_style),
                100.0 * record.home_wins as f64 / played as f64,
                record.goals_for as f64 / played as f64,
                record.goals_against as f64 / played as f64,
            );

            let away_wins = played - record.home_wins - record.draws;
            totals[h].0 += record.home_wins;
            totals[h].1 += record.draws;
            totals[h].2 += played;
            totals[h].3 += record.goals_for;
            totals[h].4 += record.goals_against;
            totals[a].0 += away_wins;
            totals[a].1 += record.draws;
            totals[a].2 += played;
            totals[a].3 += record.goals_against;
            totals[a].4 += record.goals_for;
        }
    }

    println!();
    println!(
        "{:<12} {:>8} {:>8} {:>10} {:>10}",
        "style", "win%", "draw%", "goals for", "against"
    );
    println!("{}", "-".repeat(52));
    for (index, style) in EVERY_STYLE.iter().enumerate() {
        let (wins, draws, played, goals_for, goals_against) = totals[index];
        let played = played.max(1);
        println!(
            "{:<12} {:>7.1}% {:>7.1}% {:>10.2} {:>10.2}",
            style_name(style),
            100.0 * wins as f64 / played as f64,
            100.0 * draws as f64 / played as f64,
            goals_for as f64 / played as f64,
            goals_against as f64 / played as f64,
        );
    }
    println!("{}", "-".repeat(52));
    println!(
        "Squads are equal and every pairing is played both ways, so a style far \
         from 50% is winning on its blueprint alone."
    );
    println!();
}
