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
use domain::season::TransferWindowStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

mod bids;
mod consts;
mod execution;
mod lifecycle;
mod loans;
mod market;
mod registration;

pub use bids::*;
use consts::*;
pub use execution::*;
use lifecycle::*;
pub use loans::*;
pub use market::*;
use registration::*;

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
    /// Weekly wage figures (annual / 52), so the UI can show the same before →
    /// after breakdown the renewal projection uses instead of a lone usage %.
    pub current_weekly_wage_spend: i64,
    pub projected_weekly_wage_spend: i64,
    pub weekly_wage_budget: i64,
    pub incoming_player_weekly_wage: i64,
    pub projected_wage_budget_usage_pct: i64,
    pub exceeds_transfer_budget: bool,
    pub exceeds_finance: bool,
    /// Set when the window is closed and this bid would land as
    /// PendingRegistration: the date the debit will actually fire. `None` when
    /// the window is open and the debit fires immediately on acceptance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_registration_date: Option<String>,
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

pub(crate) enum PlayerImportance {
    Key,
    Regular,
    Fringe,
}

pub(crate) struct MarketCandidate {
    player_id: String,
    owner_team_id: String,
    score: i32,
    fee: u64,
}

/// A player worth pursuing, with buyer-independent appeal precomputed once so
/// every club can reuse it instead of re-scoring the whole world.
pub(crate) struct MarketTarget {
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
}

pub(crate) struct LoanMarketCandidate {
    player_id: String,
    wage_contribution_pct: u8,
    buy_option_fee: Option<u64>,
    score: i32,
}

#[cfg(test)]
mod tests;
