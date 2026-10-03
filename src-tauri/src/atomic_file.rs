//! Writing a small JSON file so that it is either the old version or the new one, never half.
//!
//! `fs::write` truncates first, so a process that dies partway through — a crash, a full disk, a
//! machine switched off — leaves a file that exists and parses as nothing. Both callers keep
//! something the player cannot get back otherwise (`last-crash.json`, `reports.json`), so both
//! write beside the target and rename over it.

use std::path::Path;

use serde::Serialize;

/// Serialise `value` as pretty JSON and move it into place at `path` in one rename.
pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(value)
        .map_err(|error| std::io::Error::other(error.to_string()))?;

    let mut scratch_name = path.file_name().unwrap_or_default().to_owned();
    scratch_name.push(".part");
    let scratch = path.with_file_name(scratch_name);
    std::fs::write(&scratch, json)?;
    std::fs::rename(&scratch, path).inspect_err(|_| {
        // Best effort: a stray `.part` is harmless, but there is no reason to leave one.
        let _ = std::fs::remove_file(&scratch);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given a destination that does not exist and whose directory does not either,
    /// when a value is written,
    /// then the file holds exactly that JSON and no scratch file is left beside it.
    #[test]
    fn writes_the_value_and_leaves_no_scratch_behind() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("nested").join("value.json");

        write_json(&path, &serde_json::json!({ "code": "7K2M9Q4R" })).expect("write");

        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("json");
        assert_eq!(parsed, serde_json::json!({ "code": "7K2M9Q4R" }));
        let names: Vec<_> = std::fs::read_dir(path.parent().expect("parent"))
            .expect("dir")
            .flatten()
            .map(|entry| entry.file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("value.json")]);
    }

    /// Given an existing file and a write that cannot complete,
    /// when the write fails,
    /// then the existing file is untouched.
    #[test]
    fn a_failed_write_leaves_the_previous_file_intact() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("value.json");
        std::fs::write(&path, "previous").expect("seed");
        // A directory where the scratch file goes makes the write fail before any rename.
        std::fs::create_dir(dir.path().join("value.json.part")).expect("blocking dir");

        assert!(write_json(&path, &serde_json::json!({})).is_err());
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "previous");
    }
}
