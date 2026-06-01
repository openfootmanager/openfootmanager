use domain::negotiation::NegotiationFeedback;
use domain::player::Position;
use log::info;
use tauri::State;

use ofm_core::game::Game;
use ofm_core::game::{YouthScoutingObjective, YouthScoutingRegion};
use ofm_core::state::StateManager;
use ofm_core::transfers::{
    TransferBidFinancialProjection, TransferNegotiationDecision, TransferNegotiationOutcome,
};

const INVALID_YOUTH_SCOUTING_REGION_ERROR: &str =
    "be.error.transfers.invalidYouthScoutingRegion";
const INVALID_YOUTH_SCOUTING_OBJECTIVE_ERROR: &str =
    "be.error.transfers.invalidYouthScoutingObjective";
const INVALID_YOUTH_SCOUTING_TARGET_POSITION_ERROR: &str =
    "be.error.transfers.invalidYouthScoutingTargetPosition";

#[derive(Debug, Clone, serde::Serialize)]
pub struct TransferNegotiationCommandResponse {
    pub decision: TransferNegotiationDecision,
    pub suggested_fee: Option<u64>,
    pub is_terminal: bool,
    pub feedback: NegotiationFeedback,
    pub game: Game,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TransferBidFinancialProjectionCommandResponse {
    pub projection: TransferBidFinancialProjection,
}

#[tauri::command]
pub fn toggle_transfer_list(
    state: State<'_, StateManager>,
    player_id: String,
) -> Result<Game, String> {
    toggle_transfer_list_internal(&state, &player_id)
}

fn toggle_transfer_list_internal(state: &StateManager, player_id: &str) -> Result<Game, String> {
    info!("[cmd] toggle_transfer_list: player_id={}", player_id);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
        p.transfer_listed = !p.transfer_listed;
    } else {
        return Err("be.error.playerNotFound".into());
    }
    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn toggle_loan_list(state: State<'_, StateManager>, player_id: String) -> Result<Game, String> {
    toggle_loan_list_internal(&state, &player_id)
}

fn toggle_loan_list_internal(state: &StateManager, player_id: &str) -> Result<Game, String> {
    info!("[cmd] toggle_loan_list: player_id={}", player_id);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
        p.loan_listed = !p.loan_listed;
    } else {
        return Err("be.error.playerNotFound".into());
    }
    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn make_transfer_bid(
    state: State<'_, StateManager>,
    player_id: String,
    fee: u64,
) -> Result<TransferNegotiationCommandResponse, String> {
    make_transfer_bid_internal(&state, &player_id, fee)
}

#[tauri::command]
pub fn preview_transfer_bid_financial_impact(
    state: State<'_, StateManager>,
    player_id: String,
    fee: u64,
) -> Result<TransferBidFinancialProjectionCommandResponse, String> {
    preview_transfer_bid_financial_impact_internal(&state, &player_id, fee)
}

fn make_transfer_bid_internal(
    state: &StateManager,
    player_id: &str,
    fee: u64,
) -> Result<TransferNegotiationCommandResponse, String> {
    info!(
        "[cmd] make_transfer_bid: player_id={}, fee={}",
        player_id, fee
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let result = ofm_core::transfers::make_transfer_bid(&mut game, player_id, fee)?;
    state.set_game(game.clone());

    Ok(map_transfer_negotiation_response(result, game))
}

fn preview_transfer_bid_financial_impact_internal(
    state: &StateManager,
    player_id: &str,
    fee: u64,
) -> Result<TransferBidFinancialProjectionCommandResponse, String> {
    info!(
        "[cmd] preview_transfer_bid_financial_impact: player_id={}, fee={}",
        player_id, fee
    );

    let game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let projection =
        ofm_core::transfers::project_transfer_bid_financial_impact(&game, player_id, fee)?;

    Ok(TransferBidFinancialProjectionCommandResponse { projection })
}

#[tauri::command]
pub fn respond_to_offer(
    state: State<'_, StateManager>,
    player_id: String,
    offer_id: String,
    accept: bool,
) -> Result<Game, String> {
    respond_to_offer_internal(&state, &player_id, &offer_id, accept)
}

fn respond_to_offer_internal(
    state: &StateManager,
    player_id: &str,
    offer_id: &str,
    accept: bool,
) -> Result<Game, String> {
    info!(
        "[cmd] respond_to_offer: player_id={}, offer_id={}, accept={}",
        player_id, offer_id, accept
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    ofm_core::transfers::respond_to_offer(&mut game, player_id, offer_id, accept)?;
    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn counter_offer(
    state: State<'_, StateManager>,
    player_id: String,
    offer_id: String,
    requested_fee: u64,
) -> Result<TransferNegotiationCommandResponse, String> {
    counter_offer_internal(&state, &player_id, &offer_id, requested_fee)
}

fn counter_offer_internal(
    state: &StateManager,
    player_id: &str,
    offer_id: &str,
    requested_fee: u64,
) -> Result<TransferNegotiationCommandResponse, String> {
    info!(
        "[cmd] counter_offer: player_id={}, offer_id={}, requested_fee={}",
        player_id, offer_id, requested_fee
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let result = ofm_core::transfers::counter_offer(&mut game, player_id, offer_id, requested_fee)?;
    state.set_game(game.clone());

    Ok(map_transfer_negotiation_response(result, game))
}

fn map_transfer_negotiation_response(
    outcome: TransferNegotiationOutcome,
    game: Game,
) -> TransferNegotiationCommandResponse {
    TransferNegotiationCommandResponse {
        decision: outcome.decision,
        suggested_fee: outcome.suggested_fee,
        is_terminal: outcome.is_terminal,
        feedback: outcome.feedback,
        game,
    }
}

#[tauri::command]
pub fn send_scout(
    state: State<'_, StateManager>,
    scout_id: String,
    player_id: String,
) -> Result<Game, String> {
    info!(
        "[cmd] send_scout: scout_id={}, player_id={}",
        scout_id, player_id
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    ofm_core::scouting::send_scout(&mut game, &scout_id, &player_id)?;
    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn start_youth_scouting(
    state: State<'_, StateManager>,
    scout_id: String,
    region: Option<String>,
    objective: Option<String>,
    target_position: Option<String>,
) -> Result<Game, String> {
    info!(
        "[cmd] start_youth_scouting: scout_id={}, region={:?}, objective={:?}, target_position={:?}",
        scout_id, region, objective, target_position
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    let region = parse_youth_region(region.as_deref())?;
    let objective = parse_youth_objective(objective.as_deref())?;
    let target_position = parse_youth_target_position(target_position.as_deref())?;

    ofm_core::scouting::start_youth_scouting(
        &mut game,
        &scout_id,
        region,
        objective,
        target_position,
    )?;
    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn cancel_youth_scouting(
    state: State<'_, StateManager>,
    assignment_id: String,
) -> Result<Game, String> {
    info!("[cmd] cancel_youth_scouting: assignment_id={}", assignment_id);
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    ofm_core::scouting::cancel_youth_scouting(&mut game, &assignment_id)?;
    state.set_game(game.clone());
    Ok(game)
}

#[tauri::command]
pub fn reassign_youth_scouting(
    state: State<'_, StateManager>,
    assignment_id: String,
    scout_id: String,
) -> Result<Game, String> {
    info!(
        "[cmd] reassign_youth_scouting: assignment_id={}, scout_id={}",
        assignment_id, scout_id
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;

    ofm_core::scouting::reassign_youth_scouting(&mut game, &assignment_id, &scout_id)?;
    state.set_game(game.clone());
    Ok(game)
}

fn parse_youth_region(value: Option<&str>) -> Result<YouthScoutingRegion, String> {
    match value {
        None | Some("") | Some("Domestic") => Ok(YouthScoutingRegion::Domestic),
        Some("International") => Ok(YouthScoutingRegion::International),
        Some(_) => Err(INVALID_YOUTH_SCOUTING_REGION_ERROR.to_string()),
    }
}

fn parse_youth_objective(value: Option<&str>) -> Result<YouthScoutingObjective, String> {
    match value {
        None | Some("") | Some("Balanced") => Ok(YouthScoutingObjective::Balanced),
        Some("HighPotential") => Ok(YouthScoutingObjective::HighPotential),
        Some("ReadySoon") => Ok(YouthScoutingObjective::ReadySoon),
        Some(_) => Err(INVALID_YOUTH_SCOUTING_OBJECTIVE_ERROR.to_string()),
    }
}

fn parse_youth_target_position(value: Option<&str>) -> Result<Option<Position>, String> {
    match value {
        None | Some("") => Ok(None),
        Some("Defender") => Ok(Some(Position::Defender)),
        Some("Midfielder") => Ok(Some(Position::Midfielder)),
        Some("Forward") => Ok(Some(Position::Forward)),
        Some(_) => Err(INVALID_YOUTH_SCOUTING_TARGET_POSITION_ERROR.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        counter_offer_internal, make_transfer_bid_internal,
        preview_transfer_bid_financial_impact_internal, respond_to_offer_internal,
        toggle_loan_list_internal, toggle_transfer_list_internal,
    };
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position, TransferOffer, TransferOfferStatus};
    use domain::season::TransferWindowStatus;
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::StateManager;
    use ofm_core::transfers::TransferNegotiationDecision;

    fn default_attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 60,
            shooting: 60,
            tackling: 60,
            dribbling: 60,
            defending: 60,
            positioning: 60,
            vision: 60,
            decisions: 60,
            composure: 60,
            aggression: 60,
            teamwork: 60,
            leadership: 60,
            handling: 30,
            reflexes: 30,
            aerial: 60,
        }
    }

    fn make_user_team() -> Team {
        let mut team = Team::new(
            "team-1".to_string(),
            "User FC".to_string(),
            "USR".to_string(),
            "England".to_string(),
            "London".to_string(),
            "User Ground".to_string(),
            25_000,
        );
        team.finance = 5_000_000;
        team.transfer_budget = 2_000_000;
        team.manager_id = Some("manager-1".to_string());
        team
    }

    fn make_buyer_team() -> Team {
        let mut team = Team::new(
            "team-2".to_string(),
            "Buyer FC".to_string(),
            "BUY".to_string(),
            "England".to_string(),
            "Liverpool".to_string(),
            "Buyer Ground".to_string(),
            28_000,
        );
        team.finance = 6_000_000;
        team.transfer_budget = 3_000_000;
        team
    }

    fn make_player_with_offer() -> Player {
        let mut player = Player::new(
            "player-1".to_string(),
            "P. One".to_string(),
            "Player One".to_string(),
            "2000-01-01".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        player.team_id = Some("team-1".to_string());
        player.contract_end = Some("2028-06-30".to_string());
        player.market_value = 1_000_000;
        player.transfer_offers.push(TransferOffer {
            id: "offer-1".to_string(),
            from_team_id: "team-2".to_string(),
            fee: 900_000,
            wage_offered: 0,
            last_manager_fee: None,
            negotiation_round: 1,
            suggested_counter_fee: None,
            status: TransferOfferStatus::Pending,
            date: "2026-08-01".to_string(),
        });
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

        let mut game = Game::new(
            clock,
            manager,
            vec![make_user_team(), make_buyer_team()],
            vec![make_player_with_offer()],
            vec![],
            vec![],
        );
        game.season_context.transfer_window.status = TransferWindowStatus::Open;
        game
    }

    fn make_bid_target_player() -> Player {
        let mut player = Player::new(
            "player-2".to_string(),
            "P. Two".to_string(),
            "Player Two".to_string(),
            "2000-01-01".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        player.team_id = Some("team-2".to_string());
        player.contract_end = Some("2028-06-30".to_string());
        player.market_value = 1_000_000;
        player.morale = 35;
        player.stats.appearances = 1;
        player
    }

    fn make_bid_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team-1".to_string());

        let mut game = Game::new(
            clock,
            manager,
            vec![make_user_team(), make_buyer_team()],
            vec![make_bid_target_player()],
            vec![],
            vec![],
        );
        game.season_context.transfer_window.status = TransferWindowStatus::Open;
        game.teams[0].reputation = 700;
        game.teams[1].reputation = 350;
        game
    }

    #[test]
    fn counter_offer_internal_returns_payload_and_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response =
            counter_offer_internal(&state, "player-1", "offer-1", 1_050_000).expect("response");

        assert_eq!(response.decision, TransferNegotiationDecision::Accepted);
        assert_eq!(response.game.players[0].team_id.as_deref(), Some("team-2"));
        assert_eq!(
            response.game.players[0].transfer_offers[0].status,
            TransferOfferStatus::Accepted
        );

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert_eq!(
            stored_game
                .players
                .iter()
                .find(|player| player.id == "player-1")
                .and_then(|player| player.team_id.clone())
                .as_deref(),
            Some("team-2")
        );
    }

    #[test]
    fn make_transfer_bid_internal_returns_payload_and_updates_state() {
        let state = StateManager::new();
        state.set_game(make_bid_game());

        let response = make_transfer_bid_internal(&state, "player-2", 1_050_000).expect("response");

        assert_eq!(response.decision, TransferNegotiationDecision::Accepted);
        assert_eq!(response.game.players[0].team_id.as_deref(), Some("team-1"));
        assert_eq!(
            response.game.players[0].transfer_offers[0].status,
            TransferOfferStatus::Accepted
        );

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert_eq!(
            stored_game
                .players
                .iter()
                .find(|player| player.id == "player-2")
                .and_then(|player| player.team_id.clone())
                .as_deref(),
            Some("team-1")
        );
    }

    #[test]
    fn make_transfer_bid_internal_can_return_counter_offer_feedback() {
        let state = StateManager::new();
        state.set_game(make_bid_game());

        let response = make_transfer_bid_internal(&state, "player-2", 900_000).expect("response");

        assert_eq!(response.decision, TransferNegotiationDecision::CounterOffer);
        assert_eq!(response.suggested_fee, Some(950_000));
        assert!(!response.is_terminal);
        assert_eq!(response.feedback.round, 1);

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert_eq!(
            stored_game
                .players
                .iter()
                .find(|player| player.id == "player-2")
                .and_then(|player| player.team_id.clone())
                .as_deref(),
            Some("team-2")
        );
    }

    #[test]
    fn make_transfer_bid_internal_uses_existing_negotiation_round_on_follow_up_bid() {
        let state = StateManager::new();
        state.set_game(make_bid_game());

        let first = make_transfer_bid_internal(&state, "player-2", 900_000).expect("first bid");
        assert_eq!(first.decision, TransferNegotiationDecision::CounterOffer);
        assert_eq!(first.feedback.round, 1);

        let second = make_transfer_bid_internal(&state, "player-2", 950_000).expect("second bid");

        assert_eq!(second.decision, TransferNegotiationDecision::Accepted);
        assert_eq!(second.feedback.round, 2);
        assert_eq!(second.game.players[0].team_id.as_deref(), Some("team-1"));
    }

    #[test]
    fn respond_to_offer_internal_returns_game_and_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response =
            respond_to_offer_internal(&state, "player-1", "offer-1", false).expect("response");

        let player = response
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("player should exist");
        assert_eq!(player.team_id.as_deref(), Some("team-1"));
        assert_eq!(
            player.transfer_offers[0].status,
            TransferOfferStatus::Rejected
        );

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("stored player should exist");
        assert_eq!(stored_player.team_id.as_deref(), Some("team-1"));
        assert_eq!(
            stored_player.transfer_offers[0].status,
            TransferOfferStatus::Rejected
        );
    }

    #[test]
    fn toggle_transfer_list_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = toggle_transfer_list_internal(&state, "player-1").expect("response");

        let player = response
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("player should exist");
        assert!(player.transfer_listed);

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("stored player should exist");
        assert!(stored_player.transfer_listed);
    }

    #[test]
    fn toggle_loan_list_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = toggle_loan_list_internal(&state, "player-1").expect("response");

        let player = response
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("player should exist");
        assert!(player.loan_listed);

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("stored player should exist");
        assert!(stored_player.loan_listed);
    }

    #[test]
    fn preview_transfer_bid_financial_impact_internal_returns_projection() {
        let state = StateManager::new();
        state.set_game(make_bid_game());

        let response =
            preview_transfer_bid_financial_impact_internal(&state, "player-2", 1_000_000)
                .expect("response");

        assert_eq!(response.projection.transfer_budget_before, 2_000_000);
        assert_eq!(response.projection.transfer_budget_after, 1_000_000);
        assert_eq!(response.projection.finance_before, 5_000_000);
        assert_eq!(response.projection.finance_after, 4_000_000);
        assert!(!response.projection.exceeds_transfer_budget);
        assert!(!response.projection.exceeds_finance);
    }

    #[test]
    fn parse_youth_region_returns_backend_key_when_value_is_unsupported() {
        let result = super::parse_youth_region(Some("Intergalactic"));

        assert_eq!(
            result.unwrap_err(),
            super::INVALID_YOUTH_SCOUTING_REGION_ERROR
        );
    }

    #[test]
    fn parse_youth_objective_returns_backend_key_when_value_is_unsupported() {
        let result = super::parse_youth_objective(Some("WonderkidOnly"));

        assert_eq!(
            result.unwrap_err(),
            super::INVALID_YOUTH_SCOUTING_OBJECTIVE_ERROR
        );
    }

    #[test]
    fn parse_youth_target_position_returns_backend_key_when_value_is_unsupported() {
        let result = super::parse_youth_target_position(Some("Goalkeeper"));

        assert_eq!(
            result.unwrap_err(),
            super::INVALID_YOUTH_SCOUTING_TARGET_POSITION_ERROR
        );
    }
}
