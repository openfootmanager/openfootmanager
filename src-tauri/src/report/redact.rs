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
        let mut paths = Vec::new();
        if let Some(home) = home.map(str::trim).filter(|h| h.len() >= MIN_NAME_LEN) {
            paths.push(home.to_owned());
            // Logs quote paths in whichever separator the writer used; a Windows home directory
            // shows up both ways depending on whether it came from `Path` or from a URL.
            if home.contains('\\') {
                paths.push(home.replace('\\', "/"));
            }
        }
        paths.sort_by_key(|p| std::cmp::Reverse(p.len()));

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
        let host = std::env::var("HOSTNAME")
            .ok()
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
            out = out.replace(path.as_str(), "~");
        }
        for name in &self.names {
            out = replace_whole_words(&out, name, "<redacted>");
        }
        out
    }
}

/// True when `byte` can sit inside a name — so a match flanked by one of these is part of a longer
/// word and must be left alone.
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn replace_whole_words(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.to_owned();
    }
    let bytes = haystack.as_bytes();
    let mut out = String::with_capacity(haystack.len());
    let mut cursor = 0;

    while let Some(found) = haystack[cursor..].find(needle) {
        let start = cursor + found;
        let end = start + needle.len();
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_word_byte(bytes[end]);

        out.push_str(&haystack[cursor..start]);
        if before_ok && after_ok {
            out.push_str(replacement);
        } else {
            out.push_str(needle);
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
    fn redacts_every_occurrence_on_a_line() {
        let out = redactor().apply("/home/srobot -> /home/srobot/backup");

        assert_eq!(out, "~ -> ~/backup");
    }

    #[test]
    fn is_a_no_op_when_the_environment_yields_nothing() {
        let out = Redactor::new(None, None, None).apply("/home/srobot/save.db");

        assert_eq!(out, "/home/srobot/save.db");
    }

    #[test]
    fn leaves_ordinary_log_lines_untouched() {
        let line = "[turn] processed matchday 5 for competition 12 in 84ms";

        assert_eq!(redactor().apply(line), line);
    }
}
