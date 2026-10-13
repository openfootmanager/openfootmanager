//! One hash of the world after fixed runs, so a change that is meant to alter nothing can show it altered nothing: run this on
//! the base commit and on the change, and compare the lines.
//!
//! `cargo test --release -p season-harness --test fingerprint_hash -- --ignored --nocapture --test-threads=1`
//!
//! The hash is `DefaultHasher` with its fixed keys, so it is stable for one toolchain; compare runs from the same one.

use std::hash::{DefaultHasher, Hash, Hasher};

use season_harness::driver::{RunOptions, run_seasons};
use season_harness::fingerprint::fingerprint;
use season_harness::invariants::Invariants;
use season_harness::worlds::WorldSpec;

fn hash_of(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn play(spec: &WorldSpec, seasons: u32) -> u64 {
    let mut game = spec.build().expect("the world builds");
    let mut invariants = Invariants::new(&game);
    run_seasons(&mut game, RunOptions::seasons(seasons), &mut invariants)
        .expect("the seasons play");
    hash_of(&fingerprint(&game))
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
    eprintln!(
        "FINGERPRINT shipped seed 1 one season {:016x}",
        play(&WorldSpec::shipped(1), 1)
    );
}
