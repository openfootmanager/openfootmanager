//! Taking the person out of the log before it leaves their machine.
//!
//! `docs/BETA_TESTING_GUIDE.md` tells testers that the logs hold no personal information. That has
//! never been true: the app logs its database path on every launch, and on every platform that
//! path runs through the user's home directory, so their account name is written down each time
//! the game starts.
//!
//! Two rules, and the difference between them is deliberate.
//!
//! The **home directory** is replaced wherever it appears. It is long, unambiguous, and there is
//! no sentence in which it occurs by accident.
//!
//! The **user name and host name** are replaced only as whole words, and only when they are
//! distinctive enough to be worth replacing at all. A blanket substring replacement is actively
//! harmful here: an account called `root`, `admin` or `pi` — all of them common — would rewrite
//! every occurrence inside ordinary words, turning "root cause" into "<user> cause" and corrupting
//! the very diagnostics the report exists to carry. Those names also identify nobody, so the
//! trade is one-sided: real corruption in exchange for no real privacy.

use std::collections::BTreeSet;

/// Names too generic to be worth redacting, and too damaging to redact blindly.
const GENERIC_NAMES: &[&str] = &[
    "root",
    "admin",
    "administrator",
    "user",
    "guest",
    "test",
    "pi",
    "ubuntu",
    "debian",
    "localhost",
    "default",
    // Structural words, not account names in any practical sense. An account called one of these
    // would still be caught by its home directory, which is the rule that does the real work.
    "null",
    "none",
    "true",
    "false",
    "unknown",
    "system",
    "public",
];

/// Shorter than this and a name is more likely to collide with ordinary text than to identify
/// anyone.
const MIN_NAME_LEN: usize = 4;

pub struct Redactor {
    /// Longest first, so a home directory is replaced before the user name nested inside it.
    paths: Vec<String>,
    names: BTreeSet<String>,
}

impl Redactor {
    /// Build from explicit values. The command layer uses [`Redactor::from_environment`]; tests
    /// use this, so none of them depend on who is running them.
    pub fn new(home: Option<&str>, user: Option<&str>, host: Option<&str>) -> Self {
        let mut paths: Vec<String> = Vec::new();
        if let Some(home) = home.map(str::trim).filter(|h| h.len() >= MIN_NAME_LEN) {
            paths.extend(path_spellings(home));
        }
        // Longest first, and deduplicated: a path that spells the same on two routes would
        // otherwise be searched for twice, and a shorter spelling nested inside a longer one must
        // not be replaced first.
        paths.sort_by_key(|p| std::cmp::Reverse(p.len()));
        paths.dedup();

        let names = [user, host]
            .into_iter()
            .flatten()
            .map(str::trim)
            .filter(|name| name.len() >= MIN_NAME_LEN)
            .filter(|name| !GENERIC_NAMES.contains(&name.to_ascii_lowercase().as_str()))
            .map(str::to_owned)
            .collect();

        Self { paths, names }
    }

    pub fn from_environment() -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok();
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .ok();
        // Windows publishes the machine name as COMPUTERNAME and has neither HOSTNAME nor
        // /etc/hostname, so without it the preview's promise to strip the computer name was simply
        // untrue on that platform.
        let host = std::env::var("HOSTNAME")
            .ok()
            .or_else(|| std::env::var("COMPUTERNAME").ok())
            .or_else(|| std::fs::read_to_string("/etc/hostname").ok());
        Self::new(
            home.as_deref(),
            user.as_deref(),
            host.as_deref().map(str::trim),
        )
    }

    pub fn apply(&self, text: &str) -> String {
        let mut out = text.to_owned();
        for path in &self.paths {
            out = replace_ignoring_ascii_case(&out, path, "~");
        }
        for name in &self.names {
            out = replace_whole_words(&out, name, "<redacted>");
        }
        out
    }
}

