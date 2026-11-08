//! The settled world keeps the tiers and the rare wonderkids the maintainer set (#869, #965).
//!
//! After the opening cohort has aged out, a world is made by its youth intake. Twenty seasons of the
//! shipped world are played on each of two seeds, and the senior S and A shares and the wonderkid
//! share are read at seasons 10, 15 and 20.
//!
//! The band is #965's: S 1.0 to 1.8% and A 5.9 to 8.2% of seniors, with three floors lowered by the
//! maintainer on 3 and 4 Nov: S 0.7% at season 10, S 0.9% at season 15 and A 5.3% at season 10.
//! #965 measured a world in which about a tenth of the S and A players stayed unsigned among the
//! free agents (3,795 at opening, about a hundred signing a season); a generated world now puts its
//! stars in its clubs (#1078), so the same clubs read the same and the world's share is lower by what
//! the free agents used to hold. Every other floor is #965's, and the caps are unchanged. Wonderkids
//! are no more common than 0.20% of everyone (develop settled at 0.06 to 0.13%).
//!
//! The gate is the AVERAGE of the seeds at each checkpoint, not each world on its own: one world's
//! share moves by about 0.05 points with any change to the match engine or to ratings (the same
//! constants read 5.50 and 5.45 on two develop states, and 0.74 and 0.92 on two seeds), which is
//! noise, not a change in what the game makes. The numbers of every seed are still printed, for
//! diagnosis.
//!
//! Twenty seasons of 440 clubs take about twenty minutes a seed in release mode, so the test is
//! `#[ignore]`d and runs in the nightly workflow (`harness-nightly` in `nightly-e2e.yml`, its own
//! step), not on a pull request:
//!
//! `cargo test --release -p season-harness --test settled_distribution -- --ignored --nocapture`

use season_harness::distribution::{DistributionLog, DistributionReport, distribution_report};
use season_harness::driver::{RunOptions, run_seasons};
use season_harness::worlds::WorldSpec;

const SEEDS: [u64; 2] = [1, 2];
const CHECKPOINT_SEASONS: [usize; 3] = [10, 15, 20];

/// The S and A floors at a checkpoint season: lower than #965's 1.0% and 5.9% at seasons 10 and 15,
/// see the module comment.
fn floors(season: usize) -> (f64, f64) {
    match season {
        10 => (0.007, 0.053),
        15 => (0.009, 0.059),
        _ => (0.010, 0.059),
    }
}

/// The checkpoints at which the average of `worlds` (each world's report at that season) is outside
/// the band, as a line for each.
fn checkpoints_outside_the_band(worlds: &[Vec<(usize, DistributionReport)>]) -> Vec<String> {
    let mut outside = Vec::new();
    for season in CHECKPOINT_SEASONS {
        let reports: Vec<&DistributionReport> = worlds
            .iter()
            .filter_map(|world| world.iter().find(|(at, _)| *at == season).map(|(_, r)| r))
            .collect();
        let count = reports.len() as f64;
        let average = |read: fn(&DistributionReport) -> f64| {
            reports.iter().map(|report| read(report)).sum::<f64>() / count
        };
        let (s, a, wonderkids) = (
            average(|report| report.senior_ovr_s_share),
            average(|report| report.senior_ovr_a_share),
            average(|report| report.wonderkid_share),
        );
        let (s_floor, a_floor) = floors(season);
        if !(s_floor..=0.018).contains(&s) || !(a_floor..=0.082).contains(&a) || wonderkids > 0.0020
        {
            outside.push(format!(
                "season {season}, average of {} worlds: S {:.2}% A {:.2}% wonderkids {:.2}% \
                 (floors S {:.1}% A {:.1}%)",
                reports.len(),
                s * 100.0,
                a * 100.0,
                wonderkids * 100.0,
                s_floor * 100.0,
                a_floor * 100.0
            ));
        }
    }
    outside
}

