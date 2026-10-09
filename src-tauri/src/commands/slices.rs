use std::sync::Arc;

use tauri::State;

use domain::message::InboxMessage;
use ofm_core::slices::competitions::{query_competitions, CompetitionsQuery, CompetitionsView};
use ofm_core::slices::inbox::{query_messages, MessagesQuery};
use ofm_core::slices::news::{query_news_feed, NewsFeed, NewsFeedQuery};
use ofm_core::slices::players::{query_page, PlayersPage, PlayersPageQuery};
use ofm_core::slices::schedule::{query_schedule, ScheduleQuery, ScheduleSlice};
use ofm_core::slices::session::{project_session, SessionState, SessionStateQuery};
use ofm_core::slices::squad::{query_squad, SquadPlayer};
use ofm_core::slices::staff::{query_staff, StaffSlice};
use ofm_core::slices::teams::{query_directory, TeamsDirectory, TeamsDirectoryQuery};
use ofm_core::state::StateManager;

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
        .get_game(project_session)
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

#[tauri::command]
pub async fn get_squad(
    state: State<'_, Arc<StateManager>>,
    team_id: String,
) -> Result<Vec<SquadPlayer>, String> {
    get_squad_internal(&state, &team_id)
}

fn get_squad_internal(state: &StateManager, team_id: &str) -> Result<Vec<SquadPlayer>, String> {
    state
        .get_game(|game| query_squad(game, team_id))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Given no active game, when the roster adapter reads it,
    /// then the existing translated session error is returned.
    #[test]
    fn the_squad_adapter_reports_a_missing_session() {
        assert_eq!(
            get_squad_internal(&StateManager::new(), "club").unwrap_err(),
            NO_ACTIVE_GAME
        );
    }
    /// Given a thin club roster, when the IPC adapter reads the active session,
    /// then its flattened player data carries the backend eligibility verdict.
    #[test]
    fn the_squad_adapter_projects_call_ups_from_the_live_session() {
        use chrono::TimeZone;
        use domain::manager::Manager;
        use domain::player::{Player, PlayerAttributes, Position, SquadRole};
        let attrs: PlayerAttributes = serde_json::from_value(serde_json::json!({
            "pace": 60, "stamina": 60, "strength": 60, "agility": 60, "passing": 60,
            "shooting": 60, "tackling": 60, "dribbling": 60, "defending": 60,
            "positioning": 60, "vision": 60, "decisions": 60, "composure": 60,
            "aggression": 60, "teamwork": 60, "leadership": 60,
            "handling": 60, "reflexes": 60, "aerial": 60
        }))
        .unwrap();
        let mut keeper = Player::new(
            "keeper".into(),
            "Keeper".into(),
            "Senior Keeper".into(),
            "2000-01-01".into(),
            "GB".into(),
            Position::Goalkeeper,
            attrs.clone(),
        );
        keeper.team_id = Some("club".into());
        let mut youth = Player::new(
            "youth".into(),
            "Youth".into(),
            "Called Youth".into(),
            "2008-01-01".into(),
            "GB".into(),
            Position::Forward,
            attrs,
        );
        youth.team_id = Some("club".into());
        youth.squad_role = SquadRole::Youth;
        let state = StateManager::new();
        state.set_game(ofm_core::game::Game::new(
            ofm_core::clock::GameClock::new(
                chrono::Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap(),
            ),
            Manager::new(
                "m".into(),
                "M".into(),
                "Gr".into(),
                "1980-01-01".into(),
                "GB".into(),
            ),
            vec![],
            vec![keeper, youth],
            vec![],
            vec![],
        ));
        let value = serde_json::to_value(get_squad_internal(&state, "club").unwrap()).unwrap();
        let rows = value.as_array().unwrap();
        assert_eq!(rows.len(), 2);
        let youth = rows.iter().find(|p| p["id"] == "youth").unwrap();
        assert_eq!(youth["match_day_eligible"], true);
        assert_eq!(youth["squad_role"], "Youth");
        assert!(get_squad_internal(&state, "other-club").unwrap().is_empty());
    }
}
