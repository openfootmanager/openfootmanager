//! Serialize the application's save, clock and live-session lifecycle operations.
//!
//! The desktop application owns one active game. Keeping the gate here avoids
//! making the persistence crate know about a transient live-match session. The
//! gate also closes the gap between checking the session and writing a save or
//! starting a match; finish holds it until the complete result is applied.
//!
//! Same-process callers queue while another operation holds the lock. After
//! acquiring it, save/exit/advance/start refuse if a live session is still present;
//! finish may take the session and apply it. Successful career installation uses
//! the same lock and discards the previous career's transient session. Ordinary
//! same-career writes retain it; `StateManager::clear_game` discards it on clear.
//! Exit holds the lock through both persistence and clearing the active state.
use ofm_core::state::StateManager;
use std::sync::{Mutex, MutexGuard};

static OPERATION: Mutex<()> = Mutex::new(());

pub(crate) fn operation() -> MutexGuard<'static, ()> {
    OPERATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn ensure_idle(state: &StateManager) -> Result<(), String> {
    if state.with_live_match(|_| ()).is_some() {
        Err("be.error.liveMatch.inProgress".to_string())
    } else {
        Ok(())
    }
}

pub(crate) fn idle_operation(state: &StateManager) -> Result<MutexGuard<'static, ()>, String> {
    let guard = operation();
    ensure_idle(state)?;
    Ok(guard)
}
