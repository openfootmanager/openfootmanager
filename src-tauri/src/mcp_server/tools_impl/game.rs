//! MCP tool implementations: game

use mcp_results::game::{
    GameCreated, GameSaved, ReturnedToMenu, SaveDeleted, SaveList, SaveLoaded, SaveSummary,
    TeamSelected, WorldDatabase, WorldDatabases, WorldExported,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::require_game;
use std::sync::Arc;
use tauri::Manager as TauriManager;

// ─── game_list_saves ────────────────────────────────────────────────────────

pub fn game_list_saves(ctx: Arc<McpContext>) -> Result<SaveList, String> {
    let mut sm = ctx
        .save_manager_state
        .0
        .lock()
        .map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
    let saves = sm.load_saves()?;

    Ok(SaveList {
        saves: saves
            .into_iter()
            .map(|save| SaveSummary {
                id: save.id,
                manager_name: save.manager_name,
                last_played_at: save.last_played_at,
            })
            .collect(),
    })
}

// ─── game_delete_save ───────────────────────────────────────────────────────

// ─── game_save ──────────────────────────────────────────────────────────────

pub fn game_save(ctx: Arc<McpContext>) -> Result<GameSaved, String> {
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

    Ok(GameSaved { save_id, date })
}

// ─── squad_set_starting_xi ─────────────────────────────────────────────────

// ─── game_new ───────────────────────────────────────────────────────────────

pub fn game_new(
    ctx: Arc<McpContext>,
    request: crate::commands::game::McpNewCareer<'_>,
) -> Result<GameCreated, String> {
    let created = create_career(&ctx.state_manager, &ctx.save_manager_state, &request)?;
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }
    Ok(created)
}

fn create_career(
    state_manager: &ofm_core::state::StateManager,
    save_manager_state: &crate::SaveManagerState,
    request: &crate::commands::game::McpNewCareer<'_>,
) -> Result<GameCreated, String> {
    let blank = |name: Option<&str>| name.is_some_and(|name| name.trim().is_empty());
    if blank(request.manager_first_name) || blank(request.manager_last_name) {
        return Err("be.error.createManager.nameRequired".to_string());
    }
    if blank(request.manager_nationality) {
        return Err("be.error.createManager.nationalityRequired".to_string());
    }

    let save_id =
        crate::commands::game::start_career_for_mcp(state_manager, save_manager_state, request)?;
    Ok(GameCreated {
        manager_first_name: request.manager_first_name.unwrap_or("Agent").to_string(),
        manager_last_name: request.manager_last_name.unwrap_or("Manager").to_string(),
        nationality: request.manager_nationality.unwrap_or("England").to_string(),
        save_id,
    })
}

// ─── game_select_team ───────────────────────────────────────────────────────

// ─── game_select_team ───────────────────────────────────────────────────────

pub fn game_select_team(ctx: Arc<McpContext>, team_id: String) -> Result<TeamSelected, String> {
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

    Ok(TeamSelected { save_id })
}

// ─── game_load_save ─────────────────────────────────────────────────────────

// ─── game_load_save ─────────────────────────────────────────────────────────

pub fn game_load_save(ctx: Arc<McpContext>, save_id: String) -> Result<SaveLoaded, String> {
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

    Ok(SaveLoaded {
        save_id,
        manager_name: mgr_name,
        date: ctx
            .state_manager
            .get_game(|g| g.clock.current_date.format("%d %B %Y").to_string())
            .unwrap_or_default(),
    })
}

// ─── game_exit ──────────────────────────────────────────────────────────────

pub fn game_exit(ctx: Arc<McpContext>) -> Result<ReturnedToMenu, String> {
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

    Ok(ReturnedToMenu { saved })
}

// ─── game_export_world ──────────────────────────────────────────────────────

/// Safe export that writes to the app-controlled data directory.
/// The filename is auto-generated from the current date.
pub fn game_export_world_safe(ctx: Arc<McpContext>) -> Result<WorldExported, String> {
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

    Ok(WorldExported {
        path: export_path.display().to_string(),
    })
}

// ─── game_delete_save ───────────────────────────────────────────────────────

pub fn game_delete_save(ctx: Arc<McpContext>, save_id: String) -> Result<SaveDeleted, String> {
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

    Ok(SaveDeleted { save_id })
}

// ─── game_list_world_databases ──────────────────────────────────────────────

pub fn game_list_world_databases(ctx: Arc<McpContext>) -> Result<WorldDatabases, String> {
    let databases = crate::commands::world::list_world_databases(ctx.app_handle.clone())?;

    Ok(WorldDatabases {
        databases: databases
            .into_iter()
            .map(|db| WorldDatabase {
                id: db.id,
                name: db.name,
                team_count: db.team_count,
                player_count: db.player_count,
                source: db.source,
            })
            .collect(),
    })
}
