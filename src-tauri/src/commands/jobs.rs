use log::info;
use ofm_core::job_offers::{self, JobOpportunity};
use ofm_core::state::{Career, StateManager};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_available_jobs(
    state: State<'_, Arc<StateManager>>,
) -> Result<Vec<JobOpportunity>, String> {
    info!("[cmd] get_available_jobs");
    state
        .get_game(job_offers::get_available_jobs)
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub fn apply_for_job(
    state: State<'_, Arc<StateManager>>,
    team_id: String,
    career_token: u64,
) -> Result<serde_json::Value, String> {
    info!("[cmd] apply_for_job: team_id={}", team_id);
    apply_for_job_internal(&state, Career::Token(career_token), &team_id)
}

/// Applies for the job at `team_id` in the career `career` names; refused if another career is
/// active. `apply_for_job` has no error path, so it mutates in place and serializes the result.
pub fn apply_for_job_internal(
    state: &StateManager,
    career: Career,
    team_id: &str,
) -> Result<serde_json::Value, String> {
    state.update_game_for(career, |game| {
        let result = job_offers::apply_for_job(game, team_id);
        serde_json::json!({
            "result": result,
            "game": game,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::apply_for_job_internal;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::{Career, StateManager};

    fn state_in_career(token: u64) -> StateManager {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        game.career_token = token;
        let state = StateManager::new();
        state.set_game(game);
        state
    }

    /// Given career B is active (token 2) and a screen holds career A's token (1)
    /// When the screen applies for a job
    /// Then it is refused with the career-changed key and B's game is untouched; with B's own
    ///      token the application goes through.
    #[test]
    fn a_job_application_for_a_replaced_career_is_refused() {
        let state = state_in_career(2);
        let before = serde_json::to_value(state.get_game(|g| g.clone()).unwrap()).unwrap();

        let refused = apply_for_job_internal(&state, Career::Token(1), "team-9");

        assert_eq!(refused.err(), Some("be.error.careerChanged".to_string()));
        let after = serde_json::to_value(state.get_game(|g| g.clone()).unwrap()).unwrap();
        assert_eq!(after, before);
        let applied = apply_for_job_internal(&state, Career::Token(2), "team-9").unwrap();
        assert!(applied.get("result").is_some());
    }
}
