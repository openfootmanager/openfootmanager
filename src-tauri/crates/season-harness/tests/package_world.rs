//! A world built from a package, the way the app builds one from an installed package, played
//! through the same driver and held to the same rules as a generated world. A bug in how packages
//! are loaded, merged, seeded or turned into a game shows up here, in a run, and not only in the
//! package unit tests.

use season_harness::driver::{RunOptions, run_seasons};
use season_harness::fingerprint::{fingerprint, first_difference};
use season_harness::invariants::{CLUB_RESTS_BETWEEN_MATCHES, Invariants};
use season_harness::worlds::WorldSpec;

/// Given the two-division-and-a-cup package,
/// When a world is built from it,
/// Then the game has the package's forty clubs and its three competitions, the manager is at a
///      club of the package, and the game carries the seed the world was built from.
#[test]
fn a_package_becomes_a_game_with_its_clubs_and_competitions() {
    let game = WorldSpec::two_tier_package(11)
        .build()
        .expect("the package builds");

    assert_eq!(game.teams.len(), 40);
    for competition in ["harness-d1", "harness-d2", "harness-cup"] {
        assert!(
            game.competitions.iter().any(|c| c.id == competition),
            "{competition} is missing: {:?}",
            game.competitions.iter().map(|c| &c.id).collect::<Vec<_>>()
        );
    }
    let club = game
        .manager
        .team_id
        .as_deref()
        .expect("the manager has a club");
    assert!(club.ends_with("-fc"), "{club} is not a club of the package");
    assert_eq!(game.seed, 11);
}

/// Given a world built from the package,
/// When it is played through two consecutive seasons,
/// Then every invariant holds at every day and at every rollover, except the club-rest rule that
///      #553 breaks: the package's cup opens on 1 September and clashes with its leagues.
#[test]
fn a_package_world_holds_every_invariant_across_two_seasons() {
    let mut game = WorldSpec::two_tier_package(11)
        .build()
        .expect("the package builds");
    let mut invariants = Invariants::new(&game);

    run_seasons(&mut game, RunOptions::seasons(2), &mut invariants)
        .unwrap_or_else(|error| panic!("the package world: {error}"));

    let others = invariants.report_except(CLUB_RESTS_BETWEEN_MATCHES);
    assert!(
        others.is_empty(),
        "the package world broke a rule:\n{others}"
    );
    assert!(
        !invariants.report_of(CLUB_RESTS_BETWEEN_MATCHES).is_empty(),
        "#553 is fixed: delete this exemption (no club of the package played inside the rest)"
    );
}

/// Given one seed,
/// When the package world is built and played twice,
/// Then both runs end in the same world; another seed ends in another.
#[test]
fn a_package_world_replays_from_its_seed() {
    let play = |seed: u64| {
        let mut game = WorldSpec::two_tier_package(seed)
            .build()
            .expect("the package builds");
        let mut invariants = Invariants::new(&game);
        run_seasons(&mut game, RunOptions::seasons(1), &mut invariants).expect("the season plays");
        fingerprint(&game)
    };

    let (first, second) = (play(5), play(5));

    assert!(first == second, "{}", first_difference(&first, &second));
    assert_ne!(first, play(6), "another seed must be another world");
}
