//! The CI gate: a generated world is played through consecutive seasons and every
//! invariant must hold, and the gate must be able to fail.
//!
//! Budget: this runs inside the `backend` job, which `tauri-action.yml` and
//! `nightly-tauri-action.yml` reuse, so every second here is a second on every
//! release. Keep the whole file under two minutes, and change `SEASONS` or
//! `SEEDS` only with a measurement beside the change.

use season_harness::driver::{DayPath, HarnessError, RunOptions, run_seasons};
use season_harness::fingerprint::{fingerprint, first_difference};
use season_harness::invariants::Invariants;
use season_harness::worlds::WorldSpec;

const SEASONS: u32 = 2;
const SEEDS: &[u64] = &[1];

/// Given a generated world with the player in charge of a club,
/// When it is played through consecutive seasons the way the game plays a day,
/// Then every invariant holds at every day and at every rollover.
#[test]
fn a_generated_world_holds_every_invariant_across_consecutive_seasons() {
    for &seed in SEEDS {
        let mut game = WorldSpec::gate(seed).build().expect("the world builds");
        let mut invariants = Invariants::new(&game);

        run_seasons(&mut game, RunOptions::seasons(SEASONS), &mut invariants)
            .unwrap_or_else(|error| panic!("seed {seed}: {error}"));

        assert!(
            invariants.is_clean(),
            "seed {seed} broke an invariant:\n{}",
            invariants.report()
        );
    }
}

/// Given the same world,
/// When the player's own match each matchday goes through the live path
///      (`ofm_core::matchday`) instead of the batch,
/// Then every invariant still holds — a fixture stranded `Scheduled` by the
///      live day (#608) would end the season never completing or fail
///      `no-stranded-fixture` — and the run says the live path was really taken.
#[test]
fn a_season_played_through_the_live_path_holds_every_invariant() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    let mut invariants = Invariants::new(&game);
    let options = RunOptions {
        day_path: DayPath::UserMatchLive,
        ..RunOptions::seasons(1)
    };

    let summary = run_seasons(&mut game, options, &mut invariants)
        .unwrap_or_else(|error| panic!("the live path: {error}"));

    assert!(
        summary.live_days > 20,
        "the player's club plays dozens of matches a season, so the live path \
         must have been taken on dozens of days; it was taken on {}",
        summary.live_days
    );
    assert!(
        invariants.is_clean(),
        "the live path broke an invariant:\n{}",
        invariants.report()
    );
}

/// Given a generated world built for a clock in 2033,
/// When a day is played,
/// Then every club still has a squad — the world was dated for the year it opens
///      in, so no contract has already run out.
///
/// Every invariant below holds vacuously on a world whose players all became free
/// agents on the first day, and one did: the world was generated for the real
/// current year and played on a 2033 clock.
#[test]
fn every_club_still_has_a_squad_after_the_first_day() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");

    ofm_core::turn::process_day(&mut game);

    for team in &game.teams {
        let squad = game
            .players
            .iter()
            .filter(|player| player.team_id.as_deref() == Some(team.id.as_str()))
            .count();
        assert!(squad >= 11, "{} has {squad} players", team.id);
    }
}

/// Given an Argentine club chosen as the player's,
/// When the career begins,
/// Then the clock is set back to the start of Argentina's season and the run is
///      not refused — the harness builds careers the way the game does, so a
///      southern-hemisphere career is one it can play.
#[test]
fn an_argentine_career_opens_on_the_start_of_its_clubs_season() {
    let spec = WorldSpec {
        user_nation: "AR",
        ..WorldSpec::gate(1)
    };

    let game = spec.build().expect("an Argentine career begins");

    let club = game
        .manager
        .team_id
        .clone()
        .expect("the manager has a club");
    let team = game.teams.iter().find(|team| team.id == club).unwrap();
    assert_eq!(team.football_nation, "AR");
    assert!(
        game.clock.current_date < ofm_core::world::start_date_for_year(2033).unwrap(),
        "Argentina's season starts before the game's own start date, so the clock \
         is set back to it; it stayed on {}",
        game.clock.current_date
    );
}

/// The batch path never takes the live one, so a summary that counts live days
/// can be believed in the test above.
#[test]
fn the_batch_path_takes_no_live_days() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    let mut invariants = Invariants::new(&game);

    let summary =
        run_seasons(&mut game, RunOptions::seasons(1), &mut invariants).expect("the season plays");

    assert_eq!(summary.live_days, 0);
    assert!(summary.days > 0);
}