/// Every way one sensitive path can be written by the time it reaches a log line.
///
/// Missing any single spelling ships the whole home directory, so this is deliberately generous —
/// a spelling that never occurs costs one failed substring search.
///
/// - `{:?}` of a `Path` goes through `OsStr` Debug and `char::escape_debug`, which DOUBLES every
///   backslash. That is how `game_database.rs` writes the save path on every launch, and
///   `serde_json` and the frontend's `JSON.stringify` produce the same doubled form.
/// - **Doubled twice.** A panic message that already holds a `{:?}` path is itself serialised
///   into `last-crash.json`, and the escaping runs a second time over the escapes.
/// - Forward slashes turn up when a path came from a URL rather than from `Path`.
/// - Percent-encoding turns up in `asset://` URLs, and anywhere a path with a space in it — which
///   is most Windows profile folders named after a person — is put in a URL.
fn path_spellings(path: &str) -> Vec<String> {
    let mut spellings = vec![path.to_owned()];
    if path.contains('\\') {
        spellings.push(path.replace('\\', "/"));
        spellings.push(path.replace('\\', "\\\\"));
        spellings.push(path.replace('\\', "\\\\\\\\"));
    }
    for spelling in spellings.clone() {
        let encoded = spelling
            .replace('%', "%25")
            .replace(' ', "%20")
            .replace('\\', "%5C");
        if encoded != spelling {
            spellings.push(encoded);
        }
    }
    spellings
}

/// True when the match is the *key* of a JSON object member, rather than a value.
///
/// `diagnostics.json` and `last-crash.json` go through the same redactor as the log text, and an
/// account name that happens to be an ordinary word — `message`, `thread`, `error`, `path` — would
/// otherwise rewrite the field names and leave a file nothing can parse. That costs the whole
/// report: the crash record is the part a triager reads first. The value on the other side of the
/// colon is still redacted, which is where a name would actually appear.
fn is_json_key(haystack: &str, start: usize, end: usize) -> bool {
    let bytes = haystack.as_bytes();
    if start == 0 || bytes[start - 1] != b'"' {
        return false;
    }
    let after = &haystack[end..];
    let Some(rest) = after.strip_prefix('"') else {
        return false;
    };
    rest.trim_start().starts_with(':')
}

/// True when `byte` can sit inside a name — so a match flanked by one of these is part of a longer
/// word and must be left alone.
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Case-insensitive replace, ASCII only.
///
/// `to_ascii_lowercase` is deliberate rather than `to_lowercase`: it preserves byte length, so an
/// index found in the lowered copy is valid in the original. Full Unicode folding can change the
/// length (`İ` lowers to two chars) and the offsets would no longer line up. The cost is that a
/// non-ASCII account name is matched case-sensitively, which is the safe direction — it can only
/// fail to redact a spelling that never appears, never corrupt one that does.
///
/// Windows paths are case-insensitive, and Windows reports `USERNAME` as the account was typed
/// while the profile folder keeps the case it was created with. Matching exactly is how a home
/// directory survives redaction on the platform.
fn replace_ignoring_ascii_case(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.to_owned();
    }
    let lowered_haystack = haystack.to_ascii_lowercase();
    let lowered_needle = needle.to_ascii_lowercase();
    let mut out = String::with_capacity(haystack.len());
    let mut cursor = 0;

    while let Some(found) = lowered_haystack[cursor..].find(&lowered_needle) {
        let start = cursor + found;
        out.push_str(&haystack[cursor..start]);
        out.push_str(replacement);
        cursor = start + needle.len();
    }
    out.push_str(&haystack[cursor..]);
    out
}

