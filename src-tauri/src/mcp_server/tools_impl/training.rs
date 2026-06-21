//! MCP tool implementations: training

use std::sync::Arc;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game, user_team};
use crate::mcp_server::formatting::translate_error;

// ─── training_get ───────────────────────────────────────────────────────────

pub fn training_get(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    let team_id = team.id.as_str();

    // Count players by condition level
    let players: Vec<_> = game.players.iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .collect();

    let avg_condition = if players.is_empty() { 0u32 } else {
        players.iter().map(|p| p.condition as u32).sum::<u32>() / players.len() as u32
    };
    let avg_fitness = if players.is_empty() { 0u32 } else {
        players.iter().map(|p| p.fitness as u32).sum::<u32>() / players.len() as u32
    };
    let injured_count = players.iter().filter(|p| p.injury.is_some()).count();

    Ok(format!(
        "## Training Settings — {}\n\n\
         **Focus**: {:?}\n\
         **Intensity**: {:?}\n\
         **Schedule**: {:?}\n\
         **Training Groups**: {}\n\n\
         ### Squad Fitness Overview\n\
         | Metric | Value |\n|--------|-------|\n\
         | Avg Condition | {}% |\n\
         | Avg Fitness | {}% |\n\
         | Injured Players | {} |",
        team.name,
        team.training_focus,
        team.training_intensity,
        team.training_schedule,
        team.training_groups.len(),
        avg_condition,
        avg_fitness,
        injured_count,
    ))
}

// ─── training_set_focus_intensity ──────────────────────────────────────────

// ─── training_set_focus_intensity ──────────────────────────────────────────

pub fn training_set_focus_intensity(ctx: Arc<McpContext>, focus: String, intensity: String) -> Result<String, String> {
    crate::commands::squad::set_training_internal(&ctx.state_manager, &focus, &intensity)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Training Updated\n\n**Focus**: {}\n**Intensity**: {}", focus, intensity))
}

// ─── training_set_schedule ─────────────────────────────────────────────────

// ─── training_set_schedule ─────────────────────────────────────────────────

pub fn training_set_schedule(ctx: Arc<McpContext>, schedule: String) -> Result<String, String> {
    crate::commands::squad::set_training_schedule_internal(&ctx.state_manager, &schedule)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Training Schedule Updated\n\n**Schedule**: {}", schedule))
}

// ─── training_set_groups ────────────────────────────────────────────────────

// ─── training_set_groups ────────────────────────────────────────────────────

pub fn training_set_groups(ctx: Arc<McpContext>, groups_json: String) -> Result<String, String> {
    let groups: Vec<domain::team::TrainingGroup> = serde_json::from_str(&groups_json)
        .map_err(|e| format!("Invalid training groups JSON: {}", e))?;

    crate::commands::squad::set_training_groups_internal(&ctx.state_manager, groups)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("## Training Groups Updated".to_string())
}

// ─── training_set_player_focus ──────────────────────────────────────────────

// ─── training_set_player_focus ──────────────────────────────────────────────

pub fn training_set_player_focus(ctx: Arc<McpContext>, player_id: String, focus: Option<String>) -> Result<String, String> {
    crate::commands::squad::set_player_training_focus_internal(&ctx.state_manager, &player_id, focus.as_deref())
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

    Ok(format!("## Player Training Focus Updated\n\n**{}**: {}", player_name, focus.as_deref().unwrap_or("cleared (team default)")))
}

// ─── transfer_toggle_listed ────────────────────────────────────────────────
