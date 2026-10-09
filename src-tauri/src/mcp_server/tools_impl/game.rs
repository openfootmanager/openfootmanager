//! MCP tool implementations: game

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::require_game;
use std::sync::Arc;
use tauri::Manager as TauriManager;

// ─── game_list_saves ────────────────────────────────────────────────────────

pub fn game_list_saves(ctx: Arc<McpContext>) -> Result<String, String> {
    let mut sm = ctx
        .save_manager_state
        .0
        .lock()
        .map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
    let saves = sm.load_saves()?;

    if saves.is_empty() {
        return Ok("## Saves\n\nNo saves found.".to_string());
    }

    let mut lines = vec![
        "| ID | Manager | Last Played |".to_string(),
        "|---|---|---|".to_string(),
    ];
    for save in &saves {
        lines.push(format!(
            "| {} | {} | {} |",
            save.id, save.manager_name, save.last_played_at
        ));
    }

    Ok(format!("## Saves\n\n{}", lines.join("\n")))
}

// ─── game_delete_save ───────────────────────────────────────────────────────

// ─── game_save ──────────────────────────────────────────────────────────────

pub fn game_save(ctx: Arc<McpContext>) -> Result<String, String> {
    crate::application::live_session::ensure_idle(&ctx.state_manager)?;
    let save_id = ctx
        .state_manager
        .get_save_id()
        .ok_or("be.error.noActiveSaveSession")?;
    {
        let mut sm = ctx
            .save_manager_state
            .0
            .lock()
            .map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
        crate::commands::util::persist_active_game(&ctx.state_manager, &mut sm)?;
    }
    let date = ctx
        .state_manager
        .get_game(|game| game.clock.current_date.format("%d %B %Y").to_string())
        .unwrap_or_default();

    Ok(format!(
        "## Game Saved\n\n**Save ID**: {}\n**Date**: {}",
        save_id, date
    ))
}

// ─── squad_set_starting_xi ─────────────────────────────────────────────────

// ─── game_new ───────────────────────────────────────────────────────────────

pub fn game_new(
    ctx: Arc<McpContext>,
    request: crate::commands::game::McpNewCareer<'_>,
) -> Result<String, String> {
    let text = create_career(&ctx.state_manager, &ctx.save_manager_state, &request)?;
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }
    Ok(text)
}

fn create_career(
    state_manager: &ofm_core::state::StateManager,
    save_manager_state: &crate::SaveManagerState,
    request: &crate::commands::game::McpNewCareer<'_>,
) -> Result<String, String> {
    let blank = |name: Option<&str>| name.is_some_and(|name| name.trim().is_empty());
    if blank(request.manager_first_name) || blank(request.manager_last_name) {
        return Err("be.error.createManager.nameRequired".to_string());
    }
    if blank(request.manager_nationality) {
        return Err("be.error.createManager.nationalityRequired".to_string());
    }

    let save_id =
        crate::commands::game::start_career_for_mcp(state_manager, save_manager_state, request)?;
    let manager = format!(
        "Manager: {} {}\nNationality: {}",
        request.manager_first_name.unwrap_or("Agent"),
        request.manager_last_name.unwrap_or("Manager"),
        request.manager_nationality.unwrap_or("England")
    );
    Ok(match save_id {
        Some(save_id) => format!(
            "## Game Created\n\n{manager}\nSave ID: {save_id}\n\nUse `info_game_summary` to see your current state."
        ),
        None => format!(
            "## Game Created\n\n{manager}\nNo club yet: call `game_select_team` to start the career."
        ),
    })
}

// ─── game_select_team ───────────────────────────────────────────────────────

// ─── game_select_team ───────────────────────────────────────────────────────

pub fn game_select_team(ctx: Arc<McpContext>, team_id: String) -> Result<String, String> {
    let mut game = require_game(&ctx.state_manager)?;

    if game.manager.team_id.is_some() {
        return Err("be.error.mcp.teamAlreadyAssigned".to_string());
    }

    let current_stats_state = ctx
        .state_manager
        .get_stats_state(|s| s.clone())
        .unwrap_or_default();

    // The same career start as the app's `select_team`, because it is the same
    // function. This tool used to do a subset of it: it never aligned the clock to
    // the club's season, never resolved the simulation scope for the club, and
    // never made positions granular, so an agent's career differed from a player's.
    let stats_state = crate::commands::game::begin_career(
        &mut game,
        &team_id,
        crate::commands::game::CareerScope::default(),
        current_stats_state,
    )?;

    // Save
    let manager_name = format!("{} {}", game.manager.first_name, game.manager.last_name);
    let save_name = crate::commands::game::default_save_name(&manager_name);
    let mut sm = ctx
        .save_manager_state
        .0
        .lock()
        .map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
    let save_id = crate::commands::game::create_new_save(&mut sm, &game, &stats_state, &save_name)?;

    // Commit after save creation, without holding the game mutex across I/O.
    // Career installation serializes publication with a concurrent start/finish.
    crate::application::career::install_career(
        &ctx.state_manager,
        game,
        stats_state,
        Some(save_id.clone()),
    );

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!(
        "## Team Selected\n\n**Save ID**: {}\nTeam assigned and game saved.",
        save_id
    ))
}

