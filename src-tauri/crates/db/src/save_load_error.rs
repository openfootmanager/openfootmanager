//! Structured classification of why a save file could not be loaded.
//!
//! Each variant maps to a distinct user-facing i18n key, so the UI can tell the
//! player *why* a load failed — corruption, an incompatible (newer) version,
//! missing data, an invalid embedded world, or an unexpected error — instead of
//! a single generic failure.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveLoadError {
    /// The file is missing, unreadable, or not a valid game database.
    Corrupted,
    /// The save was written by a newer build than this one understands.
    IncompatibleVersion { save_version: i64, supported: i64 },
    /// The database opens, but required game data is missing or unreadable.
    MissingData,
    /// The save's embedded world / mod definition is invalid.
    InvalidWorld,
    /// An unexpected failure not covered by the cases above.
    Unknown,
}

impl SaveLoadError {
    /// The user-facing i18n key, with `?param=value` encoding where the message
    /// needs the offending version numbers.
    pub fn i18n_key(&self) -> String {
        match self {
            SaveLoadError::Corrupted => "be.error.saveLoad.corrupted".to_string(),
            SaveLoadError::IncompatibleVersion {
                save_version,
                supported,
            } => format!(
                "be.error.saveLoad.incompatibleVersion?saveVersion={save_version}&supported={supported}"
            ),
            SaveLoadError::MissingData => "be.error.saveLoad.missingData".to_string(),
            SaveLoadError::InvalidWorld => "be.error.saveLoad.invalidWorld".to_string(),
            SaveLoadError::Unknown => "be.error.saveLoad.unknown".to_string(),
        }
    }
}

impl fmt::Display for SaveLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.i18n_key())
    }
}

impl std::error::Error for SaveLoadError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn base_key(key: &str) -> String {
        key.split('?').next().unwrap_or(key).to_string()
    }

    #[test]
    fn every_category_maps_to_a_distinct_key() {
        let keys = [
            SaveLoadError::Corrupted.i18n_key(),
            SaveLoadError::IncompatibleVersion {
                save_version: 30,
                supported: 29,
            }
            .i18n_key(),
            SaveLoadError::MissingData.i18n_key(),
            SaveLoadError::InvalidWorld.i18n_key(),
            SaveLoadError::Unknown.i18n_key(),
        ];
        let unique: HashSet<String> = keys.iter().map(|k| base_key(k)).collect();
        assert_eq!(
            unique.len(),
            keys.len(),
            "each category must be distinguishable by key"
        );
        assert!(keys.iter().all(|k| k.starts_with("be.error.saveLoad.")));
    }

    #[test]
    fn incompatible_version_encodes_both_versions() {
        let key = SaveLoadError::IncompatibleVersion {
            save_version: 31,
            supported: 29,
        }
        .i18n_key();
        assert!(key.starts_with("be.error.saveLoad.incompatibleVersion?"));
        assert!(key.contains("saveVersion=31"));
        assert!(key.contains("supported=29"));
    }
}
