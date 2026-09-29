//! Packing the evidence into one file the player can hand over.
//!
//! The bundle is deliberately small and boring: a diagnostics summary, the most recent logs with
//! the person redacted out of them, the last crash if there was one, and the save only when the
//! player ticked the box. It is written to a path they chose. Nothing here sends anything.

use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

use zip::write::SimpleFileOptions;

use super::redact::Redactor;

/// How many log files a bundle carries.
///
/// Rotation keeps five (`KeepSome(5)`), and a report wants the session that broke plus enough
/// before it to show the run-up. Three covers that; the older two are almost always a different
/// week's play.
const MAX_LOG_FILES: usize = 3;

/// Ceiling on raw log text, before compression.
///
/// Logs are the only part of the bundle that can grow without the player choosing it. The save
/// file is not capped here, because attaching it is an explicit decision made on the preview.
///
/// This has to clear `MAX_LOG_FILES` times the rotation size or the file count is a fiction. It
/// was 8 MiB against a 5 MB rotation, which admitted a second file only while the newest was under
/// ~3.4 MB and made a third arithmetically impossible — so the "three files" above was never true
/// and the common case was one. 16 MiB leaves room for all three.
const MAX_LOG_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogCandidate {
    pub name: String,
    pub bytes: u64,
    pub modified: SystemTime,
}

/// What went into a bundle, for the preview screen and for the log.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BundleSummary {
    pub path: String,
    pub bytes: u64,
    pub log_files: Vec<String>,
    pub included_save: bool,
    pub included_crash: bool,
}

/// Newest first, capped by count and by total size.
///
/// Pure, and separated from the directory walk on purpose: the interesting behaviour is which
/// files get dropped and in what order, and testing that through real files would mean writing
/// megabytes to disk and manipulating timestamps to assert on a sort.
pub(crate) fn choose_logs(
    mut candidates: Vec<LogCandidate>,
    max_files: usize,
    max_bytes: u64,
) -> Vec<LogCandidate> {
    // Newest first; name breaks ties so the result is stable when two files share a timestamp,
    // which they do on filesystems with coarse mtime resolution.
    candidates.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| b.name.cmp(&a.name))
    });

    let mut chosen = Vec::new();
    let mut total = 0u64;
    for candidate in candidates.into_iter().take(max_files) {
        // The newest file is kept whatever its size: a bundle whose only log is missing because
        // that one session was long would be a report about nothing.
        if chosen.is_empty() || total.saturating_add(candidate.bytes) <= max_bytes {
            total = total.saturating_add(candidate.bytes);
            chosen.push(candidate);
        }
    }
    chosen
}

/// The logs a bundle written now would carry, newest first — without writing anything.
///
/// The preview and the export both go through this, so the screen cannot name one set of files
/// and the zip contain another. Sizes are a snapshot: the live log keeps growing while the player
/// reads the screen, which is why the preview shows what is there rather than a promise.
pub fn planned_logs(log_dir: &Path) -> Vec<LogCandidate> {
    choose_logs(log_candidates(log_dir), MAX_LOG_FILES, MAX_LOG_BYTES)
}

fn log_candidates(log_dir: &Path) -> Vec<LogCandidate> {
    let Ok(entries) = std::fs::read_dir(log_dir) else {
        // No log directory is not a reason to fail the whole bundle — the diagnostics summary and
        // the crash record are still worth having, and a first-launch crash has no logs yet.
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "log") {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            Some(LogCandidate {
                name: path.file_name()?.to_string_lossy().into_owned(),
                bytes: metadata.len(),
                modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            })
        })
        .collect()
}

pub struct BundleInputs<'a> {
    pub log_dir: &'a Path,
    pub diagnostics_json: &'a str,
    /// What the player actually wrote.
    ///
    /// The GitHub URL has a length limit, and a description in a script that percent-encodes to
    /// several bytes per character reaches it quickly — 1 000 Chinese characters encode to over
    /// 9 000. What does not fit is trimmed from the link, so unless it is also written here it is
    /// gone the moment the modal closes. This file is the copy that survives.
    pub report_text: &'a str,
    pub crash_json: Option<&'a str>,
    pub save_path: Option<&'a Path>,
}

