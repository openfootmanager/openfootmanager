use crate::contract_wage_policy::{
    project_contract_offer_financial_impact, project_renewal_financial_impact as project_renewal_financial_impact_service,
    renewal_wage_policy_allows, renewal_wage_policy_error_message,
};
use crate::delegated_renewals::delegate_renewals as delegate_renewals_service;
use crate::game::Game;
use crate::squad_safety::{SquadSafetyReport, project_user_team_release_safety};
use chrono::{Datelike, Months, NaiveDate};
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use domain::negotiation::{NegotiationFeedback, NegotiationMood};
use domain::player::{
    ContractExitIntent, ContractRenewalState, Player, RenewalSessionOutcome, RenewalSessionStatus,
};
use domain::season::TransferWindowStatus;
use domain::team::{FinancialTransaction, FinancialTransactionKind, Team};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const RENEWAL_SESSION_STALE_DAYS: i64 = 14;
const MAX_CONTRACT_YEARS: u32 = 5;
const MARKET_VALUE_TO_WAGE_RATIO: u64 = 200;
const MINIMUM_DEFAULT_WAGE: u64 = 500;
const ERR_NO_TEAM_ASSIGNED: &str = "be.error.noTeamAssigned";
const ERR_MANAGED_TEAM_NOT_FOUND: &str = "be.error.managedTeamNotFound";
const ERR_PLAYER_NOT_FOUND: &str = "be.error.playerNotFound";
const ERR_TRANSFER_WINDOW_CLOSED: &str = "be.error.transfers.transferWindowClosed";
const ERR_PLAYER_NOT_OWNED_BY_CLUB: &str = "be.error.contracts.playerNotOwnedByClub";
const ERR_PLAYER_NOT_FREE_AGENT: &str = "be.error.contracts.playerNotFreeAgent";
const ERR_UNABLE_TO_CALCULATE_CONTRACT_END_DATE: &str =
    "be.error.contracts.unableToCalculateContractEndDate";
const ERR_PLAYER_HAS_NO_ACTIVE_CONTRACT: &str = "be.error.contracts.playerHasNoActiveContract";
const ERR_TERMINATION_WOULD_LEAVE_MATCHDAY_SQUAD_SHORT: &str =
    "be.error.contracts.terminationWouldLeaveMatchdaySquadShort";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractWarningStage {
    TwelveMonths,
    SixMonths,
    ThreeMonths,
    FinalWeeks,
}

