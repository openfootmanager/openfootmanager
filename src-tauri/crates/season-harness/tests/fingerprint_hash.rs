//! One hash of the world after fixed runs, so a change that is meant to alter nothing can show it altered nothing: run this on
//! the base commit and on the change, and compare the lines.
//!
//! `cargo test --release -p season-harness --test fingerprint_hash -- --ignored --nocapture --test-threads=1`
//!
//! The hash is `DefaultHasher` with its fixed keys, so it is stable for one toolchain; compare runs from the same one.

use std::hash::{DefaultHasher, Hash, Hasher};

use season_harness::driver::{RunOptions, run_seasons};
use season_harness::fingerprint::{StatsLog, fingerprint};
use season_harness::invariants::Invariants;
use season_harness::worlds::{Scope, WorldSpec};

fn hash_of(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn play(spec: &WorldSpec, seasons: u32) -> u64 {
    let mut game = spec.build().expect("the world builds");
    let mut invariants = Invariants::new(&game);
    let mut stats = StatsLog::default();
    run_seasons(
        &mut game,
        RunOptions::seasons(seasons),
        &mut (&mut invariants, &mut stats),
    )
    .expect("the seasons play");
    hash_of(&format!("{}\n{}", fingerprint(&game), stats.fingerprint()))
}

/// Given fixed seeds,
/// When each world is played its seasons,
/// Then one line per run shows the hash of the world it ended in.
#[test]
#[ignore = "prints hashes to compare across commits; run by hand in release mode"]
fn the_fingerprint_hash_of_fixed_runs() {
    for seed in [1, 2, 3] {
        eprintln!(
            "FINGERPRINT gate seed {seed} two seasons {:016x}",
            play(&WorldSpec::gate(seed), 2)
        );
    }
    // A package of authored players only: the generator's rating bell draws nothing for it, so
    // this line must be the same before and after a change to the bell.
    let authored = WorldSpec::authored_only_package(1)
        .build()
        .expect("the package builds");
    eprintln!(
        "FINGERPRINT authored-only package opening {:016x}",
        hash_of(&fingerprint(&authored))
    );
    let shipped_seed = 1;
    eprintln!(
        "FINGERPRINT shipped seed {shipped_seed} one season {:016x}",
        play(&WorldSpec::shipped(shipped_seed), 1)
    );
    // A followed season settles most of the world by scoreline, so the full day every other club
    // plays is only covered by a run with every competition active.
    eprintln!(
        "FINGERPRINT shipped seed {shipped_seed} one season every match real {:016x}",
        play(
            &WorldSpec::shipped(shipped_seed).with_scope(Scope::Everything),
            1
        )
    );
}
