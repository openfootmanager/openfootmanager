//! The reference codes this install has been given, in `reports.json` in the app data directory.
//!
//! Its own file on purpose. `AppSettings` drops unknown fields when it deserialises, so a code
//! stored there would vanish the first time an older build rewrote the settings; and the log
//! rotates, so a code written only there is gone within a few sessions. The player may need the
//! code months later to ask the maintainer to delete what they sent.
//!
//! No status is tracked. The relay has no lookup endpoint by design — the code is an identifier
//! for a conversation with the maintainer, not a credential — so there is nothing to poll.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One report the relay accepted.
///
/// Every field carries `#[serde(default)]`: a file written by a later build with fields this one
/// does not know must still load, and a missing field must not cost the player the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmittedReport {
    #[serde(default)]
    pub code: String,
    /// RFC 3339, UTC.
    #[serde(default)]
    pub submitted_at: String,
    /// Whether the player chose to send their save with it — the one part they may want removed.
    #[serde(default)]
    pub included_save: bool,
    #[serde(default)]
    pub app_version: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ReportsFile {
    #[serde(default)]
    reports: Vec<SubmittedReport>,
}

pub fn reports_file_in(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("reports.json")
}

/// Every recorded report, newest first. A missing or unreadable file is an empty list.
pub fn load(path: &Path) -> Vec<SubmittedReport> {
    match read(path) {
        Ok(file) => {
            let mut reports = file.reports;
            // Stored oldest first, which is the order they were appended in.
            reports.reverse();
            reports
        }
        Err(Unreadable::Missing) => Vec::new(),
        Err(Unreadable::Io(error)) => {
            log::warn!("[report] could not read {}: {error}", path.display());
            Vec::new()
        }
        Err(Unreadable::Corrupt(error)) => {
            log::warn!("[report] {} is unreadable: {error}", path.display());
            Vec::new()
        }
    }
}

/// Add one report to the file.
///
/// A file that will not parse is renamed aside rather than overwritten. The new code must be kept
/// — the relay already has that report — but the old codes inside a damaged file are still the
/// player's, and recoverable by hand from the copy.
pub fn record(path: &Path, report: &SubmittedReport) -> std::io::Result<()> {
    let mut file = match read(path) {
        Ok(file) => file,
        Err(Unreadable::Missing) => ReportsFile::default(),
        // Not a damaged file but one this process cannot read right now. Writing a fresh list
        // over it would be the one way to lose codes that are still intact.
        Err(Unreadable::Io(error)) => return Err(error),
        Err(Unreadable::Corrupt(error)) => {
            let aside = set_aside_name(path);
            log::warn!(
                "[report] {} is unreadable ({error}); moving it to {}",
                path.display(),
                aside.display()
            );
            std::fs::rename(path, &aside)?;
            ReportsFile::default()
        }
    };
    file.reports.push(report.clone());
    write(path, &file)
}

enum Unreadable {
    Missing,
    Io(std::io::Error),
    Corrupt(String),
}

fn read(path: &Path) -> Result<ReportsFile, Unreadable> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(Unreadable::Missing);
        }
        Err(error) => return Err(Unreadable::Io(error)),
    };
    serde_json::from_str(&raw).map_err(|error| Unreadable::Corrupt(error.to_string()))
}

fn set_aside_name(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_owned();
    name.push(format!(
        ".corrupt-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
    ));
    path.with_file_name(name)
}

/// Beside the target and then renamed, as `crash::write_record` does: a write cut short must not
/// leave a truncated list where the player's codes used to be.
fn write(path: &Path, file: &ReportsFile) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(file)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let mut scratch_name = path.file_name().unwrap_or_default().to_owned();
    scratch_name.push(".part");
    let scratch = path.with_file_name(scratch_name);
    std::fs::write(&scratch, json)?;
    std::fs::rename(&scratch, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&scratch);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(code: &str, submitted_at: &str) -> SubmittedReport {
        SubmittedReport {
            code: code.to_owned(),
            submitted_at: submitted_at.to_owned(),
            included_save: false,
            app_version: "0.3.0".to_owned(),
        }
    }

    /// Given an install that has never sent a report,
    /// when its reports are listed,
    /// then the list is empty and no file is created.
    #[test]
    fn an_install_that_never_reported_lists_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = reports_file_in(dir.path());

        assert!(load(&path).is_empty());
        assert!(!path.exists());
    }

    /// Given two reports recorded one after the other,
    /// when the reports are listed,
    /// then both are there, newest first.
    #[test]
    fn recorded_reports_are_listed_newest_first() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = reports_file_in(dir.path());

        record(&path, &report("AAAAAAAA", "2026-10-01T10:00:00+00:00")).expect("first");
        record(&path, &report("BBBBBBBB", "2026-10-02T10:00:00+00:00")).expect("second");

        let codes: Vec<String> = load(&path).into_iter().map(|r| r.code).collect();
        assert_eq!(codes, vec!["BBBBBBBB", "AAAAAAAA"]);
    }

    /// Given a file written by another build, with a field this one does not know and one it
    /// expects missing,
    /// when the reports are listed,
    /// then the entry still loads, with the missing field defaulted.
    #[test]
    fn a_file_from_another_build_still_loads() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = reports_file_in(dir.path());
        std::fs::write(
            &path,
            r#"{"reports":[{"code":"7K2M9Q4R","submitted_at":"2026-10-01T10:00:00+00:00","future_field":1}]}"#,
        )
        .expect("write");

        let listed = load(&path);

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].code, "7K2M9Q4R");
        assert!(!listed[0].included_save);
        assert_eq!(listed[0].app_version, "");
    }

    /// Given a reports file that does not parse,
    /// when a new report is recorded,
    /// then the new code is kept, and the unreadable file is set aside rather than overwritten,
    /// so the codes inside it can still be recovered by hand.
    #[test]
    fn a_corrupt_file_is_set_aside_and_does_not_block_a_new_code() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = reports_file_in(dir.path());
        std::fs::write(&path, "{ half a file").expect("write");

        record(&path, &report("7K2M9Q4R", "2026-10-03T10:00:00+00:00")).expect("record");

        let codes: Vec<String> = load(&path).into_iter().map(|r| r.code).collect();
        assert_eq!(codes, vec!["7K2M9Q4R"]);
        let set_aside: Vec<String> = std::fs::read_dir(dir.path())
            .expect("dir")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("reports.json.corrupt"))
            .collect();
        assert_eq!(set_aside.len(), 1, "{set_aside:?}");
        assert_eq!(
            std::fs::read_to_string(dir.path().join(&set_aside[0])).expect("read"),
            "{ half a file"
        );
    }

    /// Given an app data directory that does not exist yet,
    /// when the first report is recorded,
    /// then the directory is created and no scratch file is left behind.
    #[test]
    fn the_first_record_creates_the_file_and_leaves_no_scratch() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = reports_file_in(&dir.path().join("fresh"));

        record(&path, &report("7K2M9Q4R", "2026-10-03T10:00:00+00:00")).expect("record");

        assert_eq!(load(&path).len(), 1);
        let names: Vec<String> = std::fs::read_dir(path.parent().expect("parent"))
            .expect("dir")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["reports.json"]);
    }
}
