//! A world built from a package, the way the app builds one from an installed package, played
//! through the same driver and held to the same rules as a generated world. A bug in how packages
//! are loaded, merged, seeded or turned into a game shows up here, in a run, and not only in the
//! package unit tests.

use season_harness::driver::{RunOptions, run_seasons};
use season_harness::fingerprint::{fingerprint, first_difference};
use season_harness::invariants::Invariants;
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
/// Then every invariant holds at every day and at every rollover.
#[test]
fn a_package_world_holds_every_invariant_across_two_seasons() {
    let mut game = WorldSpec::two_tier_package(11)
        .build()
        .expect("the package builds");
    let mut invariants = Invariants::new(&game);

    run_seasons(&mut game, RunOptions::seasons(2), &mut invariants)
        .unwrap_or_else(|error| panic!("the package world: {error}"));

    assert!(
        invariants.is_clean(),
        "the package world broke a rule:\n{}",
        invariants.report()
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

/// Given the package, which declares no staff,
/// When a world is built from it,
/// Then every club has the backroom the app gives an imported world (one assistant manager,
///      coach, scout and physio), the staff market is stocked, and no club opens with less cash
///      than sixteen weeks of its wage bill: the harness builds a package as the app does.
#[test]
fn a_package_world_is_prepared_for_a_career_as_the_app_prepares_one() {
    use domain::staff::StaffRole;
    let game = WorldSpec::two_tier_package(11)
        .build()
        .expect("the package builds");

    for team in &game.teams {
        for role in [
            StaffRole::AssistantManager,
            StaffRole::Coach,
            StaffRole::Scout,
            StaffRole::Physio,
        ] {
            let employed = game
                .staff
                .iter()
                .filter(|member| member.team_id.as_deref() == Some(team.id.as_str()))
                .filter(|member| member.role == role)
                .count();
            assert_eq!(employed, 1, "{} has {employed} of {role:?}", team.id);
        }
        let weekly_bill = ofm_core::finances::calc_wages(&game, &team.id);
        assert!(
            team.finance >= weekly_bill * ofm_core::finances::MIN_OPENING_RUNWAY_WEEKS,
            "{} opens with {} against a weekly bill of {weekly_bill}",
            team.id,
            team.finance
        );
    }
    assert!(
        game.staff.iter().any(|member| member.team_id.is_none()),
        "the staff market is stocked"
    );
}
