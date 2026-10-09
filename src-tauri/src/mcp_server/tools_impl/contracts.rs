//! MCP tool implementations: contracts

use mcp_results::contracts::{
    ContractTerminated, ExitIntentCleared, ExitIntentSet, NegotiationFeedback, RenewalPreview,
    RenewalProposed, RenewalsDelegated, TerminationPreview,
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
    let _response = crate::commands::contracts::preview_renewal_financial_impact_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
    )?;

    Ok(RenewalPreview { weekly_wage })
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
    let _response = crate::commands::contracts::preview_contract_termination_internal(
        &ctx.state_manager,
        &player_id,
    )?;

    Ok(TerminationPreview { player_id })
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

// ─── inbox_get_messages ─────────────────────────────────────────────────────
