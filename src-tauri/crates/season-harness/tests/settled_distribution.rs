//! The settled world keeps the tiers and the rare wonderkids the maintainer set (#869, #965).
//!
//! After the opening cohort has aged out, a world is made by its youth intake. Twenty seasons of the
//! shipped world are played and the senior S and A shares and the wonderkid share are read at
//! seasons 10, 15 and 20.
//!
//! The band is #965's: S 1.0 to 1.8% and A 5.9 to 8.2% of seniors, with one change ruled by the
//! maintainer on 3 Nov: at season 10 the floors are S 0.8% and A 5.5%. #965 measured a world in which
//! about a tenth of the S and A players stayed unsigned among the free agents (3,795 at opening, about
//! a hundred signing a season); a generated world now puts its stars in its clubs (#1078), so the
//! same clubs read the same and the world's share is lower by what the free agents used to hold.
//! Seasons 15 and 20 keep #965's floors, and the caps are unchanged. Wonderkids are no more common
//! than 0.20% of everyone (develop settled at 0.06 to 0.13%; a handful of players, so the seed noise
//! is 0.09 to 0.19%).
//!
//! Twenty seasons of 440 clubs take about twenty minutes a seed in release mode, so the test is
//! `#[ignore]`d and runs in the nightly workflow (`harness-nightly` in `nightly-e2e.yml`, its own
//! step), not on a pull request:
//!
//! `cargo test --release -p season-harness --test settled_distribution -- --ignored --nocapture`

use season_harness::distribution::{DistributionLog, distribution_report};
use season_harness::driver::{RunOptions, run_seasons};
use season_harness::worlds::WorldSpec;

/// The S and A floors at season 10: lower than #965's 1.0% and 5.9%, see the module comment.
const SEASON_TEN_FLOORS: (f64, f64) = (0.008, 0.055);

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

            let (s_floor, a_floor) = if season == 10 {
                SEASON_TEN_FLOORS
            } else {
                (0.010, 0.059)
            };
            let in_band = (s_floor..=0.018).contains(&report.senior_ovr_s_share)
                && (a_floor..=0.082).contains(&report.senior_ovr_a_share)
                && report.wonderkid_share <= 0.0020;
            if !in_band {
                outside.push(line);
            }
        }
    }

    assert!(
        outside.is_empty(),
        "outside the band (S 1.0-1.8%, A 5.9-8.2%, at season 10 S from 0.8% and A from 5.5%, wonderkids <= 0.20%):\n{}",
        outside.join("\n")
    );
}
