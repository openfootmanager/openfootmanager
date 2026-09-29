//! The bug-report commands: describe this machine, and pack the evidence.
//!
//! Neither command sends anything. Slice 3 of #569 ends at a file on disk that the player chooses
//! the location of; the upload is slice 4, and the GitHub form is the path that needs no server at
//! all.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ofm_core::state::StateManager;
use tauri::{Manager, State};

use crate::crash;
use crate::report::bundle::{self, BundleInputs, BundleSummary};
use crate::report::redact::Redactor;
use crate::SaveManagerState;

const SAVE_MANAGER_UNAVAILABLE: &str = "be.error.saveManagerUnavailable";
const REPORT_BUNDLE_FAILED: &str = "be.error.report.bundleFailed";
const REPORT_SAVE_MISSING: &str = "be.error.report.saveMissing";

/// What this machine is, for someone reading the report later.
///
/// Everything here is about the build and the platform. Nothing about the player's game is
/// included: the frontend already holds the active career and composes that part of the report
/// itself, so duplicating it across the IPC boundary would be a second copy that can disagree.
/// One log file the report would carry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LogFileSummary {
    pub name: String,
    pub bytes: u64,
}

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
    /// The log files this report would carry, newest first, with their sizes.
    ///
    /// Named here rather than described in prose because the preview is the consent screen, and
    /// "your logs" is not consent to something whose size the player cannot see. Chosen by the
    /// same `planned_logs` the export uses, so the two cannot disagree about which files those
    /// are.
    pub log_files: Vec<LogFileSummary>,
    /// Size of the save the tick box would attach, when there is one.
    ///
    /// This is the number that actually changes a decision: a career database is the largest
    /// thing in the bundle by a wide margin, and it is the one part the player chooses.
    pub save_bytes: Option<u64>,
}

fn log_dir(app_handle: &tauri::AppHandle) -> Option<PathBuf> {
    app_handle.path().app_log_dir().ok()
}

fn collect(
    app_handle: &tauri::AppHandle,
    redactor: &Redactor,
    has_active_save: bool,
    crash_on_previous_run: bool,
    save_bytes: Option<u64>,
) -> DiagnosticsReport {
    let log_files = log_dir(app_handle)
        .map(|dir| {
            bundle::planned_logs(&dir)
                .into_iter()
                .map(|candidate| LogFileSummary {
                    // The names are rotation stamps, not anything of the player's, but they go
                    // through the redactor anyway rather than being trusted for their shape.
                    name: redactor.apply(&candidate.name),
                    bytes: candidate.bytes,
                })
                .collect()
        })
        .unwrap_or_default();
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
        log_files,
        save_bytes,
    }
}

/// Size of the active save on disk, or `None` when there is no career or no file behind it.
///
/// A number the preview only displays, so a missing one is not worth failing over — the export
/// is where a save that cannot be found becomes an error the player has to answer.
fn active_save_bytes(state: &StateManager, save_manager: &SaveManagerState) -> Option<u64> {
    let save_id = state.get_save_id()?;
    let manager = save_manager.0.lock().ok()?;
    let path = manager.save_db_path(&save_id)?;
    std::fs::metadata(path).ok().map(|meta| meta.len())
}

#[tauri::command]
pub fn collect_diagnostics(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<StateManager>>,
    save_manager: State<'_, Arc<SaveManagerState>>,
    previous_crash: State<'_, crash::PreviousCrash>,
) -> DiagnosticsReport {
    collect(
        &app_handle,
        &Redactor::from_environment(),
        state.get_save_id().is_some(),
        previous_crash.0.is_some(),
        active_save_bytes(&state, &save_manager),
    )
}

/// A private copy of the save, removed when it goes out of scope.
///
/// Kept in its own directory so the copy can carry the save's real filename into the zip rather
/// than a scratch name. `Drop` does the cleanup because the bundle write can fail, and a copy of
/// the player's career must not be left behind either way.
struct TempSaveCopy {
    dir: PathBuf,
    file: PathBuf,
}

impl Drop for TempSaveCopy {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.dir) {
            log::warn!("[report] could not remove the temporary save copy: {error}");
        }
    }
}

