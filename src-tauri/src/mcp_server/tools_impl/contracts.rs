//! MCP tool implementations: contracts

use mcp_results::contracts::{
    ContractTerminated, ExitIntentCleared, ExitIntentSet, NegotiationFeedback, RenewalPreview,
    RenewalProposed, RenewalsDelegated, SquadSafetyIssue as ResultSquadSafetyIssue,
    SquadSafetyReport as ResultSquadSafetyReport, TerminationPreview,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game, serde_label};
use std::sync::Arc;

// ─── contract_propose_renewal ───────────────────────────────────────────────

pub fn contract_propose_renewal(
    ctx: Arc<McpContext>,
    player_id: String,
    weekly_wage: u32,
    contract_years: u32,
) -> Result<RenewalProposed, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    let response = crate::commands::contracts::propose_renewal_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
        contract_years,
    )?;

    let renewal = RenewalProposed {
        player_name,
        weekly_wage,
        contract_years,
        outcome: serde_label(&response.outcome),
        suggested_wage: response.suggested_wage,
        suggested_years: response.suggested_years,
        session_status: response.session_status,
        is_terminal: response.is_terminal,
        cooled_off: response.cooled_off,
        feedback: response
            .feedback
            .as_ref()
            .map(|feedback| NegotiationFeedback {
                mood: serde_label(&feedback.mood),
                tension: feedback.tension,
                patience: feedback.patience,
            }),
    };

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(renewal)
}

// ─── contract_delegate_renewals ─────────────────────────────────────────────

// ─── contract_delegate_renewals ─────────────────────────────────────────────

pub fn contract_delegate_renewals(
    ctx: Arc<McpContext>,
    player_ids: Option<Vec<String>>,
    max_wage_increase_pct: u32,
    max_contract_years: u32,
) -> Result<RenewalsDelegated, String> {
    let response = crate::commands::contracts::delegate_renewals_internal(
        &ctx.state_manager,
        player_ids,
        max_wage_increase_pct,
        max_contract_years,
    )?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(RenewalsDelegated {
        success_count: response.report.success_count,
        failure_count: response.report.failure_count,
        stalled_count: response.report.stalled_count,
        max_wage_increase_pct,
        max_contract_years,
    })
}

// ─── contract_preview_renewal ───────────────────────────────────────────────

// ─── contract_preview_renewal ───────────────────────────────────────────────

pub fn contract_preview_renewal(
    ctx: Arc<McpContext>,
    player_id: String,
    weekly_wage: u32,
) -> Result<RenewalPreview, String> {
    let response = crate::commands::contracts::preview_renewal_financial_impact_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
    )?;

    Ok(renewal_preview(weekly_wage, response))
}

fn renewal_preview(
    weekly_wage: u32,
    response: crate::commands::contracts::RenewalFinancialProjectionCommandResponse,
) -> RenewalPreview {
    let projection = response.projection;
    RenewalPreview {
        weekly_wage,
        current_annual_wage_bill: projection.current_annual_wage_bill,
        projected_annual_wage_bill: projection.projected_annual_wage_bill,
        annual_wage_budget: projection.annual_wage_budget,
        annual_soft_cap: projection.annual_soft_cap,
        current_weekly_wage_spend: projection.current_weekly_wage_spend,
        projected_weekly_wage_spend: projection.projected_weekly_wage_spend,
        current_cash_runway_weeks: projection.current_cash_runway_weeks,
        projected_cash_runway_weeks: projection.projected_cash_runway_weeks,
        currently_over_budget: projection.currently_over_budget,
        policy_allows: projection.policy_allows,
    }
}

// ─── contract_set_exit_intent ───────────────────────────────────────────────

// ─── contract_set_exit_intent ───────────────────────────────────────────────

pub fn contract_set_exit_intent(
    ctx: Arc<McpContext>,
    player_id: String,
    reason: Option<String>,
) -> Result<ExitIntentSet, String> {
    crate::commands::contracts::set_contract_exit_intent_internal(
        &ctx.state_manager,
        &player_id,
        reason,
    )?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let game = require_game(&ctx.state_manager)?;
    let player_name = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_else(|| player_id.clone());

    Ok(ExitIntentSet {
        player_id,
        player_name,
    })
}

// ─── contract_clear_exit_intent ─────────────────────────────────────────────

// ─── contract_clear_exit_intent ─────────────────────────────────────────────

pub fn contract_clear_exit_intent(
    ctx: Arc<McpContext>,
    player_id: String,
) -> Result<ExitIntentCleared, String> {
    crate::commands::contracts::clear_contract_exit_intent_internal(
        &ctx.state_manager,
        &player_id,
    )?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(ExitIntentCleared { player_id })
}

// ─── contract_preview_termination ───────────────────────────────────────────

// ─── contract_preview_termination ───────────────────────────────────────────

pub fn contract_preview_termination(
    ctx: Arc<McpContext>,
    player_id: String,
) -> Result<TerminationPreview, String> {
    let response = crate::commands::contracts::preview_contract_termination_internal(
        &ctx.state_manager,
        &player_id,
    )?;

    Ok(termination_preview(player_id, response))
}

