use log::info;
use serde::Serialize;
use tauri::State;

use ofm_core::finances::{
    BoardSupportResult, FinanceActionPreviews, MarketingCampaignResult, SponsorPitchResult,
    TeamFinanceSnapshot,
};
use ofm_core::game::Game;
use ofm_core::state::StateManager;

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

#[tauri::command]
pub async fn get_finance_snapshot(
    state: State<'_, StateManager>,
    team_id: Option<String>,
) -> Result<FinanceSnapshotCommandResponse, String> {
    get_finance_snapshot_internal(&state, team_id.as_deref())
}

fn get_finance_snapshot_internal(
    state: &StateManager,
    team_id: Option<&str>,
) -> Result<FinanceSnapshotCommandResponse, String> {
    info!("[cmd] get_finance_snapshot: team_id={:?}", team_id);

    let game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let resolved_team_id = match team_id {
        Some(team_id) => team_id.to_string(),
        None => game
            .manager
            .team_id
            .clone()
            .ok_or("be.error.noTeamAssigned".to_string())?,
    };

    let snapshot = ofm_core::finances::team_finance_snapshot(&game, &resolved_team_id)
        .ok_or("be.error.managedTeamNotFound".to_string())?;
    let previews = ofm_core::finances::finance_action_previews(&game, &resolved_team_id)
        .unwrap_or_default();

    Ok(FinanceSnapshotCommandResponse { snapshot, previews })
}

#[tauri::command]
pub async fn request_board_support(
    state: State<'_, StateManager>,
) -> Result<BoardSupportCommandResponse, String> {
    request_board_support_internal(&state)
}

#[tauri::command]
pub async fn request_sponsor_pitch(
    state: State<'_, StateManager>,
) -> Result<SponsorPitchCommandResponse, String> {
    request_sponsor_pitch_internal(&state)
}

#[tauri::command]
pub async fn request_marketing_campaign(
    state: State<'_, StateManager>,
) -> Result<MarketingCampaignCommandResponse, String> {
    request_marketing_campaign_internal(&state)
}

fn request_board_support_internal(
    state: &StateManager,
) -> Result<BoardSupportCommandResponse, String> {
    info!("[cmd] request_board_support");

    let mut game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned".to_string())?;

    let result = ofm_core::finances::request_board_support(&mut game, &team_id)?;

    state.set_game(game.clone());
    Ok(BoardSupportCommandResponse { game, result })
}

fn request_sponsor_pitch_internal(
    state: &StateManager,
) -> Result<SponsorPitchCommandResponse, String> {
    info!("[cmd] request_sponsor_pitch");

    let mut game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned".to_string())?;

    let result = ofm_core::finances::request_sponsor_pitch(&mut game, &team_id)?;

    state.set_game(game.clone());
    Ok(SponsorPitchCommandResponse { game, result })
}

fn request_marketing_campaign_internal(
    state: &StateManager,
) -> Result<MarketingCampaignCommandResponse, String> {
    info!("[cmd] request_marketing_campaign");

    let mut game = state
        .get_game(|g: &Game| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned".to_string())?;

    let result = ofm_core::finances::request_marketing_campaign(&mut game, &team_id)?;

    state.set_game(game.clone());
    Ok(MarketingCampaignCommandResponse { game, result })
}

#[cfg(test)]
mod tests {
    use super::{
        get_finance_snapshot_internal, request_board_support_internal,
        request_marketing_campaign_internal, request_sponsor_pitch_internal,
    };
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
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
        player.wage = 52_000;
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

    #[test]
    fn get_finance_snapshot_internal_returns_managed_team_snapshot() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = get_finance_snapshot_internal(&state, None).expect("response");

        assert_eq!(response.snapshot.annual_wage_bill, 52_000);
        assert_eq!(response.snapshot.weekly_wage_spend, 1_000);
        assert_eq!(response.snapshot.weekly_wage_budget, 120_000 / 52);
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

        let response = request_board_support_internal(&state).expect("response");

        assert!(response.result.support_amount >= 150_000);
        assert!(response.game.teams[0].finance > 0);
        assert_eq!(response.game.manager.satisfaction, 58);

        let stored_game = state.get_game(|current| current.clone()).expect("stored game");
        assert!(stored_game.teams[0].finance > 0);
        assert_eq!(stored_game.manager.satisfaction, 58);
    }

    #[test]
    fn request_sponsor_pitch_internal_creates_pending_offer() {
        let state = StateManager::new();
        let mut game = make_game();
        game.teams[0].wage_budget = 50_000;
        state.set_game(game);

        let response = request_sponsor_pitch_internal(&state).expect("response");

        assert!(response.result.weekly_amount >= 40_000);
        assert!(response
            .game
            .messages
            .iter()
            .any(|message| message.id == response.result.message_id));

        let stored_game = state.get_game(|current| current.clone()).expect("stored game");
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

        let response = request_marketing_campaign_internal(&state).expect("response");

        assert!(response.result.net_income > 0);
        assert_eq!(response.result.net_income, response.result.gross_revenue - response.result.campaign_cost);
        assert!(response
            .game
            .messages
            .iter()
            .any(|message| message.id == response.result.message_id));
        assert_eq!(
            response
                .game
                .teams[0]
                .financial_ledger
                .iter()
                .filter(|entry| entry.kind == domain::team::FinancialTransactionKind::CommercialCampaign)
                .count(),
            2
        );

        let stored_game = state.get_game(|current| current.clone()).expect("stored game");
        assert!(stored_game.teams[0].finance > -40_000);
        assert!(stored_game
            .messages
            .iter()
            .any(|message| message.id == response.result.message_id));
    }
}