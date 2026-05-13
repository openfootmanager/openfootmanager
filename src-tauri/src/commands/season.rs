use log::info;
use tauri::State;

use ofm_core::state::StateManager;

#[tauri::command]
pub fn check_season_complete(state: State<'_, StateManager>) -> Result<bool, String> {
    log::debug!("[cmd] check_season_complete");
    let game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;
    Ok(ofm_core::end_of_season::is_season_complete(&game))
}

#[tauri::command]
pub fn advance_to_next_season(state: State<'_, StateManager>) -> Result<serde_json::Value, String> {
    info!("[cmd] advance_to_next_season");
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    if !ofm_core::end_of_season::is_season_complete(&game) {
        return Err("be.error.seasonNotComplete".to_string());
    }

    let summary = ofm_core::end_of_season::process_end_of_season(&mut game);

    // End-of-season objective evaluation may have dropped satisfaction — check firing
    ofm_core::firing::check_manager_firing(&mut game);

    state.set_game(game.clone());

    if game.manager.team_id.is_none() {
        return Ok(serde_json::json!({
            "action": "fired",
            "game": game,
            "summary": summary,
        }));
    }

    Ok(serde_json::json!({
        "game": game,
        "summary": summary,
    }))
}

#[tauri::command]
pub fn get_season_awards(
    state: State<'_, StateManager>,
) -> Result<ofm_core::season_awards::SeasonAwards, String> {
    log::debug!("[cmd] get_season_awards");
    let game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;
    Ok(ofm_core::season_awards::compute_season_awards(&game))
}
