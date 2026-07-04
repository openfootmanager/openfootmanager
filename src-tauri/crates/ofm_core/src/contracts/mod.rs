use crate::contract_wage_policy::{
    project_contract_offer_financial_impact,
    project_renewal_financial_impact as project_renewal_financial_impact_service,
    renewal_wage_policy_allows, renewal_wage_policy_error_message,
};
use crate::delegated_renewals::delegate_renewals as delegate_renewals_service;
use crate::game::Game;
use crate::squad_safety::{SquadSafetyReport, project_user_team_release_safety};
use chrono::{Datelike, Days, Months, NaiveDate};
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use domain::negotiation::{NegotiationFeedback, NegotiationMood};
use domain::player::{
    ContractExitIntent, ContractRenewalState, Player, PlayerMovementEntry, PlayerMovementKind,
    RenewalSessionOutcome, RenewalSessionStatus,
};
use domain::team::{FinancialTransaction, FinancialTransactionKind, Team};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod consts;
mod expiry;
mod free_agent;
mod helpers;
mod renewals;
mod termination;

pub(crate) use self::consts::*;
pub(crate) use self::helpers::*;
// `pub` so the public entry points stay resolvable as `ofm_core::contracts::*`.
pub use self::expiry::*;
pub use self::free_agent::*;
pub use self::renewals::*;
pub use self::termination::*;

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

// `pub(crate)`: expiry::release_player_contract takes this and is re-exported for
// termination::terminate_contract_now, so the enum must be at least as visible.
pub(crate) enum ContractReleaseReason {
    Expired,
    ManagerTermination { severance_cost: i64 },
}
