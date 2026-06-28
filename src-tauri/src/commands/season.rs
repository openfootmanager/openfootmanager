use log::info;
use std::sync::Arc;
use tauri::State;

use ofm_core::state::StateManager;

#[tauri::command]
pub fn check_season_complete(state: State<'_, Arc<StateManager>>) -> Result<bool, String> {
    log::debug!("[cmd] check_season_complete");
    state
        .get_game(ofm_core::end_of_season::is_season_complete)
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub fn advance_to_next_season(
    state: State<'_, Arc<StateManager>>,
) -> Result<serde_json::Value, String> {
    info!("[cmd] advance_to_next_season");
    // Season-completeness is validated before any rollover mutation.
    state
        .update_game(|game| {
            if !ofm_core::end_of_season::is_season_complete(game) {
                return Err("be.error.seasonNotComplete".to_string());
            }

            let summary = ofm_core::end_of_season::process_end_of_season(game);

            // End-of-season objective evaluation may have dropped satisfaction — check firing
            ofm_core::firing::check_manager_firing(game);

            if game.manager.team_id.is_none() {
                Ok(serde_json::json!({
                    "action": "fired",
                    "game": game,
                    "summary": summary,
                }))
            } else {
                Ok(serde_json::json!({
                    "game": game,
                    "summary": summary,
                }))
            }
        })
        .unwrap_or_else(|| Err("be.error.noActiveGameSession".to_string()))
}

#[tauri::command]
pub fn get_season_awards(
    state: State<'_, Arc<StateManager>>,
) -> Result<ofm_core::season_awards::SeasonAwards, String> {
    log::debug!("[cmd] get_season_awards");
    state
        .get_game(|game| {
            // The awards screen shows the race within the user's own division.
            let user_team_id = game.manager.team_id.clone().unwrap_or_default();
            match ofm_core::end_of_season::user_division(game, &user_team_id) {
                Some(division) => {
                    ofm_core::season_awards::compute_division_season_awards(game, division)
                }
                None => ofm_core::season_awards::compute_season_awards(game),
            }
        })
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())
}