fn termination_preview(
    player_id: String,
    response: crate::commands::contracts::ContractTerminationPreviewCommandResponse,
) -> TerminationPreview {
    let preview = response.preview;
    let squad_safety = preview.squad_safety;
    TerminationPreview {
        player_id,
        player_name: preview.player_name,
        severance_cost: preview.severance_cost,
        squad_safety: ResultSquadSafetyReport {
            team_id: squad_safety.team_id,
            projected_roster_size: squad_safety.projected_roster_size,
            healthy_players: squad_safety.healthy_players,
            healthy_goalkeepers: squad_safety.healthy_goalkeepers,
            effective_xi_size: squad_safety.effective_xi_size,
            can_field_matchday_squad: squad_safety.can_field_matchday_squad,
            missing_reasons: squad_safety
                .missing_reasons
                .into_iter()
                .map(|reason| match reason {
                    ofm_core::squad_safety::SquadSafetyIssue::TooFewHealthyPlayers => {
                        ResultSquadSafetyIssue::TooFewHealthyPlayers
                    }
                    ofm_core::squad_safety::SquadSafetyIssue::NoHealthyGoalkeeper => {
                        ResultSquadSafetyIssue::NoHealthyGoalkeeper
                    }
                    ofm_core::squad_safety::SquadSafetyIssue::IncompleteFormation => {
                        ResultSquadSafetyIssue::IncompleteFormation
                    }
                })
                .collect(),
        },
    }
}

// ─── contract_terminate ─────────────────────────────────────────────────────

// ─── contract_terminate ─────────────────────────────────────────────────────

pub fn contract_terminate(
    ctx: Arc<McpContext>,
    player_id: String,
) -> Result<ContractTerminated, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_else(|| player_id.clone());

    crate::commands::contracts::terminate_contract_now_internal(&ctx.state_manager, &player_id)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(ContractTerminated {
        player_id,
        player_name,
    })
}

#[cfg(test)]
mod tests {
    use super::{renewal_preview, termination_preview};
    use crate::commands::contracts::{
        ContractTerminationPreviewCommandResponse, RenewalFinancialProjectionCommandResponse,
    };
    use mcp_results::contracts::{
        RenewalPreview, SquadSafetyIssue as ResultSquadSafetyIssue,
        SquadSafetyReport as ResultSquadSafetyReport, TerminationPreview,
    };
    use ofm_core::contracts::{ContractTerminationPreview, RenewalFinancialProjection};
    use ofm_core::squad_safety::{SquadSafetyIssue, SquadSafetyReport};

    #[test]
    fn renewal_projection_is_preserved_for_allowed_and_refused_offers() {
        let allowed = renewal_preview(
            1500,
            RenewalFinancialProjectionCommandResponse {
                projection: RenewalFinancialProjection {
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
                },
            },
        );
        assert_eq!(
            allowed,
            RenewalPreview {
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
            }
        );

        let refused = renewal_preview(
            5000,
            RenewalFinancialProjectionCommandResponse {
                projection: RenewalFinancialProjection {
                    current_annual_wage_bill: 1_100_000,
                    projected_annual_wage_bill: 1_360_000,
                    annual_wage_budget: 1_100_000,
                    annual_soft_cap: 1_250_000,
                    current_weekly_wage_spend: 21_153,
                    projected_weekly_wage_spend: 26_153,
                    current_cash_runway_weeks: None,
                    projected_cash_runway_weeks: None,
                    currently_over_budget: false,
                    policy_allows: false,
                },
            },
        );
        assert_eq!(
            refused,
            RenewalPreview {
                weekly_wage: 5000,
                current_annual_wage_bill: 1_100_000,
                projected_annual_wage_bill: 1_360_000,
                annual_wage_budget: 1_100_000,
                annual_soft_cap: 1_250_000,
                current_weekly_wage_spend: 21_153,
                projected_weekly_wage_spend: 26_153,
                current_cash_runway_weeks: None,
                projected_cash_runway_weeks: None,
                currently_over_budget: false,
                policy_allows: false,
            }
        );
    }

    #[test]
    fn termination_preview_preserves_player_cost_and_squad_report() {
        let result = termination_preview(
            "player-1".to_string(),
            ContractTerminationPreviewCommandResponse {
                preview: ContractTerminationPreview {
                    player_id: "player-1".to_string(),
                    player_name: "Alex Example".to_string(),
                    severance_cost: 12_000,
                    squad_safety: SquadSafetyReport {
                        team_id: "team-1".to_string(),
                        projected_roster_size: 20,
                        healthy_players: 18,
                        healthy_goalkeepers: 2,
                        effective_xi_size: 11,
                        can_field_matchday_squad: false,
                        missing_reasons: vec![SquadSafetyIssue::IncompleteFormation],
                    },
                },
            },
        );

        assert_eq!(
            result,
            TerminationPreview {
                player_id: "player-1".to_string(),
                player_name: "Alex Example".to_string(),
                severance_cost: 12_000,
                squad_safety: ResultSquadSafetyReport {
                    team_id: "team-1".to_string(),
                    projected_roster_size: 20,
                    healthy_players: 18,
                    healthy_goalkeepers: 2,
                    effective_xi_size: 11,
                    can_field_matchday_squad: false,
                    missing_reasons: vec![ResultSquadSafetyIssue::IncompleteFormation],
                },
            }
        );
    }
}

// ─── inbox_get_messages ─────────────────────────────────────────────────────
