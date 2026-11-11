//! The day's training injuries are rolled by the turn, in both ways a day ends.
//!
//! `training::roll_training_injuries` has its own tests, which call it directly. These pin that the
//! turn calls it: delete the call from `process_day_common` and each of these fails.

use ofm_core::game::Game;
use ofm_core::training::roll_training_injuries;
use ofm_core::turn;

use super::fixtures::make_game_without_match_today;

/// Every player at fitness 1, so each club's session injures one of them with a 6% chance a day.
fn unfit_world(seed: u64) -> Game {
    let mut game = make_game_without_match_today();
    game.seed = seed;
    for player in &mut game.players {
        player.fitness = 1;
    }
    game
}

/// Who is injured, and with what: (player id, injury name, days left), in a stable order.
fn injured(game: &Game) -> Vec<(String, String, u32)> {
    let mut hurt: Vec<(String, String, u32)> = game
        .players
        .iter()
        .filter_map(|player| {
            player.injury.as_ref().map(|injury| {
                (
                    player.id.clone(),
                    injury.name.clone(),
                    injury.days_remaining,
                )
            })
        })
        .collect();
    hurt.sort();
    hurt
}

/// Given a world whose clubs all train today, over 300 seeds, and the injuries the day's training
///      roll gives each seed (the roll alone, on a copy)
/// When the day ends, once through `process_day` and once through `finish_live_match_day`
/// Then in both, the same players are injured with the same injuries and their FULL duration: the
///      day's recovery has run before the roll, so an injury picked up today is not ticked down
///      the day it happens, and the turn does roll them at all
#[test]
fn the_day_ends_with_the_training_injuries_its_roll_gives_in_both_paths() {
    let mut any_injury = false;
    for seed in 0..300 {
        let mut rolled = unfit_world(seed);
        roll_training_injuries(&mut rolled);
        let expected = injured(&rolled);
        any_injury |= !expected.is_empty();

        let mut watched = unfit_world(seed);
        turn::process_day(&mut watched);
        assert_eq!(injured(&watched), expected, "process_day, seed {seed}");

        let mut live = unfit_world(seed);
        turn::finish_live_match_day(&mut live);
        assert_eq!(
            injured(&live),
            expected,
            "finish_live_match_day, seed {seed}"
        );
    }
    assert!(any_injury, "no seed of 300 hurt anyone in training");
}
