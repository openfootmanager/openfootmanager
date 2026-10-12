//! The bug-report commands: describe this machine, pack the evidence, and — only when the player
//! agrees on the preview — send it.
//!
//! The export (slice 3 of #569) ends at a file on disk that the player chooses the location of.
//! The upload (slice 4) builds the same bundle privately and posts it to OFMHub's bug-report endpoint
//! (`docs/api/bug-reports.md`). The GitHub form is the path that needs no server at all, and stays
//! the fallback whenever the upload is declined, unavailable, or fails.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ofm_core::state::StateManager;
use tauri::{Manager, State};

use crate::crash;
use crate::report::bundle::{self, BundleInputs, BundleSummary};
use crate::report::history::{self, SubmittedReport};
use crate::report::http::UreqTransport;
use crate::report::hub::{self, HubEndpoint, Transport, UploadFailure, UploadParts};
use crate::report::redact::Redactor;
// The key itself lives at the crate root, where the other holders of this lock already read it
// from. Spelling it out again here is how two copies of a translation key drift apart.
use crate::{SaveManagerState, SAVE_MANAGER_UNAVAILABLE_ERROR};

const REPORT_BUNDLE_FAILED: &str = "be.error.report.bundleFailed";
const REPORT_SAVE_MISSING: &str = "be.error.report.saveMissing";

/// One log file the report would carry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LogFileSummary {
    pub name: String,
    pub bytes: u64,
}

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

/// The redactor every path in this module goes through.
///
/// `from_environment` alone knows only the home directory and the account name. On a machine where
/// `XDG_DATA_HOME` (or its log and cache siblings) points outside `$HOME`, the app's own
/// directories carry the identity instead and nothing would strip them — including from
/// `log_directory`, which the preview prints under the sentence promising it has been stripped.
fn redactor_for(app_handle: &tauri::AppHandle) -> Redactor {
    let mut redactor = Redactor::from_environment();
    let paths = app_handle.path();
    for dir in [
        paths.app_log_dir(),
        paths.app_data_dir(),
        paths.app_cache_dir(),
    ]
    .into_iter()
    .flatten()
    {
        redactor.add_path(&dir.to_string_lossy());
    }
    redactor
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
        &redactor_for(&app_handle),
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
        .map_err(|_| SAVE_MANAGER_UNAVAILABLE_ERROR.to_owned())?;
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

    std::fs::create_dir_all(scratch_root).map_err(|error| {
        log::error!("[report] could not prepare the save copy: {error}");
        REPORT_BUNDLE_FAILED.to_owned()
    })?;
    // The directory must be unique even when two exports of the same career overlap.
    let dir = tempfile::Builder::new()
        .prefix("ofm-report-")
        .tempdir_in(scratch_root)
        .map_err(|error| {
            log::error!("[report] could not prepare the save copy: {error}");
            REPORT_BUNDLE_FAILED.to_owned()
        })?
        .keep();
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
    let redactor = redactor_for(app_handle);
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

fn hub_endpoint() -> Option<HubEndpoint> {
    HubEndpoint::resolve(
        std::env::var(hub::HUB_URL_VARIABLE).ok().as_deref(),
        cfg!(debug_assertions),
    )
}

/// Whether the server takes reports right now, so the preview does not offer what cannot work.
///
/// Reads the server's config when the dialog opens, as the contract asks: it carries the kill
/// switch, which is flipped without a release. Offline, disabled or unusable all mean "no" — the
/// GitHub form is the path for those.
#[tauri::command]
pub async fn report_upload_available() -> bool {
    let Some(endpoint) = hub_endpoint() else {
        return false;
    };
    tauri::async_runtime::spawn_blocking(move || {
        hub::fetch_config(UreqTransport::shared(), &endpoint).is_ok()
    })
    .await
    .unwrap_or(false)
}

/// What the player is shown once the server has accepted a report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UploadReceipt {
    /// `OFM-` and eight Crockford characters; named `code` in `reports.json` and the UI.
    pub code: String,
    /// Whether the code also made it into `reports.json`.
    ///
    /// The upload succeeded either way — the server has the report — but a player who is not told
    /// has no reason to note the code down, and then has nothing to quote when they ask for their
    /// report to be removed.
    pub recorded: bool,
}

/// Consent first, then somewhere to send to — and nothing is built until both hold.
fn upload_preconditions(
    consent: bool,
    endpoint: Option<HubEndpoint>,
) -> Result<HubEndpoint, UploadFailure> {
    if !consent {
        return Err(UploadFailure::key(hub::UPLOAD_CONSENT_REQUIRED));
    }
    endpoint.ok_or_else(|| UploadFailure::key(hub::UPLOAD_NOT_CONFIGURED))
}

