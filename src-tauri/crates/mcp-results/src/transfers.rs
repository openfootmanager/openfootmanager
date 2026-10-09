//! Results of the transfer tools.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tool_results;

/// Players the market listing shows before it summarises the rest.
const LISTED_IN_TEXT: usize = 30;

fn yes_no(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        "No"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferListingToggled {
    pub player_id: String,
    pub player_name: String,
    pub listed: bool,
}

impl fmt::Display for TransferListingToggled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.listed {
            "Transfer Listed ✓"
        } else {
            "Not Listed"
        };
        write!(
            f,
            "## Transfer Status Updated\n\n**{}**: {}",
            self.player_name, status
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoanListingToggled {
    pub player_id: String,
    pub player_name: String,
    pub listed: bool,
}

impl fmt::Display for LoanListingToggled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.listed {
            "Loan Listed ✓"
        } else {
            "Not Listed"
        };
        write!(
            f,
            "## Loan Status Updated\n\n**{}**: {}",
            self.player_name, status
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BidMade {
    pub player_name: String,
    pub fee: u64,
    pub decision: String,
    pub suggested_fee: Option<u64>,
    pub is_terminal: bool,
    pub mood: String,
    pub tension: u8,
    pub patience: u8,
    pub round: u8,
}

impl fmt::Display for BidMade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Transfer Bid: {} — {} 💰\n\n",
            self.player_name, self.fee
        )?;
        writeln!(f, "**Decision**: {}", self.decision)?;
        if let Some(suggested) = self.suggested_fee {
            writeln!(f, "**Suggested Fee**: {suggested}")?;
        }
        writeln!(f, "**Terminal**: {}", self.is_terminal)?;
        writeln!(f, "**Mood**: {}", self.mood)?;
        writeln!(f, "**Tension**: {}/100", self.tension)?;
        writeln!(f, "**Patience**: {}/100", self.patience)?;
        writeln!(f, "**Round**: {}", self.round)?;
        if self.is_terminal {
            write!(f, "\n✅ Negotiation complete.")
        } else {
            write!(
                f,
                "\n🔄 Negotiation continues — make another bid or walk away."
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BidPreview {
    pub player_name: String,
    pub fee: u64,
    pub transfer_budget_before: i64,
    pub transfer_budget_after: i64,
    pub finance_before: i64,
    pub finance_after: i64,
    pub current_weekly_wage_spend: i64,
    pub projected_weekly_wage_spend: i64,
    pub weekly_wage_budget: i64,
    pub projected_wage_budget_usage_pct: i64,
    pub exceeds_transfer_budget: bool,
    pub exceeds_finance: bool,
}

impl fmt::Display for BidPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Transfer Bid Preview: {} — {} 💰\n\n| Field | Value |\n|-------|-------|\n| Transfer Budget Before | {} |\n| Transfer Budget After | {} |\n| Finance Before | {} |\n| Finance After | {} |\n| Weekly Wage Bill Before | {} |\n| Weekly Wage Bill After | {} |\n| Weekly Wage Budget | {} |\n| Projected Wage Usage | {}% |\n| Exceeds Transfer Budget | {} |\n| Exceeds Finance | {} |\n\nThis is a preview — no bid was made.",
            self.player_name,
            self.fee,
            self.transfer_budget_before,
            self.transfer_budget_after,
            self.finance_before,
            self.finance_after,
            self.current_weekly_wage_spend,
            self.projected_weekly_wage_spend,
            self.weekly_wage_budget,
            self.projected_wage_budget_usage_pct,
            yes_no(self.exceeds_transfer_budget),
            yes_no(self.exceeds_finance),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfferAnswered {
    pub player_id: String,
    pub offer_id: String,
    pub accepted: bool,
}

impl fmt::Display for OfferAnswered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let action = if self.accepted {
            "accepted"
        } else {
            "rejected"
        };
        write!(
            f,
            "## Offer {}\n\nOffer {} for player {}.",
            action, self.offer_id, self.player_id
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterOffered {
    pub requested_fee: u64,
    pub decision: String,
    pub suggested_fee: Option<u64>,
    pub is_terminal: bool,
}

impl fmt::Display for CounterOffered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "## Counter Offer: {} 💰\n\n", self.requested_fee)?;
        writeln!(f, "**Decision**: {}", self.decision)?;
        if let Some(suggested) = self.suggested_fee {
            writeln!(f, "**Suggested Fee**: {suggested}")?;
        }
        writeln!(f, "**Terminal**: {}", self.is_terminal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketPlayer {
    pub id: String,
    pub name: String,
    pub position: String,
    pub age: String,
    pub ovr: u8,
    /// `None` for a free agent.
    pub team: Option<String>,
    pub transfer_listed: bool,
    pub loan_listed: bool,
    pub wage: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferMarket {
    pub players: Vec<MarketPlayer>,
}

impl fmt::Display for TransferMarket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.players.is_empty() {
            return write!(
                f,
                "## Transfer Market\n\nNo players found matching criteria."
            );
        }
        write!(
            f,
            "## Transfer Market ({} players)\n\n| ID | Name | Pos | Age | OVR | Team | Listed | Wage |\n|----|------|-----|-----|-----|------|--------|------|\n",
            self.players.len()
        )?;
        for p in self.players.iter().take(LISTED_IN_TEXT) {
            let listed = if p.transfer_listed {
                "T"
            } else if p.loan_listed {
                "L"
            } else {
                "-"
            };
            writeln!(
                f,
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                p.id,
                p.name,
                p.position,
                p.age,
                p.ovr,
                p.team.as_deref().unwrap_or("Free"),
                listed,
                p.wage,
            )?;
        }
        if self.players.len() > LISTED_IN_TEXT {
            write!(
                f,
                "\n... and {} more. Use filters to narrow results.",
                self.players.len() - LISTED_IN_TEXT
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeAgentOffered {
    pub weekly_wage: u32,
    pub contract_years: u32,
    pub outcome: String,
}

impl fmt::Display for FreeAgentOffered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Free Agent Offer\n\n**Wage**: {}/wk × {}yr\n**Outcome**: {}",
            self.weekly_wage, self.contract_years, self.outcome
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeAgentPreview {
    pub weekly_wage: u32,
    pub current_weekly_wage_spend: i64,
    pub projected_weekly_wage_spend: i64,
    pub annual_wage_budget: i64,
    pub annual_soft_cap: i64,
    pub current_cash_runway_weeks: Option<i64>,
    pub projected_cash_runway_weeks: Option<i64>,
    pub currently_over_budget: bool,
    pub policy_allows: bool,
}

impl fmt::Display for FreeAgentPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let weeks = |weeks: Option<i64>| weeks.map_or_else(|| "N/A".to_string(), |w| w.to_string());
        write!(
            f,
            "## Free Agent Preview\n\n| Field | Value |\n|-------|-------|\n| Weekly Wage Offered | {}/wk |\n| Current Weekly Wage Bill | {} |\n| Projected Weekly Wage Bill | {} |\n| Weekly Wage Budget | {} |\n| Weekly Soft Cap | {} |\n| Cash Runway (weeks) | {} → {} |\n| Currently Over Budget | {} |\n| Policy Allows | {} |\n\nThis is a preview — no offer was made.",
            self.weekly_wage,
            self.current_weekly_wage_spend,
            self.projected_weekly_wage_spend,
            self.annual_wage_budget,
            self.annual_soft_cap,
            weeks(self.current_cash_runway_weeks),
            weeks(self.projected_cash_runway_weeks),
            yes_no(self.currently_over_budget),
            yes_no(self.policy_allows),
        )
    }
}

tool_results!(
    TransferListingToggled,
    LoanListingToggled,
    BidMade,
    BidPreview,
    OfferAnswered,
    CounterOffered,
    TransferMarket,
    FreeAgentOffered,
    FreeAgentPreview,
);