/// The world the old scenario test built: one flat league in the deprecated
/// `game.league` mirror and nothing in `game.competitions`. Every check that
/// only looks at the competitions passes on it vacuously, which is how
/// promotion and relegation could be dead in every generated world with the
/// only end-to-end season test green. The gate has to see that it is wrong.
#[test]
fn a_flat_league_world_is_rejected_before_a_day_is_played() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    let team_ids: Vec<String> = game.teams.iter().map(|team| team.id.clone()).collect();
    game.competitions.clear();
    game.league = Some(ofm_core::schedule::generate_league(
        "Flat League",
        2033,
        &team_ids,
        game.clock.current_date,
    ));

    let invariants = Invariants::new(&game);

    assert!(
        invariants
            .violations()
            .iter()
            .any(|violation| violation.rule == "ladder-holds"
                && violation.message.contains("has clubs but no league table")),
        "a world with no competitions must not pass:\n{}",
        invariants.report()
    );
}

/// A manager the board is about to sack: satisfaction gone and a warning
/// already given. Left alone, the game fires them on the first day and every
/// later rollover does nothing.
fn about_to_be_sacked(seed: u64) -> ofm_core::game::Game {
    let mut game = WorldSpec::gate(seed).build().expect("the world builds");
    game.manager.satisfaction = 0;
    game.manager.warning_stage = 1;
    game
}

#[test]
fn a_manager_the_board_would_sack_is_reported_when_nothing_holds_them_on() {
    let mut game = about_to_be_sacked(1);
    let mut invariants = Invariants::new(&game);
    let options = RunOptions {
        keep_manager_employed: false,
        ..RunOptions::seasons(1)
    };

    let error = run_seasons(&mut game, options, &mut invariants)
        .expect_err("a sacked manager must stop the run, not be run through");

    assert!(matches!(error, HarnessError::ManagerLost { .. }), "{error}");
}

#[test]
fn the_tenure_guard_keeps_that_manager_in_the_job_for_the_season() {
    let mut game = about_to_be_sacked(1);
    let mut invariants = Invariants::new(&game);

    run_seasons(&mut game, RunOptions::seasons(1), &mut invariants)
        .expect("a guarded manager is never sacked");

    assert!(game.manager.team_id.is_some());
}

#[test]
fn a_season_that_will_not_finish_stops_the_run_and_names_what_is_unfinished() {
    let mut game = WorldSpec::gate(1).build().expect("the world builds");
    let mut invariants = Invariants::new(&game);
    let options = RunOptions {
        max_days_per_season: 3,
        ..RunOptions::seasons(1)
    };

    let error =
        run_seasons(&mut game, options, &mut invariants).expect_err("three days is not a season");

    match error {
        HarnessError::Stalled {
            unfinished, days, ..
        } => {
            assert_eq!(days, 3);
            assert!(
                unfinished.iter().any(|line| line.starts_with("eng-d1:")),
                "the report must say which competition is holding the season open: {unfinished:?}"
            );
        }
        other => panic!("expected a stall, got {other}"),
    }
}

/// Given a generated world and a seed,
/// When it is built twice and played through a season and its rollover, once on each copy,
/// Then the two end in the same world — results, tables, ids, bodies, money, staff and
///      managers — so a draw that escapes the game's seed fails the gate on the day it is added.
#[test]
fn the_same_seed_plays_the_same_seasons() {
    let play = || {
        let mut game = WorldSpec::gate(1).build().expect("the world builds");
        let mut invariants = Invariants::new(&game);
        // The invariants are not under test here: only that the two runs agree.
        let _ = run_seasons(&mut game, RunOptions::seasons(1), &mut invariants);
        fingerprint(&game)
    };

    let first = play();
    let second = play();

    assert!(first == second, "{}", first_difference(&first, &second));
}

/// The control for the test above, at the cost of a build rather than a season: another seed is
/// another world, or that test could not tell a seeded run from one that ignores its seed.
#[test]
fn another_seed_builds_another_world() {
    let built = |seed: u64| fingerprint(&WorldSpec::gate(seed).build().expect("the world builds"));

    assert_ne!(built(1), built(2));
}

/// Given a world generated from a seed,
/// When a career begins in it,
/// Then the game carries that seed: the day's dice are drawn from it, so a run is the seed's run.
#[test]
fn the_game_carries_the_seed_its_world_was_made_from() {
    assert_eq!(WorldSpec::gate(41).build().expect("builds").seed, 41);
}
