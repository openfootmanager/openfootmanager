//! The age pyramid over many seasons (#869, slice 3).
//!
//! Measurement only: every test is `#[ignore]`d and prints its numbers, so it runs outside the PR
//! gate, in release mode like the shipped game:
//!
//! `cargo test --release -p season-harness --test measure_pyramid -- --ignored --nocapture`
//!
//! The shipped world is read at its opening and after seasons 5, 10 and 20, the gate world at its
//! opening and after season 10: the share of all players and of seniors in each age band, and the
//! age profile of the best 100 players.

use season_harness::ageing::{AgeingReport, Snapshot, ageing_report};
use season_harness::driver::{RunOptions, SeasonObserver, run_seasons};
use season_harness::pyramid::{AgePyramid, age_pyramid};
use season_harness::worlds::WorldSpec;

use ofm_core::end_of_season::EndOfSeasonSummary;
use ofm_core::game::Game;

type Reading = (AgePyramid, AgeingReport);

fn read(game: &Game) -> Reading {
    (
        age_pyramid(game),
        ageing_report(
            &Snapshot::new(),
            &game.players,
            game.clock.current_date.date_naive(),
        ),
    )
}

#[derive(Default)]
struct EverySeason(Vec<Reading>);

impl SeasonObserver for EverySeason {
    fn on_season_end(&mut self, game: &Game, _summary: &EndOfSeasonSummary, _season: u32) {
        self.0.push(read(game));
    }
}

fn print_reading(label: &str, (pyramid, ageing): &Reading) {
    let bands: Vec<String> = pyramid
        .bands
        .iter()
        .map(|band| {
            format!(
                "{} {:>4.1}% of players / {:>4.1}% of seniors",
                band.label,
                band.share_of_players * 100.0,
                band.share_of_seniors * 100.0
            )
        })
        .collect();
    eprintln!(
        "{label:>9} | {} players, {} seniors | {}",
        pyramid.players,
        pyramid.seniors,
        bands.join(" | ")
    );
    eprintln!(
        "{:>9} | top100: median age {:>4.1}, 31+ {:>4.1}%, 33+ {:>4.1}%, 35+ {:>4.1}%",
        "",
        ageing.top100_median_age,
        ageing.top100_aged_31_plus_share * 100.0,
        ageing.top100_aged_33_plus_share * 100.0,
        ageing.top100_aged_35_plus_share * 100.0,
    );
}

fn measure(name: &str, spec: WorldSpec, read_after: &[u32]) {
    let mut game = spec.build().expect("the world builds");
    eprintln!("== {name}");
    print_reading("opening", &read(&game));
    let mut log = EverySeason::default();
    let seasons = read_after.iter().copied().max().unwrap_or(0);
    run_seasons(&mut game, RunOptions::seasons(seasons), &mut log).expect("the run completes");
    for &season in read_after {
        print_reading(&format!("season {season}"), &log.0[season as usize - 1]);
    }
}

/// Given the gate world on seed 1 and the shipped world on seed 1,
/// When they are played for ten and twenty seasons,
/// Then the age bands and the best players' ages are printed at the opening and later.
#[test]
#[ignore = "measurement, run by hand in release mode"]
fn the_age_pyramid_over_many_seasons() {
    measure("gate world, seed 1", WorldSpec::gate(1), &[10]);
    measure("shipped world, seed 1", WorldSpec::shipped(1), &[5, 10, 20]);
}
