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
    #[serde(default)]
    pub current_annual_wage_bill: i64,
    #[serde(default)]
    pub projected_annual_wage_bill: i64,
    #[serde(default)]
    pub annual_wage_budget: i64,
    #[serde(default)]
    pub annual_soft_cap: i64,
    #[serde(default)]
    pub current_weekly_wage_spend: i64,
    #[serde(default)]
    pub projected_weekly_wage_spend: i64,
    #[serde(default)]
    pub current_cash_runway_weeks: Option<i64>,
    #[serde(default)]
    pub projected_cash_runway_weeks: Option<i64>,
    #[serde(default)]
    pub currently_over_budget: bool,
    #[serde(default)]
    pub policy_allows: bool,
}

impl fmt::Display for RenewalPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Renewal Preview\n\n**Wage Offer**: {}/wk\n**Annual Wage Bill**: {} → {}\n**Annual Wage Budget**: {}\n**Annual Soft Cap**: {}\n**Weekly Wage Spend**: {} → {}\n**Cash Runway (weeks)**: {} → {}\n**Currently Over Budget**: {}\n**Policy Allows Offer**: {}\n\nThis is a preview — no offer was made.",
            self.weekly_wage,
            self.current_annual_wage_bill,
            self.projected_annual_wage_bill,
            self.annual_wage_budget,
            self.annual_soft_cap,
            self.current_weekly_wage_spend,
            self.projected_weekly_wage_spend,
            runway(self.current_cash_runway_weeks),
            runway(self.projected_cash_runway_weeks),
            self.currently_over_budget,
            self.policy_allows,
        )
    }
}

fn runway(weeks: Option<i64>) -> String {
    weeks.map_or_else(|| "unavailable".to_string(), |weeks| weeks.to_string())
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
    #[serde(default)]
    pub player_name: String,
    #[serde(default)]
    pub severance_cost: i64,
    #[serde(default)]
    pub squad_safety: SquadSafetyReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadSafetyIssue {
    TooFewHealthyPlayers,
    NoHealthyGoalkeeper,
    IncompleteFormation,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquadSafetyReport {
    #[serde(default)]
    pub team_id: String,
    #[serde(default)]
    pub projected_roster_size: usize,
    #[serde(default)]
    pub healthy_players: usize,
    #[serde(default)]
    pub healthy_goalkeepers: usize,
    #[serde(default)]
    pub effective_xi_size: usize,
    #[serde(default)]
    pub can_field_matchday_squad: bool,
    #[serde(default)]
    pub missing_reasons: Vec<SquadSafetyIssue>,
}

impl fmt::Display for TerminationPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Termination Preview\n\n**Player**: {}\n**Severance Cost**: {}\n**Team**: {}\n**Projected Squad Safety**: {} healthy players ({} goalkeepers), {} effective XI players across {} roster spots; matchday squad: {}.",
            self.player_name,
            self.severance_cost,
            self.squad_safety.team_id,
            self.squad_safety.healthy_players,
            self.squad_safety.healthy_goalkeepers,
            self.squad_safety.effective_xi_size,
            self.squad_safety.projected_roster_size,
            self.squad_safety.can_field_matchday_squad,
        )
        ?;
        if !self.squad_safety.missing_reasons.is_empty() {
            write!(f, "\n**Squad Safety Issues**: ")?;
            for (index, issue) in self.squad_safety.missing_reasons.iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                let label = match issue {
                    SquadSafetyIssue::TooFewHealthyPlayers => "too few healthy players",
                    SquadSafetyIssue::NoHealthyGoalkeeper => "no healthy goalkeeper",
                    SquadSafetyIssue::IncompleteFormation => "incomplete formation",
                };
                write!(f, "{label}")?;
            }
        }
        write!(f, "\n\nThis is a preview — no contract was terminated.")
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

#[cfg(test)]
mod tests {
    use super::{RenewalPreview, SquadSafetyIssue, SquadSafetyReport, TerminationPreview};
    use serde_json::json;

    /// Given renewal data, when serialized, then structured and text results expose every field.
    #[test]
    fn renewal_preview_exposes_projection_in_structured_and_text_results() {
        let result = RenewalPreview {
            weekly_wage: 1500,
            current_annual_wage_bill: 1_000_000,
            projected_annual_wage_bill: 1_078_000,
            annual_wage_budget: 1_100_000,
            annual_soft_cap: 1_200_000,
            current_weekly_wage_spend: 19_230,
            projected_weekly_wage_spend: 20_730,
            current_cash_runway_weeks: Some(52),
            projected_cash_runway_weeks: Some(48),
            currently_over_budget: false,
            policy_allows: true,
        };

        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            json!({
                "weekly_wage": 1500,
                "current_annual_wage_bill": 1000000,
                "projected_annual_wage_bill": 1078000,
                "annual_wage_budget": 1100000,
                "annual_soft_cap": 1200000,
                "current_weekly_wage_spend": 19230,
                "projected_weekly_wage_spend": 20730,
                "current_cash_runway_weeks": 52,
                "projected_cash_runway_weeks": 48,
                "currently_over_budget": false,
                "policy_allows": true
            })
        );
        let text = result.to_string();
        assert!(text.contains("Annual Wage Bill**: 1000000 → 1078000"));
        assert!(text.contains("Policy Allows Offer**: true"));
    }

    /// Given a termination preview, when serialized, then cost and squad safety appear in both results.
    #[test]
    fn termination_preview_exposes_cost_and_squad_safety_in_structured_and_text_results() {
        let result = TerminationPreview {
            player_id: "player-1".to_string(),
            player_name: "Alex Example".to_string(),
            severance_cost: 12_000,
            squad_safety: SquadSafetyReport {
                team_id: "team-1".to_string(),
                projected_roster_size: 20,
                healthy_players: 18,
                healthy_goalkeepers: 2,
                effective_xi_size: 11,
                can_field_matchday_squad: true,
                missing_reasons: vec![SquadSafetyIssue::IncompleteFormation],
            },
        };

        let wire = serde_json::to_value(&result).unwrap();
        assert_eq!(wire["player_name"], "Alex Example");
        assert_eq!(wire["severance_cost"], 12000);
        assert_eq!(wire["squad_safety"]["can_field_matchday_squad"], true);
        assert_eq!(
            wire["squad_safety"]["missing_reasons"],
            json!(["incomplete_formation"])
        );
        let text = result.to_string();
        assert!(text.contains("Severance Cost**: 12000"));
        assert!(text.contains("incomplete formation"));
    }

    /// Given an older payload, when it is deserialized, then newly added fields use defaults.
    #[test]
    fn older_preview_payloads_default_new_fields() {
        let renewal: RenewalPreview = serde_json::from_value(json!({"weekly_wage": 1500})).unwrap();
        assert_eq!(renewal.projected_annual_wage_bill, 0);
        assert_eq!(renewal.projected_cash_runway_weeks, None);
        assert!(!renewal.policy_allows);

        let termination: TerminationPreview =
            serde_json::from_value(json!({"player_id": "player-1"})).unwrap();
        assert_eq!(termination.player_name, "");
        assert_eq!(termination.severance_cost, 0);
        assert_eq!(termination.squad_safety.projected_roster_size, 0);
    }
}