// ─── game_load_save ─────────────────────────────────────────────────────────

// ─── game_load_save ─────────────────────────────────────────────────────────

pub fn game_load_save(ctx: Arc<McpContext>, save_id: String) -> Result<String, String> {
    let mut sm = ctx
        .save_manager_state
        .0
        .lock()
        .map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
    let mut game = sm.load_game(&save_id)?;
    let stats_state = sm.load_stats_state(&save_id)?;
    ofm_core::ai_hiring::seed_ai_managers(&mut game);
    ofm_core::season_context::refresh_game_context(&mut game);

    let mgr_name = format!("{} {}", game.manager.first_name, game.manager.last_name);

    // A loaded career must not inherit the previous career's transient session.
    crate::application::career::install_career(
        &ctx.state_manager,
        game,
        stats_state,
        Some(save_id.clone()),
    );

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!(
        "## Save Loaded\n\n**Save ID**: {}\n**Manager**: {}\n**Date**: {}",
        save_id,
        mgr_name,
        ctx.state_manager
            .get_game(|g| g.clock.current_date.format("%d %B %Y").to_string())
            .unwrap_or_default()
    ))
}

// ─── game_exit ──────────────────────────────────────────────────────────────

pub fn game_exit(ctx: Arc<McpContext>) -> Result<String, String> {
    let mut sm = ctx
        .save_manager_state
        .0
        .lock()
        .map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
    let saved = crate::application::saving::exit_to_menu(&ctx.state_manager, &mut sm)?;
    drop(sm);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    if saved {
        Ok(
            "## Returned to Menu\n\nGame saved and cleared. Use `game_load_save` to resume."
                .to_string(),
        )
    } else {
        Ok(
            "## Returned to Menu\n\nGame cleared without saving. Use `game_load_save` to resume."
                .to_string(),
        )
    }
}

// ─── game_export_world ──────────────────────────────────────────────────────

/// Safe export that writes to the app-controlled data directory.
/// The filename is auto-generated from the current date.
pub fn game_export_world_safe(ctx: Arc<McpContext>) -> Result<String, String> {
    let app_data_dir = ctx
        .app_handle
        .path()
        .app_data_dir()
        .map_err(|_| "Could not resolve app data directory".to_string())?;

    let date = ctx
        .state_manager
        .get_game(|g| g.clock.current_date.format("%Y%m%d").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let filename = format!("world_export_{}.json", date);
    let export_path = app_data_dir.join(&filename);

    crate::commands::world::export_world_database_internal(&ctx.state_manager, &export_path)?;

    Ok(format!(
        "## World Exported\n\nWritten to: {}",
        export_path.display()
    ))
}

// ─── game_delete_save ───────────────────────────────────────────────────────

pub fn game_delete_save(ctx: Arc<McpContext>, save_id: String) -> Result<String, String> {
    // Prevent deleting the currently active save
    if let Some(active_id) = ctx.state_manager.get_save_id() {
        if active_id == save_id {
            return Err("be.error.mcp.cannotDeleteActiveSave".to_string());
        }
    }

    let mut sm = ctx
        .save_manager_state
        .0
        .lock()
        .map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
    sm.delete_save(&save_id)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!(
        "## Save Deleted\n\nSave {} has been permanently deleted.",
        save_id
    ))
}

// ─── game_list_world_databases ──────────────────────────────────────────────

pub fn game_list_world_databases(ctx: Arc<McpContext>) -> Result<String, String> {
    let databases = crate::commands::world::list_world_databases(ctx.app_handle.clone())?;

    if databases.is_empty() {
        return Ok("## World Databases\n\nNo world databases found.".to_string());
    }

    let mut lines = vec![
        "| ID | Name | Teams | Players | Source |".to_string(),
        "|---|---|---|---|---|".to_string(),
    ];
    for db in &databases {
        lines.push(format!(
            "| {} | {} | {} | {} | {} |",
            db.id, db.name, db.team_count, db.player_count, db.source
        ));
    }

    Ok(format!("## World Databases\n\n{}\n\nUse `game_new` with `world_source` set to a database path to start with that world.", lines.join("\n")))
}
