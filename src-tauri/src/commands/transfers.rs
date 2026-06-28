use domain::negotiation::NegotiationFeedback;
use domain::player::{LoanOfferStatus, Position};
use log::info;
use std::sync::Arc;
use tauri::State;

use ofm_core::game::Game;
use ofm_core::game::{YouthScoutingObjective, YouthScoutingRegion};
use ofm_core::state::StateManager;

use crate::commands::util::mutate_active_game;
use ofm_core::transfers::{
    LoanOfferDecision, LoanOfferOutcome, TransferBidFinancialProjection,
    TransferNegotiationDecision, TransferNegotiationOutcome,
};

const INVALID_YOUTH_SCOUTING_REGION_ERROR: &str = "be.error.transfers.invalidYouthScoutingRegion";
const INVALID_YOUTH_SCOUTING_OBJECTIVE_ERROR: &str =
    "be.error.transfers.invalidYouthScoutingObjective";
const INVALID_YOUTH_SCOUTING_TARGET_POSITION_ERROR: &str =
    "be.error.transfers.invalidYouthScoutingTargetPosition";
const ERR_PLAYER_NOT_OWNED_BY_USER: &str = "be.error.transfers.playerNotOwnedByUser";
const ERR_PLAYER_ALREADY_LOANED: &str = "be.error.transfers.playerAlreadyLoaned";

