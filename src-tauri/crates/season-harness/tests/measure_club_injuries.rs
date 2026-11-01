//! Club-match injuries per club per season on the shipped world, and the match fitness clubs'
//! players carry: the measurement behind `CLUB_MATCH_MEAN_INJURY_MULTIPLIER` in
//! `ofm_core::turn::squad`.
//!
//! `OUTCOME_SEED=<n> cargo test --release -p season-harness --test measure_club_injuries -- --ignored --nocapture`
//!
//! To measure the constant again, run seeds 1 and 2 twice. First, with the constant set to 1.0,
//! the injuries the fitness table gives raw. Second, with `injury_risk` set to 1.0 in
//! `to_engine_player`, the injuries a fitness-blind roll gives. The constant is the first total
//! over the second. Do it when the table, the engine's injury probability, or anything that moves
//! players' fitness (training, condition, recovery) changes, since the fitness band shares printed
//! here are what the constant was measured against.

use std::collections::HashSet;

use ofm_core::game::Game;
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::worlds::{Scope, WorldSpec};

/// The fitness table's bands: below 30, 30 to 49, 50 to 69, 70 to 89, and 90 or more.
fn band(fitness: u8) -> usize {
    match fitness {
        0..=29 => 0,
        30..=49 => 1,
        50..=69 => 2,
        70..=89 => 3,
        _ => 4,
    }
}

#[derive(Default)]
struct Watch {
    matches: u64,
    injury_events: u64,
    clubs: HashSet<String>,
    player_days: u64,
    bands: [u64; 5],
}

impl SeasonObserver for Watch {
    fn on_day_end(&mut self, game: &Game, _season: u32, _day: u32) {
        for player in game
            .players
            .iter()
            .filter(|player| player.team_id.is_some())
        {
            self.player_days += 1;
            self.bands[band(player.fitness)] += 1;
        }
    }

    fn on_before_rollover(&mut self, game: &Game, _season: u32) {
        for fixture in game
            .competitions
            .iter()
            .flat_map(|competition| &competition.fixtures)
        {
            let Some(report) = fixture
                .result
                .as_ref()
                .and_then(|result| result.report.as_ref())
            else {
                continue;
            };
            self.matches += 1;
            self.clubs.insert(fixture.home_team_id.clone());
            self.clubs.insert(fixture.away_team_id.clone());
            self.injury_events += report
                .events
                .iter()
                .filter(|event| event.event_type == "Injury")
                .count() as u64;
        }
    }
}

/// Given the shipped world with every competition played,
/// When one season is played,
/// Then one line reports its club-match injuries and the fitness band shares of clubs' players.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn club_match_injuries_and_fitness_of_the_shipped_world() {
    let seeds: Vec<u64> = match std::env::var("OUTCOME_SEED") {
        Ok(seed) => vec![seed.parse().expect("OUTCOME_SEED is a number")],
        Err(_) => vec![1, 2],
    };
    for seed in seeds {
        let spec = WorldSpec::shipped(seed).with_scope(Scope::Everything);
        let mut game = spec.build().expect("the world builds");
        let mut watch = Watch::default();
        run_seasons(&mut game, RunOptions::seasons(1), &mut watch).expect("the season plays");
        let days = watch.player_days.max(1) as f64;
        let share = |count: u64| 100.0 * count as f64 / days;
        println!(
            "CLUB_INJURIES seed {seed}: engine matches {} | injury events {} | per match {:.4} | clubs {} | per club per season {:.3} | fitness <30 {:.2}% 30-49 {:.2}% 50-69 {:.2}% 70-89 {:.2}% 90+ {:.2}%",
            watch.matches,
            watch.injury_events,
            watch.injury_events as f64 / watch.matches.max(1) as f64,
            watch.clubs.len(),
            watch.injury_events as f64 / watch.clubs.len().max(1) as f64,
            share(watch.bands[0]),
            share(watch.bands[1]),
            share(watch.bands[2]),
            share(watch.bands[3]),
            share(watch.bands[4]),
        );
    }
}
