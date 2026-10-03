//! The dice of a training session come from the game's seed.

use ofm_core::game::Game;
use ofm_core::training;

use super::fixtures::make_game;

/// What a session does to the squad: condition, injuries and attribute growth.
fn squad_after_training(game: &Game, seed: u64) -> Vec<String> {
    let mut game = game.clone();
    game.seed = seed;
    // Growth is only drawn for a player with room to grow; a fixture of finished
    // players could not tell one seed from another.
    for player in &mut game.players {
        player.potential = 95;
    }
    // Monday is a training day for the default schedule.
    training::process_training(&mut game, 0);
    game.players
        .iter()
        .map(|player| {
            format!(
                "{} cond{} ovr{} inj{:?} attrs{:?}",
                player.id, player.condition, player.ovr, player.injury, player.attributes
            )
        })
        .collect()
}

/// Given a squad on a training day,
/// When the session is run twice from the same save and the same seed,
/// Then every player comes out of it the same way.
#[test]
fn a_session_changes_the_squad_the_same_way_from_the_same_seed() {
    let save = make_game();

    assert_eq!(
        squad_after_training(&save, 11),
        squad_after_training(&save, 11)
    );
}

/// The control: the seed is what decides the session.
#[test]
fn the_seed_decides_what_a_session_does() {
    let save = make_game();

    let outcomes: std::collections::BTreeSet<Vec<String>> = (0..40)
        .map(|seed| squad_after_training(&save, seed))
        .collect();

    assert!(
        outcomes.len() > 1,
        "forty seeds all trained the squad alike"
    );
}
