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

/// The seniors of `game` by single year of age: how many, their mean rating and mean ceiling.
fn print_age_profile(label: &str, game: &ofm_core::game::Game) {
    use std::collections::BTreeMap;
    let today = game.clock.current_date.date_naive();
    let mut by_age: BTreeMap<i32, (usize, f64, f64)> = BTreeMap::new();
    for player in game.players.iter().filter(|player| !player.retired) {
        let age = domain::age::age_on_or_unknown(today, &player.date_of_birth);
        let entry = by_age.entry(age.clamp(16, 40)).or_default();
        entry.0 += 1;
        entry.1 += f64::from(player.ovr);
        entry.2 += f64::from(player.potential);
    }
    let cells: Vec<String> = by_age
        .iter()
        .map(|(age, (count, ovr, potential))| {
            let n = *count as f64;
            format!("{age}:{count}/{:.1}/{:.1}", ovr / n, potential / n)
        })
        .collect();
    eprintln!("{label:>9} | age:count/ovr/ceiling  {}", cells.join(" "));
}

fn measure(name: &str, spec: WorldSpec, seasons: u32) {
    let mut game = spec.build().expect("the world builds");
    eprintln!("{name}, {seasons} seasons");
    print_distribution("opening", &distribution_report(&game));
    print_age_profile("opening", &game);
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
    print_age_profile("final", &game);
}

/// Read a long gate run against this: the 80-club gate world runs 0.1 to 0.6 points under the S and A
/// bands (the shipped world is in band), with a thinner top, and its mean rating slides about 0.02
/// a season. The maintainer accepted that rather than retune the small world.
/// Given the gate world on seeds 1 to 3, ten seasons each, and one seed for thirty,
/// When the seasons are played,
/// Then the tier shares, wonderkid share, per-band change and the age profile of the best are printed.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn the_development_curve_in_the_gate_worlds() {
    for seed in 1..=3 {
        measure(
            &format!("gate world, seed {seed}"),
            WorldSpec::gate(seed),
            10,
        );
    }
    measure("gate world, seed 1", WorldSpec::gate(1), 30);
}

/// Given the shipped world,
/// When twenty seasons are played,
/// Then the same numbers are printed for the 440-club world.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn the_development_curve_in_the_shipped_world() {
    measure("shipped world, seed 1", WorldSpec::shipped(1), 20);
}
