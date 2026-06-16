use ofm_core::game::Game;
use ofm_core::state::StateManager;

const NO_ACTIVE_GAME: &str = "be.error.noActiveGameSession";

/// Mutate the active game in place and return a single clone for the response.
///
/// Replaces the pervasive `get_game(|g| g.clone())` → mutate → `set_game(clone)`
/// pattern, which deep-cloned the entire world (440 teams, ~9,680 players)
/// *twice* per command. Here the world is borrowed and mutated in place, then
/// cloned once for serialization back to the UI.
///
/// The closure performs validation **before** mutation: because it mutates the
/// live game rather than a throwaway clone, any change made before it returns
/// `Err` would persist. Existing callers already validate up front.
pub fn mutate_active_game<F>(state: &StateManager, mutate: F) -> Result<Game, String>
where
    F: FnOnce(&mut Game) -> Result<(), String>,
{
    state
        .update_game(|game| {
            mutate(game)?;
            Ok(game.clone())
        })
        .unwrap_or_else(|| Err(NO_ACTIVE_GAME.to_string()))
}