/// Write the bundle to `output`, returning what went in.
pub fn write_bundle(
    inputs: &BundleInputs<'_>,
    output: &Path,
    redactor: &Redactor,
) -> std::io::Result<BundleSummary> {
    // `File::create` truncates the target before a single byte is written, so a failure partway
    // through — a full disk while copying the save, a drive unplugged — used to leave a
    // zero-length or half-written file with exactly the name the player chose. They would find it,
    // assume it was the report, and attach it. Build beside the target and rename on success:
    // the name only ever appears once the file behind it is complete.
    match write_bundle_inner(inputs, output, redactor) {
        Ok(summary) => Ok(summary),
        Err(error) => {
            let _ = std::fs::remove_file(scratch_path(output));
            Err(error)
        }
    }
}

/// Where the bundle is assembled before it takes the name the player chose.
fn scratch_path(output: &Path) -> std::path::PathBuf {
    let mut name = output.file_name().unwrap_or_default().to_owned();
    name.push(".part");
    output.with_file_name(name)
}

fn write_bundle_inner(
    inputs: &BundleInputs<'_>,
    output: &Path,
    redactor: &Redactor,
) -> std::io::Result<BundleSummary> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let scratch = scratch_path(output);
    let file = std::fs::File::create(&scratch)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let to_io = |e: zip::result::ZipError| std::io::Error::other(e.to_string());

    zip.start_file("report.md", options).map_err(to_io)?;
    zip.write_all(redactor.apply(inputs.report_text).as_bytes())?;

    zip.start_file("diagnostics.json", options).map_err(to_io)?;
    // The summary is built from values we chose, but it carries paths, so it goes through the
    // same redaction as everything else rather than being trusted because we wrote it.
    zip.write_all(redactor.apply(inputs.diagnostics_json).as_bytes())?;

    let mut included_crash = false;
    if let Some(crash) = inputs.crash_json {
        zip.start_file("last-crash.json", options).map_err(to_io)?;
        zip.write_all(redactor.apply(crash).as_bytes())?;
        included_crash = true;
    }

    let chosen = planned_logs(inputs.log_dir);
    let mut log_files = Vec::new();
    for candidate in &chosen {
        let source = inputs.log_dir.join(&candidate.name);
        // A log that cannot be read at all is skipped, not fatal: one unreadable file must not
        // cost the player the rest of the report.
        let Ok(bytes) = std::fs::read(&source) else {
            continue;
        };
        // Lossy rather than strict. A hard kill can cut a multi-byte character at the end of the
        // file, and the session that crashed is the likeliest to have one — so strict decoding
        // dropped precisely the log worth reading, silently.
        let text = String::from_utf8_lossy(&bytes);
        zip.start_file(format!("logs/{}", candidate.name), options)
            .map_err(to_io)?;
        zip.write_all(redactor.apply(text.as_ref()).as_bytes())?;
        log_files.push(candidate.name.clone());
    }

    let mut included_save = false;
    if let Some(save) = inputs.save_path {
        if let Ok(bytes) = std::fs::read(save) {
            let name = save.file_name().map_or_else(
                || "save.db".to_owned(),
                |n| n.to_string_lossy().into_owned(),
            );
            zip.start_file(format!("save/{name}"), options)
                .map_err(to_io)?;
            zip.write_all(&bytes)?;
            included_save = true;
        }
    }

    zip.finish().map_err(to_io)?;
    // Only now does the player's chosen name exist, and it names a complete file.
    std::fs::rename(&scratch, output)?;
    let bytes = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);

    Ok(BundleSummary {
        path: output.to_string_lossy().into_owned(),
        bytes,
        log_files,
        included_save,
        included_crash,
    })
}