/// Copy the active save **while holding the save-manager lock**.
///
/// Reading the database in place can capture a half-written transaction: no `journal_mode` is set
/// anywhere in `crates/db`, so SQLite's default rollback journal edits pages in place, and a raw
/// read during a commit can yield a file that will not open. Every writer goes through this same
/// mutex, so copying under it is what makes the attached save consistent.
///
/// It copies rather than holding the lock across the whole export, so a concurrent save waits for
/// one file copy instead of the entire compression pass.
fn copy_active_save(
    state: &StateManager,
    save_manager: &SaveManagerState,
    scratch_root: &Path,
) -> Result<Option<TempSaveCopy>, String> {
    // No career open is not a failure — the preview does not offer the save in that case, and the
    // request simply carries nothing.
    let Some(save_id) = state.get_save_id() else {
        return Ok(None);
    };
    let manager = save_manager
        .0
        .lock()
        .map_err(|_| SAVE_MANAGER_UNAVAILABLE.to_owned())?;
    // A career IS open and its file cannot be found — a stale id, a save deleted underneath us.
    // Returning `Ok(None)` here would write a bundle without the save and report success, so the
    // player is told their career was attached when it was not. On the one screen that exists to
    // say what is being sent, a quiet omission is worse than a failure they can retry.
    let Some(source) = manager.save_db_path(&save_id) else {
        log::error!("[report] the active save {save_id} is not in the index");
        return Err(REPORT_SAVE_MISSING.to_owned());
    };
    let file_name = source.file_name().map_or_else(
        || std::ffi::OsString::from("save.db"),
        |name| name.to_owned(),
    );

    let dir = scratch_root.join(format!("ofm-report-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|error| {
        log::error!("[report] could not prepare the save copy: {error}");
        REPORT_BUNDLE_FAILED.to_owned()
    })?;
    let file = dir.join(file_name);
    std::fs::copy(&source, &file).map_err(|error| {
        log::error!("[report] could not copy the save: {error}");
        // Best effort: the guard does not exist yet, so clean up by hand.
        let _ = std::fs::remove_dir_all(&dir);
        REPORT_BUNDLE_FAILED.to_owned()
    })?;
    Ok(Some(TempSaveCopy { dir, file }))
}

/// Write the report bundle to a path the player chose, and say what went into it.
///
/// `async` deliberately. A plain `#[tauri::command]` runs inline on the main thread, and this one
/// reads a save that can be tens of megabytes, redacts several megabytes of logs and deflates all
/// of it — which froze the window for the duration. `spawn_blocking` keeps that work off the UI
/// thread and off the async workers both.
#[tauri::command]
pub async fn export_report_bundle(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<StateManager>>,
    save_manager: State<'_, Arc<SaveManagerState>>,
    previous_crash: State<'_, crash::PreviousCrash>,
    output_path: String,
    report_text: String,
    include_save: bool,
) -> Result<BundleSummary, String> {
    // Read everything off `State` before crossing the thread boundary: the guards themselves are
    // not `Send`, and nothing may be held across the await.
    let state = state.inner().clone();
    let save_manager = save_manager.inner().clone();
    let crash_json = previous_crash.as_json();

    tauri::async_runtime::spawn_blocking(move || {
        write_report_bundle(
            &app_handle,
            &state,
            &save_manager,
            crash_json,
            &output_path,
            &report_text,
            include_save,
        )
    })
    .await
    .map_err(|error| {
        log::error!("[report] the export task did not run: {error}");
        REPORT_BUNDLE_FAILED.to_owned()
    })?
}

