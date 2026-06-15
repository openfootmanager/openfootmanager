//! MCP tool implementations: transfers

use std::sync::Arc;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game, format_position, age_from_dob};
use crate::mcp_server::formatting::translate_error;

// ─── transfer_toggle_listed ────────────────────────────────────────────────

pub fn transfer_toggle_listed(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    crate::commands::transfers::toggle_transfer_list_internal(&ctx.state_manager, &player_id)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let is_listed = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.transfer_listed)
        .unwrap_or(false);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let status = if is_listed { "Transfer Listed ✓" } else { "Not Listed" };
    Ok(format!("## Transfer Status Updated\n\n**{}**: {}", player_name, status))
}

// ─── transfer_toggle_loan ──────────────────────────────────────────────────

// ─── transfer_toggle_loan ──────────────────────────────────────────────────

pub fn transfer_toggle_loan(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    crate::commands::transfers::toggle_loan_list_internal(&ctx.state_manager, &player_id)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let is_loaned = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.loan_listed)
        .unwrap_or(false);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let status = if is_loaned { "Loan Listed ✓" } else { "Not Listed" };
    Ok(format!("## Loan Status Updated\n\n**{}**: {}", player_name, status))
}

// ─── transfer_make_bid ──────────────────────────────────────────────────────

// ─── transfer_make_bid ──────────────────────────────────────────────────────

pub fn transfer_make_bid(ctx: Arc<McpContext>, player_id: String, fee: u64) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    let response = crate::commands::transfers::make_transfer_bid_internal(
        &ctx.state_manager,
        &player_id,
        fee,
    )
    .map_err(|e| translate_error(&e))?;

    let mut output = format!("## Transfer Bid: {} — {} 💰\n\n", player_name, fee);

    output.push_str(&format!("**Decision**: {:?}\n", response.decision));
    if let Some(suggested) = response.suggested_fee {
        output.push_str(&format!("**Suggested Fee**: {}\n", suggested));
    }
    output.push_str(&format!("**Terminal**: {}\n", response.is_terminal));
    output.push_str(&format!("**Mood**: {:?}\n", response.feedback.mood));
    output.push_str(&format!("**Tension**: {}/100\n", response.feedback.tension));
    output.push_str(&format!("**Patience**: {}/100\n", response.feedback.patience));
    output.push_str(&format!("**Round**: {}\n", response.feedback.round));

    if response.is_terminal {
        output.push_str("\n✅ Negotiation complete.");
    } else {
        output.push_str("\n🔄 Negotiation continues — make another bid or walk away.");
    }

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(output)
}

// ─── transfer_preview_bid ──────────────────────────────────────────────────

// ─── transfer_preview_bid ──────────────────────────────────────────────────

pub fn transfer_preview_bid(ctx: Arc<McpContext>, player_id: String, fee: u64) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    let response = crate::commands::transfers::preview_transfer_bid_financial_impact_internal(
        &ctx.state_manager,
        &player_id,
        fee,
    )
    .map_err(|e| translate_error(&e))?;
    let p = &response.projection;

    Ok(format!(
        "## Transfer Bid Preview: {} — {} 💰\n\n| Field | Value |\n|-------|-------|\n| Transfer Budget Before | {} |\n| Transfer Budget After | {} |\n| Finance Before | {} |\n| Finance After | {} |\n| Annual Wage Bill Before | {} |\n| Annual Wage Bill After | {} |\n| Annual Wage Budget | {} |\n| Projected Wage Usage | {}% |\n| Exceeds Transfer Budget | {} |\n| Exceeds Finance | {} |\n\nThis is a preview — no bid was made.",
        player_name, fee,
        p.transfer_budget_before, p.transfer_budget_after,
        p.finance_before, p.finance_after,
        p.annual_wage_bill_before, p.annual_wage_bill_after,
        p.annual_wage_budget, p.projected_wage_budget_usage_pct,
        if p.exceeds_transfer_budget { "Yes" } else { "No" },
        if p.exceeds_finance { "Yes" } else { "No" },
    ))
}

// ─── transfer_respond_to_offer ──────────────────────────────────────────────

// ─── transfer_respond_to_offer ──────────────────────────────────────────────

pub fn transfer_respond_to_offer(ctx: Arc<McpContext>, player_id: String, offer_id: String, accept: bool) -> Result<String, String> {
    crate::commands::transfers::respond_to_offer_internal(
        &ctx.state_manager,
        &player_id,
        &offer_id,
        accept,
    )
    .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let action = if accept { "accepted" } else { "rejected" };
    Ok(format!("## Offer {}\n\nOffer {} for player {}.", action, offer_id, player_id))
}

// ─── transfer_counter_offer ─────────────────────────────────────────────────

// ─── transfer_counter_offer ─────────────────────────────────────────────────

