use log::info;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

use ofm_core::finances::{
    BoardSupportResult, EnvelopeReallocation, FinanceActionPreviews, MarketingCampaignResult,
    SponsorPitchResult, TeamFinanceSnapshot,
};
use ofm_core::game::Game;
use ofm_core::state::{Career, StateManager};

#[derive(Debug, Clone, Serialize)]
pub struct FinanceSnapshotCommandResponse {
    pub snapshot: TeamFinanceSnapshot,
    pub previews: FinanceActionPreviews,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoardSupportCommandResponse {
    pub game: Game,
    pub result: BoardSupportResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct SponsorPitchCommandResponse {
    pub game: Game,
    pub result: SponsorPitchResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketingCampaignCommandResponse {
    pub game: Game,
    pub result: MarketingCampaignResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvelopeReallocationCommandResponse {
    pub game: Game,
    pub result: EnvelopeReallocation,
}

#[tauri::command]
pub async fn get_finance_snapshot(
    state: State<'_, Arc<StateManager>>,
    team_id: Option<String>,
) -> Result<FinanceSnapshotCommandResponse, String> {
    get_finance_snapshot_internal(&state, team_id.as_deref())
}

pub fn get_finance_snapshot_internal(
    state: &StateManager,
    team_id: Option<&str>,
) -> Result<FinanceSnapshotCommandResponse, String> {
    info!("[cmd] get_finance_snapshot: team_id={:?}", team_id);

    state
        .get_game(|game| {
            let resolved_team_id = match team_id {
                Some(team_id) => team_id.to_string(),
                None => game
                    .manager
                    .team_id
                    .clone()
                    .ok_or_else(|| "be.error.noTeamAssigned".to_string())?,
            };

            let snapshot = ofm_core::finances::team_finance_snapshot(game, &resolved_team_id)
                .ok_or_else(|| "be.error.managedTeamNotFound".to_string())?;
            let previews = ofm_core::finances::finance_action_previews(game, &resolved_team_id)
                .unwrap_or_default();

            Ok(FinanceSnapshotCommandResponse { snapshot, previews })
        })
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())?
}

#[tauri::command]
pub async fn request_board_support(
    state: State<'_, Arc<StateManager>>,
    career_token: u64,
) -> Result<BoardSupportCommandResponse, String> {
    request_board_support_internal(&state, Career::Token(career_token))
}

#[tauri::command]
pub async fn request_sponsor_pitch(
    state: State<'_, Arc<StateManager>>,
    career_token: u64,
) -> Result<SponsorPitchCommandResponse, String> {
    request_sponsor_pitch_internal(&state, Career::Token(career_token))
}

#[tauri::command]
pub async fn request_marketing_campaign(
    state: State<'_, Arc<StateManager>>,
    career_token: u64,
) -> Result<MarketingCampaignCommandResponse, String> {
    request_marketing_campaign_internal(&state, Career::Token(career_token))
}

#[tauri::command]
pub async fn reallocate_board_envelopes(
    state: State<'_, Arc<StateManager>>,
    career_token: u64,
    wage_delta_weekly: i64,
) -> Result<EnvelopeReallocationCommandResponse, String> {
    reallocate_board_envelopes_internal(&state, Career::Token(career_token), wage_delta_weekly)
}

pub fn request_board_support_internal(
    state: &StateManager,
    career: Career,
) -> Result<BoardSupportCommandResponse, String> {
    info!("[cmd] request_board_support");

    state.update_game_for(career, |game| {
        let team_id = game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?;

        let result = ofm_core::finances::request_board_support(game, &team_id)?;
        Ok(BoardSupportCommandResponse {
            game: game.clone(),
            result,
        })
    })?
}

pub fn request_sponsor_pitch_internal(
    state: &StateManager,
    career: Career,
) -> Result<SponsorPitchCommandResponse, String> {
    info!("[cmd] request_sponsor_pitch");

    state.update_game_for(career, |game| {
        let team_id = game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?;

        let result = ofm_core::finances::request_sponsor_pitch(game, &team_id)?;
        Ok(SponsorPitchCommandResponse {
            game: game.clone(),
            result,
        })
    })?
}

pub fn request_marketing_campaign_internal(
    state: &StateManager,
    career: Career,
) -> Result<MarketingCampaignCommandResponse, String> {
    info!("[cmd] request_marketing_campaign");

    state.update_game_for(career, |game| {
        let team_id = game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?;

        let result = ofm_core::finances::request_marketing_campaign(game, &team_id)?;
        Ok(MarketingCampaignCommandResponse {
            game: game.clone(),
            result,
        })
    })?
}

pub fn reallocate_board_envelopes_internal(
    state: &StateManager,
    career: Career,
    wage_delta_weekly: i64,
) -> Result<EnvelopeReallocationCommandResponse, String> {
    info!(
        "[cmd] reallocate_board_envelopes: wage_delta_weekly={}",
        wage_delta_weekly
    );

    state.update_game_for(career, |game| {
        let team_id = game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?;

        let result =
            ofm_core::finances::reallocate_board_envelopes(game, &team_id, wage_delta_weekly)?;
        Ok(EnvelopeReallocationCommandResponse {
            game: game.clone(),
            result,
        })
    })?
}

#[cfg(test)]
mod tests {
    use super::{
        get_finance_snapshot_internal, reallocate_board_envelopes_internal,
        request_board_support_internal, request_marketing_campaign_internal,
        request_sponsor_pitch_internal,
    };
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::Career;
    use ofm_core::state::StateManager;

    fn make_team() -> Team {
        let mut team = Team::new(
            "team-1".to_string(),
            "User FC".to_string(),
            "USR".to_string(),
            "England".to_string(),
            "London".to_string(),
            "User Ground".to_string(),
            25_000,
        );
        team.finance = 500_000;
        team.wage_budget = 120_000;
        team.transfer_budget = 300_000;
        team.manager_id = Some("manager-1".to_string());
        team
    }

    fn make_player() -> Player {
        let attrs = PlayerAttributes {
            pace: 65,
            stamina: 65,
            strength: 65,
            agility: 65,
            passing: 65,
            shooting: 65,
            tackling: 65,
            dribbling: 65,
            defending: 65,
            positioning: 65,
            vision: 65,
            decisions: 65,
            composure: 65,
            aggression: 50,
            teamwork: 65,
            leadership: 50,
            handling: 20,
            reflexes: 30,
            aerial: 60,
        };
        let mut player = Player::new(
            "player-1".to_string(),
            "Player".to_string(),
            "Test Player".to_string(),
            "1998-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            attrs,
        );
        player.team_id = Some("team-1".to_string());
        player.stage_wage(1_000);
        player
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team-1".to_string());

        Game::new(
            clock,
            manager,
            vec![make_team()],
            vec![make_player()],
            vec![],
            vec![],
        )
    }

    /// Given career B is active (token 2) and a screen holds career A's token (1)
    /// When the board-support, sponsor-pitch and marketing requests arrive with A's token
    /// Then each is refused with the career-changed key and career B's books are untouched.
    #[test]
    fn finance_requests_for_a_replaced_career_are_refused() {
        let state = StateManager::new();
        let mut game = make_game();
        game.career_token = 2;
        state.set_game(game);
        let before = crate::commands::game_json::active_game_json(&state);
        let stale = Career::Token(1);
        let key = Some("be.error.careerChanged".to_string());

        assert_eq!(request_board_support_internal(&state, stale).err(), key);
        assert_eq!(request_sponsor_pitch_internal(&state, stale).err(), key);
        assert_eq!(
            request_marketing_campaign_internal(&state, stale).err(),
            key
        );
        assert_eq!(
            reallocate_board_envelopes_internal(&state, stale, 1_000).err(),
            key
        );

        let after = crate::commands::game_json::active_game_json(&state);
        assert_eq!(after, before);
    }

    #[test]
    fn get_finance_snapshot_internal_returns_managed_team_snapshot() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = get_finance_snapshot_internal(&state, None).expect("response");

        assert_eq!(response.snapshot.annual_wage_bill, 1_000);
        assert_eq!(response.snapshot.weekly_wage_spend, 1_000);
        assert_eq!(response.snapshot.weekly_wage_budget, 120_000);
        assert!(response.previews.board_support.is_none());
        assert!(response.previews.sponsor_pitch.is_none());
        assert!(response.previews.marketing_campaign.is_none());
    }

    #[test]
    fn get_finance_snapshot_internal_includes_recovery_previews_for_pressured_club() {
        let state = StateManager::new();
        let mut game = make_game();
        game.teams[0].finance = -25_000;
        game.teams[0].wage_budget = 40_000;
        state.set_game(game);

        let response = get_finance_snapshot_internal(&state, None).expect("response");

        assert!(response.previews.board_support.is_some());
        assert!(response.previews.sponsor_pitch.is_some());
        assert!(response.previews.marketing_campaign.is_some());
    }

    #[test]
    fn request_board_support_internal_updates_managed_team_state() {
        let state = StateManager::new();
        let mut game = make_game();
        game.teams[0].finance = -25_000;
        game.manager.satisfaction = 70;
        state.set_game(game);

        let response = request_board_support_internal(&state, Career::Active).expect("response");

        assert!(response.result.support_amount >= 150_000);
        assert!(response.game.teams[0].finance > 0);
        assert_eq!(response.game.manager.satisfaction, 58);

        let stored_game = state
            .get_game(|current| current.clone())
            .expect("stored game");
        assert!(stored_game.teams[0].finance > 0);
        assert_eq!(stored_game.manager.satisfaction, 58);
    }

    #[test]
    fn request_sponsor_pitch_internal_creates_pending_offer() {
        let state = StateManager::new();
        let mut game = make_game();
        game.teams[0].wage_budget = 500;
        state.set_game(game);

        let response = request_sponsor_pitch_internal(&state, Career::Active).expect("response");

        assert!(response.result.weekly_amount >= 40_000);
        assert!(response
            .game
            .messages
            .iter()
            .any(|message| message.id == response.result.message_id));

        let stored_game = state
            .get_game(|current| current.clone())
            .expect("stored game");
        assert!(stored_game
            .messages
            .iter()
            .any(|message| message.id == response.result.message_id));
    }

    #[test]
    fn request_marketing_campaign_internal_updates_managed_team_state() {
        let state = StateManager::new();
        let mut game = make_game();
        game.teams[0].wage_budget = 50_000;
        game.teams[0].finance = -40_000;
        state.set_game(game);

        let response =
            request_marketing_campaign_internal(&state, Career::Active).expect("response");

        assert!(response.result.net_income > 0);
        assert_eq!(
            response.result.net_income,
            response.result.gross_revenue - response.result.campaign_cost
        );
        assert!(response
            .game
            .messages
            .iter()
            .any(|message| message.id == response.result.message_id));
        assert_eq!(
            response.game.teams[0]
                .financial_ledger
                .iter()
                .filter(|entry| entry.kind
                    == domain::team::FinancialTransactionKind::CommercialCampaign)
                .count(),
            2
        );

        let stored_game = state
            .get_game(|current| current.clone())
            .expect("stored game");
        assert!(stored_game.teams[0].finance > -40_000);
        assert!(stored_game
            .messages
            .iter()
            .any(|message| message.id == response.result.message_id));
    }

    /// Given a managed club with wage and transfer limits, when weekly wage
    /// permission is raised, then cash is untouched and the transfer limit
    /// falls by 52× the weekly delta.
    #[test]
    fn reallocate_board_envelopes_internal_moves_permission_not_cash() {
        let state = StateManager::new();
        state.set_game(make_game());
        let finance_before = 500_000;

        let response =
            reallocate_board_envelopes_internal(&state, Career::Active, 5_000).expect("reallocate");
        assert_eq!(response.game.teams[0].finance, finance_before);
        assert_eq!(response.game.teams[0].wage_budget, 125_000);
        assert_eq!(response.game.teams[0].transfer_budget, 40_000);
        assert_eq!(response.result.wage_budget, 125_000);
        assert_eq!(response.result.transfer_budget, 40_000);

        let stored_game = state
            .get_game(|current| current.clone())
            .expect("stored game");
        assert_eq!(stored_game.teams[0].finance, finance_before);
        assert_eq!(stored_game.teams[0].wage_budget, 125_000);
        assert_eq!(stored_game.teams[0].transfer_budget, 40_000);
    }

    /// Given no active career, when envelopes are reallocated, then the
    /// no-session key is returned.
    #[test]
    fn reallocate_board_envelopes_internal_reports_no_session() {
        let state = StateManager::new();
        let error = reallocate_board_envelopes_internal(&state, Career::Active, 1_000)
            .expect_err("no game");
        assert_eq!(error, "be.error.noActiveGameSession");
    }
}
