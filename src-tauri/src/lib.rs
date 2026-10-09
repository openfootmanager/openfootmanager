mod application;
mod commands;
mod crash;
mod platform;
mod report;
use commands::*;

#[cfg(feature = "mcp")]
mod mcp_server;

use db::save_manager::SaveManager;
use ofm_core::state::StateManager;
use std::sync::{Arc, Mutex};

const SAVE_MANAGER_UNAVAILABLE_ERROR: &str = "be.error.saveManagerUnavailable";

/// Tauri-managed wrapper around SaveManager.
pub struct SaveManagerState(pub Mutex<SaveManager>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // First of all, so that everything below is covered — including plugin registration and the
    // graphics configuration, where a panic would otherwise leave nothing at all behind. The hook
    // has no file to write to yet; `setup` gives it one as soon as the app data directory
    // resolves. See `crash`.
    crash::install_panic_hook();

    // Must run before the webview is built: on Linux, WebKitGTK and the graphics driver read the
    // variables this sets when the web process starts. A no-op on Windows and macOS.
    // See `platform` and `docs/LINUX_GRAPHICS.md`.
    platform::configure_graphics();

    let state_manager = Arc::new(StateManager::new());

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .level_for("openfootmanager_lib", log::LevelFilter::Debug)
                .level_for("ofm_core", log::LevelFilter::Debug)
                .level_for("engine", log::LevelFilter::Debug)
                .level_for("db", log::LevelFilter::Debug)
                // `KeepAll` meant the log folder grew for the life of the install and was never
                // pruned — on a machine that had played a few hundred hours, the folder a bug
                // reporter is asked to zip up is the largest thing in the report and almost all of
                // it predates the bug. Five rotations of 5 MB bounds it at ~25 MB while still
                // covering several sessions back. `KeepSome(5)` is five archived files plus
                // the one currently being written, so the real ceiling is ~30 MB.
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .max_file_size(5_000_000) // 5 MB per log file
                .build(),
        )
        .manage(state_manager.clone())
        .setup(move |app| {
            use tauri::Manager as TauriManager;

            platform::watch_web_processes(app);

            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|_| std::io::Error::other(SAVE_MANAGER_UNAVAILABLE_ERROR))?;
            std::fs::create_dir_all(&app_data_dir)
                .map_err(|_| std::io::Error::other(SAVE_MANAGER_UNAVAILABLE_ERROR))?;

            // Give the panic hook somewhere to write, then report and clear whatever the previous
            // launch left. Done here rather than later in `setup` so that a panic in any of the
            // startup work below — save manager init, the legacy migration — is itself recorded.
            crash::set_crash_file(crash::crash_file_in(&app_data_dir));
            // Held rather than dropped: the file has to be cleared here or the same crash is
            // reported on every launch, but the bug report is filed later in this same session.
            app.manage(crash::PreviousCrash(crash::take_previous_crash(
                &app_data_dir,
            )));

            let saves_dir = app_data_dir.join("saves");
            let mut save_manager = SaveManager::init(&saves_dir).map_err(std::io::Error::other)?;

            // Run legacy migration if old saves.db exists
            if db::legacy_migration::has_legacy_db(&app_data_dir) {
                log::info!("[setup] Legacy saves.db detected, migrating...");
                match db::legacy_migration::migrate_legacy_saves(&app_data_dir, &mut save_manager) {
                    Ok(results) => {
                        let success = results
                            .iter()
                            .filter(|r| {
                                matches!(
                                    r,
                                    db::legacy_migration::LegacyMigrationResult::Success { .. }
                                )
                            })
                            .count();
                        let failed = results
                            .iter()
                            .filter(|r| {
                                matches!(
                                    r,
                                    db::legacy_migration::LegacyMigrationResult::Failed { .. }
                                )
                            })
                            .count();
                        log::info!(
                            "[setup] Legacy migration complete: {} succeeded, {} failed",
                            success,
                            failed
                        );
                    }
                    Err(e) => log::error!("[setup] Legacy migration failed: {}", e),
                }
            }

            app.manage(Arc::new(SaveManagerState(Mutex::new(save_manager))));

            // --- MCP server startup (feature-flagged) ---
            #[cfg(feature = "mcp")]
            match mcp_server::config::parse_mcp_config_from_args() {
                Ok(Some(mcp_config)) => {
                    use tauri::Emitter;
                    use tauri::Manager as TauriManager;

                    let sm: Arc<StateManager> = app.state::<Arc<StateManager>>().inner().clone();
                    let save_mgr: Arc<SaveManagerState> =
                        app.state::<Arc<SaveManagerState>>().inner().clone();
                    let app_handle = app.handle().clone();

                    // --no-gui: hide the window
                    if mcp_config.no_gui {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                            log::info!("[mcp] GUI window hidden (--no-gui)");
                        }
                    }

                    // --mcp-auto-start: bootstrap game
                    if let Some(ref auto_start) = mcp_config.auto_start {
                        log::info!(
                            "[mcp] Auto-starting: world={}, team={:?}",
                            auto_start.world_path,
                            auto_start.team_id
                        );

                        let mgr_name = mcp_config
                            .manager_name
                            .as_deref()
                            .unwrap_or("Agent")
                            .to_string();
                        let mgr_last = mcp_config
                            .manager_last_name
                            .as_deref()
                            .unwrap_or("Manager")
                            .to_string();
                        let mgr_nat = mcp_config
                            .manager_nationality
                            .as_deref()
                            .unwrap_or("England")
                            .to_string();

                        match crate::commands::game::start_career_for_mcp(
                            &sm,
                            &save_mgr,
                            &crate::commands::game::McpNewCareer {
                                world_source: Some(&auto_start.world_path),
                                team_id: auto_start.team_id.as_deref(),
                                manager_first_name: &mgr_name,
                                manager_last_name: &mgr_last,
                                manager_nationality: &mgr_nat,
                                options: auto_start.options.clone(),
                            },
                        )
                        .and_then(|save_id| {
                            save_id.ok_or_else(|| {
                                "--mcp-auto-start requires a team_id when the world's manager has no team. Format: \"world.json,team_id\"".to_string()
                            })
                        }) {
                            Ok(save_id) => {
                                log::info!("[mcp] Bootstrap complete, save_id={}", save_id);
                                // Notify GUI that a game is now active
                                let _ = app_handle.emit("game-state-changed", ());
                            }
                            Err(e) => {
                                log::error!("[mcp] Bootstrap failed: {}", e);
                                return Err(Box::new(std::io::Error::other(format!(
                                    "MCP auto-start failed: {}",
                                    e
                                )))
                                    as Box<dyn std::error::Error + Send + Sync>);
                            }
                        }
                    }

                    // Spawn MCP server on the tokio runtime
                    let mcp_port = mcp_config.port;
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) =
                            mcp_server::start_mcp_server(mcp_config, sm, save_mgr, app_handle).await
                        {
                            log::error!("[mcp] MCP server failed: {}", e);
                        }
                    });

                    log::info!("[mcp] Starting MCP server on port {}", mcp_port);
                }
                Ok(None) => {
                    // No --mcp-port, MCP server not requested
                }
                Err(e) => {
                    log::error!("[mcp-config] {}", e);
                    return Err(
                        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
                            as Box<dyn std::error::Error + Send + Sync>,
                    );
                }
            }

            // Last, deliberately: everything above can abort startup for reasons that have
            // nothing to do with rendering — an unreadable save directory, a failed migration, a
            // bad MCP config. Clearing the graphics marker only once past all of them keeps those
            // failures from being counted as evidence that the GPU path is broken and eventually
            // dropping the player onto CPU compositing for a save-system fault.
            platform::watch_startup();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_world_databases,
            install_package,
            list_installed_packages,
            uninstall_package,
            check_package_stack,
            get_nations,
            start_new_game,
            validate_competition_definitions,
            validate_world_package,
            inspect_world_package,
            export_world_database,
            write_temp_database,
            select_team,
            get_saves,
            load_game,
            get_active_game,
            get_players_page,
            get_teams_directory,
            get_schedule,
            get_news_feed,
            get_messages_page,
            get_competitions_view,
            get_session_state,
            get_squad,
            get_staff,
            get_active_save_id,
            advance_time,
            advance_time_with_mode,
            upgrade_facility,
            get_finance_snapshot,
            request_board_support,
            request_marketing_campaign,
            request_sponsor_pitch,
            propose_renewal,
            delegate_renewals,
            preview_renewal_financial_impact,
            offer_free_agent_contract,
            preview_free_agent_contract_impact,
            set_contract_exit_intent,
            clear_contract_exit_intent,
            preview_contract_termination,
            terminate_contract_now,
            set_formation,
            set_starting_xi,
            set_play_style,
            apply_tactic_preset,
            set_team_match_roles,
            set_training,
            set_training_schedule,
            set_training_groups,
            set_player_training_focus,
            set_player_squad_role,
            set_player_role,
            set_tactics_phase,
            assign_jersey_number,
            set_team_kit_pattern,
            hire_staff,
            release_staff,
            mark_message_read,
            delete_message,
            delete_messages,
            mark_all_messages_read,
            clear_old_messages,
            save_game,
            auto_select_set_pieces,
            toggle_transfer_list,
            toggle_loan_list,
            make_transfer_bid,
            make_loan_offer,
            exercise_loan_buy_option,
            preview_transfer_bid_financial_impact,
            respond_to_offer,
            respond_to_loan_offer,
            counter_loan_offer,
            counter_offer,
            send_scout,
            start_youth_scouting,
            cancel_youth_scouting,
            reassign_youth_scouting,
            check_season_complete,
            advance_to_next_season,
            get_season_awards,
            resolve_message_action,
            start_live_match,
            get_player_match_history,
            get_player_stats_overview,
            get_team_match_history,
            get_team_stats_overview,
            step_live_match,
            apply_match_command,
            get_match_snapshot,
            finish_live_match,
            generate_player_portrait,
            prewarm_player_portraits,
            delete_save,
            skip_to_match_day,
            advance_to_next_event,
            advance_one_day,
            check_blocking_actions,
            apply_team_talk,
            submit_press_conference,
            exit_to_menu,
            get_settings,
            save_settings,
            clear_all_saves,
            get_available_jobs,
            apply_for_job,
            get_manager_profiles,
            save_manager_profile,
            update_manager_profile,
            delete_manager_profile,
            touch_manager_profile,
            run_sim_batch,
            run_single_seeded_match,
            create_package_project,
            create_world_project,
            read_package_project,
            save_package_project,
            build_ofm,
            extract_ofm_for_editing,
            copy_package_asset,
            export_teams_csv,
            export_players_csv,
            read_file_as_data_url,
            commands::report::collect_diagnostics,
            commands::report::export_report_bundle,
            commands::report::redact_report_fields,
            commands::report::suggested_report_file_name
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        // `panic_any(error)` used to be the whole of this. It carried a `tauri::Error`, which no
        // panic handler can print — so the one failure that guarantees the player sees nothing at
        // all was also the one that told us least. Panicking with the formatted message puts the
        // reason in the log file, in `last-crash.json`, and on stderr.
        panic!("Tauri failed to start: {error}");
    }
}
