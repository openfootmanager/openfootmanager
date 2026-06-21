//! MCP tool implementations: scouting

use std::sync::Arc;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::require_game;
use crate::mcp_server::formatting::translate_error;

// ─── scout_send ─────────────────────────────────────────────────────────────

pub fn scout_send(ctx: Arc<McpContext>, scout_id: String, player_id: String) -> Result<String, String> {
    let mut game = require_game(&ctx.state_manager)?;
    ofm_core::scouting::send_scout(&mut game, &scout_id, &player_id)
        .map_err(|e| translate_error(&e))?;
    ctx.state_manager.set_game(game);

    let scout_name = ctx.state_manager.get_game(|g| {
        g.staff.iter().find(|s| s.id == scout_id)
            .map(|s| format!("{} {}", s.first_name, s.last_name))
            .unwrap_or_default()
    }).unwrap_or_default();

    let player_name = ctx.state_manager.get_game(|g| {
        g.players.iter().find(|p| p.id == player_id)
            .map(|p| p.match_name.clone())
            .unwrap_or_default()
    }).unwrap_or_default();

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Scout Dispatched\n\n**{}** will report on **{}**.", scout_name, player_name))
}

// ─── scout_get_reports ─────────────────────────────────────────────────────

// ─── scout_get_reports ─────────────────────────────────────────────────────

pub fn scout_get_reports(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    let reports: Vec<_> = game.messages.iter()
        .filter(|m| matches!(m.category, domain::message::MessageCategory::ScoutReport))
        .filter_map(|m| {
            m.context.scout_report.as_ref().map(|r| {
                (m.id.clone(), m.read, r)
            })
        })
        .collect();

    if reports.is_empty() {
        return Ok("## Scout Reports\n\nNo scout reports available.".to_string());
    }

    let mut output = format!("## Scout Reports ({} reports)\n\n| ID | Player | Pos | Rating | Team | Read |\n|----|--------|-----|--------|------|------|\n", reports.len());
    for (id, read, r) in &reports {
        let read_marker = if *read { "✓" } else { "●" };
        let rating = r.avg_rating.map(|v| format!("{}/100", v)).unwrap_or_else(|| "?".to_string());
        let team = r.team_name.as_deref().unwrap_or("Free");
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            id, r.player_name, r.position, rating, team, read_marker,
        ));
    }

    // Also show active scouting assignments
    let active: Vec<_> = game.scouting_assignments.iter().collect();
    if !active.is_empty() {
        output.push_str(&format!("\n### Active Assignments ({} pending)\n\n| ID | Scout | Player | Days Left |\n|----|-------|--------|------------|\n", active.len()));
        for a in &active {
            let scout_name = game.staff.iter().find(|s| s.id == a.scout_id)
                .map(|s| format!("{} {}", s.first_name, s.last_name))
                .unwrap_or_default();
            let player_name = game.players.iter().find(|p| p.id == a.player_id)
                .map(|p| p.match_name.clone())
                .unwrap_or_default();
            output.push_str(&format!("| {} | {} | {} | {} |\n", a.id, scout_name, player_name, a.days_remaining));
        }
    }

    Ok(output)
}

// ─── scout_youth_start ──────────────────────────────────────────────────────

// ─── scout_youth_start ──────────────────────────────────────────────────────

pub fn scout_youth_start(ctx: Arc<McpContext>, scout_id: String, region: Option<String>, objective: Option<String>, target_position: Option<String>) -> Result<String, String> {
    let mut game = require_game(&ctx.state_manager)?;

    let region = parse_youth_region(region.as_deref())?;
    let objective = parse_youth_objective(objective.as_deref())?;
    let target_position = parse_youth_target_position(target_position.as_deref())?;

    ofm_core::scouting::start_youth_scouting(
        &mut game,
        &scout_id,
        region,
        objective,
        target_position,
    )
    .map_err(|e| translate_error(&e))?;

    ctx.state_manager.set_game(game);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("## Youth Scouting Started\n\nAssignment created. Check `scout_get_reports` for results over time.".to_string())
}

fn parse_youth_region(region: Option<&str>) -> Result<ofm_core::game::YouthScoutingRegion, String> {
    match region.unwrap_or("Domestic") {
        "Domestic" => Ok(ofm_core::game::YouthScoutingRegion::Domestic),
        "International" => Ok(ofm_core::game::YouthScoutingRegion::International),
        other => Err(format!("Unknown youth scouting region: {}", other)),
    }
}

fn parse_youth_objective(objective: Option<&str>) -> Result<ofm_core::game::YouthScoutingObjective, String> {
    match objective.unwrap_or("Balanced") {
        "Balanced" => Ok(ofm_core::game::YouthScoutingObjective::Balanced),
        "HighPotential" | "Potential" => Ok(ofm_core::game::YouthScoutingObjective::HighPotential),
        "ReadySoon" | "Immediate" => Ok(ofm_core::game::YouthScoutingObjective::ReadySoon),
        other => Err(format!("Unknown youth scouting objective: {}", other)),
    }
}

fn parse_youth_target_position(pos: Option<&str>) -> Result<Option<domain::player::Position>, String> {
    let Some(pos_str) = pos else { return Ok(None) };
    match pos_str.to_uppercase().as_str() {
        // Goalkeeper
        "GK" | "GOALKEEPER" => Ok(Some(domain::player::Position::Goalkeeper)),
        // Defender — broad + specific codes
        "DF" | "DEFENDER" | "CB" | "LCB" | "RCB" | "LB" | "RB" | "LWB" | "RWB" => Ok(Some(domain::player::Position::Defender)),
        // Midfielder — broad + specific codes
        "MF" | "MIDFIELDER" | "DM" | "CDM" | "CM" | "LCM" | "RCM" | "AM" | "CAM" | "LM" | "RM" => Ok(Some(domain::player::Position::Midfielder)),
        // Forward — broad + specific codes
        "FW" | "FORWARD" | "ST" | "CF" | "LS" | "RS" | "LW" | "RW" | "LF" | "RF" => Ok(Some(domain::player::Position::Forward)),
        _ => Err(format!("Unknown position: {}", pos_str)),
    }
}

// ─── scout_youth_cancel ─────────────────────────────────────────────────────

// ─── scout_youth_cancel ─────────────────────────────────────────────────────

pub fn scout_youth_cancel(ctx: Arc<McpContext>, assignment_id: String) -> Result<String, String> {
    let mut game = require_game(&ctx.state_manager)?;
    ofm_core::scouting::cancel_youth_scouting(&mut game, &assignment_id)
        .map_err(|e| translate_error(&e))?;
    ctx.state_manager.set_game(game);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("## Youth Scouting Cancelled\n\nAssignment has been cancelled.".to_string())
}

// ─── scout_youth_reassign ───────────────────────────────────────────────────

// ─── scout_youth_reassign ───────────────────────────────────────────────────

pub fn scout_youth_reassign(ctx: Arc<McpContext>, assignment_id: String, scout_id: String) -> Result<String, String> {
    let mut game = require_game(&ctx.state_manager)?;
    ofm_core::scouting::reassign_youth_scouting(&mut game, &assignment_id, &scout_id)
        .map_err(|e| translate_error(&e))?;
    ctx.state_manager.set_game(game);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("## Youth Scouting Reassigned\n\nScout has been changed.".to_string())
}