fn replace_whole_words(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.to_owned();
    }
    let bytes = haystack.as_bytes();
    let lowered_haystack = haystack.to_ascii_lowercase();
    let lowered_needle = needle.to_ascii_lowercase();
    let mut out = String::with_capacity(haystack.len());
    let mut cursor = 0;

    while let Some(found) = lowered_haystack[cursor..].find(&lowered_needle) {
        let start = cursor + found;
        let end = start + needle.len();
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_word_byte(bytes[end]);

        out.push_str(&haystack[cursor..start]);
        if before_ok && after_ok && !is_json_key(haystack, start, end) {
            out.push_str(replacement);
        } else {
            // The text as it was written, not as the needle spells it. Matching folds case, so
            // these differ — and putting the needle back would rewrite a word the rule just
            // decided *not* to redact.
            out.push_str(&haystack[start..end]);
        }
        cursor = end;
    }
    out.push_str(&haystack[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redactor() -> Redactor {
        Redactor::new(Some("/home/srobot"), Some("srobot"), Some("thinkpad-x1"))
    }

    #[test]
    fn replaces_the_home_directory_with_a_tilde() {
        let line = "[game_db] opening database at \"/home/srobot/.local/share/ofm/saves/a.db\"";

        let out = redactor().apply(line);

        assert!(out.contains("~/.local/share/ofm/saves/a.db"), "{out}");
        assert!(!out.contains("srobot"), "{out}");
    }

    #[test]
    fn replaces_the_home_directory_before_the_user_name_inside_it() {
        // Order is the whole trick: redact the name first and the home rule can never match, so
        // the path survives as `/home/<redacted>/...` instead of collapsing to `~`.
        let out = redactor().apply("/home/srobot/games");

        assert_eq!(out, "~/games");
    }

    #[test]
    fn replaces_a_standalone_user_name() {
        let out = redactor().apply("running as srobot on thinkpad-x1");

        assert!(!out.contains("srobot"), "{out}");
        assert!(!out.contains("thinkpad-x1"), "{out}");
    }

    #[test]
    fn leaves_a_name_that_is_part_of_a_longer_word() {
        // `srobot` inside `srobotics` is not the user; rewriting it corrupts the line and hides
        // nothing.
        let out = redactor().apply("loaded srobotics.ofm and srobot_backup");

        assert!(out.contains("srobotics.ofm"), "{out}");
        assert!(out.contains("srobot_backup"), "{out}");
    }

    #[test]
    fn never_redacts_a_generic_account_name() {
        // An account called `root` identifies nobody, and replacing it everywhere would rewrite
        // ordinary prose — "root cause" is the case that made this a rule.
        let out = Redactor::new(Some("/root"), Some("root"), Some("localhost"))
            .apply("root cause: connection to localhost refused");

        assert_eq!(out, "root cause: connection to localhost refused");
    }

    #[test]
    fn still_redacts_the_home_directory_of_a_generic_account() {
        // The name is not worth redacting; the path still is, because the path is unambiguous.
        let out = Redactor::new(Some("/home/admin"), Some("admin"), None)
            .apply("db at /home/admin/save.db, admin privileges required");

        assert!(out.contains("~/save.db"), "{out}");
        assert!(out.contains("admin privileges"), "{out}");
    }

    #[test]
    fn ignores_a_name_too_short_to_be_distinctive() {
        let out = Redactor::new(None, Some("ab"), None).apply("ab cd ab");

        assert_eq!(out, "ab cd ab");
    }

    #[test]
    fn handles_a_windows_home_directory_in_either_separator() {
        let redactor = Redactor::new(Some("C:\\Users\\Sturdy"), Some("Sturdy"), None);

        assert!(redactor
            .apply("opening C:\\Users\\Sturdy\\AppData\\logs")
            .contains("~\\AppData\\logs"));
        assert!(redactor
            .apply("opening C:/Users/Sturdy/AppData/logs")
            .contains("~/AppData/logs"));
    }

    #[test]
    fn redacts_a_path_as_rust_debug_actually_writes_it() {
        // `game_database.rs` logs `{:?}` of a `&Path`, and Path -> OsStr Debug escapes through
        // `char::escape_debug`, which DOUBLES every backslash. The unescaped spelling the older
        // test used is a string no producer in this app ever emits, so the suite stayed green
        // while the whole home directory shipped in the bundle.
        let redactor = Redactor::new(Some(r"C:\Users\Bob"), Some("Bob"), None);

        let out =
            redactor.apply(r#"[game_db] opening database at "C:\\Users\\Bob\\AppData\\a.db""#);

        assert!(!out.contains("Bob"), "{out}");
        assert!(out.contains('~'), "{out}");
    }

    #[test]
    fn redacts_a_path_json_escaped_the_way_serde_writes_it() {
        // `last-crash.json` and the frontend's `JSON.stringify` produce the same doubled form.
        let redactor = Redactor::new(Some(r"C:\Users\Bob"), Some("Bob"), None);

        let out = redactor.apply(r#"{"message":"could not open C:\\Users\\Bob\\x.db"}"#);

        assert!(!out.contains("Bob"), "{out}");
    }

    #[test]
    fn redacts_a_home_directory_whose_case_differs_from_the_account_name() {
        // Windows reports USERNAME as the account was typed and the profile folder as it was
        // created; the two differ often enough to matter, and paths are case-insensitive there.
        let redactor = Redactor::new(Some(r"C:\Users\Sturdy"), Some("sturdy"), None);

        let out = redactor.apply(r#"[game_db] ready at "C:\\USERS\\STURDY\\AppData""#);

        assert!(!out.to_ascii_lowercase().contains("sturdy"), "{out}");
    }

    #[test]
    fn redacts_a_short_account_name_through_its_own_home_path() {
        // `Bob` is under MIN_NAME_LEN, so the whole-word rule skips it by design. The path rule has
        // to carry it, or a three-letter account leaks on every launch.
        let out = Redactor::new(Some("/home/bob"), Some("bob"), None)
            .apply(r#"[game_db] opening database at "/home/bob/.local/share/ofm/a.db""#);

        assert!(!out.contains("/home/bob"), "{out}");
        assert!(out.contains("~/.local/share/ofm/a.db"), "{out}");
    }

    #[test]
    fn redacts_every_occurrence_on_a_line() {
        let out = redactor().apply("/home/srobot -> /home/srobot/backup");

        assert_eq!(out, "~ -> ~/backup");
    }

    #[test]
    fn redacts_a_windows_path_escaped_twice_over() {
        // A panic message already holds the `{:?}` spelling, with its backslashes doubled. Writing
        // that message into last-crash.json escapes it AGAIN, so the path arrives with four
        // backslashes per separator — and the crash file is the one part of a report nobody reads
        // before it is sent.
        // No user name is given, so the path rule is the only thing that can satisfy this.
        let out = Redactor::new(Some("C:\\Users\\srobot"), None, None)
            .apply(r"panicked at C:\\\\Users\\\\srobot\\\\saves\\\\a.db");

        assert!(!out.contains("srobot"), "{out}");
        assert!(out.contains('~'), "{out}");
    }

    #[test]
    fn redacts_a_percent_encoded_path() {
        // `asset://` URLs and anything else that puts a path in a URL. A profile folder named
        // after a person usually has a space in it, and a space is where the encoding starts.
        let redactor = Redactor::new(Some(r"C:\Users\Ada Lovelace"), None, None);

        let out = redactor.apply("asset://localhost/C:%5CUsers%5CAda%20Lovelace%5Clogs%5Capp.log");

        assert!(!out.contains("Lovelace"), "{out}");
        assert!(out.contains('~'), "{out}");
    }

    #[test]
    fn leaves_json_field_names_alone() {
        // `message` and `thread` are real account names as well as the field names in
        // last-crash.json. Rewriting the keys leaves a file nothing can parse — and the crash
        // record is the part of a bundle a triager reads first.
        let redactor = Redactor::new(None, Some("message"), None);

        let out = redactor.apply(r#"{"message": "it broke", "thread": "main"}"#);

        assert!(out.contains(r#""message":"#), "{out}");
    }

    #[test]
    fn still_redacts_the_name_where_it_is_a_value() {
        // The guard is about keys only. A name appearing as a value is exactly what redaction is
        // for, and skipping that to protect the file would be the wrong trade.
        let redactor = Redactor::new(None, Some("message"), None);

        let out = redactor.apply(r#"{"user": "message"}"#);

        assert!(!out.contains(r#""message""#), "{out}");
        assert!(out.contains(r#""user":"#), "{out}");
    }

    #[test]
    fn redacts_a_windows_computer_name() {
        // Windows exposes it as COMPUTERNAME; the preview promises it is stripped on every
        // platform, so the promise has to hold on the one that spells it differently.
        let out = Redactor::new(None, None, Some("THINKPAD-X1")).apply("host THINKPAD-X1 ready");

        assert!(!out.contains("THINKPAD-X1"), "{out}");
    }

    #[test]
    fn is_a_no_op_when_the_environment_yields_nothing() {
        let out = Redactor::new(None, None, None).apply("/home/srobot/save.db");

        assert_eq!(out, "/home/srobot/save.db");
    }

    #[test]
    fn preserves_the_original_casing_of_a_word_it_declines_to_redact() {
        // Case-insensitive matching means the text found is not always spelled like the needle.
        // Putting the needle back would silently rewrite the log: `SROBOTICS` is not the user, so
        // it must survive exactly as written, not become `SrobotICS`.
        let redactor = Redactor::new(None, Some("Srobot"), None);

        let out = redactor.apply("loaded SROBOTICS.ofm and srobot_backup");

        assert!(out.contains("SROBOTICS.ofm"), "{out}");
        assert!(out.contains("srobot_backup"), "{out}");
    }

    #[test]
    fn leaves_ordinary_log_lines_untouched() {
        let line = "[turn] processed matchday 5 for competition 12 in 84ms";

        assert_eq!(redactor().apply(line), line);
    }
}
