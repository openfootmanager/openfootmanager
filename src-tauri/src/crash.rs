//! Recording a Rust panic so that something survives it.
//!
//! Before this existed a panic in the backend produced the default message on stderr and nothing
//! else: no log line, because the log plugin never sees a panic, and no file, so the next launch
//! had no idea the previous one had died. A player whose game vanished could only report that it
//! vanished, and the logs they attached ended at the last thing that went *right*.
//!
//! Two pieces. The hook writes `last-crash.json` next to the logs and mirrors the panic into the
//! log file; the next launch reports and clears it. Keeping the crash in its own file rather than
//! relying on the log alone means the next launch can *find* it without parsing 5 MB of text.

use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// What a panic left behind.
///
/// Every field carries `#[serde(default)]` for the same reason save files do: a crash file written
/// by an older build must still parse, and the one moment we must not fail to read it is while
/// reporting that the app already failed once.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashRecord {
    #[serde(default)]
    pub app_version: String,
    #[serde(default)]
    pub occurred_at: String,
    #[serde(default)]
    pub thread: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub backtrace: String,
}

impl CrashRecord {
    /// One line, for the log file and for prefilling a report.
    pub fn summary(&self) -> String {
        format!(
            "{} (thread `{}` at {}, version {})",
            self.message, self.thread, self.location, self.app_version
        )
    }
}

/// Where the hook writes.
///
/// A `OnceLock` because the hook has to be installed before Tauri builds — a panic during plugin
/// setup is exactly the kind we most want to catch — but the app data directory is only resolvable
/// once the app handle exists. Until it is set the hook still logs; it just has nowhere durable to
/// write, which is the right trade against not being installed at all.
static CRASH_FILE: OnceLock<PathBuf> = OnceLock::new();

pub fn crash_file_in(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("last-crash.json")
}

/// Point the already-installed hook at a real file. Ignored if called twice.
pub fn set_crash_file(path: PathBuf) {
    let _ = CRASH_FILE.set(path);
}

fn build_record(
    message: String,
    location: String,
    thread: String,
    backtrace: String,
) -> CrashRecord {
    CrashRecord {
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        occurred_at: chrono::Utc::now().to_rfc3339(),
        thread,
        location,
        message,
        backtrace,
    }
}

/// A panic payload is `Box<dyn Any>`; in practice it is one of these two, and anything else is
/// reported as such rather than silently becoming an empty string.
fn payload_message(info: &PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_owned()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "panic with a non-string payload".to_owned()
    }
}

fn record_from(info: &PanicHookInfo<'_>) -> CrashRecord {
    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "unknown".to_owned());
    let thread = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .to_owned();
    build_record(
        payload_message(info),
        location,
        thread,
        // `force_capture`, not `capture`: the latter is a no-op unless the player happens to
        // have set RUST_BACKTRACE, which no player ever has. The release profile sets
        // `strip = "debuginfo"`, which keeps the symbol table — so this comes back with function
        // names, though without file and line numbers.
        std::backtrace::Backtrace::force_capture().to_string(),
    )
}

fn write_record(path: &Path, record: &CrashRecord) -> std::io::Result<()> {
    // Beside the target and then renamed, rather than straight over the top. `fs::write` truncates
    // first, so a process that dies partway through this — which is not far-fetched, given what
    // has just happened to it — used to leave a `last-crash.json` that exists, parses as nothing,
    // and makes the next launch believe it has a crash record when it has half of one.
    crate::atomic_file::write_json(path, record)
}

/// Install the hook. Safe to call once, early, before anything else can panic.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let record = record_from(info);

        // The file first, the log second, and the order is load-bearing. The logger formats its
        // arguments *inside* its own stream lock, so a panic raised while a log line is being
        // formatted arrives here with that lock already held by this thread — and the `log::error!`
        // below then blocks on it forever. Writing the record first means the evidence survives
        // even in that case; a crash that hangs is bad, a crash that hangs having recorded nothing
        // is the failure this module exists to prevent.
        if let Some(path) = CRASH_FILE.get() {
            if let Err(error) = write_record(path, &record) {
                // Deliberately not `log::error!`: see below — this is the one path where the
                // logger may be the thing that is stuck.
                eprintln!("[crash] could not write {}: {error}", path.display());
            }
        }

        // Chain rather than replace, and do it here rather than last: the default hook's stderr
        // output is what a developer running `cargo tauri dev` actually reads. Leaving it until
        // after the `log::error!` below meant that in the deadlock this comment describes — the
        // one case where the ordering matters at all — stderr never printed a thing.
        previous(info);

        log::error!("[crash] {}\n{}", record.summary(), record.backtrace);
    }));
}