fn write_report_bundle(
    app_handle: &tauri::AppHandle,
    state: &StateManager,
    save_manager: &SaveManagerState,
    crash_json: Option<String>,
    output_path: &str,
    report_text: &str,
    include_save: bool,
) -> Result<BundleSummary, String> {
    let redactor = Redactor::from_environment();
    let diagnostics = collect(
        app_handle,
        &redactor,
        state.get_save_id().is_some(),
        crash_json.is_some(),
        active_save_bytes(state, save_manager),
    );
    let diagnostics_json =
        serde_json::to_string_pretty(&diagnostics).map_err(|_| REPORT_BUNDLE_FAILED.to_owned())?;

    // Only when the player ticked the box on the preview screen, and only if a career is open.
    // The copy lives until the bundle is written, then `Drop` removes it.
    let save_copy = if include_save {
        let scratch_root = app_handle
            .path()
            .app_cache_dir()
            .unwrap_or_else(|_| std::env::temp_dir());
        copy_active_save(state, save_manager, &scratch_root)?
    } else {
        None
    };

    let summary = bundle::write_bundle(
        &BundleInputs {
            log_dir: &log_dir(app_handle).unwrap_or_default(),
            diagnostics_json: &diagnostics_json,
            report_text,
            crash_json: crash_json.as_deref(),
            save_path: save_copy.as_ref().map(|copy| copy.file.as_path()),
        },
        Path::new(output_path),
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

#[cfg(test)]
mod tests {
    use super::*;
    use db::save_manager::SaveManager;
    use std::sync::Mutex;

    fn save_manager_in(dir: &Path) -> SaveManagerState {
        SaveManagerState(Mutex::new(
            SaveManager::init(&dir.join("saves")).expect("a save manager"),
        ))
    }

    #[test]
    fn redacts_every_field_the_url_will_carry() {
        // The prefilled issue URL used to carry what the player typed verbatim, while the copy of
        // the same words inside the zip was redacted. A path in the description then reached
        // GitHub and the browser's history, and neither gives it back.
        let redactor = Redactor::new(Some("/home/srobot"), Some("srobot"), None);

        let out = redact_all(
            &redactor,
            &[
                "it died loading /home/srobot/saves/a.db".to_owned(),
                "srobot expected it to open".to_owned(),
            ],
        );

        assert_eq!(out[0], "it died loading ~/saves/a.db");
        assert!(!out[1].contains("srobot"), "{out:?}");
    }

    #[test]
    fn copies_nothing_when_no_career_is_open() {
        let dir = tempfile::tempdir().expect("temp dir");
        let state = StateManager::new();

        let copied =
            copy_active_save(&state, &save_manager_in(dir.path()), dir.path()).expect("no error");

        assert!(copied.is_none());
    }

    #[test]
    fn fails_rather_than_quietly_omitting_a_save_it_cannot_find() {
        // The player ticked the box. Writing the bundle without the save and calling it a success
        // tells them their career went along when it did not.
        let dir = tempfile::tempdir().expect("temp dir");
        let state = StateManager::new();
        state.set_save_id("no-such-save".to_owned());

        let result = copy_active_save(&state, &save_manager_in(dir.path()), dir.path());

        assert_eq!(result.err(), Some(REPORT_SAVE_MISSING.to_owned()));
    }

    #[test]
    fn the_temporary_copy_is_removed_when_it_goes_out_of_scope() {
        // It is a copy of the player's career; leaving it in a cache directory is not acceptable,
        // and the bundle write above it can fail.
        let dir = tempfile::tempdir().expect("temp dir");
        let scratch = dir.path().join("ofm-report-test");
        std::fs::create_dir_all(&scratch).expect("scratch");
        let file = scratch.join("career.db");
        std::fs::write(&file, b"bytes").expect("write");

        {
            let _copy = TempSaveCopy {
                dir: scratch.clone(),
                file: file.clone(),
            };
            assert!(file.exists());
        }

        assert!(!scratch.exists(), "the copy should not outlive its guard");
    }
}

/// A dated default for the save dialog, so a second report does not overwrite the first.
/// Redact free text the same way the bundle does, for the parts that leave by another route.
///
/// The prefilled issue URL carried the player's own words verbatim while the copy inside the zip
/// was redacted — so a path they pasted into the description reached GitHub and their browser
/// history, and neither of those is somewhere it can be taken back from. One call for the whole
/// set rather than one per field: the redactor reads the environment on construction, and doing
/// that four times to answer one screen is waste.
#[tauri::command]
pub fn redact_report_fields(values: Vec<String>) -> Vec<String> {
    redact_all(&Redactor::from_environment(), &values)
}

/// Split from the command so it can be tested against a redactor built for the test.
///
/// `from_environment` reads the real machine, and a test that set `HOME` to assert on the result
/// would be mutating process-wide state under a parallel test runner.
fn redact_all(redactor: &Redactor, values: &[String]) -> Vec<String> {
    values.iter().map(|value| redactor.apply(value)).collect()
}

#[tauri::command]
pub fn suggested_report_file_name() -> String {
    bundle::suggested_file_name(chrono::Utc::now())
}
