//! MCP tool implementations: scouting

use mcp_results::scouting::{
    ScoutDispatched, ScoutReport, ScoutReports, ScoutingAssignment, YouthScoutingCancelled,
    YouthScoutingReassigned, YouthScoutingStarted,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::require_game;
use std::sync::Arc;

// ─── scout_send ─────────────────────────────────────────────────────────────

pub fn scout_send(
    ctx: Arc<McpContext>,
    scout_id: String,
    player_id: String,
) -> Result<ScoutDispatched, String> {
    // `send_scout` validates fully before its single push, so an error path
    // leaves the game untouched even though `update_game` cannot roll back.
    ctx.state_manager
        .update_game(|game| ofm_core::scouting::send_scout(game, &scout_id, &player_id))
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())??;

    let scout_name = ctx
        .state_manager
        .get_game(|g| {
            g.staff
                .iter()
                .find(|s| s.id == scout_id)
                .map(|s| format!("{} {}", s.first_name, s.last_name))
                .unwrap_or_default()
        })
        .unwrap_or_default();

    let player_name = ctx
        .state_manager
        .get_game(|g| {
            g.players
                .iter()
                .find(|p| p.id == player_id)
                .map(|p| p.match_name.clone())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(ScoutDispatched {
        scout_id,
        scout_name,
        player_id,
        player_name,
    })
}

// ─── scout_get_reports ─────────────────────────────────────────────────────

// ─── scout_get_reports ─────────────────────────────────────────────────────

pub fn scout_get_reports(ctx: Arc<McpContext>) -> Result<ScoutReports, String> {
    let game = require_game(&ctx.state_manager)?;

    let reports = game
        .messages
        .iter()
        .filter(|m| matches!(m.category, domain::message::MessageCategory::ScoutReport))
        .filter_map(|m| {
            m.context.scout_report.as_ref().map(|r| ScoutReport {
                message_id: m.id.clone(),
                player_name: r.player_name.clone(),
                position: r.position.clone(),
                avg_rating: r.avg_rating,
                team: r.team_name.clone(),
                read: m.read,
            })
        })
        .collect();

    let active_assignments = game
        .scouting_assignments
        .iter()
        .map(|a| ScoutingAssignment {
            id: a.id.clone(),
            scout_name: game
                .staff
                .iter()
                .find(|s| s.id == a.scout_id)
                .map(|s| format!("{} {}", s.first_name, s.last_name))
                .unwrap_or_default(),
            player_name: game
                .players
                .iter()
                .find(|p| p.id == a.player_id)
                .map(|p| p.match_name.clone())
                .unwrap_or_default(),
            days_remaining: a.days_remaining,
        })
        .collect();

    Ok(ScoutReports {
        reports,
        active_assignments,
    })
}

// ─── scout_youth_start ──────────────────────────────────────────────────────

// ─── scout_youth_start ──────────────────────────────────────────────────────

pub fn scout_youth_start(
    ctx: Arc<McpContext>,
    scout_id: String,
    region: Option<String>,
    objective: Option<String>,
    target_position: Option<String>,
) -> Result<YouthScoutingStarted, String> {
    let region = parse_youth_region(region.as_deref())?;
    let objective = parse_youth_objective(objective.as_deref())?;
    let target_position = parse_youth_target_position(target_position.as_deref())?;

    // Validates fully before its single push, so the error path leaves the game
    // untouched even though `update_game` cannot roll back.
    ctx.state_manager
        .update_game(|game| {
            ofm_core::scouting::start_youth_scouting(
                game,
                &scout_id,
                region,
                objective,
                target_position,
            )
        })
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())??;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(YouthScoutingStarted { scout_id })
}

fn parse_youth_region(region: Option<&str>) -> Result<ofm_core::game::YouthScoutingRegion, String> {
    match region.unwrap_or("Domestic") {
        "Domestic" => Ok(ofm_core::game::YouthScoutingRegion::Domestic),
        "International" => Ok(ofm_core::game::YouthScoutingRegion::International),
        other => Err(format!("Unknown youth scouting region: {}", other)),
    }
}

fn parse_youth_objective(
    objective: Option<&str>,
) -> Result<ofm_core::game::YouthScoutingObjective, String> {
    match objective.unwrap_or("Balanced") {
        "Balanced" => Ok(ofm_core::game::YouthScoutingObjective::Balanced),
        "HighPotential" | "Potential" => Ok(ofm_core::game::YouthScoutingObjective::HighPotential),
        "ReadySoon" | "Immediate" => Ok(ofm_core::game::YouthScoutingObjective::ReadySoon),
        other => Err(format!("Unknown youth scouting objective: {}", other)),
    }
}

fn parse_youth_target_position(
    pos: Option<&str>,
) -> Result<Option<domain::player::Position>, String> {
    let Some(pos_str) = pos else { return Ok(None) };
    match pos_str.to_uppercase().as_str() {
        // Goalkeeper
        "GK" | "GOALKEEPER" => Ok(Some(domain::player::Position::Goalkeeper)),
        // Defender — broad + specific codes
        "DF" | "DEFENDER" | "CB" | "LCB" | "RCB" | "LB" | "RB" | "LWB" | "RWB" => {
            Ok(Some(domain::player::Position::Defender))
        }
        // Midfielder — broad + specific codes
        "MF" | "MIDFIELDER" | "DM" | "CDM" | "CM" | "LCM" | "RCM" | "AM" | "CAM" | "LM" | "RM" => {
            Ok(Some(domain::player::Position::Midfielder))
        }
        // Forward — broad + specific codes
        "FW" | "FORWARD" | "ST" | "CF" | "LS" | "RS" | "LW" | "RW" | "LF" | "RF" => {
            Ok(Some(domain::player::Position::Forward))
        }
        _ => Err(format!("Unknown position: {}", pos_str)),
    }
}

// ─── scout_youth_cancel ─────────────────────────────────────────────────────

// ─── scout_youth_cancel ─────────────────────────────────────────────────────

pub fn scout_youth_cancel(
    ctx: Arc<McpContext>,
    assignment_id: String,
) -> Result<YouthScoutingCancelled, String> {
    // The odd one out: `cancel_youth_scouting` retains first and reports the
    // failure afterwards. That is still safe to run in place — when it errors,
    // the retain matched nothing and removed nothing.
    ctx.state_manager
        .update_game(|game| ofm_core::scouting::cancel_youth_scouting(game, &assignment_id))
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())??;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(YouthScoutingCancelled { assignment_id })
}

// ─── scout_youth_reassign ───────────────────────────────────────────────────

// ─── scout_youth_reassign ───────────────────────────────────────────────────

pub fn scout_youth_reassign(
    ctx: Arc<McpContext>,
    assignment_id: String,
    scout_id: String,
) -> Result<YouthScoutingReassigned, String> {
    // Validates fully before the one field it assigns.
    ctx.state_manager
        .update_game(|game| {
            ofm_core::scouting::reassign_youth_scouting(game, &assignment_id, &scout_id)
        })
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())??;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(YouthScoutingReassigned {
        assignment_id,
        scout_id,
    })
}
