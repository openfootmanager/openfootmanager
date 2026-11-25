//! Where morale sits in the shipped world, as the match engine reads it (#1213): every club's
//! players carry a morale from kick-off, and the engine turns it into a factor on their rating
//! (`1 + 0.03 * (morale - 50) / 50`, the bridge's table). If the world's morale is centred on 50
//! the factor's neutral is the world's; if it is centred above or below, every match is nudged
//! the same way. And if morale follows results, a club that wins raises its morale and so wins
//! more, which is the loop most likely to move a band.
//!
//! Measurement only: the test is `#[ignore]`d and prints its numbers, so it runs outside the PR
//! gate, in release mode:
//!
//! `cargo test --release -p season-harness --test measure_morale -- --ignored --nocapture`

use std::collections::HashMap;

use domain::league::CompetitionType;
use ofm_core::game::Game;
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::worlds::{Scope, WorldSpec};

const SEEDS: [u64; 3] = [1, 2, 3];

/// The bridge's table: the factor a morale gives (`ofm_core::turn::squad::morale`).
fn factor(morale: f64) -> f64 {
    1.0 + 0.03 * (morale - 50.0) / 50.0
}

fn mean_and_spread(values: &[f64]) -> (f64, f64) {
    let n = values.len().max(1) as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    (mean, variance.sqrt())
}

fn correlation(pairs: &[(f64, f64)]) -> f64 {
    let n = pairs.len().max(1) as f64;
    let (mx, my) = (
        pairs.iter().map(|p| p.0).sum::<f64>() / n,
        pairs.iter().map(|p| p.1).sum::<f64>() / n,
    );
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in pairs {
        sxy += (x - mx) * (y - my);
        sxx += (x - mx).powi(2);
        syy += (y - my).powi(2);
    }
    if sxx == 0.0 || syy == 0.0 {
        0.0
    } else {
        sxy / (sxx.sqrt() * syy.sqrt())
    }
}

/// The morale of every club's players in the senior squads, by club.
fn morale_by_club(game: &Game) -> HashMap<String, Vec<f64>> {
    let mut by_club: HashMap<String, Vec<f64>> = HashMap::new();
    for player in &game.players {
        if let Some(club) = &player.team_id {
            by_club
                .entry(club.clone())
                .or_default()
                .push(f64::from(player.morale));
        }
    }
    by_club
}

fn report(label: &str, seed: u64, game: &Game) {
    let by_club = morale_by_club(game);
    let all: Vec<f64> = by_club.values().flatten().copied().collect();
    let (mean, spread) = mean_and_spread(&all);
    let factors: Vec<f64> = all.iter().map(|m| factor(*m)).collect();
    let (factor_mean, factor_spread) = mean_and_spread(&factors);
    let club_means: Vec<f64> = by_club
        .values()
        .map(|morales| mean_and_spread(morales).0)
        .collect();
    let (club_mean, club_spread) = mean_and_spread(&club_means);
    let reputation: HashMap<&str, f64> = game
        .teams
        .iter()
        .map(|team| (team.id.as_str(), f64::from(team.reputation)))
        .collect();
    let against_reputation: Vec<(f64, f64)> = by_club
        .iter()
        .filter_map(|(club, morales)| {
            Some((mean_and_spread(morales).0, *reputation.get(club.as_str())?))
        })
        .collect();
    println!(
        "MORALE {label} seed {seed}: players {} in {} clubs | morale mean {:.2} spread {:.2} | factor mean {:.5} spread {:.5} | club means: mean {:.2} spread {:.2} | correlation of club morale with club reputation {:.3}",
        all.len(),
        by_club.len(),
        mean,
        spread,
        factor_mean,
        factor_spread,
        club_mean,
        club_spread,
        correlation(&against_reputation),
    );
}

/// Reads the table of every league at the end of a season, against its clubs' morale: does morale
/// track the table, once results have fed it?
#[derive(Default)]
struct TableAgainstMorale {
    pairs: Vec<(f64, f64)>,
}

impl SeasonObserver for TableAgainstMorale {
    fn on_before_rollover(&mut self, game: &Game, _season: u32) {
        let by_club = morale_by_club(game);
        for competition in game
            .competitions
            .iter()
            .filter(|competition| competition.kind == CompetitionType::League)
        {
            let points: Vec<f64> = competition
                .standings
                .iter()
                .map(|entry| f64::from(entry.points))
                .collect();
            let (league_mean, _) = mean_and_spread(&points);
            for entry in &competition.standings {
                if let Some(morales) = by_club.get(&entry.team_id) {
                    self.pairs.push((
                        mean_and_spread(morales).0,
                        f64::from(entry.points) - league_mean,
                    ));
                }
            }
        }
    }
}

/// Given the shipped world on three seeds,
/// When it is read at kick-off and again at the end of its first season,
/// Then lines report the distribution of morale and of the factor it gives, and how club morale
/// goes with club reputation (at kick-off) and with the table (at the end of the season).
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn morale_in_the_shipped_world() {
    for seed in SEEDS {
        let spec = WorldSpec::shipped(seed).with_scope(Scope::Everything);
        let mut game = spec.build().expect("the world builds");
        report("at kick-off", seed, &game);
        let mut table = TableAgainstMorale::default();
        run_seasons(&mut game, RunOptions::seasons(1), &mut table).expect("the season plays");
        println!(
            "MORALE_TABLE seed {seed}: clubs {} | correlation of end-of-season club morale with points above the league mean {:.3}",
            table.pairs.len(),
            correlation(&table.pairs)
        );
    }
}