impl ContractWarningStage {
    pub(crate) fn message_suffix(self) -> &'static str {
        match self {
            ContractWarningStage::TwelveMonths => "12m",
            ContractWarningStage::SixMonths => "6m",
            ContractWarningStage::ThreeMonths => "3m",
            ContractWarningStage::FinalWeeks => "final",
        }
    }

    pub(crate) fn morale_pressure(self) -> i16 {
        match self {
            ContractWarningStage::TwelveMonths => 2,
            ContractWarningStage::SixMonths => 4,
            ContractWarningStage::ThreeMonths => 6,
            ContractWarningStage::FinalWeeks => 9,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenewalOffer {
    pub weekly_wage: u32,
    pub contract_years: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RenewalDecision {
    Accepted,
    Rejected,
    CounterOffer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenewalOutcome {
    pub decision: RenewalDecision,
    pub suggested_wage: Option<u32>,
    pub suggested_years: Option<u32>,
    pub session_status: RenewalSessionStatus,
    pub is_terminal: bool,
    pub cooled_off: bool,
    pub feedback: Option<NegotiationFeedback>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenewalFinancialProjection {
    pub current_annual_wage_bill: i64,
    pub projected_annual_wage_bill: i64,
    pub annual_wage_budget: i64,
    pub annual_soft_cap: i64,
    pub current_weekly_wage_spend: i64,
    pub projected_weekly_wage_spend: i64,
    pub current_cash_runway_weeks: Option<i64>,
    pub projected_cash_runway_weeks: Option<i64>,
    pub currently_over_budget: bool,
    pub policy_allows: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelegatedRenewalOptions {
    pub player_ids: Option<Vec<String>>,
    pub max_wage_increase_pct: u32,
    pub max_contract_years: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DelegatedRenewalResultStatus {
    Successful,
    Failed,
    Stalled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelegatedRenewalCase {
    pub player_id: String,
    pub player_name: String,
    pub status: DelegatedRenewalResultStatus,
    pub agreed_wage: Option<u32>,
    pub agreed_years: Option<u32>,
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_key: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub note_params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelegatedRenewalReport {
    pub success_count: u32,
    pub failure_count: u32,
    pub stalled_count: u32,
    pub cases: Vec<DelegatedRenewalCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractTerminationPreview {
    pub player_id: String,
    pub player_name: String,
    pub severance_cost: i64,
    pub squad_safety: SquadSafetyReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractTerminationResult {
    pub severance_cost: i64,
    pub squad_safety: SquadSafetyReport,
}

enum ContractReleaseReason {
    Expired,
    ManagerTermination { severance_cost: i64 },
}

fn renewal_outcome(
    decision: RenewalDecision,
    suggested_wage: Option<u32>,
    suggested_years: Option<u32>,
    session_status: RenewalSessionStatus,
    is_terminal: bool,
    cooled_off: bool,
    feedback: Option<NegotiationFeedback>,
) -> RenewalOutcome {
    RenewalOutcome {
        decision,
        suggested_wage,
        suggested_years,
        session_status,
        is_terminal,
        cooled_off,
        feedback,
    }
}

pub fn project_renewal_financial_impact(
    game: &Game,
    player_id: &str,
    offered_wage: u32,
) -> Result<RenewalFinancialProjection, String> {
    project_renewal_financial_impact_service(game, player_id, offered_wage)
}

pub fn project_free_agent_contract_impact(
    game: &Game,
    player_id: &str,
    offered_wage: u32,
) -> Result<RenewalFinancialProjection, String> {
    let manager_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or(ERR_NO_TEAM_ASSIGNED.to_string())?;
    let team = game
        .teams
        .iter()
        .find(|candidate| candidate.id == manager_team_id)
        .ok_or(ERR_MANAGED_TEAM_NOT_FOUND.to_string())?;
    let player = game
        .players
        .iter()
        .find(|candidate| candidate.id == player_id)
        .ok_or(ERR_PLAYER_NOT_FOUND.to_string())?;

    if player.retired || player.team_id.is_some() {
        return Err(ERR_PLAYER_NOT_FREE_AGENT.to_string());
    }

    Ok(project_contract_offer_financial_impact(
        game,
        team,
        0,
        offered_wage,
    ))
}

pub fn evaluate_renewal_offer(
    player: &Player,
    team: &Team,
    current_date: NaiveDate,
    offer: &RenewalOffer,
) -> RenewalOutcome {
    let round = next_renewal_round(player, None);
    let expected_wage = expected_wage(player, team, current_date);
    let expected_years = expected_contract_years(player, current_date);
    let minimum_wage = minimum_acceptable_wage(player.wage);

    if offer.weekly_wage < minimum_wage || offer.contract_years == 0 {
        let feedback = build_renewal_feedback(
            player,
            current_date,
            RenewalDecision::Rejected,
            RenewalSessionStatus::Stalled,
            round,
            expected_wage,
            false,
        );
        return renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Stalled,
            false,
            false,
            Some(feedback),
        );
    }

    if offer.weekly_wage >= expected_wage && offer.contract_years >= expected_years {
        let feedback = build_renewal_feedback(
            player,
            current_date,
            RenewalDecision::Accepted,
            RenewalSessionStatus::Agreed,
            round,
            expected_wage,
            false,
        );
        return renewal_outcome(
            RenewalDecision::Accepted,
            None,
            None,
            RenewalSessionStatus::Agreed,
            true,
            false,
            Some(feedback),
        );
    }

    let feedback = build_renewal_feedback(
        player,
        current_date,
        RenewalDecision::CounterOffer,
        RenewalSessionStatus::Open,
        round,
        expected_wage,
        false,
    );

    renewal_outcome(
        RenewalDecision::CounterOffer,
        Some(expected_wage),
        Some(expected_years),
        RenewalSessionStatus::Open,
        false,
        false,
        Some(feedback),
    )
}

pub fn propose_renewal(
    game: &mut Game,
    player_id: &str,
    offer: RenewalOffer,
) -> Result<RenewalOutcome, String> {
    let manager_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("be.error.noTeamAssigned".to_string())?;

    let team = game
        .teams
        .iter()
        .find(|candidate| candidate.id == manager_team_id)
        .ok_or("be.error.managedTeamNotFound".to_string())?
        .clone();

    let player_index = game
        .players
        .iter()
        .position(|candidate| candidate.id == player_id)
        .ok_or("be.error.playerNotFound".to_string())?;

    if game.players[player_index].team_id.as_deref() != Some(team.id.as_str()) {
        return Err(ERR_PLAYER_NOT_OWNED_BY_CLUB.to_string());
    }

    let current_date = game.clock.current_date.date_naive();
    let cooled_off = cool_stale_renewal_session(&mut game.players[player_index], current_date);
    let today = current_date.format("%Y-%m-%d").to_string();
    let round = next_renewal_round(&game.players[player_index], Some(today.as_str()));

    if has_active_manager_block(&game.players[player_index], current_date) {
        return Ok(renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Blocked,
            true,
            cooled_off,
            Some(build_renewal_feedback(
                &game.players[player_index],
                current_date,
                RenewalDecision::Rejected,
                RenewalSessionStatus::Blocked,
                round,
                0,
                false,
            )),
        ));
    }

    if let Some(state) = game.players[player_index]
        .morale_core
        .renewal_state
        .as_ref()
        && state.status == RenewalSessionStatus::Agreed
        && state.last_attempt_date.as_deref() == Some(today.as_str())
    {
        return Ok(renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Agreed,
            true,
            cooled_off,
            Some(build_renewal_feedback(
                &game.players[player_index],
                current_date,
                RenewalDecision::Accepted,
                RenewalSessionStatus::Agreed,
                round,
                game.players[player_index].wage,
                false,
            )),
        ));
    }

    let expected_wage = expected_wage(&game.players[player_index], &team, current_date);
    let mut outcome =
        evaluate_renewal_offer(&game.players[player_index], &team, current_date, &offer);
    outcome.cooled_off = cooled_off;
    let relationship_blocked = should_manual_renewal_fail_on_relationship(
        &game.players[player_index],
        expected_wage,
        offer.weekly_wage,
    );

    if relationship_blocked {
        outcome = renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Stalled,
            false,
            cooled_off,
            Some(build_renewal_feedback(
                &game.players[player_index],
                current_date,
                RenewalDecision::Rejected,
                RenewalSessionStatus::Stalled,
                round,
                expected_wage,
                true,
            )),
        );
    }

    if outcome.decision == RenewalDecision::Accepted {
        if !renewal_wage_policy_allows(
            game,
            &team,
            game.players[player_index].wage,
            offer.weekly_wage,
        ) {
            return Err(renewal_wage_policy_error_message(&team));
        }

        let new_contract_end = current_date
            .checked_add_months(Months::new(offer.contract_years * 12))
            .ok_or(ERR_UNABLE_TO_CALCULATE_CONTRACT_END_DATE.to_string())?;

        let player = &mut game.players[player_index];
        player.wage = offer.weekly_wage;
        player.contract_end = Some(new_contract_end.format("%Y-%m-%d").to_string());
        let state = player
            .morale_core
            .renewal_state
            .get_or_insert_with(ContractRenewalState::default);
        state.status = RenewalSessionStatus::Agreed;
        state.manager_blocked_until = None;
        state.last_attempt_date = Some(today);
        state.last_outcome = Some(RenewalSessionOutcome::AcceptedByManager);
        state.conversation_round = round;
        state.exit_intent = None;
        return Ok(renewal_outcome(
            RenewalDecision::Accepted,
            None,
            None,
            RenewalSessionStatus::Agreed,
            true,
            cooled_off,
            Some(build_renewal_feedback(
                player,
                current_date,
                RenewalDecision::Accepted,
                RenewalSessionStatus::Agreed,
                round,
                expected_wage,
                false,
            )),
        ));
    }

    let player = &mut game.players[player_index];
    let state = player
        .morale_core
        .renewal_state
        .get_or_insert_with(ContractRenewalState::default);
    state.last_attempt_date = Some(today);
    state.conversation_round = round;

    match outcome.decision {
        RenewalDecision::Rejected => {
            state.status = outcome.session_status.clone();
            state.last_outcome = Some(RenewalSessionOutcome::RejectedByPlayer);
        }
        RenewalDecision::CounterOffer => {
            state.status = RenewalSessionStatus::Open;
            state.last_outcome = Some(RenewalSessionOutcome::Stalled);
        }
        RenewalDecision::Accepted => {}
    }

    if outcome.feedback.is_none() {
        outcome.feedback = Some(build_renewal_feedback(
            player,
            current_date,
            outcome.decision.clone(),
            outcome.session_status.clone(),
            round,
            expected_wage,
            relationship_blocked,
        ));
    }

    Ok(outcome)
}

pub fn offer_free_agent_contract(
    game: &mut Game,
    player_id: &str,
    offer: RenewalOffer,
) -> Result<RenewalOutcome, String> {
    if !transfer_window_is_open(game) {
        return Err(ERR_TRANSFER_WINDOW_CLOSED.to_string());
    }

    let manager_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or(ERR_NO_TEAM_ASSIGNED.to_string())?;

    let team = game
        .teams
        .iter()
        .find(|candidate| candidate.id == manager_team_id)
        .ok_or(ERR_MANAGED_TEAM_NOT_FOUND.to_string())?
        .clone();

    let player_index = game
        .players
        .iter()
        .position(|candidate| candidate.id == player_id)
        .ok_or(ERR_PLAYER_NOT_FOUND.to_string())?;

    if game.players[player_index].retired || game.players[player_index].team_id.is_some() {
        return Err(ERR_PLAYER_NOT_FREE_AGENT.to_string());
    }

    let current_date = game.clock.current_date.date_naive();
    let cooled_off = cool_stale_renewal_session(&mut game.players[player_index], current_date);
    let today = current_date.format("%Y-%m-%d").to_string();
    let round = next_renewal_round(&game.players[player_index], Some(today.as_str()));
    let expected_wage = expected_wage(&game.players[player_index], &team, current_date);
    let expected_years = expected_contract_years(&game.players[player_index], current_date);
    let minimum_wage = minimum_acceptable_wage(reference_player_wage(&game.players[player_index]));

    if offer.weekly_wage < minimum_wage
        || offer.contract_years == 0
        || offer.contract_years > MAX_CONTRACT_YEARS
    {
        return Ok(renewal_outcome(
            RenewalDecision::Rejected,
            None,
            None,
            RenewalSessionStatus::Stalled,
            false,
            cooled_off,
            Some(build_renewal_feedback(
                &game.players[player_index],
                current_date,
                RenewalDecision::Rejected,
                RenewalSessionStatus::Stalled,
                round,
                expected_wage,
                false,
            )),
        ));
    }

    if offer.weekly_wage >= expected_wage && offer.contract_years >= expected_years {
        if !renewal_wage_policy_allows(game, &team, 0, offer.weekly_wage) {
            return Err(renewal_wage_policy_error_message(&team));
        }

        let new_contract_end = current_date
            .checked_add_months(Months::new(offer.contract_years * 12))
            .ok_or(ERR_UNABLE_TO_CALCULATE_CONTRACT_END_DATE.to_string())?;

        let player = &mut game.players[player_index];
        player.team_id = Some(team.id.clone());
        player.wage = offer.weekly_wage;
        player.contract_end = Some(new_contract_end.format("%Y-%m-%d").to_string());
        player.transfer_listed = false;
        player.loan_listed = false;
        player.transfer_offers.clear();
        if matches!(
            player.morale_core.unresolved_issue.as_ref().map(|issue| &issue.category),
            Some(domain::player::PlayerIssueCategory::Contract)
        ) {
            player.morale_core.unresolved_issue = None;
        }
        player.morale_core.renewal_state = None;
        player.morale = (i16::from(player.morale) + 6).clamp(0, 100) as u8;
        player.morale_core.manager_trust = player.morale_core.manager_trust.max(55);

        game.messages.push(free_agent_signed_message(
            &player.id,
            &player.full_name,
            &team.name,
            offer.contract_years,
            &today,
        ));

        return Ok(renewal_outcome(
            RenewalDecision::Accepted,
            None,
            None,
            RenewalSessionStatus::Agreed,
            true,
            cooled_off,
            Some(build_renewal_feedback(
                player,
                current_date,
                RenewalDecision::Accepted,
                RenewalSessionStatus::Agreed,
                round,
                expected_wage,
                false,
            )),
        ));
    }

    let player = &mut game.players[player_index];
    let state = player
        .morale_core
        .renewal_state
        .get_or_insert_with(ContractRenewalState::default);
    state.last_attempt_date = Some(today);
    state.conversation_round = round;
    state.status = RenewalSessionStatus::Open;
    state.last_outcome = Some(RenewalSessionOutcome::Stalled);

    Ok(renewal_outcome(
        RenewalDecision::CounterOffer,
        Some(expected_wage),
        Some(expected_years),
        RenewalSessionStatus::Open,
        false,
        cooled_off,
        Some(build_renewal_feedback(
            player,
            current_date,
            RenewalDecision::CounterOffer,
            RenewalSessionStatus::Open,
            round,
            expected_wage,
            false,
        )),
    ))
}

pub fn delegate_renewals(
    game: &mut Game,
    options: DelegatedRenewalOptions,
) -> Result<DelegatedRenewalReport, String> {
    delegate_renewals_service(game, options)
}

pub fn set_contract_exit_intent(
    game: &mut Game,
    player_id: &str,
    reason: Option<String>,
) -> Result<(), String> {
    let player_index = owned_player_index(game, player_id)?;
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let player = &mut game.players[player_index];

    if player.contract_end.is_none() {
        return Err(ERR_PLAYER_HAS_NO_ACTIVE_CONTRACT.to_string());
    }

    let state = player
        .morale_core
        .renewal_state
        .get_or_insert_with(ContractRenewalState::default);
    state.status = RenewalSessionStatus::Blocked;
    state.manager_blocked_until = None;
    state.last_attempt_date = Some(today.clone());
    state.last_outcome = Some(RenewalSessionOutcome::BlockedByManager);
    state.conversation_round = 0;
    state.exit_intent = Some(ContractExitIntent::LetExpire {
        set_on: today,
        reason,
    });

    Ok(())
}

pub fn clear_contract_exit_intent(game: &mut Game, player_id: &str) -> Result<(), String> {
    let player_index = owned_player_index(game, player_id)?;
    let Some(state) = game.players[player_index]
        .morale_core
        .renewal_state
        .as_mut()
    else {
        return Ok(());
    };
    let had_exit_intent = state.exit_intent.is_some();

    state.exit_intent = None;

    if had_exit_intent && state.status == RenewalSessionStatus::Blocked {
        state.status = RenewalSessionStatus::Idle;
        state.manager_blocked_until = None;
        state.last_outcome = None;
        state.conversation_round = 0;
    }

    Ok(())
}

pub fn preview_contract_termination(
    game: &Game,
    player_id: &str,
) -> Result<ContractTerminationPreview, String> {
    let player = owned_player(game, player_id)?;

    if player.contract_end.is_none() {
        return Err(ERR_PLAYER_HAS_NO_ACTIVE_CONTRACT.to_string());
    }

    let current_date = game.clock.current_date.date_naive();
    Ok(ContractTerminationPreview {
        player_id: player.id.clone(),
        player_name: player.match_name.clone(),
        severance_cost: termination_severance_cost(player, current_date),
        squad_safety: project_user_team_release_safety(game, player_id)?,
    })
}

pub fn terminate_contract_now(
    game: &mut Game,
    player_id: &str,
) -> Result<ContractTerminationResult, String> {
    let preview = preview_contract_termination(game, player_id)?;

    if !preview.squad_safety.can_field_matchday_squad {
        return Err(ERR_TERMINATION_WOULD_LEAVE_MATCHDAY_SQUAD_SHORT.to_string());
    }

    let player_index = owned_player_index(game, player_id)?;
    let team_id = game.players[player_index]
        .team_id
        .clone()
        .ok_or(ERR_PLAYER_NOT_OWNED_BY_CLUB.to_string())?;
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    if let Some(team) = game
        .teams
        .iter_mut()
        .find(|candidate| candidate.id == team_id)
    {
        team.finance -= preview.severance_cost;
        team.season_expenses += preview.severance_cost;
        team.financial_ledger.push(FinancialTransaction {
            date: today,
            description: backend_text_with_param(
                "be.msg.contractTerminated.ledgerDescription",
                "player",
                &preview.player_name,
            ),
            amount: -preview.severance_cost,
            kind: FinancialTransactionKind::ContractTermination,
        });
    }

    release_player_contract(
        game,
        player_index,
        ContractReleaseReason::ManagerTermination {
            severance_cost: preview.severance_cost,
        },
    );

    Ok(ContractTerminationResult {
        severance_cost: preview.severance_cost,
        squad_safety: preview.squad_safety,
    })
}

pub fn has_let_expire_intent(player: &Player) -> bool {
    player
        .morale_core
        .renewal_state
        .as_ref()
        .and_then(|state| state.exit_intent.as_ref())
        .is_some_and(|intent| matches!(intent, ContractExitIntent::LetExpire { .. }))
}

pub fn contract_warning_stage(
    contract_end: Option<&str>,
    current_date: NaiveDate,
) -> Option<ContractWarningStage> {
    let days_remaining = contract_days_remaining(contract_end, current_date)?;

    if days_remaining <= 0 {
        return None;
    }

    if days_remaining <= 30 {
        return Some(ContractWarningStage::FinalWeeks);
    }

    if days_remaining <= 90 {
        return Some(ContractWarningStage::ThreeMonths);
    }

    if days_remaining <= 180 {
        return Some(ContractWarningStage::SixMonths);
    }

    if days_remaining <= 365 {
        return Some(ContractWarningStage::TwelveMonths);
    }

    None
}

pub fn process_contract_expiries(game: &mut Game) {
    let current_date = game.clock.current_date.date_naive();

    let expired_player_indices: Vec<usize> = game
        .players
        .iter()
        .enumerate()
        .filter_map(|(index, player)| {
            let days_remaining =
                contract_days_remaining(player.contract_end.as_deref(), current_date)?;
            if player.team_id.is_some() && days_remaining <= 0 {
                Some(index)
            } else {
                None
            }
        })
        .collect();

    for player_index in expired_player_indices {
        release_player_contract(game, player_index, ContractReleaseReason::Expired);
    }
}

fn owned_player<'a>(game: &'a Game, player_id: &str) -> Result<&'a Player, String> {
    let manager_team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or("be.error.noTeamAssigned".to_string())?;
    let player = game
        .players
        .iter()
        .find(|candidate| candidate.id == player_id)
        .ok_or("be.error.playerNotFound".to_string())?;

    if player.team_id.as_deref() != Some(manager_team_id) {
        return Err(ERR_PLAYER_NOT_OWNED_BY_CLUB.to_string());
    }

    Ok(player)
}

fn owned_player_index(game: &Game, player_id: &str) -> Result<usize, String> {
    let manager_team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or("be.error.noTeamAssigned".to_string())?;
    let player_index = game
        .players
        .iter()
        .position(|candidate| candidate.id == player_id)
        .ok_or("be.error.playerNotFound".to_string())?;

    if game.players[player_index].team_id.as_deref() != Some(manager_team_id) {
        return Err(ERR_PLAYER_NOT_OWNED_BY_CLUB.to_string());
    }

    Ok(player_index)
}

fn termination_severance_cost(player: &Player, current_date: NaiveDate) -> i64 {
    let remaining_days = contract_days_remaining(player.contract_end.as_deref(), current_date)
        .unwrap_or(0)
        .max(0);
    let remaining_weeks = (remaining_days + 6) / 7;

    remaining_weeks * i64::from(player.wage)
}

fn backend_text_with_param(key: &str, param_name: &str, param_value: &str) -> String {
    let mut message = String::with_capacity(key.len() + param_name.len() + param_value.len() + 2);
    message.push_str(key);
    message.push('?');
    message.push_str(param_name);
    message.push('=');
    message.push_str(param_value);
    message
}

pub(crate) fn expected_wage(player: &Player, team: &Team, current_date: NaiveDate) -> u32 {
    let mut wage = reference_player_wage(player) as f32;
    let age = player_age_on(current_date, &player.date_of_birth);
    let remaining_days = remaining_contract_days(player, current_date);

    if age <= 27 {
        wage *= 1.05;
    } else if age >= 32 {
        wage *= 0.95;
    }

    if player.morale <= 50 {
        wage *= 1.10;
    }

    wage *= importance_wage_multiplier(player);

    if team.reputation < 40 {
        wage *= 1.05;
    }

    if remaining_days <= 180 {
        wage *= 1.10;
    } else if remaining_days <= 365 {
        wage *= 1.05;
    }

    let rounded = round_up_to_nearest_thousand(wage.ceil() as u32);
    rounded.max(reference_player_wage(player))
}

fn reference_player_wage(player: &Player) -> u32 {
    if player.wage > 0 {
        return player.wage;
    }

    let derived_wage =
        (player.market_value / MARKET_VALUE_TO_WAGE_RATIO).max(MINIMUM_DEFAULT_WAGE);

    round_up_to_nearest_thousand(derived_wage.min(u32::MAX as u64) as u32)
}

fn importance_wage_multiplier(player: &Player) -> f32 {
    if player.market_value >= 2_000_000 {
        return 1.18;
    }

    if player.market_value >= 750_000 {
        return 1.10;
    }

    if player.market_value <= 150_000 {
        return 0.95;
    }

    1.0
}

pub(crate) fn expected_contract_years(player: &Player, current_date: NaiveDate) -> u32 {
    let age = player_age_on(current_date, &player.date_of_birth);

    if age <= 28 {
        return 3;
    }

    if age <= 32 {
        return 2;
    }

    1
}

fn minimum_acceptable_wage(current_wage: u32) -> u32 {
    ((current_wage as f32) * 0.85).floor() as u32
}

fn next_renewal_round(player: &Player, today: Option<&str>) -> u8 {
    let Some(state) = player.morale_core.renewal_state.as_ref() else {
        return 1;
    };

    if let Some(today) = today {
        if state.last_attempt_date.as_deref() != Some(today) {
            return 1;
        }
    }

    state.conversation_round.saturating_add(1).max(1)
}

fn cool_stale_renewal_session(player: &mut Player, current_date: NaiveDate) -> bool {
    let Some(state) = player.morale_core.renewal_state.as_mut() else {
        return false;
    };

    if matches!(
        state.status,
        RenewalSessionStatus::Blocked | RenewalSessionStatus::Agreed | RenewalSessionStatus::Idle
    ) {
        return false;
    }

    let Some(last_attempt_date) = state.last_attempt_date.as_deref() else {
        return false;
    };

    let Ok(last_attempt) = NaiveDate::parse_from_str(last_attempt_date, "%Y-%m-%d") else {
        return false;
    };

    if (current_date - last_attempt).num_days() < RENEWAL_SESSION_STALE_DAYS {
        return false;
    }

    state.status = RenewalSessionStatus::Idle;
    state.last_outcome = None;
    state.conversation_round = 0;
    true
}

fn build_renewal_feedback(
    player: &Player,
    current_date: NaiveDate,
    decision: RenewalDecision,
    session_status: RenewalSessionStatus,
    round: u8,
    expected_wage: u32,
    relationship_blocked: bool,
) -> NegotiationFeedback {
    let trust = player.morale_core.manager_trust;
    let remaining_days = remaining_contract_days(player, current_date);
    let urgency_pressure = if remaining_days <= 90 {
        24
    } else if remaining_days <= 180 {
        16
    } else if remaining_days <= 365 {
        8
    } else {
        2
    };
    let morale_pressure = if player.morale <= 40 {
        24
    } else if player.morale <= 60 {
        12
    } else {
        0
    };
    let trust_pressure = if trust <= 25 {
        26
    } else if trust <= 40 {
        12
    } else {
        0
    };
    let value_pressure = if player.market_value >= 2_000_000 {
        12
    } else if player.market_value >= 750_000 {
        6
    } else {
        0
    };
    let tension = (22 + urgency_pressure + morale_pressure + trust_pressure + value_pressure)
        .clamp(10, 92) as u8;
    let patience = (100_i32 - i32::from(round.saturating_sub(1)) * 18 - i32::from(tension) / 3)
        .clamp(18, 92) as u8;

    let (mood, headline_key, detail_key) = if session_status == RenewalSessionStatus::Blocked {
        (
            NegotiationMood::Guarded,
            "playerProfile.renewalFeedbackBlockedHeadline",
            Some("playerProfile.renewalFeedbackBlockedDetail"),
        )
    } else if decision == RenewalDecision::Accepted && round >= 2 {
        (
            NegotiationMood::Positive,
            "playerProfile.renewalFeedbackAcceptedLateHeadline",
            Some("playerProfile.renewalFeedbackAcceptedLateDetail"),
        )
    } else if decision == RenewalDecision::Accepted {
        (
            NegotiationMood::Positive,
            "playerProfile.renewalFeedbackAcceptedHeadline",
            Some("playerProfile.renewalFeedbackAcceptedDetail"),
        )
    } else if relationship_blocked || tension >= 70 {
        (
            NegotiationMood::Tense,
            "playerProfile.renewalFeedbackTenseHeadline",
            Some("playerProfile.renewalFeedbackTenseDetail"),
        )
    } else if expected_wage > player.wage || round >= 2 {
        (
            NegotiationMood::Firm,
            "playerProfile.renewalFeedbackFirmHeadline",
            Some("playerProfile.renewalFeedbackFirmDetail"),
        )
    } else {
        (
            NegotiationMood::Calm,
            "playerProfile.renewalFeedbackCalmHeadline",
            Some("playerProfile.renewalFeedbackCalmDetail"),
        )
    };

    NegotiationFeedback {
        mood,
        headline_key: headline_key.to_string(),
        detail_key: detail_key.map(str::to_string),
        tension,
        patience,
        round,
        params: HashMap::new(),
    }
}

fn should_manual_renewal_fail_on_relationship(
    player: &Player,
    expected_wage: u32,
    offered_wage: u32,
) -> bool {
    let trust = player.morale_core.manager_trust;
    let relationship_margin = if trust <= 20 {
        2_000
    } else if trust <= 30 {
        1_000
    } else {
        0
    };

    relationship_margin > 0 && offered_wage < expected_wage.saturating_add(relationship_margin)
}

pub(crate) fn has_active_manager_block(player: &Player, current_date: NaiveDate) -> bool {
    let Some(state) = player.morale_core.renewal_state.as_ref() else {
        return false;
    };

    if state.status != RenewalSessionStatus::Blocked {
        return false;
    }

    let Some(blocked_until) = state.manager_blocked_until.as_deref() else {
        return true;
    };

    NaiveDate::parse_from_str(blocked_until, "%Y-%m-%d")
        .map(|blocked_until| blocked_until >= current_date)
        .unwrap_or(true)
}

fn player_age_on(current_date: NaiveDate, date_of_birth: &str) -> i32 {
    let Ok(dob) = NaiveDate::parse_from_str(date_of_birth, "%Y-%m-%d") else {
        return 30;
    };

    let mut age = current_date.year() - dob.year();
    if current_date.ordinal() < dob.ordinal() {
        age -= 1;
    }
    age
}

fn remaining_contract_days(player: &Player, current_date: NaiveDate) -> i64 {
    contract_days_remaining(player.contract_end.as_deref(), current_date)
        .unwrap_or(0)
        .max(0)
}

pub(crate) fn round_up_to_nearest_thousand(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }

    ((value + 999) / 1000) * 1000
}

fn contract_days_remaining(contract_end: Option<&str>, current_date: NaiveDate) -> Option<i64> {
    let contract_end = contract_end?;
    let contract_end_date = NaiveDate::parse_from_str(contract_end, "%Y-%m-%d").ok()?;
    Some((contract_end_date - current_date).num_days())
}

fn transfer_window_is_open(game: &Game) -> bool {
    matches!(
        game.season_context.transfer_window.status,
        TransferWindowStatus::Open | TransferWindowStatus::DeadlineDay
    )
}

fn remove_player_from_team_references(team: &mut Team, player_id: &str) {
    team.remove_player_references(player_id);
}

fn release_player_contract(game: &mut Game, player_index: usize, reason: ContractReleaseReason) {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let player_id = game.players[player_index].id.clone();
    let player_name = game.players[player_index].match_name.clone();
    let team_id = game.players[player_index].team_id.clone();

    let Some(team_id) = team_id.as_deref() else {
        return;
    };
    let Some(team) = game
        .teams
        .iter_mut()
        .find(|candidate| candidate.id == team_id)
    else {
        return;
    };

    let team_name = team.name.clone();
    remove_player_from_team_references(team, &player_id);

    let player = &mut game.players[player_index];
    player.team_id = None;
    player.contract_end = None;
    player.wage = 0;
    player.transfer_listed = false;
    player.loan_listed = false;
    player.transfer_offers.clear();
    player.morale_core.renewal_state = None;

    let message = match reason {
        ContractReleaseReason::Expired => {
            contract_expired_message(&player_id, &player_name, &team_name, &today)
        }
        ContractReleaseReason::ManagerTermination { severance_cost } => {
            contract_terminated_message(
                &player_id,
                &player_name,
                &team_name,
                severance_cost,
                &today,
            )
        }
    };

    game.messages.push(message);
}

fn contract_expired_message(
    player_id: &str,
    player_name: &str,
    team_name: &str,
    date: &str,
) -> InboxMessage {
    let mut i18n_params = std::collections::HashMap::new();
    i18n_params.insert("player".to_string(), player_name.to_string());
    i18n_params.insert("team".to_string(), team_name.to_string());

    InboxMessage::new(
        format!("contract_expired_{}", player_id),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("")
    .with_i18n(
        "be.msg.contractExpired.subject",
        "be.msg.contractExpired.body",
        i18n_params,
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

fn contract_terminated_message(
    player_id: &str,
    player_name: &str,
    team_name: &str,
    severance_cost: i64,
    date: &str,
) -> InboxMessage {
    let mut i18n_params = std::collections::HashMap::new();
    i18n_params.insert("player".to_string(), player_name.to_string());
    i18n_params.insert("team".to_string(), team_name.to_string());
    i18n_params.insert("severance".to_string(), severance_cost.to_string());

    InboxMessage::new(
        format!("contract_terminated_{}", player_id),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("")
    .with_i18n(
        "be.msg.contractTerminated.subject",
        "be.msg.contractTerminated.body",
        i18n_params,
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

fn free_agent_signed_message(
    player_id: &str,
    player_name: &str,
    team_name: &str,
    contract_years: u32,
    date: &str,
) -> InboxMessage {
    let mut i18n_params = std::collections::HashMap::new();
    i18n_params.insert("player".to_string(), player_name.to_string());
    i18n_params.insert("team".to_string(), team_name.to_string());
    i18n_params.insert("years".to_string(), contract_years.to_string());

    InboxMessage::new(
        format!("free_agent_signed_{}", player_id),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("")
    .with_i18n(
        "be.msg.freeAgentSigned.subject",
        "be.msg.freeAgentSigned.body",
        i18n_params,
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}
