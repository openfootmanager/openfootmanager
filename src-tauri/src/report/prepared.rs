//! The report as it was first built, kept so that sending again sends the same bytes.
//!
//! The server recognises a byte-identical retry (same fields, same files) and answers it with the
//! first report's reference. Rebuilding the bundle would not give that: the log has gained lines
//! about the failed attempt and the zip entries carry the time they were written.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::hub::{UploadFailure, UPLOAD_PREPARE_FAILED};

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
pub struct PreparedUploads(Mutex<Option<Held>>);

impl PreparedUploads {
    /// The report held for `key`, or the one `prepare` builds in a fresh scratch directory.
    /// A different key replaces what was held; a failed `prepare` holds nothing.
    pub fn get_or_prepare(
        &self,
        key: &str,
        scratch_root: &Path,
        prepare: impl FnOnce(&Path) -> Result<Built, UploadFailure>,
    ) -> Result<Built, UploadFailure> {
        let mut held = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = held.as_ref().filter(|existing| existing.key == key) {
            return Ok((existing.report_json.clone(), existing.bundle.clone()));
        }
        let scratch = std::fs::create_dir_all(scratch_root)
            .and_then(|()| {
                tempfile::Builder::new()
                    .prefix("ofm-upload-")
                    .tempdir_in(scratch_root)
            })
            .map_err(|error| {
                log::error!("[report] could not prepare the upload: {error}");
                UploadFailure::key(UPLOAD_PREPARE_FAILED)
            })?;
        let (report_json, bundle) = prepare(scratch.path())?;
        *held = Some(Held {
            key: key.to_owned(),
            report_json: report_json.clone(),
            bundle: bundle.clone(),
            _scratch: scratch,
        });
        Ok((report_json, bundle))
    }

    /// Once a report has been accepted, the next one is a new report.
    pub fn forget(&self) {
        *self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
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
            .get_or_prepare("k", root.path(), build_in(&builds, "{}"))
            .expect("first");
        let second = held
            .get_or_prepare("k", root.path(), build_in(&builds, "{}"))
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
            .get_or_prepare("a", root.path(), build_in(&builds, "{}"))
            .expect("a");

        let (_, new_bundle) = held
            .get_or_prepare("b", root.path(), build_in(&builds, "{}"))
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
        held.get_or_prepare("k", root.path(), build_in(&builds, "{}"))
            .expect("first");

        held.forget();
        held.get_or_prepare("k", root.path(), build_in(&builds, "{}"))
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
        let failed = held.get_or_prepare("k", root.path(), |_| {
            Err(UploadFailure::key(UPLOAD_PREPARE_FAILED))
        });
        let builds = Cell::new(0);

        let retried = held.get_or_prepare("k", root.path(), build_in(&builds, "{}"));

        assert!(failed.is_err());
        assert!(retried.is_ok());
        assert_eq!(builds.get(), 1);
    }
}
