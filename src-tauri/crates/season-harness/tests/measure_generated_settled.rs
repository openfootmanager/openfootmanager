//! How a generated world's concentration and tiers hold over seasons (#869, GEN-1 acceptance).
//!
//! Measurement only, `#[ignore]`d and run in the cloud in release mode. One line per checkpoint
//! with the reputation fifths (S and A share of seniors, first-team rating), the free agents and
//! the world's tiers, mean and wonderkids:
//!
//! `cargo test --release -p season-harness --test measure_generated_settled -- --ignored --nocapture --test-threads=1`
//!
//! The shipped world (seeds 1 to 3, one test each) is read at its opening and after seasons 5, 10 and 20; the
//! gate world (seeds 1 to 3) at its opening and after season 5.

use chrono::NaiveDate;
use domain::age::age_on_or_unknown;
use ofm_core::game::Game;
use season_harness::distribution::{
    DistributionLog, DistributionReport, SENIOR_MIN_AGE, distribution_report,
};
use season_harness::driver::{RunOptions, run_seasons};
use season_harness::reputation::{ReputationLog, ReputationReport, reputation_report};
use season_harness::worlds::WorldSpec;

/// S and A shares and the mean rating of the seniors who belong to a club (free agents left out),
/// so the world's tiers can be read with and without the unsigned players.
fn clubs_only(game: &Game) -> String {
    let on: NaiveDate = game.clock.current_date.date_naive();
    let ratings: Vec<u8> = game
        .players
        .iter()
        .filter(|player| {
            !player.retired
                && player.team_id.is_some()
                && age_on_or_unknown(on, &player.date_of_birth) >= SENIOR_MIN_AGE
        })
        .map(|player| player.ovr)
        .collect();
    let count = ratings.len().max(1) as f64;
    let share = |from: u8, to: u8| {
        100.0
            * ratings
                .iter()
                .filter(|ovr| (from..to).contains(ovr))
                .count() as f64
            / count
    };
    format!(
        "clubs only S {:.2}% A {:.2}% mean {:.2}",
        share(81, 100),
        share(77, 81),
        ratings.iter().map(|ovr| f64::from(*ovr)).sum::<f64>() / count
    )
}

#[derive(Default)]
struct ClubsOnlyLog {
    after_each_season: Vec<String>,
}

impl season_harness::driver::SeasonObserver for ClubsOnlyLog {
    fn on_season_end(
        &mut self,
        game: &Game,
        _summary: &ofm_core::end_of_season::EndOfSeasonSummary,
        _season: u32,
    ) {
        self.after_each_season.push(clubs_only(game));
    }
}

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
    eprintln!("{name} opening {}", clubs_only(&game));
    let (mut reputation, mut world, mut clubs) = (
        ReputationLog::default(),
        DistributionLog::default(),
        ClubsOnlyLog::default(),
    );
    run_seasons(
        &mut game,
        RunOptions::seasons(seasons),
        &mut (&mut reputation, &mut (&mut world, &mut clubs)),
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
        eprintln!("{name} season {season} {}", clubs.after_each_season[index]);
    }
}

/// One test per seed, so that `--test-threads` runs the seeds side by side.
fn measure_shipped(seed: u64) {
    measure(
        &format!("shipped seed {seed}"),
        WorldSpec::shipped(seed),
        &[5, 10, 20],
    );
}

#[test]
#[ignore = "measurement: prints, asserts nothing"]
fn measure_shipped_seed_1_over_twenty_seasons() {
    measure_shipped(1);
}

#[test]
#[ignore = "measurement: prints, asserts nothing"]
fn measure_shipped_seed_2_over_twenty_seasons() {
    measure_shipped(2);
}

#[test]
#[ignore = "measurement: prints, asserts nothing"]
fn measure_shipped_seed_3_over_twenty_seasons() {
    measure_shipped(3);
}

#[test]
#[ignore = "measurement: prints, asserts nothing"]
fn measure_the_gate_world_over_five_seasons() {
    for seed in [1, 2, 3] {
        measure(&format!("gate seed {seed}"), WorldSpec::gate(seed), &[5]);
    }
}
