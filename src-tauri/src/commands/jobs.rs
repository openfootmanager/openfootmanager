use log::info;
use ofm_core::job_offers::{self, JobApplicationResult, JobOpportunity};
use ofm_core::state::StateManager;
use tauri::State;

#[tauri::command]
pub fn get_available_jobs(state: State<'_, StateManager>) -> Result<Vec<JobOpportunity>, String> {
    info!("[cmd] get_available_jobs");
    let game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;
    Ok(job_offers::get_available_jobs(&game))
}

#[tauri::command]
pub fn apply_for_job(
    state: State<'_, StateManager>,
    team_id: String,
) -> Result<serde_json::Value, String> {
    info!("[cmd] apply_for_job: team_id={}", team_id);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let result = job_offers::apply_for_job(&mut game, &team_id);
    state.set_game(game.clone());

    Ok(serde_json::json!({
        "result": match result {
            JobApplicationResult::Hired => "hired",
            JobApplicationResult::Rejected => "rejected",
            JobApplicationResult::InvalidTeam => "invalid_team",
            JobApplicationResult::AlreadyEmployed => "already_employed",
        },
        "game": game,
    }))
}
