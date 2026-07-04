use crate::contract_wage_policy::{
    renewal_wage_policy_error_message, wage_policy_allows_projection,
};
use crate::finances::calc_annual_wages;
use crate::game::Game;
use chrono::{Datelike, Duration, NaiveDate};
use domain::league::CompletedTransfer;
use domain::negotiation::{NegotiationFeedback, NegotiationMood};
use domain::player::{
    ActiveLoan, LoanOfferStatus, PlayerMovementEntry, PlayerMovementKind, Position,
    TransferOfferStatus,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

mod bids;
mod consts;
mod execution;
mod loans;
mod market;
mod registration;

pub(crate) use self::consts::*;
pub(crate) use self::registration::*;
// `pub` (not `pub(crate)`): these clusters re-export public entry points that
// external crates call as `ofm_core::transfers::*` (process_*, make_transfer_bid,
// respond_to_offer, counter_offer, project_transfer_bid_financial_impact, the
// loan offer/response/counter fns, exercise_loan_buy_option,
// seed_opening_ai_loan_market, process_loan_*, evaluate_transfer_market,
// generate_incoming_transfer_offers).
pub use self::bids::*;
pub use self::execution::*;
pub use self::loans::*;
pub use self::market::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferNegotiationDecision {
    Accepted,
    Rejected,
    CounterOffer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferNegotiationOutcome {
    pub decision: TransferNegotiationDecision,
    pub suggested_fee: Option<u64>,
    pub is_terminal: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_date: Option<String>,
    pub feedback: NegotiationFeedback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferBidFinancialProjection {
    pub transfer_budget_before: i64,
    pub transfer_budget_after: i64,
    pub finance_before: i64,
    pub finance_after: i64,
    pub annual_wage_bill_before: i64,
    pub annual_wage_bill_after: i64,
    pub annual_wage_budget: i64,
    pub projected_wage_budget_usage_pct: i64,
    pub exceeds_transfer_budget: bool,
    pub exceeds_finance: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoanOfferDecision {
    Accepted,
    Rejected,
    CounterOffer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoanOfferOutcome {
    pub decision: LoanOfferDecision,
    pub offer_id: String,
    pub suggested_wage_contribution_pct: Option<u8>,
    pub suggested_end_date: Option<String>,
    pub suggested_buy_option_fee: Option<u64>,
    pub is_terminal: bool,
}

// `pub(crate)`: market::infer_player_importance returns this, and that helper is
// re-exported for the bid/loan clusters, so the enum must be at least as visible.
pub(crate) enum PlayerImportance {
    Key,
    Regular,
    Fringe,
}

// `pub(crate)`: bids::create_incoming_user_offer takes `&MarketCandidate`, so
// the type must be at least as visible as that re-exported helper.
pub(crate) struct MarketCandidate {
    player_id: String,
    owner_team_id: String,
    score: i32,
    fee: u64,
}

/// A player worth pursuing, with buyer-independent appeal precomputed once so
/// every club can reuse it instead of re-scoring the whole world.
struct MarketTarget {
    player_id: String,
    owner_team_id: String,
    is_user_owned: bool,
    score: i32,
    fee: u64,
    /// Broad position group (0=GK, 1=DEF, 2=MID, 3=FWD), used to gate buyers
    /// that are already stacked in that area.
    position_group_index: usize,
    /// Reputation of the player's current club, used for reputation-fit gating.
    owner_reputation: u32,
    /// Clubs that already hold a pending bid (only tracked for user players,
    /// the one case where we must avoid duplicate incoming offers).
    pending_offer_clubs: HashSet<String>,
}

struct LoanMarketCandidate {
    player_id: String,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
    score: i32,
}

fn withdraw_pending_transfer_offers(player: &mut domain::player::Player) {
    for offer in &mut player.transfer_offers {
        if offer.status == TransferOfferStatus::Pending {
            offer.status = TransferOfferStatus::Withdrawn;
            offer.suggested_counter_fee = None;
        }
    }
}

fn should_generate_major_transfer_news(player: &domain::player::Player, fee: u64) -> bool {
    fee >= 1_000_000 || player.market_value >= 1_000_000
}

fn team_name_or_id(game: &Game, team_id: &str) -> String {
    game.teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| team_id.to_string())
}

#[cfg(test)]
mod tests;
