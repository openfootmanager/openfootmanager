//! The development curve over many seasons (#869, slice 4).
//!
//! Measurement only: every test is `#[ignore]`d and prints its numbers, so it runs outside the PR
//! gate, in release mode like the shipped game:
//!
//! `cargo test --release -p season-harness --test measure_ageing -- --ignored --nocapture`

use season_harness::ageing::{AgeingLog, AgeingReport};
use season_harness::distribution::{DistributionLog, DistributionReport, distribution_report};
use season_harness::driver::{RunOptions, run_seasons};
use season_harness::worlds::WorldSpec;

fn print_distribution(label: &str, report: &DistributionReport) {
    eprintln!(
        "{label:>9} | mean ovr {:>5.1} | ovr S {:>4.1}% A {:>4.1}% | ceiling S {:>4.1}% A {:>4.1}% | <21 {:>4.1}% | wonderkids {:>4.1}%",
        report.mean_senior_ovr,
        report.senior_ovr_s_share * 100.0,
        report.senior_ovr_a_share * 100.0,
        report.senior_ceiling_s_share * 100.0,
        report.senior_ceiling_a_share * 100.0,
        report.youth_share * 100.0,
        report.wonderkid_share * 100.0,
    );
}

fn print_ageing(label: &str, report: &AgeingReport) {
    let bands: Vec<String> = report
        .bands
        .iter()
        .map(|band| format!("{} {:+.2} ({})", band.label, band.mean_change, band.players))
        .collect();
    eprintln!(
        "{label:>9} | top100 median age {:>4.1}, 31+ {:>4.1}%, 33+ {:>4.1}%, 35+ {:>4.1}% | top50 mean {:>5.1} | gk31+ {:>4.1}% outfield31+ {:>4.1}% | 33+ A/S {:>4.1}% of {}",
        report.top100_median_age,
        report.top100_aged_31_plus_share * 100.0,
        report.top100_aged_33_plus_share * 100.0,
        report.top100_aged_35_plus_share * 100.0,
        report.top50_mean_ovr,
        report.top100_goalkeepers_aged_31_plus_share * 100.0,
        report.top100_outfield_aged_31_plus_share * 100.0,
        report.aged_33_plus_a_or_s_share * 100.0,
        report.aged_33_plus,
    );
    eprintln!("{:>9} | change per band: {}", "", bands.join(", "));
}

fn measure(name: &str, spec: WorldSpec, seasons: u32) {
    let mut game = spec.build().expect("the world builds");
    eprintln!("{name}, {seasons} seasons");
    print_distribution("opening", &distribution_report(&game));
    let mut distribution = DistributionLog::default();
    let mut ageing = AgeingLog::starting_from(&game);
    run_seasons(
        &mut game,
        RunOptions::seasons(seasons),
        &mut (&mut distribution, &mut ageing),
    )
    .expect("the run completes");
    for (index, (shares, curve)) in distribution
        .after_each_season
        .iter()
        .zip(&ageing.after_each_season)
        .enumerate()
    {
        let label = format!("season {}", index + 1);
        print_distribution(&label, shares);
        print_ageing(&label, curve);
    }
}

/// Given the gate world on seeds 1 to 3, ten seasons each, and one seed for thirty,
/// When the seasons are played,
/// Then the tier shares, wonderkid share, per-band change and the age profile of the best are printed.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn the_development_curve_in_the_gate_worlds() {
    for seed in 1..=3 {
        measure(&format!("gate world, seed {seed}"), WorldSpec::gate(seed), 10);
    }
    measure("gate world, seed 1", WorldSpec::gate(1), 30);
}

/// Given the shipped world,
/// When ten seasons are played,
/// Then the same numbers are printed for the 440-club world.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn the_development_curve_in_the_shipped_world() {
    measure("shipped world, seed 1", WorldSpec::shipped(1), 10);
}
