use db::save_manager::SaveManager;
use ofm_core::state::StateManager;
use std::collections::HashSet;

/// Snapshot dirty journal ids, persist `&Game`, then drop flushed ids on the live Game.
///
/// Every `StateManager`-owned save must go through this helper so MCP autosave
/// and Tauri `save_game` share one flush protocol. Collision on insert is a
/// no-op (`INSERT OR IGNORE`).
pub fn persist_active_game(
    state: &StateManager,
    save_manager: &mut SaveManager,
) -> Result<(), String> {
    let _operation = super::live_session::idle_operation(state)?;
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
