//! The report as it was first built, kept so that sending again sends the same bytes.
//!
//! The server recognises a byte-identical retry (same fields, same files) and answers it with the
//! first report's reference. Rebuilding the bundle would not give that: the log has gained lines
//! about the failed attempt and the zip entries carry the time they were written.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::hub::{UploadFailure, UPLOAD_ALREADY_SENDING, UPLOAD_PREPARE_FAILED};

struct Held {
    key: String,
    report_json: String,
    bundle: PathBuf,
    // Dropping this removes the bundle with it.
    _scratch: tempfile::TempDir,
}

/// What was built: the `report` part and the path of the bundle.
pub type Built = (String, PathBuf);

#[derive(Default)]
struct Inner {
    held: Option<Held>,
    in_flight: bool,
    /// The dialog went away while the upload was running: drop the build when it settles.
    discard_when_settled: bool,
}

#[derive(Default)]
pub struct PreparedUploads(Mutex<Inner>);

const SCRATCH_PREFIX: &str = "ofm-upload-";

impl PreparedUploads {
    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Starts an upload: the report held for `key`, or the one `prepare` builds in a fresh
    /// scratch directory. A different key replaces what was held; a failed `prepare` holds
    /// nothing. Pair with [`PreparedUploads::settle`].
    pub fn begin(
        &self,
        key: &str,
        scratch_root: &Path,
        prepare: impl FnOnce(&Path) -> Result<Built, UploadFailure>,
    ) -> Result<Built, UploadFailure> {
        let mut inner = self.inner();
        if inner.in_flight {
            return Err(UploadFailure::key(UPLOAD_ALREADY_SENDING));
        }
        inner.in_flight = true;
        inner.discard_when_settled = false;
        if let Some(existing) = inner.held.as_ref().filter(|existing| existing.key == key) {
            return Ok((existing.report_json.clone(), existing.bundle.clone()));
        }
        let scratch = std::fs::create_dir_all(scratch_root)
            .and_then(|()| {
                tempfile::Builder::new()
                    .prefix(SCRATCH_PREFIX)
                    .tempdir_in(scratch_root)
            })
            .map_err(|error| {
                log::error!("[report] could not prepare the upload: {error}");
                inner.in_flight = false;
                UploadFailure::key(UPLOAD_PREPARE_FAILED)
            })?;
        let built = prepare(scratch.path()).inspect_err(|_| inner.in_flight = false)?;
        inner.held = Some(Held {
            key: key.to_owned(),
            report_json: built.0.clone(),
            bundle: built.1.clone(),
            _scratch: scratch,
        });
        Ok(built)
    }

    /// The upload ended. The build is kept only when it can still be resent unchanged (an
    /// unknown outcome) and the dialog has not gone away meanwhile.
    pub fn settle(&self, keep_for_resend: bool) {
        let mut inner = self.inner();
        inner.in_flight = false;
        if !keep_for_resend || inner.discard_when_settled {
            inner.held = None;
        }
        inner.discard_when_settled = false;
    }

    /// The dialog is gone. A running upload has already read what it needs, but the build is
    /// removed as soon as it settles; otherwise it is removed now.
    pub fn discard(&self) {
        let mut inner = self.inner();
        if inner.in_flight {
            inner.discard_when_settled = true;
        } else {
            inner.held = None;
        }
    }
}

