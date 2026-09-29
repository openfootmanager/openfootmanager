//! The bug-report commands: describe this machine, and pack the evidence.
//!
//! Neither command sends anything. Slice 3 of #569 ends at a file on disk that the player chooses
//! the location of; the upload is slice 4, and the GitHub form is the path that needs no server at
//! all.

use std::path::PathBuf;
use std::sync::Arc;

use ofm_core::state::StateManager;
use tauri::{Manager, State};

use crate::crash;
use crate::report::bundle::{self, BundleInputs, BundleSummary};
use crate::report::redact::Redactor;
use crate::SaveManagerState;

const SAVE_MANAGER_UNAVAILABLE: &str = "be.error.saveManagerUnavailable";
const REPORT_BUNDLE_FAILED: &str = "be.error.report.bundleFailed";

/// What this machine is, for someone reading the report later.
///
/// Everything here is about the build and the platform. Nothing about the player's game is
/// included: the frontend already holds the active career and composes that part of the report
/// itself, so duplicating it across the IPC boundary would be a second copy that can disagree.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiagnosticsReport {
    pub app_version: String,
    pub os: String,
    pub arch: String,
    pub webview_version: String,
    pub log_directory: String,
    pub crash_on_previous_run: bool,
    /// Whether a career is open, so the preview and the backend cannot disagree about it.
    ///
    /// The preview decides from this whether to offer the save at all. Deriving it here, from the
    /// same `get_save_id()` the export uses, is the point: when the screen computed it for itself
    /// it said "no career is open" while the export attached one.
    pub has_active_save: bool,
}

fn log_dir(app_handle: &tauri::AppHandle) -> Option<PathBuf> {
    app_handle.path().app_log_dir().ok()
}

fn collect(
    app_handle: &tauri::AppHandle,
    redactor: &Redactor,
    has_active_save: bool,
) -> DiagnosticsReport {
    let crash_on_previous_run = app_handle
        .path()
        .app_data_dir()
        .map(|dir| crash::crash_file_in(&dir).exists())
        .unwrap_or(false);

    DiagnosticsReport {
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        // Free function, not a method: `tauri::webview_version` is re-exported from
        // `tauri_runtime_wry` behind the default `wry` feature.
        webview_version: tauri::webview_version().unwrap_or_else(|_| "unknown".to_owned()),
        // The log directory runs through the player's home directory on every platform, so it is
        // redacted like everything else — this value is shown on the preview screen.
        log_directory: log_dir(app_handle)
            .map(|dir| redactor.apply(&dir.to_string_lossy()))
            .unwrap_or_default(),
        crash_on_previous_run,
        has_active_save,
    }
}

#[tauri::command]
pub fn collect_diagnostics(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<StateManager>>,
) -> DiagnosticsReport {
    collect(
        &app_handle,
        &Redactor::from_environment(),
        state.get_save_id().is_some(),
    )
}

/// Write the report bundle to a path the player chose, and say what went into it.
#[tauri::command]
pub fn export_report_bundle(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<StateManager>>,
    save_manager: State<'_, Arc<SaveManagerState>>,
    output_path: String,
    include_save: bool,
) -> Result<BundleSummary, String> {
    let redactor = Redactor::from_environment();
    let has_active_save = state.get_save_id().is_some();
    let diagnostics = collect(&app_handle, &redactor, has_active_save);
    let diagnostics_json =
        serde_json::to_string_pretty(&diagnostics).map_err(|_| REPORT_BUNDLE_FAILED.to_owned())?;

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|_| SAVE_MANAGER_UNAVAILABLE.to_owned())?;
    let crash_json = std::fs::read_to_string(crash::crash_file_in(&app_data_dir)).ok();

    // Only when the player ticked the box on the preview screen, and only if a career is open.
    let save_path = if include_save {
        let save_id = state.get_save_id();
        let manager = save_manager
            .0
            .lock()
            .map_err(|_| SAVE_MANAGER_UNAVAILABLE.to_owned())?;
        save_id.and_then(|id| manager.save_db_path(&id))
    } else {
        None
    };

    let summary = bundle::write_bundle(
        &BundleInputs {
            log_dir: &log_dir(&app_handle).unwrap_or_default(),
            diagnostics_json: &diagnostics_json,
            crash_json: crash_json.as_deref(),
            save_path: save_path.as_deref(),
        },
        std::path::Path::new(&output_path),
        &redactor,
    )
    .map_err(|error| {
        log::error!("[report] could not write the bundle: {error}");
        REPORT_BUNDLE_FAILED.to_owned()
    })?;

    log::info!(
        "[report] bundle written: {} bytes, {} log file(s), save={}, crash={}",
        summary.bytes,
        summary.log_files.len(),
        summary.included_save,
        summary.included_crash
    );
    Ok(summary)
}

/// A dated default for the save dialog, so a second report does not overwrite the first.
#[tauri::command]
pub fn suggested_report_file_name() -> String {
    bundle::suggested_file_name(chrono::Utc::now())
}
