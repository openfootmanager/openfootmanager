//! How good the world's staff are, role by role, and how that drifts over seasons: where the
//! per-role tier cut-offs (the top 2.5%, 11.5% and 51.5% of a role) fall.
//!
//! Measurement only: the test is `#[ignore]`d and prints its numbers, so it runs outside the PR
//! gate, in release mode like the shipped game:
//!
//! `cargo test --release -p season-harness --test measure_staff_distribution -- --ignored --nocapture`

use season_harness::driver::{RunOptions, run_seasons};
use season_harness::staff_distribution::{
    StaffDistributionLog, StaffDistributionReport, staff_distribution_report,
};
use season_harness::worlds::WorldSpec;

const SEASONS: u32 = 20;
/// The maintainer's tiers (3 Nov): S the top 2.5% of a role, A the next 9%, B the next 40%.
const TOP_SHARES: [(&str, f64); 3] = [("S", 0.025), ("A", 0.115), ("B", 0.515)];

fn print_report(label: &str, report: &StaffDistributionReport) {
    for role in &report.roles {
        let clubs = &role.at_clubs;
        let cuts: Vec<String> = TOP_SHARES
            .iter()
            .map(
                |(tier, share)| match clubs.lowest_rating_of_the_top(*share) {
                    Some(cut) => format!(
                        "{tier} {cut} ({:.1}% at or above)",
                        clubs.share_at_or_above(cut) * 100.0
                    ),
                    None => format!("{tier} -"),
                },
            )
            .collect();
        eprintln!(
            "{label:>10} | {:<16} | clubs {:>4} mean {:>5.1} sd {:>4.1} | {} | retired-player scouts {:>3} | market {:>3} mean {:>5.1}",
            format!("{:?}", role.role),
            clubs.count(),
            clubs.mean(),
            clubs.sd(),
            cuts.join(", "),
            role.retired_player_scouts_at_clubs,
            role.free_agents.count(),
            role.free_agents.mean(),
        );
    }
}

fn measure(name: &str, spec: WorldSpec) {
    let mut game = spec.build().expect("the world builds");
    eprintln!("{name}");
    print_report("opening", &staff_distribution_report(&game));
    let mut log = StaffDistributionLog::default();
    run_seasons(&mut game, RunOptions::seasons(SEASONS), &mut log).expect("the run completes");
    for (index, report) in log.after_each_season.iter().enumerate() {
        print_report(&format!("season {}", index + 1), report);
    }
}

/// Given the shipped world on seeds 1 and 2
/// When twenty seasons are played
/// Then each role's staff at clubs are printed per season: count, mean, spread, the lowest rating
///      of its top 2.5%, 11.5% and 51.5%, the retired players among its scouts, and the market.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn the_staff_distribution_over_twenty_seasons() {
    for seed in 1..=2 {
        measure(
            &format!("shipped world, seed {seed}"),
            WorldSpec::shipped(seed),
        );
    }
}