pub fn transfer_counter_offer(ctx: Arc<McpContext>, player_id: String, offer_id: String, requested_fee: u64) -> Result<String, String> {
    let response = crate::commands::transfers::counter_offer_internal(
        &ctx.state_manager,
        &player_id,
        &offer_id,
        requested_fee,
    )
    .map_err(|e| translate_error(&e))?;

    let mut output = format!("## Counter Offer: {} 💰\n\n", requested_fee);
    output.push_str(&format!("**Decision**: {:?}\n", response.decision));
    if let Some(suggested) = response.suggested_fee {
        output.push_str(&format!("**Suggested Fee**: {}\n", suggested));
    }
    output.push_str(&format!("**Terminal**: {}\n", response.is_terminal));

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(output)
}

// ─── contract_propose_renewal ───────────────────────────────────────────────

// ─── transfer_market_browse ─────────────────────────────────────────────────

pub fn transfer_market_browse(ctx: Arc<McpContext>, position: Option<String>, max_price: Option<u64>, listed_only: Option<bool>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let team_id = game.manager.team_id.as_deref().ok_or("be.error.noTeamAssigned")?;

    let players: Vec<_> = game.players.iter()
        .filter(|p| {
            // Exclude own players
            p.team_id.as_deref() != Some(team_id)
        })
        .filter(|p| {
            // Filter by listed status
            if let Some(true) = listed_only {
                p.transfer_listed || p.loan_listed
            } else {
                true
            }
        })
        .filter(|p| {
            // Filter by position
            if let Some(ref pos) = position {
                format_position(&p.position).to_lowercase() == pos.to_lowercase()
                    || format!("{:?}", p.position).to_lowercase() == pos.to_lowercase()
            } else {
                true
            }
        })
        .filter(|p| {
            // Filter by max price (use estimated value/wage)
            if let Some(max) = max_price {
                (p.wage as u64 * 52) <= max // Rough annual cost estimate
            } else {
                true
            }
        })
        .collect();

    if players.is_empty() {
        return Ok("## Transfer Market\n\nNo players found matching criteria.".to_string());
    }

    let mut output = format!("## Transfer Market ({} players)\n\n| ID | Name | Pos | Age | OVR | Team | Listed | Wage |\n|----|------|-----|-----|-----|------|--------|------|\n", players.len());
    for p in players.iter().take(30) {
        let team_name = p.team_id.as_deref()
            .and_then(|tid| game.teams.iter().find(|t| t.id == tid))
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Free".to_string());
        let listed = if p.transfer_listed { "T" } else if p.loan_listed { "L" } else { "-" };
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            p.id,
            p.match_name,
            format_position(&p.position),
            age_from_dob(&p.date_of_birth, &game),
            p.ovr,
            team_name,
            listed,
            p.wage,
        ));
    }
    if players.len() > 30 {
        output.push_str(&format!("\n... and {} more. Use filters to narrow results.", players.len() - 30));
    }

    Ok(output)
}

// ─── transfer_free_agent_offer ───────────────────────────────────────────────

// ─── transfer_free_agent_offer ───────────────────────────────────────────────

pub fn transfer_free_agent_offer(ctx: Arc<McpContext>, player_id: String, weekly_wage: u32, contract_years: u32) -> Result<String, String> {
    let response = crate::commands::contracts::offer_free_agent_contract_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
        contract_years,
    )
    .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Free Agent Offer\n\n**Wage**: {}/wk × {}yr\n**Outcome**: {:?}", weekly_wage, contract_years, response.outcome))
}

// ─── transfer_free_agent_preview ────────────────────────────────────────────

// ─── transfer_free_agent_preview ────────────────────────────────────────────

pub fn transfer_free_agent_preview(ctx: Arc<McpContext>, player_id: String, weekly_wage: u32) -> Result<String, String> {
    let response = crate::commands::contracts::preview_free_agent_contract_impact_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
    )
    .map_err(|e| translate_error(&e))?;
    let p = &response.projection;

    Ok(format!(
        "## Free Agent Preview\n\n| Field | Value |\n|-------|-------|\n| Weekly Wage Offered | {}/wk |\n| Current Annual Wage Bill | {} |\n| Projected Annual Wage Bill | {} |\n| Annual Wage Budget | {} |\n| Annual Soft Cap | {} |\n| Current Weekly Spend | {} |\n| Projected Weekly Spend | {} |\n| Cash Runway (weeks) | {} → {} |\n| Currently Over Budget | {} |\n| Policy Allows | {} |\n\nThis is a preview — no offer was made.",
        weekly_wage,
        p.current_annual_wage_bill, p.projected_annual_wage_bill,
        p.annual_wage_budget, p.annual_soft_cap,
        p.current_weekly_wage_spend, p.projected_weekly_wage_spend,
        p.current_cash_runway_weeks.map(|w| w.to_string()).unwrap_or_else(|| "N/A".to_string()),
        p.projected_cash_runway_weeks.map(|w| w.to_string()).unwrap_or_else(|| "N/A".to_string()),
        if p.currently_over_budget { "Yes" } else { "No" },
        if p.policy_allows { "Yes" } else { "No" },
    ))
}

// ─── info_player_stats ──────────────────────────────────────────────────────
