use db::save_manager::SaveManager;
use ofm_core::state::StateManager;
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
        persist_game_under_operation(state, save_manager, &operation)?;
    }
    state.clear_game();
    state.clear_save_id();
    Ok(saved)
}

/// Persist the active career under the live-session idle gate.
pub fn persist_active_game(
    state: &StateManager,
    save_manager: &mut SaveManager,
) -> Result<(), String> {
    let operation = super::live_session::idle_operation(state)?;
    persist_game_under_operation(state, save_manager, &operation)
}

// Ordinary saves and save-on-exit snapshot dirty journal ids before I/O, then
// remove only the flushed ids so concurrent new posts remain dirty. The guard
// token keeps this shared flush protocol inside its caller's idle gate.
fn persist_game_under_operation(
    state: &StateManager,
    save_manager: &mut SaveManager,
    _operation: &MutexGuard<'_, ()>,
) -> Result<(), String> {
    let save_id = state
        .get_save_id()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "be.error.noActiveSaveSession".to_string())?;
    let (game, flushed) = state
        .get_game(|game| (game.clone(), game.cash_journal_dirty_ids.clone()))
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())?;
    let stats = state
        .get_stats_state(|stats| stats.clone())
        .unwrap_or_default();
    save_manager.save_game_with_stats(&game, &stats, &save_id)?;
    let flushed: HashSet<String> = flushed.into_iter().collect();
    state.update_game(|live| {
        live.cash_journal_dirty_ids
            .retain(|id| !flushed.contains(id));
    });
    Ok(())
}
