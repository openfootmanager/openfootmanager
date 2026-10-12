//! MCP tool implementations: training

use mcp_results::training::{
    PlayerTrainingFocusUpdated, TrainingGroupsUpdated, TrainingScheduleUpdated, TrainingSettings,
    TrainingUpdated,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game, serde_label, user_team};
use std::sync::Arc;

// ─── training_get ───────────────────────────────────────────────────────────

pub fn training_get(ctx: Arc<McpContext>) -> Result<TrainingSettings, String> {
    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    let team_id = team.id.as_str();

    // Count players by condition level
    let players: Vec<_> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .collect();

    let avg_condition = if players.is_empty() {
        0u32
    } else {
        players.iter().map(|p| p.condition as u32).sum::<u32>() / players.len() as u32
    };
    let avg_fitness = if players.is_empty() {
        0u32
    } else {
        players.iter().map(|p| p.fitness as u32).sum::<u32>() / players.len() as u32
    };
    let injured_count = players.iter().filter(|p| p.injury.is_some()).count();

    Ok(TrainingSettings {
        team_name: team.name.clone(),
        focus: serde_label(&team.training_focus),
        intensity: serde_label(&team.training_intensity),
        schedule: serde_label(&team.training_schedule),
        group_count: team.training_groups.len(),
        avg_condition,
        avg_fitness,
        injured_players: injured_count,
    })
}

// ─── training_set_focus_intensity ──────────────────────────────────────────

// ─── training_set_focus_intensity ──────────────────────────────────────────

pub fn training_set_focus_intensity(
    ctx: Arc<McpContext>,
    focus: String,
    intensity: String,
) -> Result<TrainingUpdated, String> {
    crate::commands::squad::set_training_internal(&ctx.state_manager, &focus, &intensity)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(TrainingUpdated { focus, intensity })
}

// ─── training_set_schedule ─────────────────────────────────────────────────

// ─── training_set_schedule ─────────────────────────────────────────────────

pub fn training_set_schedule(
    ctx: Arc<McpContext>,
    schedule: String,
) -> Result<TrainingScheduleUpdated, String> {
    crate::commands::squad::set_training_schedule_internal(&ctx.state_manager, &schedule)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(TrainingScheduleUpdated { schedule })
}

// ─── training_set_groups ────────────────────────────────────────────────────

// ─── training_set_groups ────────────────────────────────────────────────────

pub fn training_set_groups(
    ctx: Arc<McpContext>,
    groups_json: String,
) -> Result<TrainingGroupsUpdated, String> {
    let groups: Vec<domain::team::TrainingGroup> = serde_json::from_str(&groups_json)
        .map_err(|e| format!("Invalid training groups JSON: {}", e))?;

    let group_count = groups.len();
    crate::commands::squad::set_training_groups_internal(&ctx.state_manager, groups)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(TrainingGroupsUpdated { group_count })
}

// ─── training_set_player_focus ──────────────────────────────────────────────

// ─── training_set_player_focus ──────────────────────────────────────────────

pub fn training_set_player_focus(
    ctx: Arc<McpContext>,
    player_id: String,
    focus: Option<String>,
) -> Result<PlayerTrainingFocusUpdated, String> {
    crate::commands::squad::set_player_training_focus_internal(
        &ctx.state_manager,
        &player_id,
        focus.as_deref(),
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

    Ok(PlayerTrainingFocusUpdated {
        player_id,
        player_name,
        focus,
    })
}

// ─── transfer_toggle_listed ────────────────────────────────────────────────
