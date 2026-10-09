//! Results of the contract tools.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tool_results;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NegotiationFeedback {
    pub mood: String,
    pub tension: u8,
    pub patience: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenewalProposed {
    pub player_name: String,
    pub weekly_wage: u32,
    pub contract_years: u32,
    pub outcome: String,
    pub suggested_wage: Option<u32>,
    pub suggested_years: Option<u32>,
    pub session_status: String,
    pub is_terminal: bool,
    pub cooled_off: bool,
    pub feedback: Option<NegotiationFeedback>,
}

impl fmt::Display for RenewalProposed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Contract Renewal: {} — {}💰/wk × {}yr\n\n",
            self.player_name, self.weekly_wage, self.contract_years
        )?;
        writeln!(f, "**Outcome**: {}", self.outcome)?;
        if let Some(wage) = self.suggested_wage {
            writeln!(f, "**Suggested Wage**: {wage}/wk")?;
        }
        if let Some(years) = self.suggested_years {
            writeln!(f, "**Suggested Years**: {years}")?;
        }
        writeln!(f, "**Session**: {}", self.session_status)?;
        writeln!(f, "**Terminal**: {}", self.is_terminal)?;
        writeln!(f, "**Cooled Off**: {}", self.cooled_off)?;
        if let Some(feedback) = &self.feedback {
            writeln!(f, "**Mood**: {}", feedback.mood)?;
            writeln!(f, "**Tension**: {}/100", feedback.tension)?;
            writeln!(f, "**Patience**: {}/100", feedback.patience)?;
        }
        if self.is_terminal {
            write!(f, "\n✅ Negotiation complete.")
        } else if self.cooled_off {
            write!(f, "\n❄️ Player has cooled off — wait before re-offering.")
        } else {
            write!(f, "\n🔄 Negotiation continues — adjust your offer.")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenewalsDelegated {
    pub success_count: u32,
    pub failure_count: u32,
    pub stalled_count: u32,
    pub max_wage_increase_pct: u32,
    pub max_contract_years: u32,
}

impl fmt::Display for RenewalsDelegated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Delegated Renewals\n\n**Results**: {} success, {} failed, {} stalled.\n**Max Wage Increase**: {}%\n**Max Years**: {}",
            self.success_count,
            self.failure_count,
            self.stalled_count,
            self.max_wage_increase_pct,
            self.max_contract_years
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenewalPreview {
    pub weekly_wage: u32,
}

impl fmt::Display for RenewalPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Renewal Preview\n\n**Wage Offer**: {}/wk\nThis is a preview — no offer was made.",
            self.weekly_wage
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitIntentSet {
    pub player_id: String,
    pub player_name: String,
}

impl fmt::Display for ExitIntentSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Exit Intent Set\n\n**{}**: Contract will be allowed to expire.",
            self.player_name
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitIntentCleared {
    pub player_id: String,
}

impl fmt::Display for ExitIntentCleared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Exit Intent Cleared\n\nContract will proceed normally."
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminationPreview {
    pub player_id: String,
}

impl fmt::Display for TerminationPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Termination Preview\n\n**Cost**: (see projection details)\nThis is a preview — no contract was terminated."
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractTerminated {
    pub player_id: String,
    pub player_name: String,
}

impl fmt::Display for ContractTerminated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Contract Terminated\n\n**{}** has been released.",
            self.player_name
        )
    }
}

tool_results!(
    RenewalProposed,
    RenewalsDelegated,
    RenewalPreview,
    ExitIntentSet,
    ExitIntentCleared,
    TerminationPreview,
    ContractTerminated,
);