/// Given the shipped world on each of two seeds,
/// When twenty seasons are played,
/// Then at seasons 10, 15 and 20 the average senior S and A shares are inside the band and the
///      average wonderkid share is rare.
#[test]
#[ignore = "nightly tier: twenty seasons of the shipped world"]
fn the_settled_world_stays_inside_its_band() {
    let mut worlds = Vec::new();
    for seed in SEEDS {
        let mut game = WorldSpec::shipped(seed).build().expect("the world builds");
        eprintln!("seed {seed} opening: {:?}", distribution_report(&game));
        let mut log = DistributionLog::default();
        run_seasons(&mut game, RunOptions::seasons(20), &mut log).expect("the seasons play");

        let mut world = Vec::new();
        for season in CHECKPOINT_SEASONS {
            let report = log.after_each_season[season - 1];
            eprintln!(
                "seed {seed} season {season}: S {:.2}% A {:.2}% mean {:.2} wonderkids {:.2}%",
                report.senior_ovr_s_share * 100.0,
                report.senior_ovr_a_share * 100.0,
                report.mean_senior_ovr,
                report.wonderkid_share * 100.0
            );
            world.push((season, report));
        }
        worlds.push(world);
    }

    let outside = checkpoints_outside_the_band(&worlds);
    assert!(
        outside.is_empty(),
        "average outside the band (S 1.0-1.8%, A 5.9-8.2%, S from 0.7% at season 10 and 0.9% at 15, \
         A from 5.3% at season 10, wonderkids <= 0.20%):\n{}",
        outside.join("\n")
    );
}

#[cfg(test)]
mod gate {
    use super::*;

    fn world(s: f64, a: f64) -> Vec<(usize, DistributionReport)> {
        CHECKPOINT_SEASONS
            .iter()
            .map(|season| {
                (
                    *season,
                    DistributionReport {
                        players: 14_000,
                        seniors: 9_000,
                        mean_senior_ovr: 69.0,
                        senior_ovr_s_share: s,
                        senior_ovr_a_share: a,
                        senior_ceiling_s_share: s,
                        senior_ceiling_a_share: a,
                        youth_share: 0.3,
                        wonderkid_share: 0.0012,
                    },
                )
            })
            .collect()
    }

    /// Given two worlds that are each inside the band,
    /// When the band is checked,
    /// Then their average is inside it too.
    #[test]
    fn two_worlds_inside_the_band_average_inside_it() {
        let outside = checkpoints_outside_the_band(&[world(0.0116, 0.066), world(0.0156, 0.066)]);

        assert!(outside.is_empty(), "{outside:?}");
    }

    /// Given one world 0.4 points under every S floor (0.6%) and one 0.6 points above S 1.0%
    ///       (1.6%), whose average S of 1.1% is inside the band,
    /// When the band is checked,
    /// Then it passes: judging each world on its own would fail the first, the average does not.
    #[test]
    fn a_world_outside_the_band_is_forgiven_by_an_average_inside_it() {
        let outside = checkpoints_outside_the_band(&[world(0.006, 0.066), world(0.016, 0.066)]);

        assert!(outside.is_empty(), "{outside:?}");
    }

    /// Given one world inside the band and one far under the A floor, so that the average A of
    ///       4.9% is under it too,
    /// When the band is checked,
    /// Then all three checkpoints fail although the first world is inside on its own.
    #[test]
    fn an_average_outside_the_band_fails_although_one_world_is_inside() {
        let outside = checkpoints_outside_the_band(&[world(0.012, 0.060), world(0.012, 0.038)]);

        assert_eq!(outside.len(), 3, "{outside:?}");
    }

    /// Given two worlds that are both under the A floor at every checkpoint,
    /// When the band is checked,
    /// Then the average is outside it at all three checkpoints.
    #[test]
    fn two_low_worlds_average_outside_the_band() {
        let outside = checkpoints_outside_the_band(&[world(0.012, 0.050), world(0.013, 0.052)]);

        assert_eq!(outside.len(), 3, "{outside:?}");
    }

    /// Given two worlds whose average A is 5.4% at season 10,
    /// When the band is checked,
    /// Then season 10 passes (its A floor is 5.3%) while seasons 15 and 20 (floor 5.9%) fail.
    #[test]
    fn the_season_ten_a_floor_is_lower_than_the_others() {
        let outside = checkpoints_outside_the_band(&[world(0.012, 0.054), world(0.012, 0.054)]);

        assert_eq!(outside.len(), 2, "{outside:?}");
        assert!(outside.iter().all(|line| !line.contains("season 10,")));
    }
}