/// The export's errors, reworded for the upload where they would mislead.
///
/// `bundleFailed` tells the player to check the folder they chose; an upload never asks for one.
/// Everything else — a missing save, the save manager — means the same on both paths.
fn as_upload_error(key: String) -> UploadFailure {
    if key == REPORT_BUNDLE_FAILED {
        UploadFailure::key(hub::UPLOAD_PREPARE_FAILED)
    } else {
        UploadFailure::key(&key)
    }
}

/// What the player typed on the report screen, as the separate fields the endpoint stores.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ReportFields {
    pub what_happened: String,
    pub expected: String,
    pub steps: String,
    /// The frequency choice's key (`always`, `sometimes`, ...), not its translated label.
    pub frequency: String,
    pub game_context: String,
    pub locale: String,
}

/// Longest each text field may be, in characters, as the endpoint's contract states.
const TEXT_FIELD_MAX_CHARS: usize = 10_000;
const FREQUENCY_MAX_CHARS: usize = 32;
const LOCALE_MAX_CHARS: usize = 16;
const OS_MAX_CHARS: usize = 32;
const ARCH_MAX_CHARS: usize = 16;
const WEBVIEW_MAX_CHARS: usize = 64;
/// The `diagnostics` object may be this big once re-encoded as JSON.
const DIAGNOSTICS_MAX_BYTES: usize = 64 * 1024;

fn clipped(redactor: &Redactor, text: &str, max_chars: usize) -> String {
    redactor.apply(text).chars().take(max_chars).collect()
}

/// The `report` part: the player's words, redacted like everything else that leaves the machine,
/// and this machine's identity, each field within the length the endpoint accepts.
fn report_json(
    redactor: &Redactor,
    fields: &ReportFields,
    diagnostics: &DiagnosticsReport,
) -> String {
    let diagnostics = serde_json::to_value(diagnostics)
        .ok()
        .filter(|value| value.to_string().len() <= DIAGNOSTICS_MAX_BYTES);
    serde_json::json!({
        "what_happened": clipped(redactor, &fields.what_happened, TEXT_FIELD_MAX_CHARS),
        "expected": clipped(redactor, &fields.expected, TEXT_FIELD_MAX_CHARS),
        "steps": clipped(redactor, &fields.steps, TEXT_FIELD_MAX_CHARS),
        "frequency": clipped(redactor, &fields.frequency, FREQUENCY_MAX_CHARS),
        "game_context": clipped(redactor, &fields.game_context, TEXT_FIELD_MAX_CHARS),
        "locale": clipped(redactor, &fields.locale, LOCALE_MAX_CHARS),
        "os": clipped(redactor, &diagnostics_field(&diagnostics, "os"), OS_MAX_CHARS),
        "arch": clipped(redactor, &diagnostics_field(&diagnostics, "arch"), ARCH_MAX_CHARS),
        "webview_version": clipped(
            redactor,
            &diagnostics_field(&diagnostics, "webview_version"),
            WEBVIEW_MAX_CHARS
        ),
        "diagnostics": diagnostics,
    })
    .to_string()
}

