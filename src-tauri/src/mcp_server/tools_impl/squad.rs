//! MCP tool implementations: squad

use mcp_results::squad::{
    FormationChanged, MatchRolesUpdated, PlayStyleChanged, PlayerRoleUpdated, RoleHolder,
    SetPiecesAssigned, SquadOverview, SquadPlayer, StartingXiUpdated,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{
    age_from_dob, format_position, require_game, serde_label, user_team,
};
use std::sync::Arc;

// ─── squad_get ──────────────────────────────────────────────────────────────

pub fn squad_get(ctx: Arc<McpContext>) -> Result<SquadOverview, String> {
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

    let players = squad
        .iter()
        .map(|p| SquadPlayer {
            id: p.id.clone(),
            name: p.match_name.clone(),
            in_starting_xi: team.starting_xi_ids.contains(&p.id),
            injured: p.injury.is_some(),
            position: format_position(&p.position).to_string(),
            age: age_from_dob(&p.date_of_birth, &game),
            ovr: p.ovr,
            condition: p.condition,
            morale: p.morale,
            wage: p.wage(),
            contract_end: p.contract_end().map(str::to_string),
        })
        .collect();

    let starting_xi = team
        .starting_xi_ids
        .iter()
        .filter_map(|id| game.players.iter().find(|p| p.id == *id))
        .map(|p| format!("{} {}", format_position(&p.position), p.match_name))
        .collect();

    Ok(SquadOverview {
        team_name: team.name.clone(),
        players,
        starting_xi,
        formation: team.formation.clone(),
        play_style: serde_label(&team.play_style),
    })
}

// ─── squad_set_starting_xi ─────────────────────────────────────────────────

pub fn squad_set_starting_xi(
    ctx: Arc<McpContext>,
    player_ids: Vec<String>,
) -> Result<StartingXiUpdated, String> {
    // Call the internal function from commands/squad.rs
    crate::commands::squad::set_starting_xi_internal(&ctx.state_manager, player_ids.clone())?;

    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    // Format the starting XI
    let xi_names: Vec<String> = player_ids
        .iter()
        .map(|id| {
            game.players
                .iter()
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

    Ok(StartingXiUpdated {
        starting_xi: xi_names,
        formation: team.formation.clone(),
    })
}

// ─── squad_set_formation ────────────────────────────────────────────────────

// ─── squad_set_formation ────────────────────────────────────────────────────

pub fn squad_set_formation(
    ctx: Arc<McpContext>,
    formation: String,
) -> Result<FormationChanged, String> {
    crate::commands::squad::set_formation_internal(&ctx.state_manager, &formation)?;

    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    // Notify GUI
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(FormationChanged {
        formation: team.formation.clone(),
    })
}

// ─── squad_set_play_style ───────────────────────────────────────────────────

// ─── squad_set_play_style ───────────────────────────────────────────────────

pub fn squad_set_play_style(
    ctx: Arc<McpContext>,
    play_style: String,
) -> Result<PlayStyleChanged, String> {
    crate::commands::squad::set_play_style_internal(&ctx.state_manager, &play_style)?;

    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(PlayStyleChanged {
        play_style: serde_label(&team.play_style),
    })
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
) -> Result<MatchRolesUpdated, String> {
    let match_roles = domain::team::MatchRoles {
        captain: captain.clone(),
        vice_captain: vice_captain.clone(),
        penalty_taker: penalty_taker.clone(),
        free_kick_taker: free_kick_taker.clone(),
        corner_taker: corner_taker.clone(),
    };

    crate::commands::squad::set_team_match_roles_internal(&ctx.state_manager, match_roles)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(MatchRolesUpdated {
        captain,
        vice_captain,
        penalty_taker,
        free_kick_taker,
        corner_taker,
    })
}

// ─── squad_auto_set_pieces ──────────────────────────────────────────────────

// ─── squad_auto_set_pieces ──────────────────────────────────────────────────

pub fn squad_auto_set_pieces(ctx: Arc<McpContext>) -> Result<SetPiecesAssigned, String> {
    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    let result = crate::commands::squad::auto_select_set_pieces_internal(
        &ctx.state_manager,
        &team.starting_xi_ids,
    )?;

    // Apply the auto-selected roles
    let match_roles = domain::team::MatchRoles {
        captain: result
            .get("captain")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        vice_captain: team.match_roles.vice_captain.clone(),
        penalty_taker: result
            .get("penalty_taker")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        free_kick_taker: result
            .get("free_kick_taker")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        corner_taker: result
            .get("corner_taker")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    };

    crate::commands::squad::set_team_match_roles_internal(&ctx.state_manager, match_roles)?;

    let game = require_game(&ctx.state_manager)?;

    let roles = &user_team(&game)?.match_roles;
    let holder = |id: &Option<String>| {
        let id = id.as_ref()?;
        let player = game.players.iter().find(|p| p.id == *id)?;
        Some(RoleHolder {
            player_id: id.clone(),
            player_name: player.match_name.clone(),
        })
    };
    let assigned = SetPiecesAssigned {
        captain: holder(&roles.captain),
        penalty_taker: holder(&roles.penalty_taker),
        free_kick_taker: holder(&roles.free_kick_taker),
        corner_taker: holder(&roles.corner_taker),
    };

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(assigned)
}

// ─── squad_set_player_role ──────────────────────────────────────────────────

// ─── squad_set_player_role ──────────────────────────────────────────────────

pub fn squad_set_player_role(
    ctx: Arc<McpContext>,
    player_id: String,
    squad_role: String,
) -> Result<PlayerRoleUpdated, String> {
    crate::commands::squad::set_player_squad_role_internal(
        &ctx.state_manager,
        &player_id,
        &squad_role,
    )?;

    let game = require_game(&ctx.state_manager)?;
    let player_name = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_else(|| player_id.clone());

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(PlayerRoleUpdated {
        player_id,
        player_name,
        squad_role,
    })
}

// ─── training_get ───────────────────────────────────────────────────────────
