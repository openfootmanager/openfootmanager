use log::info;
use ofm_core::job_offers::{self, JobOpportunity};
use ofm_core::state::StateManager;
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
) -> Result<serde_json::Value, String> {
    info!("[cmd] apply_for_job: team_id={}", team_id);
    // apply_for_job has no error path; mutate in place and serialize the result.
    state
        .update_game(|game| {
            let result = job_offers::apply_for_job(game, &team_id);
            serde_json::json!({
                "result": result,
                "game": game,
            })
        })
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())
}
