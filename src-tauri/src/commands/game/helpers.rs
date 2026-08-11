//! Small shared helpers for the game commands: error shaping, and the
//! defaults a new career is created with.

use domain::stats::StatsState;
use ofm_core::state::StateManager;

/// Surface the first concrete validation error (e.g. *which* country is unknown)
/// rather than a generic "invalid package", so a failed import is diagnosable.
/// Uses the `key?param=value` convention `resolveBackendError` understands (see
/// `src/utils/backendI18n.ts`); the `country` param is localised frontend-side.
pub(crate) fn first_package_error_message(errors: &[ofm_core::generator::PackageError]) -> String {
    let Some(first) = errors.first() else {
        return "be.error.package.invalid".to_string();
    };
    if first.params.is_empty() {
        return first.code.clone();
    }
    let query = first
        .params
        .iter()
        .map(|(key, value)| format!("{}={}", encode_error_param(key), encode_error_param(value)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{}?{}", first.code, query)
}

/// Minimal percent-encoding for error-message query params so the frontend's
/// `URLSearchParams` parse stays intact for ids/names with reserved characters.
fn encode_error_param(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => c.to_string().bytes().map(|b| format!("%{b:02X}")).collect(),
        })
        .collect()
}

pub(super) fn map_save_manager_lock_error<T>(
    result: std::sync::LockResult<T>,
) -> Result<T, String> {
    result.map_err(|_| "be.error.saveManagerUnavailable".to_string())
}

pub(super) fn require_active_stats_state(state: &StateManager) -> Result<StatsState, String> {
    state
        .get_stats_state(|stats| stats.clone())
        .ok_or("be.error.noActiveStatsSession".to_string())
}

pub(super) fn default_league_name() -> String {
    ["Premier", "Division"].join(" ")
}

pub(super) fn long_date_format() -> String {
    ['%', 'B', ' ', '%', 'd', ',', ' ', '%', 'Y']
        .into_iter()
        .collect()
}

pub(crate) fn default_save_name(manager_name: &str) -> String {
    let mut save_name = manager_name.to_string();
    save_name.push('\'');
    save_name.push('s');
    save_name.push(' ');
    save_name.push_str("Career");
    save_name
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::game::testkit::sample_stats_state;
    use std::sync::Mutex;

    #[test]
    fn require_active_stats_state_returns_backend_key_when_missing() {
        let state = StateManager::new();

        let result = require_active_stats_state(&state);

        assert_eq!(result.unwrap_err(), "be.error.noActiveStatsSession");
    }

    #[test]
    fn require_active_stats_state_clones_active_stats() {
        let state = StateManager::new();
        let stats = sample_stats_state();
        state.set_stats_state(stats.clone());

        let result = require_active_stats_state(&state).unwrap();

        assert_eq!(result.team_matches.len(), stats.team_matches.len());
        assert_eq!(result.player_matches.len(), stats.player_matches.len());
    }

    #[test]
    fn map_save_manager_lock_error_returns_backend_key_for_poisoned_mutex() {
        let mutex = Mutex::new(());
        let _ = std::panic::catch_unwind(|| {
            let _guard = mutex.lock().unwrap();
            panic!("poison save manager mutex for test");
        });

        let result = map_save_manager_lock_error(mutex.lock());

        assert_eq!(result.unwrap_err(), "be.error.saveManagerUnavailable");
    }
}
