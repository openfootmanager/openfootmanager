//! MCP tool implementations: game

use std::sync::Arc;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game};
use crate::mcp_server::formatting::translate_error;
use tauri::Manager as TauriManager;

// ─── game_list_saves ────────────────────────────────────────────────────────

pub fn game_list_saves(ctx: Arc<McpContext>) -> Result<String, String> {
    let mut sm = ctx.save_manager_state.0.lock().map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
    let saves = sm.load_saves()?;

    if saves.is_empty() {
        return Ok("## Saves\n\nNo saves found.".to_string());
    }

    let mut lines = vec!["| ID | Manager | Last Played |".to_string(), "|---|---|---|".to_string()];
    for save in &saves {
        lines.push(format!("| {} | {} | {} |", save.id, save.manager_name, save.last_played_at));
    }

    Ok(format!("## Saves\n\n{}", lines.join("\n")))
}

// ─── game_delete_save ───────────────────────────────────────────────────────

// ─── game_save ──────────────────────────────────────────────────────────────

pub fn game_save(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let save_id = ctx.state_manager
        .get_save_id()
        .ok_or("be.error.noActiveSaveSession")?;

    let stats_state = ctx.state_manager
        .get_stats_state(|s| s.clone())
        .unwrap_or_default();

    {
        let mut sm = ctx.save_manager_state.0.lock().map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
        sm.save_game_with_stats(&game, &stats_state, &save_id)?;
    }

    Ok(format!("## Game Saved\n\n**Save ID**: {}\n**Date**: {}", save_id, game.clock.current_date.format("%d %B %Y")))
}

// ─── squad_set_starting_xi ─────────────────────────────────────────────────

// ─── game_new ───────────────────────────────────────────────────────────────

pub fn game_new(ctx: Arc<McpContext>, first_name: String, last_name: String, nationality: String, world_source: Option<String>, team_id: Option<String>) -> Result<String, String> {
    // Validate inputs
    if first_name.trim().is_empty() || last_name.trim().is_empty() {
        return Err("be.error.createManager.nameRequired".to_string());
    }
    if nationality.trim().is_empty() {
        return Err("be.error.createManager.nationalityRequired".to_string());
    }

    // Determine world path
    let world_path = world_source.unwrap_or_default();

    // Use the MCP bootstrap path to create the game
    let result = crate::commands::game::bootstrap_game_for_mcp(
        &ctx.state_manager,
        &ctx.save_manager_state,
        &world_path,
        team_id.as_deref(),
        &first_name,
        &last_name,
        &nationality,
    );

    match result {
        Ok(save_id) => {
            {
                use tauri::Emitter;
                let _ = ctx.app_handle.emit("game-state-changed", ());
            }
            Ok(format!(
                "## Game Created\n\nManager: {} {}\nNationality: {}\nSave ID: {}\n\nUse `info_game_summary` to see your current state.",
                first_name, last_name, nationality, save_id
            ))
        }
        Err(e) => Err(e),
    }
}

// ─── game_select_team ───────────────────────────────────────────────────────

// ─── game_select_team ───────────────────────────────────────────────────────

pub fn game_select_team(ctx: Arc<McpContext>, team_id: String) -> Result<String, String> {
    let mut game = require_game(&ctx.state_manager)?;

    if game.manager.team_id.is_some() {
        return Err("Already have a team assigned. Use `jobs_apply` to switch.".to_string());
    }

    // Use bootstrap_team_selection logic
    let current_stats_state = ctx.state_manager
        .get_stats_state(|s| s.clone())
        .unwrap_or_default();

    let start_phase = crate::commands::game::start_phase_for_game(&game);
    let stats_state = crate::commands::game::bootstrap_team_selection(&mut game, &team_id, start_phase, current_stats_state)?;

    // Save
    let manager_name = format!("{} {}", game.manager.first_name, game.manager.last_name);
    let save_name = crate::commands::game::default_save_name(&manager_name);
    let mut sm = ctx.save_manager_state.0.lock().map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
    let save_id = crate::commands::game::create_new_save(&mut sm, &game, &stats_state, &save_name)?;

    ctx.state_manager.set_save_id(save_id.clone());
    ctx.state_manager.set_game(game);
    ctx.state_manager.set_stats_state(stats_state);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Team Selected\n\n**Save ID**: {}\nTeam assigned and game saved.", save_id))
}

