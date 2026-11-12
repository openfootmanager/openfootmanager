//! A game as JSON, for the tests that hold a refused write to "the game is exactly as it was".
//!
//! `StateManager::update_game` raises `Game::revision` before it runs the closure, so a write that
//! is then refused (a stale career, a failed check) still moves that one number: it orders the
//! snapshots a screen receives, and it is not part of what the game holds. Comparing two
//! snapshots with it left in would call every refusal a change, so the comparison takes it out
//! and holds every other field to equality.

use ofm_core::game::Game;
use ofm_core::state::StateManager;
use serde_json::Value;

/// `game` as JSON without its `revision`.
pub(crate) fn game_json(game: &Game) -> Value {
    let mut value = serde_json::to_value(game).expect("a game serialises");
    if let Some(fields) = value.as_object_mut() {
        fields.remove("revision");
    }
    value
}

/// The active game as JSON without its `revision`.
pub(crate) fn active_game_json(state: &StateManager) -> Value {
    state.get_game(game_json).expect("an active game")
}

#[cfg(test)]
mod tests {
    use super::{active_game_json, game_json};
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::StateManager;

    fn game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2031, 7, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    /// Given a game and the same game after a write that raised only its revision
    /// When both are taken as JSON
    /// Then they are equal, and a game that differs in anything else is not.
    #[test]
    fn a_snapshot_ignores_the_revision_and_nothing_else() {
        let base = game();
        let mut bumped = base.clone();
        bumped.revision += 7;
        let mut changed = base.clone();
        changed.career_token += 1;

        assert_eq!(game_json(&base), game_json(&bumped));
        assert_ne!(game_json(&base), game_json(&changed));
    }

    /// Given an installed game that a refused write has raised the revision of
    /// When the active game is taken as JSON before and after
    /// Then the two are equal.
    #[test]
    fn the_active_game_snapshot_survives_a_write_that_changes_nothing() {
        let state = StateManager::new();
        state.set_game(game());
        let before = active_game_json(&state);

        state.update_game(|_| ());

        assert_eq!(active_game_json(&state), before);
    }
}
