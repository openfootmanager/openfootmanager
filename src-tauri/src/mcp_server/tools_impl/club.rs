//! MCP tool implementations: club

use mcp_results::club::{
    BoardSupport, FacilityUpgraded, MarketingCampaign, SponsorPitch, StaffHired, StaffList,
    StaffMember, StaffReleased,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game, serde_label};
use std::sync::Arc;

// ─── club_upgrade_facility ──────────────────────────────────────────────────

pub fn club_upgrade_facility(
    ctx: Arc<McpContext>,
    facility: String,
) -> Result<FacilityUpgraded, String> {
    crate::commands::club::upgrade_facility_internal(&ctx.state_manager, &facility)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(FacilityUpgraded { facility })
}

// ─── staff_get ──────────────────────────────────────────────────────────────

// ─── staff_get ──────────────────────────────────────────────────────────────

pub fn staff_get(ctx: Arc<McpContext>) -> Result<StaffList, String> {
    let game = require_game(&ctx.state_manager)?;

    let staff = game
        .staff
        .iter()
        .map(|s| StaffMember {
            id: s.id.clone(),
            name: format!("{} {}", s.first_name, s.last_name),
            role: serde_label(&s.role),
            team: s
                .team_id
                .as_deref()
                .and_then(|tid| game.teams.iter().find(|t| t.id == tid))
                .map(|t| t.name.clone()),
        })
        .collect();

    Ok(StaffList { staff })
}

// ─── staff_hire ──────────────────────────────────────────────────────────────

// ─── staff_hire ──────────────────────────────────────────────────────────────

pub fn staff_hire(ctx: Arc<McpContext>, staff_id: String) -> Result<StaffHired, String> {
    crate::commands::staff::hire_staff_internal(&ctx.state_manager, &staff_id)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(StaffHired { staff_id })
}

// ─── staff_release ───────────────────────────────────────────────────────────

// ─── staff_release ───────────────────────────────────────────────────────────

pub fn staff_release(ctx: Arc<McpContext>, staff_id: String) -> Result<StaffReleased, String> {
    crate::commands::staff::release_staff_internal(&ctx.state_manager, &staff_id)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(StaffReleased { staff_id })
}

// ─── season_check_complete ──────────────────────────────────────────────────

// ─── club_request_board_support ─────────────────────────────────────────────

pub fn club_request_board_support(ctx: Arc<McpContext>) -> Result<BoardSupport, String> {
    let response = crate::commands::finances::request_board_support_internal(&ctx.state_manager)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(BoardSupport {
        support_amount: response.result.support_amount,
        transfer_budget_reduction: response.result.transfer_budget_reduction,
        satisfaction_penalty: response.result.satisfaction_penalty,
    })
}

// ─── club_request_marketing ─────────────────────────────────────────────────

// ─── club_request_marketing ─────────────────────────────────────────────────

pub fn club_request_marketing(ctx: Arc<McpContext>) -> Result<MarketingCampaign, String> {
    let response =
        crate::commands::finances::request_marketing_campaign_internal(&ctx.state_manager)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(MarketingCampaign {
        gross_revenue: response.result.gross_revenue,
    })
}

// ─── club_request_sponsor_pitch ─────────────────────────────────────────────

// ─── club_request_sponsor_pitch ─────────────────────────────────────────────

pub fn club_request_sponsor_pitch(ctx: Arc<McpContext>) -> Result<SponsorPitch, String> {
    let response = crate::commands::finances::request_sponsor_pitch_internal(&ctx.state_manager)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(SponsorPitch {
        sponsor_name: response.result.sponsor_name,
        weekly_amount: response.result.weekly_amount,
        duration_weeks: response.result.duration_weeks,
    })
}

// ─── scout_send ─────────────────────────────────────────────────────────────