fn player_has_active_or_pending_loan(player: &domain::player::Player) -> bool {
    player.active_loan.is_some()
        || player
            .loan_offers
            .iter()
            .any(|offer| offer.status == LoanOfferStatus::PendingRegistration)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TransferNegotiationCommandResponse {
    pub decision: TransferNegotiationDecision,
    pub suggested_fee: Option<u64>,
    pub is_terminal: bool,
    pub registration_date: Option<String>,
    pub feedback: NegotiationFeedback,
    pub game: Game,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TransferBidFinancialProjectionCommandResponse {
    pub projection: TransferBidFinancialProjection,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LoanOfferCommandResponse {
    pub decision: LoanOfferDecision,
    pub offer_id: String,
    pub suggested_wage_contribution_pct: Option<u8>,
    pub suggested_end_date: Option<String>,
    pub suggested_buy_option_fee: Option<u64>,
    pub is_terminal: bool,
    pub game: Game,
}

#[tauri::command]
pub fn toggle_transfer_list(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<Game, String> {
    toggle_transfer_list_internal(&state, &player_id)
}

pub fn toggle_transfer_list_internal(
    state: &StateManager,
    player_id: &str,
) -> Result<Game, String> {
    info!("[cmd] toggle_transfer_list: player_id={}", player_id);
    state.update_game(|game| {
        if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
            p.transfer_listed = !p.transfer_listed;
            Ok(game.clone())
        } else {
            Err("be.error.playerNotFound".into())
        }
    })
    .ok_or("be.error.noActiveGameSession".to_string())
    .and_then(|r| r)
}

#[tauri::command]
pub fn toggle_loan_list(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<Game, String> {
    toggle_loan_list_internal(&state, &player_id)
}

pub fn toggle_loan_list_internal(state: &StateManager, player_id: &str) -> Result<Game, String> {
    info!("[cmd] toggle_loan_list: player_id={}", player_id);
    mutate_active_game(state, |game| {
        let user_team_id = game
            .manager
            .team_id
            .clone()
            .ok_or_else(|| "be.error.noTeamAssigned".to_string())?;
        let player = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
            .ok_or_else(|| "be.error.playerNotFound".to_string())?;

        if player.team_id.as_deref() != Some(user_team_id.as_str()) {
            return Err(ERR_PLAYER_NOT_OWNED_BY_USER.to_string());
        }

        if player_has_active_or_pending_loan(player) {
            return Err(ERR_PLAYER_ALREADY_LOANED.to_string());
        }

        player.loan_listed = !player.loan_listed;
        Ok(())
    })
}

#[tauri::command]
pub fn make_transfer_bid(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    fee: u64,
) -> Result<TransferNegotiationCommandResponse, String> {
    make_transfer_bid_internal(&state, &player_id, fee)
}

#[tauri::command]
pub fn preview_transfer_bid_financial_impact(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    fee: u64,
) -> Result<TransferBidFinancialProjectionCommandResponse, String> {
    preview_transfer_bid_financial_impact_internal(&state, &player_id, fee)
}

pub fn make_transfer_bid_internal(
    state: &StateManager,
    player_id: &str,
    fee: u64,
) -> Result<TransferNegotiationCommandResponse, String> {
    info!(
        "[cmd] make_transfer_bid: player_id={}, fee={}",
        player_id, fee
    );
    state
        .update_game(|current| {
            let mut game = current.clone();
            let result = ofm_core::transfers::make_transfer_bid(&mut game, player_id, fee)?;
            *current = game.clone();
            Ok(map_transfer_negotiation_response(result, game))
        })
        .ok_or("be.error.noActiveGameSession".to_string())
        .and_then(|r| r)
}

#[tauri::command]
pub fn make_loan_offer(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    end_date: String,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
) -> Result<LoanOfferCommandResponse, String> {
    make_loan_offer_internal(
        &state,
        &player_id,
        &end_date,
        wage_contribution_pct,
        buy_option_fee,
    )
}

pub fn make_loan_offer_internal(
    state: &StateManager,
    player_id: &str,
    end_date: &str,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
) -> Result<LoanOfferCommandResponse, String> {
    info!(
        "[cmd] make_loan_offer: player_id={}, end_date={}, wage_contribution_pct={}, buy_option_fee={:?}",
        player_id, end_date, wage_contribution_pct, buy_option_fee
    );
    state
        .update_game(|current| {
            let mut game = current.clone();
            let result = ofm_core::transfers::make_loan_offer(
                &mut game,
                player_id,
                end_date,
                wage_contribution_pct,
                buy_option_fee,
            )?;
            *current = game.clone();
            Ok(map_loan_offer_response(result, game))
        })
        .ok_or("be.error.noActiveGameSession".to_string())
        .and_then(|r| r)
}

#[tauri::command]
pub fn exercise_loan_buy_option(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
) -> Result<Game, String> {
    exercise_loan_buy_option_internal(&state, &player_id)
}

pub fn exercise_loan_buy_option_internal(
    state: &StateManager,
    player_id: &str,
) -> Result<Game, String> {
    info!("[cmd] exercise_loan_buy_option: player_id={}", player_id);
    state.update_game(|current| {
        let mut game = current.clone();
        ofm_core::transfers::exercise_loan_buy_option(&mut game, player_id)?;
        *current = game.clone();
        Ok(game)
    })
    .ok_or("be.error.noActiveGameSession".to_string())
    .and_then(|r| r)
}

pub fn preview_transfer_bid_financial_impact_internal(
    state: &StateManager,
    player_id: &str,
    fee: u64,
) -> Result<TransferBidFinancialProjectionCommandResponse, String> {
    info!(
        "[cmd] preview_transfer_bid_financial_impact: player_id={}, fee={}",
        player_id, fee
    );
    state.get_game(|game| ofm_core::transfers::project_transfer_bid_financial_impact(game, player_id, fee))
        .ok_or("be.error.noActiveGameSession".to_string())
        .and_then(|r| r)
        .map(|projection| TransferBidFinancialProjectionCommandResponse { projection })
}

#[tauri::command]
pub fn respond_to_offer(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    offer_id: String,
    accept: bool,
) -> Result<Game, String> {
    respond_to_offer_internal(&state, &player_id, &offer_id, accept)
}

pub fn respond_to_offer_internal(
    state: &StateManager,
    player_id: &str,
    offer_id: &str,
    accept: bool,
) -> Result<Game, String> {
    info!(
        "[cmd] respond_to_offer: player_id={}, offer_id={}, accept={}",
        player_id, offer_id, accept
    );
    mutate_active_game(state, |game| {
        ofm_core::transfers::respond_to_offer(game, player_id, offer_id, accept)
    })
}

#[tauri::command]
pub fn respond_to_loan_offer(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    offer_id: String,
    accept: bool,
) -> Result<Game, String> {
    respond_to_loan_offer_internal(&state, &player_id, &offer_id, accept)
}

pub fn respond_to_loan_offer_internal(
    state: &StateManager,
    player_id: &str,
    offer_id: &str,
    accept: bool,
) -> Result<Game, String> {
    info!(
        "[cmd] respond_to_loan_offer: player_id={}, offer_id={}, accept={}",
        player_id, offer_id, accept
    );
    state.update_game(|current| {
        let mut game = current.clone();
        ofm_core::transfers::respond_to_loan_offer(&mut game, player_id, offer_id, accept)?;
        *current = game.clone();
        Ok(game)
    })
    .ok_or("be.error.noActiveGameSession".to_string())
    .and_then(|r| r)
}

#[tauri::command]
pub fn counter_loan_offer(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    offer_id: String,
    end_date: String,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
) -> Result<LoanOfferCommandResponse, String> {
    counter_loan_offer_internal(
        &state,
        &player_id,
        &offer_id,
        &end_date,
        wage_contribution_pct,
        buy_option_fee,
    )
}

pub fn counter_loan_offer_internal(
    state: &StateManager,
    player_id: &str,
    offer_id: &str,
    end_date: &str,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
) -> Result<LoanOfferCommandResponse, String> {
    info!(
        "[cmd] counter_loan_offer: player_id={}, offer_id={}, end_date={}, wage_contribution_pct={}, buy_option_fee={:?}",
        player_id, offer_id, end_date, wage_contribution_pct, buy_option_fee
    );
    state
        .update_game(|current| {
            let mut game = current.clone();
            let result = ofm_core::transfers::counter_loan_offer(
                &mut game,
                player_id,
                offer_id,
                end_date,
                wage_contribution_pct,
                buy_option_fee,
            )?;
            *current = game.clone();
            Ok(map_loan_offer_response(result, game))
        })
        .ok_or("be.error.noActiveGameSession".to_string())
        .and_then(|r| r)
}

#[tauri::command]
pub fn counter_offer(
    state: State<'_, Arc<StateManager>>,
    player_id: String,
    offer_id: String,
    requested_fee: u64,
) -> Result<TransferNegotiationCommandResponse, String> {
    counter_offer_internal(&state, &player_id, &offer_id, requested_fee)
}

pub fn counter_offer_internal(
    state: &StateManager,
    player_id: &str,
    offer_id: &str,
    requested_fee: u64,
) -> Result<TransferNegotiationCommandResponse, String> {
    info!(
        "[cmd] counter_offer: player_id={}, offer_id={}, requested_fee={}",
        player_id, offer_id, requested_fee
    );
    state
        .update_game(|current| {
            let mut game = current.clone();
            let result =
                ofm_core::transfers::counter_offer(&mut game, player_id, offer_id, requested_fee)?;
            *current = game.clone();
            Ok(map_transfer_negotiation_response(result, game))
        })
        .ok_or("be.error.noActiveGameSession".to_string())
        .and_then(|r| r)
}

fn map_transfer_negotiation_response(
    outcome: TransferNegotiationOutcome,
    game: Game,
) -> TransferNegotiationCommandResponse {
    TransferNegotiationCommandResponse {
        decision: outcome.decision,
        suggested_fee: outcome.suggested_fee,
        is_terminal: outcome.is_terminal,
        registration_date: outcome.registration_date,
        feedback: outcome.feedback,
        game,
    }
}

fn map_loan_offer_response(outcome: LoanOfferOutcome, game: Game) -> LoanOfferCommandResponse {
    LoanOfferCommandResponse {
        decision: outcome.decision,
        offer_id: outcome.offer_id,
        suggested_wage_contribution_pct: outcome.suggested_wage_contribution_pct,
        suggested_end_date: outcome.suggested_end_date,
        suggested_buy_option_fee: outcome.suggested_buy_option_fee,
        is_terminal: outcome.is_terminal,
        game,
    }
}

#[tauri::command]
pub fn send_scout(
    state: State<'_, Arc<StateManager>>,
    scout_id: String,
    player_id: String,
) -> Result<Game, String> {
    info!(
        "[cmd] send_scout: scout_id={}, player_id={}",
        scout_id, player_id
    );
    state.update_game(|game| {
        ofm_core::scouting::send_scout(game, &scout_id, &player_id)?;
        Ok(game.clone())
    })
    .ok_or("be.error.noActiveGameSession".to_string())
    .and_then(|r| r)
}

#[tauri::command]
pub fn start_youth_scouting(
    state: State<'_, Arc<StateManager>>,
    scout_id: String,
    region: Option<String>,
    objective: Option<String>,
    target_position: Option<String>,
) -> Result<Game, String> {
    info!(
        "[cmd] start_youth_scouting: scout_id={}, region={:?}, objective={:?}, target_position={:?}",
        scout_id, region, objective, target_position
    );
    let region = parse_youth_region(region.as_deref())?;
    let objective = parse_youth_objective(objective.as_deref())?;
    let target_position = parse_youth_target_position(target_position.as_deref())?;
    state.update_game(|game| {
        ofm_core::scouting::start_youth_scouting(game, &scout_id, region, objective, target_position)?;
        Ok(game.clone())
    })
    .ok_or("be.error.noActiveGameSession".to_string())
    .and_then(|r| r)
}

#[tauri::command]
pub fn cancel_youth_scouting(
    state: State<'_, Arc<StateManager>>,
    assignment_id: String,
) -> Result<Game, String> {
    info!(
        "[cmd] cancel_youth_scouting: assignment_id={}",
        assignment_id
    );
    state.update_game(|game| {
        ofm_core::scouting::cancel_youth_scouting(game, &assignment_id)?;
        Ok(game.clone())
    })
    .ok_or("be.error.noActiveGameSession".to_string())
    .and_then(|r| r)
}

#[tauri::command]
pub fn reassign_youth_scouting(
    state: State<'_, Arc<StateManager>>,
    assignment_id: String,
    scout_id: String,
) -> Result<Game, String> {
    info!(
        "[cmd] reassign_youth_scouting: assignment_id={}, scout_id={}",
        assignment_id, scout_id
    );
    state.update_game(|game| {
        ofm_core::scouting::reassign_youth_scouting(game, &assignment_id, &scout_id)?;
        Ok(game.clone())
    })
    .ok_or("be.error.noActiveGameSession".to_string())
    .and_then(|r| r)
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
        counter_loan_offer_internal, counter_offer_internal, exercise_loan_buy_option_internal,
        make_loan_offer_internal, make_transfer_bid_internal,
        preview_transfer_bid_financial_impact_internal, respond_to_loan_offer_internal,
        respond_to_offer_internal, toggle_loan_list_internal, toggle_transfer_list_internal,
    };
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{
        ActiveLoan, LoanOffer, LoanOfferStatus, Player, PlayerAttributes, Position, TransferOffer,
        TransferOfferStatus,
    };
    use domain::season::TransferWindowStatus;
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::StateManager;
    use ofm_core::transfers::{LoanOfferDecision, TransferNegotiationDecision};

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
            registration_date: None,
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
    fn make_transfer_bid_internal_preserves_scheduled_registration_date() {
        let state = StateManager::new();
        let mut game = make_bid_game();
        game.clock.current_date = Utc.with_ymd_and_hms(2026, 12, 20, 12, 0, 0).unwrap();
        game.season_context.transfer_window.status = TransferWindowStatus::Closed;
        game.season_context.transfer_window.opens_on = Some("2027-01-01".to_string());
        state.set_game(game);

        let response = make_transfer_bid_internal(&state, "player-2", 1_050_000).expect("response");

        assert_eq!(response.decision, TransferNegotiationDecision::Accepted);
        assert_eq!(response.registration_date.as_deref(), Some("2027-01-01"));
        assert_eq!(response.game.players[0].team_id.as_deref(), Some("team-2"));
        assert_eq!(
            response.game.players[0].transfer_offers[0].status,
            TransferOfferStatus::PendingRegistration
        );
        assert_eq!(
            response.game.players[0].transfer_offers[0]
                .registration_date
                .as_deref(),
            Some("2027-01-01")
        );
    }

    #[test]
    fn make_loan_offer_internal_returns_payload_and_updates_state() {
        let state = StateManager::new();
        let mut game = make_bid_game();
        game.players[0].loan_listed = true;
        state.set_game(game);

        let response = make_loan_offer_internal(&state, "player-2", "2027-01-01", 100, None)
            .expect("response");

        assert_eq!(response.decision, LoanOfferDecision::Accepted);
        assert_eq!(response.game.players[0].team_id.as_deref(), Some("team-1"));
        assert!(response.game.players[0].active_loan.is_some());
        assert_eq!(
            response.game.players[0].loan_offers[0].status,
            LoanOfferStatus::Accepted
        );

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert_eq!(
            stored_game
                .players
                .iter()
                .find(|player| player.id == "player-2")
                .and_then(|player| player.active_loan.as_ref())
                .map(|loan| loan.parent_team_id.as_str()),
            Some("team-2")
        );
    }

    #[test]
    fn exercise_loan_buy_option_internal_updates_state() {
        let state = StateManager::new();
        let mut game = make_bid_game();
        game.players[0].loan_listed = true;
        state.set_game(game);

        make_loan_offer_internal(&state, "player-2", "2027-01-01", 100, Some(1_000_000))
            .expect("loan-to-buy offer");
        let response =
            exercise_loan_buy_option_internal(&state, "player-2").expect("exercise option");

        let player = response
            .players
            .iter()
            .find(|player| player.id == "player-2")
            .expect("player should exist");
        assert_eq!(player.team_id.as_deref(), Some("team-1"));
        assert!(player.active_loan.is_none());

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert_eq!(
            stored_game
                .players
                .iter()
                .find(|player| player.id == "player-2")
                .and_then(|player| player.active_loan.as_ref()),
            None
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
    fn respond_to_loan_offer_internal_returns_game_and_updates_state() {
        let state = StateManager::new();
        let mut game = make_game();
        game.players[0].loan_listed = true;
        game.players[0].loan_offers.push(LoanOffer {
            id: "loan-offer-1".to_string(),
            from_team_id: "team-2".to_string(),
            parent_team_id: "team-1".to_string(),
            start_date: "2026-08-01".to_string(),
            end_date: "2027-01-01".to_string(),
            wage_contribution_pct: 75,
            buy_option_fee: None,
            last_manager_wage_contribution_pct: None,
            last_manager_end_date: None,
            last_manager_buy_option_fee: None,
            negotiation_round: 1,
            suggested_wage_contribution_pct: None,
            suggested_end_date: None,
            suggested_buy_option_fee: None,
            status: LoanOfferStatus::Pending,
            date: "2026-08-01".to_string(),
        });
        state.set_game(game);

        let response = respond_to_loan_offer_internal(&state, "player-1", "loan-offer-1", true)
            .expect("response");

        let player = response
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("player should exist");
        assert_eq!(player.team_id.as_deref(), Some("team-2"));
        assert_eq!(player.loan_offers[0].status, LoanOfferStatus::Accepted);
        assert!(player.active_loan.is_some());

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("stored player should exist");
        assert_eq!(stored_player.team_id.as_deref(), Some("team-2"));
        assert!(stored_player.active_loan.is_some());
    }

    #[test]
    fn counter_loan_offer_internal_returns_payload_and_updates_state() {
        let state = StateManager::new();
        let mut game = make_game();
        game.players[0].loan_listed = true;
        game.players[0].wage = 520_000;
        game.players[0].ovr = 68;
        game.players[0].potential = 78;
        game.teams[1].finance = 6_000_000;
        game.players[0].loan_offers.push(LoanOffer {
            id: "loan-offer-counter".to_string(),
            from_team_id: "team-2".to_string(),
            parent_team_id: "team-1".to_string(),
            start_date: "2026-08-01".to_string(),
            end_date: "2027-01-01".to_string(),
            wage_contribution_pct: 65,
            buy_option_fee: None,
            last_manager_wage_contribution_pct: None,
            last_manager_end_date: None,
            last_manager_buy_option_fee: None,
            negotiation_round: 1,
            suggested_wage_contribution_pct: None,
            suggested_end_date: None,
            suggested_buy_option_fee: None,
            status: LoanOfferStatus::Pending,
            date: "2026-08-01".to_string(),
        });
        state.set_game(game);

        let response = counter_loan_offer_internal(
            &state,
            "player-1",
            "loan-offer-counter",
            "2027-01-01",
            85,
            None,
        )
        .expect("response");

        assert_eq!(response.decision, LoanOfferDecision::Accepted);
        assert_eq!(response.game.players[0].team_id.as_deref(), Some("team-2"));
        assert_eq!(
            response.game.players[0].loan_offers[0].last_manager_wage_contribution_pct,
            Some(85)
        );

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("stored player should exist");
        assert_eq!(stored_player.team_id.as_deref(), Some("team-2"));
        assert!(stored_player.active_loan.is_some());
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
    fn toggle_loan_list_internal_rejects_other_club_player() {
        let state = StateManager::new();
        state.set_game(make_bid_game());

        let error = toggle_loan_list_internal(&state, "player-2")
            .expect_err("loan listing another club's player should fail");

        assert_eq!(error, "be.error.transfers.playerNotOwnedByUser");
        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-2")
            .expect("stored player should exist");
        assert!(!stored_player.loan_listed);
    }

    #[test]
    fn toggle_loan_list_internal_rejects_active_loan_player() {
        let state = StateManager::new();
        let mut game = make_game();
        game.players[0].active_loan = Some(ActiveLoan {
            parent_team_id: "team-1".to_string(),
            loan_team_id: "team-2".to_string(),
            start_date: "2026-08-01".to_string(),
            end_date: "2027-01-01".to_string(),
            wage_contribution_pct: 75,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });
        state.set_game(game);

        let error = toggle_loan_list_internal(&state, "player-1")
            .expect_err("active loan player should not be loan-listed");

        assert_eq!(error, "be.error.transfers.playerAlreadyLoaned");
        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("stored player should exist");
        assert!(!stored_player.loan_listed);
    }

    #[test]
    fn toggle_loan_list_internal_rejects_pending_loan_registration_player() {
        let state = StateManager::new();
        let mut game = make_game();
        game.players[0].loan_offers.push(LoanOffer {
            id: "pending-loan-registration".to_string(),
            from_team_id: "team-2".to_string(),
            parent_team_id: "team-1".to_string(),
            start_date: "2026-09-01".to_string(),
            end_date: "2027-01-01".to_string(),
            wage_contribution_pct: 75,
            buy_option_fee: None,
            last_manager_wage_contribution_pct: None,
            last_manager_end_date: None,
            last_manager_buy_option_fee: None,
            negotiation_round: 1,
            suggested_wage_contribution_pct: None,
            suggested_end_date: None,
            suggested_buy_option_fee: None,
            status: LoanOfferStatus::PendingRegistration,
            date: "2026-08-01".to_string(),
        });
        state.set_game(game);

        let error = toggle_loan_list_internal(&state, "player-1")
            .expect_err("pending loan registration player should not be loan-listed");

        assert_eq!(error, "be.error.transfers.playerAlreadyLoaned");
        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_player = stored_game
            .players
            .iter()
            .find(|player| player.id == "player-1")
            .expect("stored player should exist");
        assert!(!stored_player.loan_listed);
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