fn diagnostics_field(diagnostics: &Option<serde_json::Value>, key: &str) -> String {
    diagnostics
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Send a bundle that already exists, and remember the reference that comes back.
fn send_and_record(
    transport: &dyn Transport,
    endpoint: &HubEndpoint,
    parts: &UploadParts,
    include_save: bool,
    reports_path: Option<&Path>,
    submitted_at: String,
) -> Result<UploadReceipt, UploadFailure> {
    let code = hub::submit(transport, endpoint, env!("CARGO_PKG_VERSION"), parts)?;
    let entry = SubmittedReport {
        code: code.clone(),
        submitted_at,
        included_save: include_save,
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
    };
    let recorded = match reports_path {
        Some(path) => history::record(path, &entry)
            .inspect_err(|error| {
                log::error!("[report] report {code} was sent but not recorded: {error}");
            })
            .is_ok(),
        None => false,
    };
    Ok(UploadReceipt { code, recorded })
}

/// Build the report bundle privately and send it to OFMHub, with the player's consent.
///
/// `consent` is the tick on the preview screen, passed through rather than assumed: this is the
/// last place that can refuse to send on the player's behalf. `include_save` is likewise the
/// player's own choice and is never dropped to make a bundle fit — a bundle too large for the
/// server is an error the player answers. The save travels inside the bundle, never as its own
/// part. Screenshots and `.ofm` packages are separate parts, chosen by the player.
///
/// The bundle lives in a private scratch directory only for the length of the request. On any
/// failure the player is offered the GitHub form, which exports a bundle where they choose.
#[tauri::command]
#[expect(
    clippy::too_many_arguments,
    reason = "a Tauri command takes each IPC argument separately; owner=#569"
)]
pub async fn upload_report_bundle(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<StateManager>>,
    save_manager: State<'_, Arc<SaveManagerState>>,
    previous_crash: State<'_, crash::PreviousCrash>,
    report_text: String,
    fields: ReportFields,
    include_save: bool,
    consent: bool,
    screenshot_paths: Vec<String>,
    package_paths: Vec<String>,
) -> Result<UploadReceipt, UploadFailure> {
    let endpoint = upload_preconditions(consent, hub_endpoint())?;
    let state = state.inner().clone();
    let save_manager = save_manager.inner().clone();
    let crash_json = previous_crash.as_json();
    let crash_on_previous_run = previous_crash.0.is_some();

    tauri::async_runtime::spawn_blocking(move || {
        let scratch_root = app_handle
            .path()
            .app_cache_dir()
            .unwrap_or_else(|_| std::env::temp_dir());
        let scratch = std::fs::create_dir_all(&scratch_root)
            .and_then(|()| {
                tempfile::Builder::new()
                    .prefix("ofm-upload-")
                    .tempdir_in(&scratch_root)
            })
            .map_err(|error| {
                log::error!("[report] could not prepare the upload: {error}");
                UploadFailure::key(hub::UPLOAD_PREPARE_FAILED)
            })?;
        let zip_path = scratch.path().join("report.zip");
        write_report_bundle(
            &app_handle,
            &state,
            &save_manager,
            crash_json,
            &zip_path.to_string_lossy(),
            &report_text,
            include_save,
        )
        .map_err(as_upload_error)?;

        let redactor = redactor_for(&app_handle);
        let diagnostics = collect(
            &app_handle,
            &redactor,
            state.get_save_id().is_some(),
            crash_on_previous_run,
            active_save_bytes(&state, &save_manager),
        );
        let parts = UploadParts {
            report_json: report_json(&redactor, &fields, &diagnostics),
            bundle: zip_path,
            screenshots: screenshot_paths.into_iter().map(PathBuf::from).collect(),
            packages: package_paths.into_iter().map(PathBuf::from).collect(),
        };
        let reports_path = app_handle
            .path()
            .app_data_dir()
            .ok()
            .map(|dir| history::reports_file_in(&dir));
        send_and_record(
            UreqTransport::shared(),
            &endpoint,
            &parts,
            include_save,
            reports_path.as_deref(),
            chrono::Utc::now().to_rfc3339(),
        )
        // `scratch` drops here, and the bundle with it.
    })
    .await
    .map_err(|error| {
        log::error!("[report] the upload task did not run: {error}");
        UploadFailure::key(hub::UPLOAD_PREPARE_FAILED)
    })?
}

