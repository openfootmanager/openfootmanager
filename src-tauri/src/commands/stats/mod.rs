use std::sync::Arc;
mod dto;
mod player;
mod shared;
mod team;

#[cfg(test)]
mod tests;

use ofm_core::state::StateManager;
use tauri::State;

use self::dto::{
    PlayerMatchHistoryEntryDto, PlayerStatsOverviewDto, TeamMatchHistoryEntryDto,
    TeamStatsOverviewDto,
};
pub use self::player::{get_player_match_history_internal, get_player_stats_overview_internal};
pub use self::team::{get_team_match_history_internal, get_team_stats_overview_internal};

#[tauri::command]
pub fn get_player_match_history(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    limit: Option<usize>,
) -> Result<Vec<PlayerMatchHistoryEntryDto>, String> {
    get_player_match_history_internal(&state, &player_id, limit)
}

#[tauri::command]
pub fn get_player_stats_overview(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<PlayerStatsOverviewDto, String> {
    get_player_stats_overview_internal(&state, &player_id)
}

#[tauri::command]
pub fn get_team_stats_overview(
    state: State<'_, Arc<StateManager>>,
    team_id: String,
) -> Result<Option<TeamStatsOverviewDto>, String> {
    get_team_stats_overview_internal(&state, &team_id)
}

#[tauri::command]
pub fn get_team_match_history(
    state: State<'_, Arc<StateManager>>,
    team_id: String,
    limit: Option<usize>,
) -> Result<Vec<TeamMatchHistoryEntryDto>, String> {
    get_team_match_history_internal(&state, &team_id, limit)
}
