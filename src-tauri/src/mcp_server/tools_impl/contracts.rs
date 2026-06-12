//! MCP tool implementations: contracts

use std::sync::Arc;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game};
use crate::mcp_server::formatting::translate_error;

// ─── contract_propose_renewal ───────────────────────────────────────────────

pub fn contract_propose_renewal(ctx: Arc<McpContext>, player_id: String, weekly_wage: u32, contract_years: u32) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    let response = crate::commands::contracts::propose_renewal_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
        contract_years,
    )
    .map_err(|e| translate_error(&e))?;

    let mut output = format!("## Contract Renewal: {} — {}💰/wk × {}yr\n\n", player_name, weekly_wage, contract_years);
    output.push_str(&format!("**Outcome**: {:?}\n", response.outcome));
    if let Some(wage) = response.suggested_wage {
        output.push_str(&format!("**Suggested Wage**: {}/wk\n", wage));
    }
    if let Some(years) = response.suggested_years {
        output.push_str(&format!("**Suggested Years**: {}\n", years));
    }
    output.push_str(&format!("**Session**: {}\n", response.session_status));
    output.push_str(&format!("**Terminal**: {}\n", response.is_terminal));
    output.push_str(&format!("**Cooled Off**: {}\n", response.cooled_off));

    if let Some(ref feedback) = response.feedback {
        output.push_str(&format!("**Mood**: {:?}\n", feedback.mood));
        output.push_str(&format!("**Tension**: {}/100\n", feedback.tension));
        output.push_str(&format!("**Patience**: {}/100\n", feedback.patience));
    }

    if response.is_terminal {
        output.push_str("\n✅ Negotiation complete.");
    } else if response.cooled_off {
        output.push_str("\n❄️ Player has cooled off — wait before re-offering.");
    } else {
        output.push_str("\n🔄 Negotiation continues — adjust your offer.");
    }

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(output)
}

// ─── contract_delegate_renewals ─────────────────────────────────────────────

// ─── contract_delegate_renewals ─────────────────────────────────────────────

pub fn contract_delegate_renewals(ctx: Arc<McpContext>, player_ids: Option<Vec<String>>, max_wage_increase_pct: u32, max_contract_years: u32) -> Result<String, String> {
    let response = crate::commands::contracts::delegate_renewals_internal(
        &ctx.state_manager,
        player_ids,
        max_wage_increase_pct,
        max_contract_years,
    )
    .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Delegated Renewals\n\n**Results**: {} success, {} failed, {} stalled.\n**Max Wage Increase**: {}%\n**Max Years**: {}", response.report.success_count, response.report.failure_count, response.report.stalled_count, max_wage_increase_pct, max_contract_years))
}

// ─── contract_preview_renewal ───────────────────────────────────────────────

// ─── contract_preview_renewal ───────────────────────────────────────────────

pub fn contract_preview_renewal(ctx: Arc<McpContext>, player_id: String, weekly_wage: u32) -> Result<String, String> {
    let _response = crate::commands::contracts::preview_renewal_financial_impact_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
    )
    .map_err(|e| translate_error(&e))?;

    Ok(format!("## Renewal Preview\n\n**Wage Offer**: {}/wk\nThis is a preview — no offer was made.", weekly_wage))
}

// ─── contract_set_exit_intent ───────────────────────────────────────────────

// ─── contract_set_exit_intent ───────────────────────────────────────────────

pub fn contract_set_exit_intent(ctx: Arc<McpContext>, player_id: String, reason: Option<String>) -> Result<String, String> {
    crate::commands::contracts::set_contract_exit_intent_internal(&ctx.state_manager, &player_id, reason)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or(player_id);

    Ok(format!("## Exit Intent Set\n\n**{}**: Contract will be allowed to expire.", player_name))
}

// ─── contract_clear_exit_intent ─────────────────────────────────────────────

// ─── contract_clear_exit_intent ─────────────────────────────────────────────

pub fn contract_clear_exit_intent(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    crate::commands::contracts::clear_contract_exit_intent_internal(&ctx.state_manager, &player_id)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("## Exit Intent Cleared\n\nContract will proceed normally.".to_string())
}

// ─── contract_preview_termination ───────────────────────────────────────────

// ─── contract_preview_termination ───────────────────────────────────────────

pub fn contract_preview_termination(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    let _response = crate::commands::contracts::preview_contract_termination_internal(
        &ctx.state_manager,
        &player_id,
    )
    .map_err(|e| translate_error(&e))?;

    Ok(format!("## Termination Preview\n\n**Cost**: (see projection details)\nThis is a preview — no contract was terminated."))
}

// ─── contract_terminate ─────────────────────────────────────────────────────

// ─── contract_terminate ─────────────────────────────────────────────────────

pub fn contract_terminate(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_else(|| player_id.clone());

    crate::commands::contracts::terminate_contract_now_internal(&ctx.state_manager, &player_id)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Contract Terminated\n\n**{}** has been released.", player_name))
}

// ─── inbox_get_messages ─────────────────────────────────────────────────────
