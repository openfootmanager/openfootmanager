use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use serde_json::Value;

use crate::app::App;
use crate::client::{CallError, Refusal};

/// Everything a refused call must leave alone: the live game, and every byte on disk under the
/// saves directory.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// `None` before any game exists.
    pub game: Option<Value>,
    /// Saves-relative path → content hash of every file the app wrote.
    pub files: BTreeMap<String, u64>,
}

impl Snapshot {
    pub fn take(app: &App) -> Result<Self, CallError> {
        let game = match app
            .client()
            .call_value("info_game_state", serde_json::json!({}))
        {
            Ok(state) => Some(state["game"].clone()),
            Err(CallError::Refused(refusal))
                if refusal.key.as_deref() == Some("be.error.noActiveGameSession") =>
            {
                None
            }
            Err(other) => return Err(other),
        };
        let files = app
            .saves_dir()
            .map(|dir| hash_files(&dir))
            .unwrap_or_default();
        Ok(Self { game, files })
    }

    /// What differs from `other`, one line each; empty when they are the same.
    pub fn differences(&self, other: &Self) -> Vec<String> {
        let mut found = Vec::new();
        match (&self.game, &other.game) {
            (Some(a), Some(b)) => found.extend(object_differences(a, b)),
            (None, None) => {}
            _ => found.push("a game appeared or disappeared".to_string()),
        }
        for (path, hash) in &self.files {
            match other.files.get(path) {
                None => found.push(format!("file removed: {path}")),
                Some(other_hash) if other_hash != hash => {
                    found.push(format!("file changed: {path}"))
                }
                Some(_) => {}
            }
        }
        for path in other.files.keys().filter(|p| !self.files.contains_key(*p)) {
            found.push(format!("file added: {path}"));
        }
        found
    }
}

/// Calls `refused_call`, which must be refused with `expected_key`, and proves the game and the
/// saves directory are identical before and after.
pub fn assert_refusal_changes_nothing<T: std::fmt::Debug>(
    app: &App,
    expected_key: &str,
    refused_call: impl FnOnce() -> Result<T, CallError>,
) -> Refusal {
    let before = Snapshot::take(app).expect("a snapshot before the call");
    let outcome = refused_call();
    let after = Snapshot::take(app).expect("a snapshot after the call");

    let refusal = match outcome {
        Err(CallError::Refused(refusal)) => refusal,
        other => panic!("expected a refusal with {expected_key}, got {other:?}"),
    };
    assert_eq!(refusal.key.as_deref(), Some(expected_key), "{refusal:?}");
    let changed = before.differences(&after);
    assert!(
        changed.is_empty(),
        "a refused call changed state:\n{}",
        changed.join("\n")
    );
    refusal
}

fn object_differences(before: &Value, after: &Value) -> Vec<String> {
    match (before.as_object(), after.as_object()) {
        (Some(a), Some(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            keys.into_iter()
                .filter(|key| a.get(*key) != b.get(*key))
                .map(|key| format!("game.{key} changed"))
                .collect()
        }
        _ if before != after => vec!["game changed".to_string()],
        _ => Vec::new(),
    }
}

fn hash_files(dir: &Path) -> BTreeMap<String, u64> {
    let mut found = BTreeMap::new();
    collect(dir, dir, &mut found);
    found
}

fn collect(root: &Path, dir: &Path, found: &mut BTreeMap<String, u64>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, found);
        } else if let Ok(bytes) = fs::read(&path) {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut hasher);
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            found.insert(relative, hasher.finish());
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn snapshot(game: Value, files: &[(&str, u64)]) -> Snapshot {
        Snapshot {
            game: Some(game),
            files: files.iter().map(|(p, h)| (p.to_string(), *h)).collect(),
        }
    }

    /// Given two snapshots of the same state
    /// When they are compared
    /// Then nothing differs.
    #[test]
    fn identical_snapshots_differ_in_nothing() {
        let a = snapshot(json!({"clock": 1, "teams": []}), &[("s.db", 1)]);
        assert!(a.differences(&a.clone()).is_empty());
    }

    /// Given a call that changes a game field, a file, or adds a file
    /// When the snapshots around it are compared
    /// Then each change is reported by name, so the helper goes red for a mutating call.
    #[test]
    fn any_change_is_reported() {
        let before = snapshot(json!({"clock": 1, "teams": []}), &[("s.db", 1)]);

        let game_changed = snapshot(json!({"clock": 2, "teams": []}), &[("s.db", 1)]);
        let file_changed = snapshot(json!({"clock": 1, "teams": []}), &[("s.db", 2)]);
        let file_added = snapshot(
            json!({"clock": 1, "teams": []}),
            &[("s.db", 1), ("t.db", 3)],
        );
        let file_removed = snapshot(json!({"clock": 1, "teams": []}), &[]);

        assert_eq!(
            before.differences(&game_changed),
            vec!["game.clock changed"]
        );
        assert_eq!(
            before.differences(&file_changed),
            vec!["file changed: s.db"]
        );
        assert_eq!(before.differences(&file_added), vec!["file added: t.db"]);
        assert_eq!(
            before.differences(&file_removed),
            vec!["file removed: s.db"]
        );
    }
}
