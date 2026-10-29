//! Where the S and A players play, by club reputation, over many seasons (#869, slice 5).
//!
//! Measurement only: every test is `#[ignore]`d and prints its numbers, so it runs outside the PR
//! gate, in release mode like the shipped game:
//!
//! `cargo test --release -p season-harness --test measure_reputation -- --ignored --nocapture`
//!
//! The shipped world is read at its opening and after seasons 5, 10 and 20; the gate world at its
//! opening and after season 10.

use season_harness::driver::{RunOptions, run_seasons};
use season_harness::reputation::{ReputationLog, ReputationReport, reputation_report};
use season_harness::worlds::WorldSpec;

fn print_report(label: &str, report: &ReputationReport) {
    eprintln!("{label}");
    eprintln!("  fifth | clubs | reputation  | S players | A players | first-team ovr");
    for (index, group) in report.quintiles.iter().enumerate().rev() {
        let (low, high) = report.reputation_range[index];
        eprintln!(
            "  {:>5} | {:>5} | {:>4} - {:>4} | {:>8.1}% | {:>8.1}% | {:>6.1}",
            if index == 4 {
                "top".to_string()
            } else {
                format!("{}", 5 - index)
            },
            group.clubs,
            low,
            high,
            group.s_share * 100.0,
            group.a_share * 100.0,
            group.first_team_ovr,
        );
    }
    eprintln!(
        "  no club |       |             | {:>8.1}% | {:>8.1}% |",
        report.unattached.s_share * 100.0,
        report.unattached.a_share * 100.0,
    );
    for (tier, group) in &report.tiers {
        eprintln!(
            "  tier {tier:>2} | {:>5} clubs | S {:>5.1}% A {:>5.1}% | first-team ovr {:>5.1}",
            group.clubs,
            group.s_share * 100.0,
            group.a_share * 100.0,
            group.first_team_ovr,
        );
    }
}

fn measure(name: &str, spec: WorldSpec, read_after: &[u32]) {
    let mut game = spec.build().expect("the world builds");
    eprintln!("== {name}");
    print_report("opening", &reputation_report(&game));
    let seasons = read_after.iter().copied().max().unwrap_or(0);
    let mut log = ReputationLog::default();
    run_seasons(&mut game, RunOptions::seasons(seasons), &mut log).expect("the run completes");
    for &season in read_after {
        print_report(
            &format!("after season {season}"),
            &log.after_each_season[season as usize - 1],
        );
    }
}

/// Given the gate world on seed 1 and the shipped world on seed 1,
/// When they are played for ten and twenty seasons,
/// Then the share of S and A players per reputation fifth is printed at the opening and later.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn the_players_by_club_reputation_over_many_seasons() {
    measure("gate world, seed 1", WorldSpec::gate(1), &[10]);
    measure("shipped world, seed 1", WorldSpec::shipped(1), &[5, 10, 20]);
}
