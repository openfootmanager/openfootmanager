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

#[cfg(test)]
thread_local! {
    pub(crate) static ROLE_INITIAL_SNAPSHOTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    pub(crate) static ROLE_HANDOFF_PROJECTIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

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

/// Hold the lifecycle gate across a player move and hand off only roles it changed.
pub(crate) fn player_role_operation<T>(
    state: &StateManager,
    player_id: &str,
    write: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let _operation = operation();
    if state.with_live_match(|_| ()).is_none() {
        return write();
    }
    let before = state.get_game(|game| {
        #[cfg(test)]
        ROLE_INITIAL_SNAPSHOTS.with(|count| count.set(count.get() + 1));
        let team_id = game
            .players
            .iter()
            .find(|player| player.id == player_id)
            .and_then(|player| player.team_id.clone());
        let roles = game
            .teams
            .iter()
            .filter_map(|team| {
                team.player_roles
                    .get(player_id)
                    .map(|role| (team.id.clone(), role.clone()))
            })
            .collect::<std::collections::HashMap<_, _>>();
        (team_id, roles)
    });
    let result = write()?;
    let projection = state.get_game(|game| {
        let mut changed = game
            .teams
            .iter()
            .filter(|team| {
                before.as_ref().and_then(|(_, roles)| roles.get(&team.id))
                    != team.player_roles.get(player_id)
            })
            .map(|team| (team.id.clone(), team.player_roles.get(player_id).cloned()))
            .collect::<Vec<_>>();
        let current_team = game
            .players
            .iter()
            .find(|player| player.id == player_id)
            .and_then(|player| player.team_id.as_ref());
        if let Some(old_team) = before
            .as_ref()
            .and_then(|(team, _)| team.as_ref())
            .filter(|team| {
                current_team != Some(*team)
                    && !changed
                        .iter()
                        .any(|(changed_team, _)| changed_team == *team)
            })
        {
            changed.push((old_team.clone(), None));
        }
        #[cfg(test)]
        ROLE_HANDOFF_PROJECTIONS.with(|count| count.set(count.get() + 1));
        (game.career_token, changed)
    });
    if let Some((career_token, changed)) = projection {
        for (team_id, role) in changed {
            let result = state.with_live_match(|session| {
                session.follow_team_role_before_kick_off(
                    career_token,
                    &team_id,
                    player_id,
                    role.as_ref(),
                )
            });
            if let Some(Err(error)) = result {
                log::warn!("[live_match] could not follow a changed saved role: {error}");
            }
        }
    }
    Ok(result)
}
