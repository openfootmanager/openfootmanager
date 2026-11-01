//! The settled world keeps the tiers and the rare wonderkids the maintainer set (#869, #965).
//!
//! After the opening cohort has aged out, a world is made by its youth intake. Twenty seasons of the
//! shipped world are played and the senior S and A shares and the wonderkid share are read at
//! seasons 10, 15 and 20: S 1.0 to 1.8% and A 5.9 to 8.2% of seniors (the band #965 set), and
//! wonderkids no more common than 0.20% of everyone (develop settled at 0.06 to 0.13%; a handful of players, so seed noise is 0.09 to 0.19%).
//!
//! Twenty seasons of 440 clubs take about twenty minutes in release mode, so it is `#[ignore]`d and
//! runs in the nightly tier:
//!
//! `cargo test --release -p season-harness --test settled_distribution -- --ignored --nocapture`

use season_harness::distribution::{DistributionLog, distribution_report};
use season_harness::driver::{RunOptions, run_seasons};
use season_harness::worlds::WorldSpec;

/// Given the shipped world on each of two seeds,
/// When twenty seasons are played,
/// Then at seasons 10, 15 and 20 the senior S and A shares are inside the band and the wonderkid
///      share is rare.
#[test]
#[ignore = "nightly tier: twenty seasons of the shipped world"]
fn the_settled_world_stays_inside_its_band() {
    let mut outside: Vec<String> = Vec::new();
    for seed in [1, 2] {
        let mut game = WorldSpec::shipped(seed).build().expect("the world builds");
        eprintln!("seed {seed} opening: {:?}", distribution_report(&game));
        let mut log = DistributionLog::default();
        run_seasons(&mut game, RunOptions::seasons(20), &mut log).expect("the seasons play");

        for season in [10usize, 15, 20] {
            let report = &log.after_each_season[season - 1];
            let line = format!(
                "seed {seed} season {season}: S {:.2}% A {:.2}% mean {:.2} wonderkids {:.2}%",
                report.senior_ovr_s_share * 100.0,
                report.senior_ovr_a_share * 100.0,
                report.mean_senior_ovr,
                report.wonderkid_share * 100.0
            );
            eprintln!("{line}");

            let in_band = (0.010..=0.018).contains(&report.senior_ovr_s_share)
                && (0.059..=0.082).contains(&report.senior_ovr_a_share)
                && report.wonderkid_share <= 0.0020;
            if !in_band {
                outside.push(line);
            }
        }
    }

    assert!(
        outside.is_empty(),
        "outside the band (S 1.0-1.8%, A 5.9-8.2%, wonderkids <= 0.20%):\n{}",
        outside.join("\n")
    );
}
