//! MCP tool implementations: transfers

use mcp_results::transfers::{
    BidMade, BidPreview, CounterOffered, FreeAgentOffered, FreeAgentPreview, LoanListingToggled,
    MarketPlayer, OfferAnswered, TransferListingToggled, TransferMarket,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{
    age_from_dob, format_position, require_game, serde_label,
};
use std::sync::Arc;

// ─── transfer_toggle_listed ────────────────────────────────────────────────

pub fn transfer_toggle_listed(
    ctx: Arc<McpContext>,
    player_id: String,
) -> Result<TransferListingToggled, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    crate::commands::transfers::toggle_transfer_list_internal(&ctx.state_manager, &player_id)?;

    let game = require_game(&ctx.state_manager)?;
    let is_listed = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.transfer_listed)
        .unwrap_or(false);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(TransferListingToggled {
        player_id,
        player_name,
        listed: is_listed,
    })
}

// ─── transfer_toggle_loan ──────────────────────────────────────────────────

// ─── transfer_toggle_loan ──────────────────────────────────────────────────

pub fn transfer_toggle_loan(
    ctx: Arc<McpContext>,
    player_id: String,
) -> Result<LoanListingToggled, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    crate::commands::transfers::toggle_loan_list_internal(&ctx.state_manager, &player_id)?;

    let game = require_game(&ctx.state_manager)?;
    let is_loaned = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.loan_listed)
        .unwrap_or(false);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(LoanListingToggled {
        player_id,
        player_name,
        listed: is_loaned,
    })
}

// ─── transfer_make_bid ──────────────────────────────────────────────────────

// ─── transfer_make_bid ──────────────────────────────────────────────────────

pub fn transfer_make_bid(
    ctx: Arc<McpContext>,
    player_id: String,
    fee: u64,
) -> Result<BidMade, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    let response = crate::commands::transfers::make_transfer_bid_internal(
        &ctx.state_manager,
        &player_id,
        fee,
    )?;

    let bid = BidMade {
        player_name,
        fee,
        decision: serde_label(&response.decision),
        suggested_fee: response.suggested_fee,
        is_terminal: response.is_terminal,
        mood: serde_label(&response.feedback.mood),
        tension: response.feedback.tension,
        patience: response.feedback.patience,
        round: response.feedback.round,
    };

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(bid)
}

// ─── transfer_preview_bid ──────────────────────────────────────────────────

// ─── transfer_preview_bid ──────────────────────────────────────────────────

pub fn transfer_preview_bid(
    ctx: Arc<McpContext>,
    player_id: String,
    fee: u64,
) -> Result<BidPreview, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    let response = crate::commands::transfers::preview_transfer_bid_financial_impact_internal(
        &ctx.state_manager,
        &player_id,
        fee,
    )?;
    let p = &response.projection;

    Ok(BidPreview {
        player_name,
        fee,
        transfer_budget_before: p.transfer_budget_before,
        transfer_budget_after: p.transfer_budget_after,
        finance_before: p.finance_before,
        finance_after: p.finance_after,
        current_weekly_wage_spend: p.current_weekly_wage_spend,
        projected_weekly_wage_spend: p.projected_weekly_wage_spend,
        weekly_wage_budget: p.weekly_wage_budget,
        projected_wage_budget_usage_pct: p.projected_wage_budget_usage_pct,
        exceeds_transfer_budget: p.exceeds_transfer_budget,
        exceeds_finance: p.exceeds_finance,
    })
}

// ─── transfer_respond_to_offer ──────────────────────────────────────────────

// ─── transfer_respond_to_offer ──────────────────────────────────────────────

pub fn transfer_respond_to_offer(
    ctx: Arc<McpContext>,
    player_id: String,
    offer_id: String,
    accept: bool,
) -> Result<OfferAnswered, String> {
    crate::commands::transfers::respond_to_offer_internal(
        &ctx.state_manager,
        &player_id,
        &offer_id,
        accept,
    )?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(OfferAnswered {
        player_id,
        offer_id,
        accepted: accept,
    })
}

// ─── transfer_counter_offer ─────────────────────────────────────────────────

// ─── transfer_counter_offer ─────────────────────────────────────────────────

pub fn transfer_counter_offer(
    ctx: Arc<McpContext>,
    player_id: String,
    offer_id: String,
    requested_fee: u64,
) -> Result<CounterOffered, String> {
    let response = crate::commands::transfers::counter_offer_internal(
        &ctx.state_manager,
        &player_id,
        &offer_id,
        requested_fee,
    )?;

    let countered = CounterOffered {
        requested_fee,
        decision: serde_label(&response.decision),
        suggested_fee: response.suggested_fee,
        is_terminal: response.is_terminal,
    };

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(countered)
}

// ─── contract_propose_renewal ───────────────────────────────────────────────

// ─── transfer_market_browse ─────────────────────────────────────────────────

pub fn transfer_market_browse(
    ctx: Arc<McpContext>,
    position: Option<String>,
    max_price: Option<u64>,
    listed_only: Option<bool>,
) -> Result<TransferMarket, String> {
    let game = require_game(&ctx.state_manager)?;
    let team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or("be.error.noTeamAssigned")?;

    let players: Vec<_> = game
        .players
        .iter()
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
                (p.wage() as u64 * 52) <= max // Rough annual cost estimate
            } else {
                true
            }
        })
        .collect();

    Ok(TransferMarket {
        players: players
            .into_iter()
            .map(|p| MarketPlayer {
                id: p.id.clone(),
                name: p.match_name.clone(),
                position: format_position(&p.position).to_string(),
                age: age_from_dob(&p.date_of_birth, &game),
                ovr: p.ovr,
                team: p
                    .team_id
                    .as_deref()
                    .and_then(|tid| game.teams.iter().find(|t| t.id == tid))
                    .map(|t| t.name.clone()),
                transfer_listed: p.transfer_listed,
                loan_listed: p.loan_listed,
                wage: p.wage(),
            })
            .collect(),
    })
}

// ─── transfer_free_agent_offer ───────────────────────────────────────────────

// ─── transfer_free_agent_offer ───────────────────────────────────────────────

pub fn transfer_free_agent_offer(
    ctx: Arc<McpContext>,
    player_id: String,
    weekly_wage: u32,
    contract_years: u32,
) -> Result<FreeAgentOffered, String> {
    let response = crate::commands::contracts::offer_free_agent_contract_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
        contract_years,
    )?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(FreeAgentOffered {
        weekly_wage,
        contract_years,
        outcome: serde_label(&response.outcome),
    })
}

// ─── transfer_free_agent_preview ────────────────────────────────────────────

// ─── transfer_free_agent_preview ────────────────────────────────────────────

pub fn transfer_free_agent_preview(
    ctx: Arc<McpContext>,
    player_id: String,
    weekly_wage: u32,
) -> Result<FreeAgentPreview, String> {
    let response = crate::commands::contracts::preview_free_agent_contract_impact_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
    )?;
    let p = &response.projection;

    Ok(FreeAgentPreview {
        weekly_wage,
        current_weekly_wage_spend: p.current_weekly_wage_spend,
        projected_weekly_wage_spend: p.projected_weekly_wage_spend,
        annual_wage_budget: p.annual_wage_budget,
        annual_soft_cap: p.annual_soft_cap,
        current_cash_runway_weeks: p.current_cash_runway_weeks,
        projected_cash_runway_weeks: p.projected_cash_runway_weeks,
        currently_over_budget: p.currently_over_budget,
        policy_allows: p.policy_allows,
    })
}

// ─── info_player_stats ──────────────────────────────────────────────────────