/// The crash the previous launch left, held for as long as this process runs.
///
/// `take_previous_crash` deletes the file it reads — it has to, or the same crash is reported on
/// every launch forever. But the bug report is filed *later in the same session*, and it used to
/// look for that file, so the row it shows for "a crash from your last session" could never appear
/// for an actual crash. Keeping the record in managed state is what closes that gap: the file is
/// still cleared exactly once, and the evidence still reaches the report.
pub struct PreviousCrash(pub Option<CrashRecord>);

impl PreviousCrash {
    /// The record as the bundle stores it, or `None` when the last run ended cleanly.
    pub fn as_json(&self) -> Option<String> {
        self.0
            .as_ref()
            .and_then(|record| serde_json::to_string_pretty(record).ok())
    }
}

/// Report and clear a crash left by the previous launch.
///
/// Returns what was found, so a caller can do more than log it — which is what the crash-report
/// prompt will want.
pub fn take_previous_crash(app_data_dir: &Path) -> Option<CrashRecord> {
    let path = crash_file_in(app_data_dir);
    let raw = std::fs::read_to_string(&path).ok()?;
    // Remove it whether or not it parsed: an unreadable crash file that stays put would be
    // re-reported on every launch for the rest of the install's life.
    if let Err(error) = std::fs::remove_file(&path) {
        log::warn!("[crash] could not clear {}: {error}", path.display());
    }
    match serde_json::from_str::<CrashRecord>(&raw) {
        Ok(record) => {
            log::warn!(
                "[crash] previous launch ended in a panic: {}",
                record.summary()
            );
            Some(record)
        }
        Err(error) => {
            log::warn!("[crash] previous crash file was unreadable: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CrashRecord {
        build_record(
            "something exploded".to_owned(),
            "src/lib.rs:10:5".to_owned(),
            "main".to_owned(),
            "0: frame".to_owned(),
        )
    }

    #[test]
    fn stamps_the_version_and_a_timestamp() {
        let record = sample();
        assert_eq!(record.app_version, env!("CARGO_PKG_VERSION"));
        assert!(
            record.occurred_at.contains('T'),
            "expected an RFC 3339 timestamp, got {:?}",
            record.occurred_at
        );
    }

    #[test]
    fn summary_names_the_message_thread_and_location() {
        let summary = sample().summary();
        assert!(summary.contains("something exploded"), "{summary}");
        assert!(summary.contains("main"), "{summary}");
        assert!(summary.contains("src/lib.rs:10:5"), "{summary}");
    }

    #[test]
    fn round_trips_through_the_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = crash_file_in(dir.path());
        let record = sample();

        write_record(&path, &record).expect("write");
        let raw = std::fs::read_to_string(&path).expect("read");
        let parsed: CrashRecord = serde_json::from_str(&raw).expect("parse");

        assert_eq!(parsed, record);
    }

    #[test]
    fn never_creates_the_crash_file_it_cannot_fill() {
        // `fs::write` truncates the destination before writing a byte, so a process that died
        // partway through — not far-fetched, given what has just happened to it — left a
        // `last-crash.json` that exists and parses as nothing, and the next launch believed it had
        // a crash record when it had half of one.
        //
        // A directory sitting where the scratch file goes is the cheapest way to fail the write
        // after the point `fs::write` would already have created the destination.
        let dir = tempfile::tempdir().expect("temp dir");
        let path = crash_file_in(dir.path());
        std::fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        let mut scratch_name = path.file_name().expect("name").to_owned();
        scratch_name.push(".part");
        std::fs::create_dir(path.with_file_name(scratch_name)).expect("blocking dir");

        assert!(write_record(&path, &sample()).is_err());
        assert!(
            !path.exists(),
            "the name must not appear without a record behind it"
        );
    }

    #[test]
    fn reads_a_crash_file_written_before_the_fields_existed() {
        // The compatibility that matters: a file from an older build must not make the reporting
        // path itself fail.
        let dir = tempfile::tempdir().expect("temp dir");
        let path = crash_file_in(dir.path());
        std::fs::write(&path, r#"{"message":"old format"}"#).expect("write");

        let record = take_previous_crash(dir.path()).expect("a record");

        assert_eq!(record.message, "old format");
        assert_eq!(record.backtrace, "");
    }

    #[test]
    fn clears_the_file_so_it_is_reported_once() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_record(&crash_file_in(dir.path()), &sample()).expect("write");

        assert!(take_previous_crash(dir.path()).is_some());
        assert!(
            take_previous_crash(dir.path()).is_none(),
            "a crash must not be reported on every subsequent launch"
        );
    }

    #[test]
    fn clears_an_unreadable_file_too() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = crash_file_in(dir.path());
        std::fs::write(&path, "not json at all").expect("write");

        assert!(take_previous_crash(dir.path()).is_none());
        assert!(
            !path.exists(),
            "a corrupt crash file must not be re-read forever"
        );
    }

    #[test]
    fn a_held_crash_serialises_for_the_bundle() {
        // One instance throughout: `sample()` stamps a fresh timestamp on every call, so
        // comparing against a second one compares two different records.
        let record = sample();
        let held = PreviousCrash(Some(record.clone()));

        let json = held.as_json().expect("a held crash should serialise");

        assert!(json.contains("something exploded"), "{json}");
        assert_eq!(
            serde_json::from_str::<CrashRecord>(&json).expect("round trip"),
            record
        );
    }

    #[test]
    fn a_clean_previous_run_holds_nothing() {
        assert!(PreviousCrash(None).as_json().is_none());
    }

    #[test]
    fn the_record_outlives_the_file_it_came_from() {
        // The whole point: the file is gone after startup, and the record is not.
        let dir = tempfile::tempdir().expect("temp dir");
        write_record(&crash_file_in(dir.path()), &sample()).expect("write");

        let held = PreviousCrash(take_previous_crash(dir.path()));

        assert!(
            !crash_file_in(dir.path()).exists(),
            "the file should be cleared"
        );
        assert!(held.as_json().is_some(), "the record should survive it");
    }

    #[test]
    fn reports_nothing_when_the_app_has_never_crashed() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(take_previous_crash(dir.path()).is_none());
    }

    /// The end-to-end path: an installed hook, a real panic, a file on disk.
    ///
    /// Note the process-wide residue this leaves, because it is not obvious: `CRASH_FILE` is a
    /// `OnceLock`, so the temporary directory set here is where *every* later panic in this test
    /// binary writes — including ordinary assertion failures — and the directory is gone by then,
    /// so each one recreates it and captures a backtrace. Harmless, but it is why a failing test
    /// run leaves a stray `last-crash.json` behind and costs a little extra time.
    ///
    /// Everything else here tests the pieces, which would all keep passing if the hook were never
    /// installed or never fired — the one failure that makes the whole module pointless. Worth the
    /// process-global side effects: the hook chains to the previous one, so any other test that
    /// panics behaves exactly as before and merely also writes this file.
    #[test]
    fn an_installed_hook_writes_a_record_for_a_real_panic() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = crash_file_in(dir.path());
        set_crash_file(path.clone());
        install_panic_hook();

        let result = std::panic::catch_unwind(|| panic!("deliberate test panic"));
        assert!(result.is_err(), "the panic should still propagate");

        let raw = std::fs::read_to_string(&path).expect("the hook should have written a record");
        let record: CrashRecord = serde_json::from_str(&raw).expect("a well-formed record");
        // Not asserting the message: tests run in parallel, and another test panicking would
        // legitimately overwrite this file. What must hold is that a panic produced a complete
        // record — with the backtrace that makes it worth having.
        assert!(
            !record.backtrace.is_empty(),
            "expected a captured backtrace"
        );
        assert!(!record.location.is_empty(), "expected a panic location");
        assert_eq!(record.app_version, env!("CARGO_PKG_VERSION"));
    }
}
