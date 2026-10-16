//! How good the world's players are, and how that drifts over seasons (#869, slice 1).
//!
//! Measurement only: every test is `#[ignore]`d and prints its numbers, so it runs outside the PR
//! gate, in release mode like the shipped game:
//!
//! `cargo test --release -p season-harness --test measure_distribution -- --ignored --nocapture`

use season_harness::distribution::{DistributionLog, DistributionReport, distribution_report};
use season_harness::driver::{RunOptions, run_seasons};
use season_harness::worlds::WorldSpec;

const SEASONS: u32 = 5;

fn print_row(label: &str, report: &DistributionReport) {
    eprintln!(
        "{label:>10} | players {:>6} seniors {:>6} | mean ovr {:>5.1} | ovr S {:>5.1}% A {:>5.1}% | ceiling S {:>5.1}% A {:>5.1}% | <21 {:>5.1}% | wonderkids {:>5.1}%",
        report.players,
        report.seniors,
        report.mean_senior_ovr,
        report.senior_ovr_s_share * 100.0,
        report.senior_ovr_a_share * 100.0,
        report.senior_ceiling_s_share * 100.0,
        report.senior_ceiling_a_share * 100.0,
        report.youth_share * 100.0,
        report.wonderkid_share * 100.0,
    );
}

fn measure(name: &str, spec: WorldSpec) {
    let mut game = spec.build().expect("the world builds");
    eprintln!("{name}");
    print_row("opening", &distribution_report(&game));
    let mut log = DistributionLog::default();
    run_seasons(&mut game, RunOptions::seasons(SEASONS), &mut log).expect("the run completes");
    for (index, report) in log.after_each_season.iter().enumerate() {
        print_row(&format!("season {}", index + 1), report);
    }
}

/// Given the gate world on seeds 1 to 3 and the shipped world on seed 1,
/// When five seasons are played,
/// Then the S and A shares, mean rating, youth share and wonderkid share are printed per season.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn the_player_distribution_over_five_seasons() {
    for seed in 1..=3 {
        measure(&format!("gate world, seed {seed}"), WorldSpec::gate(seed));
    }
    measure("shipped world, seed 1", WorldSpec::shipped(1));
}
