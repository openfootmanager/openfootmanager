mod application;
mod commands;
use commands::*;

use db::save_manager::SaveManager;
use ofm_core::state::StateManager;
use std::sync::Mutex;

/// Tauri-managed wrapper around SaveManager.
pub struct SaveManagerState(pub Mutex<SaveManager>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Workaround for WebKitGTK DMABuf rendering issues on Wayland (Linux)
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .level_for("openfootmanager_lib", log::LevelFilter::Debug)
                .level_for("ofm_core", log::LevelFilter::Debug)
                .level_for("engine", log::LevelFilter::Debug)
                .level_for("db", log::LevelFilter::Debug)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                .max_file_size(5_000_000) // 5 MB per log file
                .build(),
        )
        .manage(StateManager::new())
        .setup(|app| {
            use tauri::Manager as TauriManager;
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data dir");

            let saves_dir = app_data_dir.join("saves");
            let mut save_manager =
                SaveManager::init(&saves_dir).expect("Failed to initialize SaveManager");

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

            app.manage(SaveManagerState(Mutex::new(save_manager)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_world_databases,
            start_new_game,
            export_world_database,
            write_temp_database,
            select_team,
            get_saves,
            load_game,
            get_active_game,
            advance_time,
            advance_time_with_mode,
            upgrade_facility,
            propose_renewal,
            delegate_renewals,
            preview_renewal_financial_impact,
            set_contract_exit_intent,
            clear_contract_exit_intent,
            preview_contract_termination,
            terminate_contract_now,
            set_formation,
            set_starting_xi,
            set_play_style,
            set_team_match_roles,
            set_training,
            set_training_schedule,
            set_training_groups,
            set_player_training_focus,
            set_player_squad_role,
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
            preview_transfer_bid_financial_impact,
            respond_to_offer,
            counter_offer,
            send_scout,
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
            delete_save,
            skip_to_match_day,
            check_blocking_actions,
            apply_team_talk,
            submit_press_conference,
            exit_to_menu,
            get_settings,
            save_settings,
            clear_all_saves,
            get_available_jobs,
            apply_for_job
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
