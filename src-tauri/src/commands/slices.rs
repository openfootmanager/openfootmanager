use std::sync::Arc;

use tauri::State;

use ofm_core::slices::competitions::{CompetitionsQuery, CompetitionsView, query_competitions};
use ofm_core::slices::inbox::{MessagesQuery, query_messages};
use ofm_core::slices::news::{NewsFeed, NewsFeedQuery, query_news_feed};
use ofm_core::slices::players::{PlayersPage, PlayersPageQuery, query_page};
use ofm_core::slices::schedule::{ScheduleQuery, ScheduleSlice, query_schedule};
use ofm_core::slices::session::{SessionState, SessionStateQuery, project_session};
use ofm_core::slices::squad::query_squad;
use ofm_core::slices::staff::{StaffSlice, query_staff};
use ofm_core::slices::teams::{
    TeamsDirectory, TeamsDirectoryQuery, query_directory,
};
use ofm_core::state::StateManager;
use domain::message::InboxMessage;

const NO_ACTIVE_GAME: &str = "be.error.noActiveGameSession";

#[tauri::command]
pub async fn get_players_page(
    state: State<'_, Arc<StateManager>>,
    query: PlayersPageQuery,
) -> Result<PlayersPage, String> {
    state
        .get_game(|game| query_page(game, &query))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_teams_directory(
    state: State<'_, Arc<StateManager>>,
    query: TeamsDirectoryQuery,
) -> Result<TeamsDirectory, String> {
    state
        .get_game(|game| query_directory(game, &query))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_schedule(
    state: State<'_, Arc<StateManager>>,
    query: ScheduleQuery,
) -> Result<ScheduleSlice, String> {
    state
        .get_game(|game| query_schedule(game, &query))
        .flatten()
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_news_feed(
    state: State<'_, Arc<StateManager>>,
    query: NewsFeedQuery,
) -> Result<NewsFeed, String> {
    state
        .get_game(|game| query_news_feed(game, &query))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_messages_page(
    state: State<'_, Arc<StateManager>>,
    query: MessagesQuery,
) -> Result<Vec<InboxMessage>, String> {
    state
        .get_game(|game| query_messages(game, &query))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_competitions_view(
    state: State<'_, Arc<StateManager>>,
    query: CompetitionsQuery,
) -> Result<CompetitionsView, String> {
    state
        .get_game(|game| query_competitions(game, &query))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_session_state(
    state: State<'_, Arc<StateManager>>,
    _query: SessionStateQuery,
) -> Result<SessionState, String> {
    state
        .get_game(|game| project_session(game))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_squad(
    state: State<'_, Arc<StateManager>>,
    team_id: String,
) -> Result<Vec<domain::player::Player>, String> {
    state
        .get_game(|game| query_squad(game, &team_id))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_staff(
    state: State<'_, Arc<StateManager>>,
    team_id: String,
) -> Result<StaffSlice, String> {
    state
        .get_game(|game| query_staff(game, &team_id))
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}