// ─── game_load_save ─────────────────────────────────────────────────────────

// ─── game_load_save ─────────────────────────────────────────────────────────

pub fn game_load_save(ctx: Arc<McpContext>, save_id: String) -> Result<String, String> {
    let mut sm = ctx.save_manager_state.0.lock().map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
    let mut game = sm.load_game(&save_id)?;
    let stats_state = sm.load_stats_state(&save_id)?;
    ofm_core::ai_hiring::seed_ai_managers(&mut game);
    ofm_core::season_context::refresh_game_context(&mut game);

    let mgr_name = format!("{} {}", game.manager.first_name, game.manager.last_name);

    ctx.state_manager.set_save_id(save_id.clone());
    ctx.state_manager.set_game(game);
    ctx.state_manager.set_stats_state(stats_state);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Save Loaded\n\n**Save ID**: {}\n**Manager**: {}\n**Date**: {}", save_id, mgr_name,
        ctx.state_manager.get_game(|g| g.clock.current_date.format("%d %B %Y").to_string()).unwrap_or_default()))
}

// ─── game_exit ──────────────────────────────────────────────────────────────

// ─── game_exit ──────────────────────────────────────────────────────────────

pub fn game_exit(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    // Auto-save
    if let Some(save_id) = ctx.state_manager.get_save_id() {
        let stats_state = ctx.state_manager
            .get_stats_state(|s| s.clone())
            .unwrap_or_default();
        let mut sm = ctx.save_manager_state.0.lock().map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
        sm.save_game_with_stats(&game, &stats_state, &save_id)?;
    }

    ctx.state_manager.clear_game();
    ctx.state_manager.set_save_id(String::new());

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("## Returned to Menu\n\nGame saved and cleared. Use `game_load_save` to resume.".to_string())
}

// ─── game_export_world ──────────────────────────────────────────────────────

/// Safe export that writes to the app-controlled data directory.
/// The filename is auto-generated from the current date.
pub fn game_export_world_safe(ctx: Arc<McpContext>) -> Result<String, String> {
    let app_data_dir = ctx.app_handle
        .path()
        .app_data_dir()
        .map_err(|_| "Could not resolve app data directory".to_string())?;

    let date = ctx.state_manager
        .get_game(|g| g.clock.current_date.format("%Y%m%d").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let filename = format!("world_export_{}.json", date);
    let export_path = app_data_dir.join(&filename);

    crate::commands::world::export_world_database_internal(
        &ctx.state_manager,
        &export_path,
    )
    .map_err(|e| translate_error(&e))?;

    Ok(format!("## World Exported\n\nWritten to: {}", export_path.display()))
}

// ─── game_delete_save ───────────────────────────────────────────────────────

pub fn game_delete_save(ctx: Arc<McpContext>, save_id: String) -> Result<String, String> {
    // Prevent deleting the currently active save
    if let Some(active_id) = ctx.state_manager.get_save_id() {
        if active_id == save_id {
            return Err("Cannot delete the currently active save. Use game_exit first.".to_string());
        }
    }

    let mut sm = ctx.save_manager_state.0.lock().map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
    sm.delete_save(&save_id)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Save Deleted\n\nSave {} has been permanently deleted.", save_id))
}

// ─── game_list_world_databases ──────────────────────────────────────────────

pub fn game_list_world_databases(ctx: Arc<McpContext>) -> Result<String, String> {
    let databases = crate::commands::world::list_world_databases(ctx.app_handle.clone())?;

    if databases.is_empty() {
        return Ok("## World Databases\n\nNo world databases found.".to_string());
    }

    let mut lines = vec!["| ID | Name | Teams | Players | Source |".to_string(), "|---|---|---|---|---|".to_string()];
    for db in &databases {
        lines.push(format!("| {} | {} | {} | {} | {} |", db.id, db.name, db.team_count, db.player_count, db.source));
    }

    Ok(format!("## World Databases\n\n{}\n\nUse `game_new` with `world_source` set to a database path to start with that world.", lines.join("\n")))
}
