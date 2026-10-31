use db::save_manager::SaveManager;
use ofm_core::state::{Career, StateManager};
use std::collections::HashSet;
use std::sync::MutexGuard;

/// Keep exit atomic with start/finish and career replacement, including unsaved careers.
pub(crate) fn exit_to_menu(
    state: &StateManager,
    save_manager: &mut SaveManager,
) -> Result<bool, String> {
    let operation = super::live_session::idle_operation(state)?;
    let saved = state.get_save_id().is_some_and(|id| !id.is_empty());
    if saved {
        // Leaving is the player's explicit act on the career in front of them, and a screen that has
        // lost its game must still be able to leave, so it names no career: whichever is active is
        // saved and closed.
        persist_game_under_operation(state, save_manager, Career::Active, &operation)?;
    }
    state.clear_game();
    state.clear_save_id();
    Ok(saved)
}

/// Persist the active career under the live-session idle gate.
pub fn persist_active_game(
    state: &StateManager,
    save_manager: &mut SaveManager,
    career: Career,
) -> Result<(), String> {
    let operation = super::live_session::idle_operation(state)?;
    persist_game_under_operation(state, save_manager, career, &operation)
}

// Ordinary saves and save-on-exit snapshot dirty journal ids before I/O, then
// remove only the flushed ids so concurrent new posts remain dirty. The guard
// token keeps this shared flush protocol inside its caller's idle gate.
fn persist_game_under_operation(
    state: &StateManager,
    save_manager: &mut SaveManager,
    career: Career,
    _operation: &MutexGuard<'_, ()>,
) -> Result<(), String> {
    let save_id = state
        .get_save_id()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "be.error.noActiveSaveSession".to_string())?;
    let (game, flushed) = state
        .get_game(|game| {
            career
                .check(game)
                .map(|()| (game.clone(), game.cash_journal_dirty_ids.clone()))
        })
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())??;
    let stats = state
        .get_stats_state(|stats| stats.clone())
        .unwrap_or_default();
    save_manager.save_game_with_stats(&game, &stats, &save_id)?;
    let flushed: HashSet<String> = flushed.into_iter().collect();
    // Bookkeeping on the game just saved, under the idle gate `install_career` also takes, so
    // it is that career: `Active` is exact here, not a way round the check above.
    state.update_game_for(Career::Active, |live| {
        live.cash_journal_dirty_ids
            .retain(|id| !flushed.contains(id));
    })?;
    Ok(())
}
