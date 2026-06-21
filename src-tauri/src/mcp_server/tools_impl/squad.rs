//! MCP tool implementations: squad

use std::sync::Arc;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game, user_team, format_position, age_from_dob};
use crate::mcp_server::formatting::translate_error;

// ─── squad_get ──────────────────────────────────────────────────────────────

pub fn squad_get(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;
    let team_id = team.id.as_str();

    let mut squad: Vec<&domain::player::Player> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .collect();

    // Sort: starting XI first (by starting_xi_ids order), then rest by OVR descending
    squad.sort_by(|a, b| {
        let a_in_xi = team.starting_xi_ids.contains(&a.id);
        let b_in_xi = team.starting_xi_ids.contains(&b.id);
        match (a_in_xi, b_in_xi) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.ovr.cmp(&a.ovr),
        }
    });

    let mut rows = String::new();
    for p in &squad {
        let in_xi = if team.starting_xi_ids.contains(&p.id) { "★" } else { "" };
        let pos = format_position(&p.position);
        let inj = if p.injury.is_some() { "⚠" } else { "" };
        rows.push_str(&format!(
            "| {} | {}{}{} | {} | {} | {} | {} | {} | {} | {} |\n",
            p.id,
            p.match_name,
            in_xi,
            inj,
            pos,
            age_from_dob(&p.date_of_birth, &game),
            p.ovr,
            p.condition,
            p.morale,
            p.wage,
            p.contract_end.as_deref().unwrap_or("-"),
        ));
    }

    // Starting XI summary
    let xi_names: Vec<String> = team.starting_xi_ids.iter()
        .filter_map(|id| game.players.iter().find(|p| p.id == *id))
        .map(|p| format!("{} {}", format_position(&p.position), p.match_name))
        .collect();

    Ok(format!(
        "## {} — Squad Overview\n\n\
         | ID | Name | Pos | Age | OVR | Con | Mor | Wage | Contract |\n\
         |----|-------|-----|-----|-----|-----|-----|------|----------|\n\
         {}\
         \n**Starting XI**: {}\n\
         **Formation**: {} | **Play Style**: {:?}",
        team.name,
        rows,
        xi_names.join(", "),
        team.formation,
        team.play_style,
    ))
}

// ─── squad_set_starting_xi ─────────────────────────────────────────────────

pub fn squad_set_starting_xi(ctx: Arc<McpContext>, player_ids: Vec<String>) -> Result<String, String> {
    // Call the internal function from commands/squad.rs
    crate::commands::squad::set_starting_xi_internal(&ctx.state_manager, player_ids.clone())
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    // Format the starting XI
    let xi_names: Vec<String> = player_ids.iter()
        .map(|id| {
            game.players.iter()
                .find(|p| p.id == *id)
                .map(|p| format!("{} {}", format_position(&p.position), p.match_name))
                .unwrap_or_else(|| id.clone())
        })
        .collect();

    // Notify GUI
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Starting XI Updated\n\n{}\n**Formation**: {}", xi_names.join(", "), team.formation))
}

// ─── squad_set_formation ────────────────────────────────────────────────────

// ─── squad_set_formation ────────────────────────────────────────────────────

pub fn squad_set_formation(ctx: Arc<McpContext>, formation: String) -> Result<String, String> {
    crate::commands::squad::set_formation_internal(&ctx.state_manager, &formation)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    // Notify GUI
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Formation Changed\n\n**New Formation**: {}\n**Note**: Outfield player positions have been reassigned based on defending ability.", team.formation))
}

// ─── squad_set_play_style ───────────────────────────────────────────────────

// ─── squad_set_play_style ───────────────────────────────────────────────────

pub fn squad_set_play_style(ctx: Arc<McpContext>, play_style: String) -> Result<String, String> {
    crate::commands::squad::set_play_style_internal(&ctx.state_manager, &play_style)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Play Style Changed\n\n**New Style**: {:?}", team.play_style))
}

// ─── squad_set_match_roles ───────────────────────────────────────────────────

// ─── squad_set_match_roles ───────────────────────────────────────────────────

pub fn squad_set_match_roles(
    ctx: Arc<McpContext>,
    captain: Option<String>,
    vice_captain: Option<String>,
    penalty_taker: Option<String>,
    free_kick_taker: Option<String>,
    corner_taker: Option<String>,
) -> Result<String, String> {
    let match_roles = domain::team::MatchRoles {
        captain,
        vice_captain,
        penalty_taker,
        free_kick_taker,
        corner_taker,
    };

    crate::commands::squad::set_team_match_roles_internal(&ctx.state_manager, match_roles)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("## Match Roles Updated\n\nCaptain, set-piece takers set as specified.".to_string())
}

// ─── squad_auto_set_pieces ──────────────────────────────────────────────────

// ─── squad_auto_set_pieces ──────────────────────────────────────────────────

pub fn squad_auto_set_pieces(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    let result = crate::commands::squad::auto_select_set_pieces_internal(
        &ctx.state_manager,
        &team.starting_xi_ids,
    )
    .map_err(|e| translate_error(&e))?;

    // Apply the auto-selected roles
    let match_roles = domain::team::MatchRoles {
        captain: result.get("captain").and_then(|v| v.as_str()).map(|s| s.to_string()),
        vice_captain: team.match_roles.vice_captain.clone(),
        penalty_taker: result.get("penalty_taker").and_then(|v| v.as_str()).map(|s| s.to_string()),
        free_kick_taker: result.get("free_kick_taker").and_then(|v| v.as_str()).map(|s| s.to_string()),
        corner_taker: result.get("corner_taker").and_then(|v| v.as_str()).map(|s| s.to_string()),
    };

    crate::commands::squad::set_team_match_roles_internal(&ctx.state_manager, match_roles)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;

    // Format the result
    let mut names = Vec::new();
    if let Some(ref id) = game.teams.iter().find(|t| t.id == team.id).unwrap().match_roles.captain {
        if let Some(p) = game.players.iter().find(|p| p.id == *id) {
            names.push(format!("Captain: {}", p.match_name));
        }
    }
    if let Some(ref id) = game.teams.iter().find(|t| t.id == team.id).unwrap().match_roles.penalty_taker {
        if let Some(p) = game.players.iter().find(|p| p.id == *id) {
            names.push(format!("Penalties: {}", p.match_name));
        }
    }
    if let Some(ref id) = game.teams.iter().find(|t| t.id == team.id).unwrap().match_roles.free_kick_taker {
        if let Some(p) = game.players.iter().find(|p| p.id == *id) {
            names.push(format!("Free Kicks: {}", p.match_name));
        }
    }
    if let Some(ref id) = game.teams.iter().find(|t| t.id == team.id).unwrap().match_roles.corner_taker {
        if let Some(p) = game.players.iter().find(|p| p.id == *id) {
            names.push(format!("Corners: {}", p.match_name));
        }
    }

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Auto-Assigned Set Pieces\n\n{}", names.join("\n")))
}

// ─── squad_set_player_role ──────────────────────────────────────────────────

// ─── squad_set_player_role ──────────────────────────────────────────────────

pub fn squad_set_player_role(ctx: Arc<McpContext>, player_id: String, squad_role: String) -> Result<String, String> {
    crate::commands::squad::set_player_squad_role_internal(&ctx.state_manager, &player_id, &squad_role)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or(player_id);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Player Role Updated\n\n**{}**: {}", player_name, squad_role))
}

// ─── training_get ───────────────────────────────────────────────────────────