/// The reports this install has sent, newest first, for the list in Settings → Help.
#[tauri::command]
pub fn list_submitted_reports(app_handle: tauri::AppHandle) -> Vec<SubmittedReport> {
    app_handle
        .path()
        .app_data_dir()
        .map(|dir| history::load(&history::reports_file_in(&dir)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use db::save_manager::SaveManager;
    use domain::manager::Manager;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use std::sync::Mutex;

    fn save_manager_in(dir: &Path) -> SaveManagerState {
        SaveManagerState(Mutex::new(
            SaveManager::init(&dir.join("saves")).expect("a save manager"),
        ))
    }

    const CONFIG_JSON: &str = r#"{"enabled":true,"max_total_bytes":26214400,"max_bundle_bytes":20971520,"max_screenshot_bytes":5242880,"max_package_bytes":20971520,"max_files":6,"accepted_types":{"bundle":["zip"],"screenshots":["png"],"packages":["ofm"]}}"#;

    /// Answers the config read with an enabled server and the post with one canned result, and
    /// counts the posts.
    struct CannedHub {
        post: Result<hub::HubResponse, hub::TransportFailure>,
        posts: std::cell::Cell<usize>,
    }

    impl CannedHub {
        fn answering(status: u16, body: &str) -> Self {
            Self {
                post: Ok(hub::HubResponse {
                    status,
                    body: body.as_bytes().to_vec(),
                    retry_after_seconds: Some(90),
                }),
                posts: std::cell::Cell::new(0),
            }
        }
    }

    impl Transport for CannedHub {
        fn get(&self, _url: &str) -> Result<hub::HubResponse, hub::TransportFailure> {
            Ok(hub::HubResponse {
                status: 200,
                body: CONFIG_JSON.as_bytes().to_vec(),
                retry_after_seconds: None,
            })
        }

        fn post_multipart(
            &self,
            _url: &str,
            _game_version: &str,
            _content_type: &str,
            _body: Vec<u8>,
        ) -> Result<hub::HubResponse, hub::TransportFailure> {
            self.posts.set(self.posts.get() + 1);
            self.post.clone()
        }
    }

    fn test_endpoint() -> HubEndpoint {
        HubEndpoint::from_base_url("https://hub.example.test/api/v1").expect("endpoint")
    }

    fn small_parts(dir: &Path) -> UploadParts {
        let path = dir.join("report.zip");
        std::fs::write(&path, b"PK").expect("zip");
        UploadParts {
            report_json: "{}".to_owned(),
            bundle: path,
            screenshots: Vec::new(),
            packages: Vec::new(),
        }
    }

    fn key(key: &str) -> UploadFailure {
        UploadFailure::key(key)
    }

    /// Given a player who has not ticked the consent box,
    /// when an upload is requested — even with an endpoint configured,
    /// then it is refused before anything is built or sent.
    #[test]
    fn an_upload_without_consent_is_refused() {
        assert_eq!(
            upload_preconditions(false, Some(test_endpoint())),
            Err(key(hub::UPLOAD_CONSENT_REQUIRED))
        );
    }

    /// Given an override that is not a valid https endpoint,
    /// when a consented upload is requested,
    /// then it is refused as not configured, so the player is sent to the GitHub form.
    #[test]
    fn an_upload_with_no_endpoint_is_refused() {
        assert_eq!(
            upload_preconditions(true, None),
            Err(key(hub::UPLOAD_NOT_CONFIGURED))
        );
        assert_eq!(
            upload_preconditions(true, Some(test_endpoint())),
            Ok(test_endpoint())
        );
    }

    /// Given a bundle that could not be written for an upload,
    /// when the error reaches the player,
    /// then it does not talk about a folder they never chose — but a missing save still says so.
    #[test]
    fn an_upload_never_blames_a_folder_the_player_did_not_choose() {
        assert_eq!(
            as_upload_error(REPORT_BUNDLE_FAILED.to_owned()),
            key(hub::UPLOAD_PREPARE_FAILED)
        );
        assert_eq!(
            as_upload_error(REPORT_SAVE_MISSING.to_owned()),
            key(REPORT_SAVE_MISSING)
        );
    }

    /// Given a server that accepts the report,
    /// when it is sent,
    /// then the reference comes back and is recorded in reports.json with the save choice and date.
    #[test]
    fn an_accepted_report_is_recorded_with_its_reference() {
        let dir = tempfile::tempdir().expect("temp dir");
        let reports = history::reports_file_in(dir.path());
        let hub = CannedHub::answering(201, r#"{"reference":"OFM-7K2QX9DM"}"#);

        let receipt = send_and_record(
            &hub,
            &test_endpoint(),
            &small_parts(dir.path()),
            true,
            Some(&reports),
            "2026-10-03T12:00:00+00:00".to_owned(),
        );

        assert_eq!(
            receipt,
            Ok(UploadReceipt {
                code: "OFM-7K2QX9DM".to_owned(),
                recorded: true
            })
        );
        let listed = history::load(&reports);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].code, "OFM-7K2QX9DM");
        assert!(listed[0].included_save);
        assert_eq!(listed[0].submitted_at, "2026-10-03T12:00:00+00:00");
        assert_eq!(listed[0].app_version, env!("CARGO_PKG_VERSION"));
    }

    /// Given a server that rate-limits the report,
    /// when it is sent,
    /// then the player gets the translated reason with the wait, and nothing is recorded.
    #[test]
    fn a_refused_report_records_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let reports = history::reports_file_in(dir.path());
        let hub = CannedHub::answering(429, r#"{"message":"Slow down.","code":"rate_limited"}"#);

        let receipt = send_and_record(
            &hub,
            &test_endpoint(),
            &small_parts(dir.path()),
            false,
            Some(&reports),
            "2026-10-03T12:00:00+00:00".to_owned(),
        );

        assert_eq!(
            receipt,
            Err(UploadFailure {
                key: hub::UPLOAD_RATE_LIMITED.to_owned(),
                retry_after_seconds: Some(90)
            })
        );
        assert!(history::load(&reports).is_empty());
        assert_eq!(hub.posts.get(), 1);
    }

    /// Given a server that accepts the report but a reports file that cannot be written,
    /// when it is sent,
    /// then the upload still succeeds with its reference, marked as not recorded so the player is
    /// told to note it down.
    #[test]
    fn an_accepted_report_that_cannot_be_recorded_still_returns_its_reference() {
        let dir = tempfile::tempdir().expect("temp dir");
        // A directory where the file should be: every write to it fails.
        let reports = history::reports_file_in(dir.path());
        std::fs::create_dir_all(&reports).expect("blocking dir");
        let hub = CannedHub::answering(201, r#"{"reference":"OFM-7K2QX9DM"}"#);

        let receipt = send_and_record(
            &hub,
            &test_endpoint(),
            &small_parts(dir.path()),
            false,
            Some(&reports),
            "2026-10-03T12:00:00+00:00".to_owned(),
        );

        assert_eq!(
            receipt,
            Ok(UploadReceipt {
                code: "OFM-7K2QX9DM".to_owned(),
                recorded: false
            })
        );
    }

    fn sample_diagnostics() -> DiagnosticsReport {
        DiagnosticsReport {
            app_version: "0.3.0".to_owned(),
            os: "linux".to_owned(),
            arch: "x86_64".to_owned(),
            webview_version: "2.40".to_owned(),
            log_directory: "~/logs".to_owned(),
            crash_on_previous_run: false,
            has_active_save: true,
            log_files: Vec::new(),
            save_bytes: Some(10),
        }
    }

    fn sample_fields() -> ReportFields {
        ReportFields {
            what_happened: "it died loading /home/srobot/saves/a.db".to_owned(),
            expected: "to open".to_owned(),
            steps: "load".to_owned(),
            frequency: "always".to_owned(),
            game_context: "season 2".to_owned(),
            locale: "en".to_owned(),
        }
    }

    /// Given words that mention the player's home directory,
    /// when the `report` part is built,
    /// then they are redacted like everything else that leaves the machine.
    #[test]
    fn the_report_part_is_redacted() {
        let redactor = Redactor::new(Some("/home/srobot"), Some("srobot"), None);

        let json: serde_json::Value = serde_json::from_str(&report_json(
            &redactor,
            &sample_fields(),
            &sample_diagnostics(),
        ))
        .expect("json");

        assert_eq!(json["what_happened"], "it died loading ~/saves/a.db");
        assert_eq!(json["frequency"], "always");
        assert_eq!(json["os"], "linux");
        assert_eq!(json["arch"], "x86_64");
        assert_eq!(json["webview_version"], "2.40");
        assert_eq!(json["diagnostics"]["has_active_save"], true);
    }

    /// Given fields longer than the endpoint accepts,
    /// when the `report` part is built,
    /// then each is cut to its limit in characters, not bytes.
    #[test]
    fn the_report_part_respects_the_field_limits() {
        let redactor = Redactor::new(None, None, None);
        let mut fields = sample_fields();
        fields.what_happened = "é".repeat(TEXT_FIELD_MAX_CHARS + 5);
        fields.frequency = "f".repeat(FREQUENCY_MAX_CHARS + 5);
        fields.locale = "l".repeat(LOCALE_MAX_CHARS + 5);

        let json: serde_json::Value =
            serde_json::from_str(&report_json(&redactor, &fields, &sample_diagnostics()))
                .expect("json");

        assert_eq!(
            json["what_happened"].as_str().unwrap().chars().count(),
            TEXT_FIELD_MAX_CHARS
        );
        assert_eq!(
            json["frequency"].as_str().unwrap().chars().count(),
            FREQUENCY_MAX_CHARS
        );
        assert_eq!(
            json["locale"].as_str().unwrap().chars().count(),
            LOCALE_MAX_CHARS
        );
    }

    /// Given diagnostics too big for the endpoint's `diagnostics` limit,
    /// when the `report` part is built,
    /// then the object is left out rather than failing the whole report.
    #[test]
    fn oversized_diagnostics_are_left_out_not_sent() {
        let redactor = Redactor::new(None, None, None);
        let mut diagnostics = sample_diagnostics();
        diagnostics.log_directory = "x".repeat(DIAGNOSTICS_MAX_BYTES + 1);

        let json: serde_json::Value =
            serde_json::from_str(&report_json(&redactor, &sample_fields(), &diagnostics))
                .expect("json");

        assert!(json["diagnostics"].is_null());
        assert_eq!(json["os"], "");
    }

    #[test]
    fn redacts_every_field_the_url_will_carry() {
        // The prefilled issue URL used to carry what the player typed verbatim, while the copy of
        // the same words inside the zip was redacted. A path in the description then reached
        // GitHub and the browser's history, and neither gives it back.
        let mut redactor = Redactor::new(Some("/home/srobot"), Some("srobot"), None);
        redactor.add_path("/mnt/diagnostics-data/openfootmanager/logs");

        let out = redact_all(
            &redactor,
            &[
                "it died loading /home/srobot/saves/a.db".to_owned(),
                "srobot expected it to open".to_owned(),
                "logs at /mnt/diagnostics-data/openfootmanager/logs/today.log".to_owned(),
            ],
        );

        assert_eq!(out[0], "it died loading ~/saves/a.db");
        assert!(!out[1].contains("srobot"), "{out:?}");
        assert_eq!(out[2], "logs at ~/today.log");
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

    #[test]
    fn overlapping_exports_keep_their_own_save_copies() {
        let dir = tempfile::tempdir().expect("temp dir");
        let state = StateManager::new();
        let manager = save_manager_in(dir.path());
        let game = Game::new(
            GameClock::new(Utc.with_ymd_and_hms(2026, 9, 29, 0, 0, 0).unwrap()),
            Manager::new(
                "manager-1".to_owned(),
                "Test".to_owned(),
                "Manager".to_owned(),
                "1980-01-01".to_owned(),
                "England".to_owned(),
            ),
            vec![],
            vec![],
            vec![],
            vec![],
        );
        let save_id = manager
            .0
            .lock()
            .unwrap()
            .create_save(&game, "Career")
            .expect("save");
        state.set_save_id(save_id);
        let scratch_root = dir.path().join("cache");
        let first = copy_active_save(&state, &manager, &scratch_root)
            .expect("first copy")
            .expect("active save");
        let second = copy_active_save(&state, &manager, &scratch_root)
            .expect("second copy")
            .expect("active save");
        assert_ne!(first.dir, second.dir);

        let redactor = Redactor::new(None, None, None);
        let first_bundle = bundle::write_bundle(
            &BundleInputs {
                log_dir: dir.path(),
                diagnostics_json: "{}",
                report_text: "Report",
                crash_json: None,
                save_path: Some(&first.file),
            },
            &dir.path().join("first.zip"),
            &redactor,
        )
        .expect("first export");
        assert!(first_bundle.included_save);
        drop(first);

        let second_bundle = bundle::write_bundle(
            &BundleInputs {
                log_dir: dir.path(),
                diagnostics_json: "{}",
                report_text: "Report",
                crash_json: None,
                save_path: Some(&second.file),
            },
            &dir.path().join("second.zip"),
            &redactor,
        )
        .expect("second export after first copy is cleaned up");
        assert!(second_bundle.included_save);
        assert!(second.file.exists());
        let second_dir = second.dir.clone();
        drop(second);
        assert!(!second_dir.exists());
    }
}

/// Redact free text the same way the bundle does, for the parts that leave by another route.
///
/// The prefilled issue URL carried the player's own words verbatim while the copy inside the zip
/// was redacted — so a path they pasted into the description reached GitHub and their browser
/// history, and neither of those is somewhere it can be taken back from. One call for the whole
/// set rather than one per field: the redactor reads the environment on construction, and doing
/// that four times to answer one screen is waste.
#[tauri::command]
pub fn redact_report_fields(app_handle: tauri::AppHandle, values: Vec<String>) -> Vec<String> {
    redact_all(&redactor_for(&app_handle), &values)
}

/// Split from the command so it can be tested against a redactor built for the test.
///
/// `from_environment` reads the real machine, and a test that set `HOME` to assert on the result
/// would be mutating process-wide state under a parallel test runner.
fn redact_all(redactor: &Redactor, values: &[String]) -> Vec<String> {
    values.iter().map(|value| redactor.apply(value)).collect()
}

/// A dated default for the save dialog, so a second report does not overwrite the first.
#[tauri::command]
pub fn suggested_report_file_name() -> String {
    bundle::suggested_file_name(chrono::Utc::now())
}
