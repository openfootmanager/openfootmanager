//! The PR-gate guard for the generated calendar: one season, its rollover and one reload, a few
//! seconds. The four-season version is in `nightly.rs`.

use ofm_core::calendar_booking::MIN_GAP_DAYS;
use season_harness::calendar::{reloaded, unplayed_clashes};
use season_harness::driver::{RunOptions, run_seasons};
use season_harness::invariants::Invariants;
use season_harness::worlds::WorldSpec;

/// Given a generated world played through one season, saved and reloaded,
/// When the rollover has generated the next season's calendar,
/// Then no club has two matches to play within the minimum rest in it, and neither has the
///      calendar the world was built with.
#[test]
fn a_rollover_generates_a_clash_free_calendar_that_survives_a_reload() {
    assert_eq!(MIN_GAP_DAYS, 3, "the rule this test states");
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    assert_eq!(
        unplayed_clashes(&game),
        Vec::<String>::new(),
        "the built world"
    );
    let mut invariants = Invariants::new(&game);

    run_seasons(&mut game, RunOptions::seasons(1), &mut invariants)
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(
        unplayed_clashes(&game),
        Vec::<String>::new(),
        "the generated next season"
    );
    assert_eq!(
        unplayed_clashes(&reloaded(&game)),
        Vec::<String>::new(),
        "after a reload"
    );
}
