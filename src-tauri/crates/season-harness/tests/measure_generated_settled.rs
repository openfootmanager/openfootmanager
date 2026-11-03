//! How a generated world's concentration and tiers hold over seasons (#869, GEN-1 acceptance).
//!
//! Measurement only, `#[ignore]`d and run in the cloud in release mode. One line per checkpoint
//! with the reputation fifths (S and A share of seniors, first-team rating), the free agents and
//! the world's tiers, mean and wonderkids:
//!
//! `cargo test --release -p season-harness --test measure_generated_settled -- --ignored --nocapture --test-threads=1`
//!
//! The shipped world (seeds 1 and 2) is read at its opening and after seasons 5, 10 and 20; the
//! gate world (seeds 1 to 3) at its opening and after season 5.

use season_harness::distribution::{DistributionLog, DistributionReport, distribution_report};
use season_harness::driver::{RunOptions, run_seasons};
use season_harness::reputation::{ReputationLog, ReputationReport, reputation_report};
use season_harness::worlds::WorldSpec;

fn line(label: &str, reputation: &ReputationReport, world: &DistributionReport) -> String {
    let fifths: Vec<String> = reputation
        .quintiles
        .iter()
        .map(|group| {
            format!(
                "S {:.1}% A {:.1}% ovr {:.1}",
                group.s_share * 100.0,
                group.a_share * 100.0,
                group.first_team_ovr
            )
        })
        .collect();
    format!(
        "{label}: fifths (lowest first) [{}]; free agents S {:.1}% A {:.1}%; world S {:.2}% A {:.2}% mean {:.2} wonderkids {:.2}%",
        fifths.join("; "),
        reputation.unattached.s_share * 100.0,
        reputation.unattached.a_share * 100.0,
        world.senior_ovr_s_share * 100.0,
        world.senior_ovr_a_share * 100.0,
        world.mean_senior_ovr,
        world.wonderkid_share * 100.0,
    )
}

fn measure(name: &str, spec: WorldSpec, read_after: &[u32]) {
    let mut game = spec.build().expect("the world builds");
    eprintln!(
        "{}",
        line(
            &format!("{name} opening"),
            &reputation_report(&game),
            &distribution_report(&game)
        )
    );
    let seasons = read_after.iter().copied().max().unwrap_or(0);
    let (mut reputation, mut world) = (ReputationLog::default(), DistributionLog::default());
    run_seasons(
        &mut game,
        RunOptions::seasons(seasons),
        &mut (&mut reputation, &mut world),
    )
    .expect("the run completes");
    for &season in read_after {
        let index = season as usize - 1;
        eprintln!(
            "{}",
            line(
                &format!("{name} season {season}"),
                &reputation.after_each_season[index],
                &world.after_each_season[index]
            )
        );
    }
}

#[test]
#[ignore = "measurement: prints, asserts nothing"]
fn measure_the_shipped_world_over_twenty_seasons() {
    for seed in [1, 2] {
        measure(
            &format!("shipped seed {seed}"),
            WorldSpec::shipped(seed),
            &[5, 10, 20],
        );
    }
}

#[test]
#[ignore = "measurement: prints, asserts nothing"]
fn measure_the_gate_world_over_five_seasons() {
    for seed in [1, 2, 3] {
        measure(&format!("gate seed {seed}"), WorldSpec::gate(seed), &[5]);
    }
}