/// Removes `ofm-upload-*` directories left behind by a run that ended before it could clean up:
/// they can hold a copy of the player's save.
pub fn sweep_stale_uploads(scratch_root: &Path) {
    let Ok(entries) = std::fs::read_dir(scratch_root) else {
        return;
    };
    for entry in entries.flatten() {
        let is_stale_upload = entry
            .file_name()
            .to_string_lossy()
            .starts_with(SCRATCH_PREFIX)
            && entry.file_type().is_ok_and(|kind| kind.is_dir());
        if !is_stale_upload {
            continue;
        }
        if let Err(error) = std::fs::remove_dir_all(entry.path()) {
            log::warn!("[report] could not remove a stale upload folder: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn build_in<'a>(
        counter: &'a Cell<u32>,
        json: &str,
    ) -> impl FnOnce(&Path) -> Result<Built, UploadFailure> + 'a {
        let json = json.to_owned();
        move |dir: &Path| {
            counter.set(counter.get() + 1);
            let bundle = dir.join("report.zip");
            std::fs::write(&bundle, format!("zip built {} times", counter.get())).expect("write");
            Ok((json, bundle))
        }
    }

    /// Given a report that was built for some inputs,
    /// when the same inputs are sent again,
    /// then nothing is rebuilt: the same `report` part and the same bundle bytes come back.
    #[test]
    fn the_same_inputs_resend_the_same_bytes() {
        let root = tempfile::tempdir().expect("root");
        let held = PreparedUploads::default();
        let builds = Cell::new(0);

        let first = held
            .begin("k", root.path(), build_in(&builds, "{}"))
            .expect("first");
        held.settle(true);
        let second = held
            .begin("k", root.path(), build_in(&builds, "{}"))
            .expect("second");

        assert_eq!(builds.get(), 1);
        assert_eq!(first, second);
        assert_eq!(
            std::fs::read(&second.1).expect("read"),
            b"zip built 1 times"
        );
    }

    /// Given a report built for some inputs,
    /// when the player changes what is sent,
    /// then a new report is built and the old bundle is removed.
    #[test]
    fn different_inputs_build_a_new_report_and_drop_the_old_bundle() {
        let root = tempfile::tempdir().expect("root");
        let held = PreparedUploads::default();
        let builds = Cell::new(0);
        let (_, old_bundle) = held
            .begin("a", root.path(), build_in(&builds, "{}"))
            .expect("a");
        held.settle(true);

        let (_, new_bundle) = held
            .begin("b", root.path(), build_in(&builds, "{}"))
            .expect("b");

        assert_eq!(builds.get(), 2);
        assert!(!old_bundle.exists());
        assert!(new_bundle.exists());
    }

    /// Given an accepted report,
    /// when the player sends once more with the same inputs,
    /// then it is built afresh, as a new report.
    #[test]
    fn an_accepted_report_is_forgotten() {
        let root = tempfile::tempdir().expect("root");
        let held = PreparedUploads::default();
        let builds = Cell::new(0);
        held.begin("k", root.path(), build_in(&builds, "{}"))
            .expect("first");

        held.settle(false);
        held.begin("k", root.path(), build_in(&builds, "{}"))
            .expect("again");

        assert_eq!(builds.get(), 2);
    }

    /// Given a build that fails,
    /// when it is tried and then tried again,
    /// then nothing was held in between, and the retry builds.
    #[test]
    fn a_failed_build_holds_nothing() {
        let root = tempfile::tempdir().expect("root");
        let held = PreparedUploads::default();
        let failed = held.begin("k", root.path(), |_| {
            Err(UploadFailure::key(UPLOAD_PREPARE_FAILED))
        });
        let builds = Cell::new(0);

        let retried = held.begin("k", root.path(), build_in(&builds, "{}"));

        assert!(failed.is_err());
        assert!(retried.is_ok());
        assert_eq!(builds.get(), 1);
    }

    /// Given a held build,
    /// when the dialog is torn down while nothing is running,
    /// then the build — and the save copy inside it — is removed at once.
    #[test]
    fn discarding_an_idle_build_removes_it() {
        let root = tempfile::tempdir().expect("root");
        let held = PreparedUploads::default();
        let builds = Cell::new(0);
        let (_, bundle) = held
            .begin("k", root.path(), build_in(&builds, "{}"))
            .expect("built");
        held.settle(true);

        held.discard();

        assert!(!bundle.exists());
    }

    /// Given an upload that is still running,
    /// when the dialog is torn down,
    /// then the build stays until the upload settles, and is then removed even though the
    /// outcome was unknown.
    #[test]
    fn discarding_during_an_upload_waits_for_it_to_settle() {
        let root = tempfile::tempdir().expect("root");
        let held = PreparedUploads::default();
        let builds = Cell::new(0);
        let (_, bundle) = held
            .begin("k", root.path(), build_in(&builds, "{}"))
            .expect("built");

        held.discard();
        assert!(bundle.exists(), "the running upload still needs it");
        held.settle(true);

        assert!(!bundle.exists());
    }

    /// Given an upload whose outcome is unknown and a dialog that is still open,
    /// when it settles,
    /// then the build is kept for a byte-identical resend.
    #[test]
    fn an_unknown_outcome_keeps_the_build_while_the_dialog_is_open() {
        let root = tempfile::tempdir().expect("root");
        let held = PreparedUploads::default();
        let builds = Cell::new(0);
        let (_, bundle) = held
            .begin("k", root.path(), build_in(&builds, "{}"))
            .expect("built");

        held.settle(true);

        assert!(bundle.exists());
    }

    /// Given scratch folders from earlier runs and unrelated entries beside them,
    /// when the startup sweep runs,
    /// then only the `ofm-upload-` folders are removed.
    #[test]
    fn the_startup_sweep_removes_only_stale_upload_folders() {
        let root = tempfile::tempdir().expect("root");
        let stale = root.path().join("ofm-upload-abc123");
        std::fs::create_dir_all(&stale).expect("stale");
        std::fs::write(stale.join("report.zip"), b"PK").expect("zip");
        let other_dir = root.path().join("webview-cache");
        std::fs::create_dir_all(&other_dir).expect("other dir");
        let lookalike_file = root.path().join("ofm-upload-notes.txt");
        std::fs::write(&lookalike_file, b"keep").expect("file");

        sweep_stale_uploads(root.path());

        assert!(!stale.exists());
        assert!(other_dir.exists());
        assert!(lookalike_file.exists());
    }

    /// Given a cache directory that does not exist,
    /// when the startup sweep runs,
    /// then nothing happens and nothing is created.
    #[test]
    fn the_sweep_ignores_a_missing_directory() {
        let root = tempfile::tempdir().expect("root");
        let missing = root.path().join("nope");

        sweep_stale_uploads(&missing);

        assert!(!missing.exists());
    }

    /// Given an upload that is still running (its dialog closed, a new one opened),
    /// when a second upload begins,
    /// then it is refused rather than sharing the first one's build, and the first one's build
    /// is neither replaced nor discarded by it — and once the first settles a new one may begin.
    #[test]
    fn a_second_upload_is_refused_while_one_is_in_flight() {
        let root = tempfile::tempdir().expect("root");
        let held = PreparedUploads::default();
        let builds = Cell::new(0);
        let (_, first) = held
            .begin("a", root.path(), build_in(&builds, "{}"))
            .expect("first");
        held.discard();

        let second = held.begin("b", root.path(), build_in(&builds, "{}"));

        assert_eq!(second, Err(UploadFailure::key(UPLOAD_ALREADY_SENDING)));
        assert_eq!(builds.get(), 1, "the second build was never made");
        assert!(first.exists(), "the first upload still needs its bundle");
        held.settle(true);
        assert!(
            !first.exists(),
            "the dialog had gone away, so it is removed on settling"
        );
        assert!(held
            .begin("b", root.path(), build_in(&builds, "{}"))
            .is_ok());
    }
}