/// A default file name for the save dialog: dated, so a second report does not overwrite the first.
pub fn suggested_file_name(now: chrono::DateTime<chrono::Utc>) -> String {
    format!("ofm-report-{}.zip", now.format("%Y%m%d-%H%M"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn candidate(name: &str, bytes: u64, secs_ago: u64) -> LogCandidate {
        LogCandidate {
            name: name.to_owned(),
            bytes,
            modified: SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000 - secs_ago),
        }
    }

    fn names(chosen: &[LogCandidate]) -> Vec<&str> {
        chosen.iter().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn keeps_the_newest_logs_first() {
        let chosen = choose_logs(
            vec![
                candidate("old.log", 10, 300),
                candidate("newest.log", 10, 0),
                candidate("middle.log", 10, 100),
            ],
            3,
            1_000,
        );

        assert_eq!(names(&chosen), ["newest.log", "middle.log", "old.log"]);
    }

    #[test]
    fn drops_the_oldest_beyond_the_file_limit() {
        let chosen = choose_logs(
            vec![
                candidate("a.log", 10, 0),
                candidate("b.log", 10, 10),
                candidate("c.log", 10, 20),
                candidate("d.log", 10, 30),
            ],
            3,
            1_000,
        );

        assert_eq!(names(&chosen), ["a.log", "b.log", "c.log"]);
    }

    #[test]
    fn the_ceiling_admits_as_many_files_as_the_count_promises() {
        // Guards the relationship rather than the constants: whichever is changed, three rotations
        // at the plugin's configured size must still fit, or MAX_LOG_FILES is decoration.
        const ROTATION_BYTES: u64 = 5_000_000; // `max_file_size` in lib.rs

        let chosen = choose_logs(
            (0..MAX_LOG_FILES)
                .map(|i| candidate(&format!("{i}.log"), ROTATION_BYTES, i as u64 * 10))
                .collect(),
            MAX_LOG_FILES,
            MAX_LOG_BYTES,
        );

        assert_eq!(
            chosen.len(),
            MAX_LOG_FILES,
            "the byte ceiling must admit MAX_LOG_FILES rotations"
        );
    }

    #[test]
    fn stops_adding_once_the_byte_ceiling_is_reached() {
        let chosen = choose_logs(
            vec![
                candidate("a.log", 400, 0),
                candidate("b.log", 400, 10),
                candidate("c.log", 400, 20),
            ],
            3,
            1_000,
        );

        assert_eq!(names(&chosen), ["a.log", "b.log"]);
    }

    #[test]
    fn keeps_the_newest_log_even_when_it_alone_exceeds_the_ceiling() {
        // A single long session can outgrow the cap on its own. Dropping it would produce a
        // bundle with no logs at all, which is the one outcome worse than a large bundle.
        let chosen = choose_logs(
            vec![
                candidate("huge.log", 9_999, 0),
                candidate("small.log", 10, 10),
            ],
            3,
            1_000,
        );

        assert_eq!(names(&chosen), ["huge.log"]);
    }

    #[test]
    fn orders_deterministically_when_timestamps_collide() {
        let chosen = choose_logs(
            vec![candidate("a.log", 10, 0), candidate("b.log", 10, 0)],
            3,
            1_000,
        );

        assert_eq!(names(&chosen), ["b.log", "a.log"]);
    }

    #[test]
    fn returns_nothing_for_no_candidates() {
        assert!(choose_logs(Vec::new(), 3, 1_000).is_empty());
    }

    fn read_zip(path: &Path) -> Vec<(String, String)> {
        let file = std::fs::File::open(path).expect("open bundle");
        let mut archive = zip::ZipArchive::new(file).expect("read bundle");
        let mut out = Vec::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).expect("entry");
            let name = entry.name().to_owned();
            let mut contents = String::new();
            use std::io::Read;
            // Binary entries (the save) are not text; record the name with empty contents.
            let _ = entry.read_to_string(&mut contents);
            out.push((name, contents));
        }
        out
    }

    fn redactor() -> Redactor {
        Redactor::new(Some("/home/srobot"), Some("srobot"), None)
    }

    #[test]
    fn keeps_the_players_own_words_whatever_the_url_does_with_them() {
        // The link trims long text to stay under the URL limit. If the bundle did not hold the
        // full text, that trim would be the only copy and the rest would be lost on close.
        let dir = tempfile::tempdir().expect("temp dir");
        let out = dir.path().join("report.zip");
        let long = "x".repeat(20_000);

        write_bundle(
            &BundleInputs {
                log_dir: &dir.path().join("logs"),
                diagnostics_json: "{}",
                report_text: &long,
                crash_json: None,
                save_path: None,
            },
            &out,
            &redactor(),
        )
        .expect("write bundle");

        let packed = read_zip(&out);
        let report = packed
            .iter()
            .find(|(name, _)| name == "report.md")
            .expect("the report entry");
        assert_eq!(report.1.len(), 20_000, "the full text should survive");
    }

    #[test]
    fn redacts_the_report_text_too() {
        // The player can paste a path into their own description.
        let dir = tempfile::tempdir().expect("temp dir");
        let out = dir.path().join("report.zip");

        write_bundle(
            &BundleInputs {
                log_dir: &dir.path().join("logs"),
                diagnostics_json: "{}",
                report_text: "it died opening /home/srobot/saves/a.db",
                crash_json: None,
                save_path: None,
            },
            &out,
            &redactor(),
        )
        .expect("write bundle");

        let packed = read_zip(&out);
        let report = packed
            .iter()
            .find(|(name, _)| name == "report.md")
            .expect("the report entry");
        assert!(!report.1.contains("srobot"), "{report:?}");
    }

    #[test]
    fn writes_the_diagnostics_summary() {
        let dir = tempfile::tempdir().expect("temp dir");
        let out = dir.path().join("report.zip");

        write_bundle(
            &BundleInputs {
                log_dir: &dir.path().join("logs"),
                diagnostics_json: "{\"version\":\"0.3.0\"}",
                report_text: "",
                crash_json: None,
                save_path: None,
            },
            &out,
            &redactor(),
        )
        .expect("write bundle");

        // The scratch file the bundle is assembled in must not outlive a successful write.
        assert!(
            !dir.path().join("report.zip.part").exists(),
            "the scratch file should have been renamed, not left behind"
        );

        let entries = read_zip(&out);
        let diagnostics = entries
            .iter()
            .find(|(name, _)| name == "diagnostics.json")
            .expect("the diagnostics entry");
        assert!(diagnostics.1.contains("0.3.0"), "{diagnostics:?}");
        // No logs, no crash, no save were given, so those are the entries that must be absent.
        assert!(
            !entries.iter().any(|(name, _)| name.starts_with("logs/")
                || name.starts_with("save/")
                || name == "last-crash.json"),
            "{entries:?}"
        );
    }

    #[test]
    fn redacts_the_logs_it_packs() {
        // The whole point of the bundle: what lands in the zip must already be anonymous, because
        // after this it is out of our hands.
        let dir = tempfile::tempdir().expect("temp dir");
        let logs = dir.path().join("logs");
        std::fs::create_dir_all(&logs).expect("logs dir");
        std::fs::write(
            logs.join("app.log"),
            "[game_db] opening database at /home/srobot/saves/a.db",
        )
        .expect("write log");
        let out = dir.path().join("report.zip");

        let summary = write_bundle(
            &BundleInputs {
                log_dir: &logs,
                diagnostics_json: "{}",
                report_text: "",
                crash_json: None,
                save_path: None,
            },
            &out,
            &redactor(),
        )
        .expect("write bundle");

        assert_eq!(summary.log_files, ["app.log"]);
        let packed = read_zip(&out);
        let log = packed
            .iter()
            .find(|(name, _)| name == "logs/app.log")
            .expect("the log entry");
        assert!(log.1.contains("~/saves/a.db"), "{log:?}");
        assert!(!log.1.contains("srobot"), "{log:?}");
    }

    #[test]
    fn the_preview_names_the_files_the_bundle_packs() {
        // The preview screen is the consent surface: it lists the logs by name and size before
        // anything is written. If it read the directory for itself, the two could disagree —
        // a rotation between the two reads is enough — and the player would have consented to a
        // set of files that is not the set that left the machine.
        let dir = tempfile::tempdir().expect("temp dir");
        let logs = dir.path().join("logs");
        std::fs::create_dir_all(&logs).expect("logs dir");
        for i in 0..MAX_LOG_FILES + 2 {
            std::fs::write(logs.join(format!("app-{i}.log")), format!("line {i}"))
                .expect("write log");
        }
        let out = dir.path().join("report.zip");

        let planned: Vec<String> = planned_logs(&logs).into_iter().map(|c| c.name).collect();
        let summary = write_bundle(
            &BundleInputs {
                log_dir: &logs,
                diagnostics_json: "{}",
                report_text: "",
                crash_json: None,
                save_path: None,
            },
            &out,
            &redactor(),
        )
        .expect("write bundle");

        assert_eq!(planned.len(), MAX_LOG_FILES, "{planned:?}");
        assert_eq!(summary.log_files, planned);
    }

    #[test]
    fn includes_the_save_only_when_one_is_given() {
        let dir = tempfile::tempdir().expect("temp dir");
        let save = dir.path().join("career.db");
        std::fs::write(&save, b"sqlite-ish bytes").expect("write save");

        let without = write_bundle(
            &BundleInputs {
                log_dir: &dir.path().join("logs"),
                diagnostics_json: "{}",
                report_text: "",
                crash_json: None,
                save_path: None,
            },
            &dir.path().join("a.zip"),
            &redactor(),
        )
        .expect("bundle without save");
        assert!(!without.included_save);

        let with = write_bundle(
            &BundleInputs {
                log_dir: &dir.path().join("logs"),
                diagnostics_json: "{}",
                report_text: "",
                crash_json: None,
                save_path: Some(&save),
            },
            &dir.path().join("b.zip"),
            &redactor(),
        )
        .expect("bundle with save");
        assert!(with.included_save);
        assert!(read_zip(&dir.path().join("b.zip"))
            .iter()
            .any(|(name, _)| name == "save/career.db"));
    }

    #[test]
    fn records_a_crash_when_there_was_one() {
        let dir = tempfile::tempdir().expect("temp dir");
        let out = dir.path().join("report.zip");

        let summary = write_bundle(
            &BundleInputs {
                log_dir: &dir.path().join("logs"),
                diagnostics_json: "{}",
                report_text: "",
                crash_json: Some("{\"message\":\"boom at /home/srobot/x\"}"),
                save_path: None,
            },
            &out,
            &redactor(),
        )
        .expect("write bundle");

        assert!(summary.included_crash);
        let packed = read_zip(&out);
        let crash = packed
            .iter()
            .find(|(name, _)| name == "last-crash.json")
            .expect("the crash entry");
        assert!(!crash.1.contains("srobot"), "{crash:?}");
    }

    #[test]
    fn leaves_no_file_behind_when_the_bundle_cannot_be_written() {
        // The player picks a path, the write fails, and they must not find a truncated file
        // wearing the name they chose — they would attach it to the issue.
        let dir = tempfile::tempdir().expect("temp dir");
        // A regular file where a directory would have to be, so `create_dir_all` fails.
        let blocked = dir.path().join("blocked");
        std::fs::write(&blocked, b"not a directory").expect("write blocker");
        let out = blocked.join("report.zip");

        let result = write_bundle(
            &BundleInputs {
                log_dir: &dir.path().join("logs"),
                diagnostics_json: "{}",
                report_text: "",
                crash_json: None,
                save_path: None,
            },
            &out,
            &redactor(),
        );

        assert!(result.is_err(), "the write should have failed");
        assert!(!out.exists(), "no file should wear the chosen name");
        assert!(
            blocked.is_file(),
            "the blocker should be untouched, proving nothing was created under it"
        );
    }

    #[test]
    fn survives_a_missing_log_directory() {
        // First launch, or a player who cleared the folder. The summary is still worth sending.
        let dir = tempfile::tempdir().expect("temp dir");
        let out = dir.path().join("report.zip");

        let summary = write_bundle(
            &BundleInputs {
                log_dir: &dir.path().join("nothing-here"),
                diagnostics_json: "{}",
                report_text: "",
                crash_json: None,
                save_path: None,
            },
            &out,
            &redactor(),
        )
        .expect("write bundle");

        assert!(summary.log_files.is_empty());
        assert!(out.exists());
    }

    #[test]
    fn ignores_files_that_are_not_logs() {
        let dir = tempfile::tempdir().expect("temp dir");
        let logs = dir.path().join("logs");
        std::fs::create_dir_all(&logs).expect("logs dir");
        std::fs::write(logs.join("app.log"), "fine").expect("log");
        std::fs::write(logs.join("notes.txt"), "not a log").expect("txt");

        let summary = write_bundle(
            &BundleInputs {
                log_dir: &logs,
                diagnostics_json: "{}",
                report_text: "",
                crash_json: None,
                save_path: None,
            },
            &dir.path().join("report.zip"),
            &redactor(),
        )
        .expect("write bundle");

        assert_eq!(summary.log_files, ["app.log"]);
    }

    #[test]
    fn suggested_name_is_dated_so_reports_do_not_overwrite_each_other() {
        let at = chrono::DateTime::from_timestamp(1_757_000_000, 0).expect("a valid instant");

        let name = suggested_file_name(at);

        assert!(name.starts_with("ofm-report-"), "{name}");
        assert!(name.ends_with(".zip"), "{name}");
        assert_ne!(name, suggested_file_name(at + chrono::Duration::minutes(1)));
    }
}
