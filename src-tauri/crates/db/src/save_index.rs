use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use crate::game_database::GameDatabase;
use crate::repositories::meta_repo;

/// A single entry in the save index, representing one save session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveEntry {
    pub id: String,
    pub name: String,
    pub manager_name: String,
    pub db_filename: String,
    pub checksum: String,
    pub created_at: String,
    pub last_played_at: String,
}

/// The save index file that tracks all save sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveIndex {
    pub version: u32,
    pub saves: Vec<SaveEntry>,
}

impl SaveIndex {
    /// Create a new empty save index.
    pub fn new() -> Self {
        Self {
            version: 1,
            saves: Vec::new(),
        }
    }

    /// Add a save entry to the index.
    pub fn add(&mut self, entry: SaveEntry) {
        self.saves.push(entry);
    }

    /// Update an existing entry by id. Returns false if not found.
    pub fn update(&mut self, entry: &SaveEntry) -> bool {
        if let Some(existing) = self.saves.iter_mut().find(|e| e.id == entry.id) {
            existing.name = entry.name.clone();
            existing.manager_name = entry.manager_name.clone();
            existing.checksum = entry.checksum.clone();
            existing.last_played_at = entry.last_played_at.clone();
            true
        } else {
            false
        }
    }

    /// Remove a save entry by id. Returns true if removed.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.saves.len();
        self.saves.retain(|e| e.id != id);
        self.saves.len() < before
    }

    /// Find a save entry by id.
    pub fn find(&self, id: &str) -> Option<&SaveEntry> {
        self.saves.iter().find(|e| e.id == id)
    }
}

impl Default for SaveIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute SHA-256 checksum of a file. Returns hex string.
pub fn compute_checksum(path: &Path) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("Failed to read file for checksum: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(result.iter().map(|byte| format!("{:02x}", byte)).collect())
}

/// Load save index from a JSON file. Returns None if the file doesn't exist.
pub fn load_index(index_path: &Path) -> Result<Option<SaveIndex>, String> {
    if !index_path.exists() {
        debug!("[save_index] index file not found at {:?}", index_path);
        return Ok(None);
    }
    let data =
        fs::read_to_string(index_path).map_err(|e| format!("Failed to read save index: {}", e))?;
    let index: SaveIndex =
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse save index: {}", e))?;
    debug!("[save_index] loaded index with {} saves", index.saves.len());
    Ok(Some(index))
}

/// Write save index to a JSON file.
pub fn write_index(index_path: &Path, index: &SaveIndex) -> Result<(), String> {
    let data = serde_json::to_string_pretty(index)
        .map_err(|e| format!("Failed to serialize save index: {}", e))?;
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create index directory: {}", e))?;
    }
    fs::write(index_path, data).map_err(|e| format!("Failed to write save index: {}", e))?;
    debug!("[save_index] wrote index to {:?}", index_path);
    Ok(())
}

/// Result of validating a single database file during rebuild.
#[derive(Debug)]
pub enum DbValidation {
    /// Database is valid and has this metadata.
    Valid(SaveEntry),
    /// Database file is corrupted or has invalid schema.
    Invalid { filename: String, reason: String },
}

/// Rebuild the save index by scanning a directory for `.db` files,
/// validating each one, and constructing a new index.
pub fn rebuild_index(saves_dir: &Path) -> Result<(SaveIndex, Vec<DbValidation>), String> {
    info!(
        "[save_index] rebuilding index from directory {:?}",
        saves_dir
    );
    let mut index = SaveIndex::new();
    let mut validations = Vec::new();

    if !saves_dir.exists() {
        debug!("[save_index] saves directory does not exist, returning empty index");
        return Ok((index, validations));
    }

    let entries =
        fs::read_dir(saves_dir).map_err(|e| format!("Failed to read saves directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) if name.ends_with(".db") => name.to_string(),
            _ => continue,
        };

        debug!("[save_index] validating database: {}", filename);

        match validate_db_file(&path) {
            Ok(save_entry) => {
                info!("[save_index] valid database: {}", filename);
                index.add(save_entry.clone());
                validations.push(DbValidation::Valid(save_entry));
            }
            Err(reason) => {
                warn!("[save_index] invalid database {}: {}", filename, reason);
                validations.push(DbValidation::Invalid {
                    filename: filename.clone(),
                    reason,
                });
            }
        }
    }

    Ok((index, validations))
}

/// Validate a single `.db` file: check schema and extract metadata.
fn validate_db_file(path: &Path) -> Result<SaveEntry, String> {
    let db = GameDatabase::open(path)?;

    if !db.validate_schema()? {
        return Err("Schema version mismatch".to_string());
    }

    let meta = meta_repo::load_meta(db.conn())?
        .ok_or_else(|| "No game_meta found in database".to_string())?;

    let checksum = compute_checksum(path)?;

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.db")
        .to_string();

    Ok(SaveEntry {
        id: meta.save_id.clone(),
        name: meta.save_name,
        manager_name: String::new(), // Will be filled from managers table later
        db_filename: filename,
        checksum,
        created_at: meta.created_at,
        last_played_at: meta.last_played_at,
    })
}

/// Load or rebuild the save index.
/// If the index file exists, load it. Otherwise, rebuild from the saves directory.
pub fn load_or_rebuild_index(
    index_path: &Path,
    saves_dir: &Path,
) -> Result<(SaveIndex, Vec<DbValidation>), String> {
    if let Some(index) = load_index(index_path)? {
        return Ok((index, Vec::new()));
    }

    info!("[save_index] index file missing, rebuilding...");
    let (index, validations) = rebuild_index(saves_dir)?;
    write_index(index_path, &index)?;
    Ok((index, validations))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::meta_repo::{GameMeta, upsert_meta};

    #[test]
    fn test_save_index_new() {
        let index = SaveIndex::new();
        assert_eq!(index.version, 1);
        assert!(index.saves.is_empty());
    }

    #[test]
    fn test_save_index_add_and_find() {
        let mut index = SaveIndex::new();
        let entry = SaveEntry {
            id: "save-001".to_string(),
            name: "Test Career".to_string(),
            manager_name: "John Smith".to_string(),
            db_filename: "save-001.db".to_string(),
            checksum: "abc123".to_string(),
            created_at: "2026-01-01".to_string(),
            last_played_at: "2026-01-02".to_string(),
        };
        index.add(entry);

        assert_eq!(index.saves.len(), 1);
        let found = index.find("save-001").unwrap();
        assert_eq!(found.name, "Test Career");
    }

    #[test]
    fn test_save_index_update() {
        let mut index = SaveIndex::new();
        index.add(SaveEntry {
            id: "save-001".to_string(),
            name: "Old Name".to_string(),
            manager_name: "John".to_string(),
            db_filename: "save-001.db".to_string(),
            checksum: "old".to_string(),
            created_at: "2026-01-01".to_string(),
            last_played_at: "2026-01-01".to_string(),
        });

        let updated = index.update(&SaveEntry {
            id: "save-001".to_string(),
            name: "New Name".to_string(),
            manager_name: "John".to_string(),
            db_filename: "save-001.db".to_string(),
            checksum: "new".to_string(),
            created_at: "2026-01-01".to_string(),
            last_played_at: "2026-01-05".to_string(),
        });

        assert!(updated);
        assert_eq!(index.find("save-001").unwrap().name, "New Name");
        assert_eq!(index.find("save-001").unwrap().checksum, "new");
    }

    #[test]
    fn test_save_index_update_not_found() {
        let mut index = SaveIndex::new();
        let result = index.update(&SaveEntry {
            id: "nonexistent".to_string(),
            name: "x".to_string(),
            manager_name: "x".to_string(),
            db_filename: "x.db".to_string(),
            checksum: "x".to_string(),
            created_at: "x".to_string(),
            last_played_at: "x".to_string(),
        });
        assert!(!result);
    }

    #[test]
    fn test_save_index_remove() {
        let mut index = SaveIndex::new();
        index.add(SaveEntry {
            id: "save-001".to_string(),
            name: "Career".to_string(),
            manager_name: "John".to_string(),
            db_filename: "save-001.db".to_string(),
            checksum: "abc".to_string(),
            created_at: "2026-01-01".to_string(),
            last_played_at: "2026-01-01".to_string(),
        });
        assert!(index.remove("save-001"));
        assert!(index.saves.is_empty());
        assert!(!index.remove("save-001")); // already removed
    }

    #[test]
    fn test_save_index_serialization_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let index_path = dir.path().join("save_index.json");

        let mut index = SaveIndex::new();
        index.add(SaveEntry {
            id: "save-001".to_string(),
            name: "Career".to_string(),
            manager_name: "John".to_string(),
            db_filename: "save-001.db".to_string(),
            checksum: "abc123".to_string(),
            created_at: "2026-01-01".to_string(),
            last_played_at: "2026-01-02".to_string(),
        });

        write_index(&index_path, &index).unwrap();
        let loaded = load_index(&index_path).unwrap().unwrap();

        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.saves.len(), 1);
        assert_eq!(loaded.saves[0].id, "save-001");
        assert_eq!(loaded.saves[0].checksum, "abc123");
    }

    #[test]
    fn test_load_index_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let index_path = dir.path().join("nonexistent.json");
        let result = load_index(&index_path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_compute_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let checksum = compute_checksum(&file_path).unwrap();
        assert!(!checksum.is_empty());
        assert_eq!(checksum.len(), 64); // SHA-256 hex is 64 chars

        // Same content should produce same checksum
        let checksum2 = compute_checksum(&file_path).unwrap();
        assert_eq!(checksum, checksum2);
    }

    #[test]
    fn test_rebuild_index_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        fs::create_dir(&saves_dir).unwrap();

        let (index, validations) = rebuild_index(&saves_dir).unwrap();
        assert!(index.saves.is_empty());
        assert!(validations.is_empty());
    }

    #[test]
    fn test_rebuild_index_nonexistent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("nonexistent");

        let (index, validations) = rebuild_index(&saves_dir).unwrap();
        assert!(index.saves.is_empty());
        assert!(validations.is_empty());
    }

    #[test]
    fn test_rebuild_index_with_valid_db() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        fs::create_dir(&saves_dir).unwrap();

        // Create a valid game database with metadata
        let db_path = saves_dir.join("test-save.db");
        let db = GameDatabase::open(&db_path).unwrap();
        upsert_meta(
            db.conn(),
            &GameMeta {
                save_id: "test-save".to_string(),
                save_name: "Test Career".to_string(),
                manager_id: "mgr-001".to_string(),
                start_date: "2026-07-01".to_string(),
                game_date: "2026-08-01".to_string(),
                created_at: "2026-01-01".to_string(),
                last_played_at: "2026-01-02".to_string(),
                vacant_team_days_json: "{}".to_string(),
            },
        )
        .unwrap();
        drop(db); // Close the database

        let (index, validations) = rebuild_index(&saves_dir).unwrap();
        assert_eq!(index.saves.len(), 1);
        assert_eq!(index.saves[0].id, "test-save");
        assert_eq!(index.saves[0].name, "Test Career");
        assert!(!index.saves[0].checksum.is_empty());

        assert_eq!(validations.len(), 1);
        assert!(matches!(&validations[0], DbValidation::Valid(_)));
    }

    #[test]
    fn test_rebuild_index_with_invalid_db() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        fs::create_dir(&saves_dir).unwrap();

        // Create a DB file with valid schema but no metadata
        let db_path = saves_dir.join("empty.db");
        let _db = GameDatabase::open(&db_path).unwrap();
        drop(_db);

        let (index, validations) = rebuild_index(&saves_dir).unwrap();
        assert!(index.saves.is_empty());
        assert_eq!(validations.len(), 1);
        assert!(matches!(&validations[0], DbValidation::Invalid { .. }));
    }

    #[test]
    fn test_rebuild_index_with_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        fs::create_dir(&saves_dir).unwrap();

        // Create a corrupt .db file
        let db_path = saves_dir.join("corrupt.db");
        fs::write(&db_path, "not a sqlite database").unwrap();

        let (index, validations) = rebuild_index(&saves_dir).unwrap();
        assert!(index.saves.is_empty());
        assert_eq!(validations.len(), 1);
        assert!(matches!(&validations[0], DbValidation::Invalid { .. }));
    }

    #[test]
    fn test_rebuild_index_ignores_non_db_files() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        fs::create_dir(&saves_dir).unwrap();

        fs::write(saves_dir.join("readme.txt"), "not a db").unwrap();
        fs::write(saves_dir.join("data.json"), "{}").unwrap();

        let (index, validations) = rebuild_index(&saves_dir).unwrap();
        assert!(index.saves.is_empty());
        assert!(validations.is_empty());
    }

    #[test]
    fn test_load_or_rebuild_uses_existing_index() {
        let dir = tempfile::tempdir().unwrap();
        let index_path = dir.path().join("save_index.json");
        let saves_dir = dir.path().join("saves");

        let mut index = SaveIndex::new();
        index.add(SaveEntry {
            id: "existing".to_string(),
            name: "Existing".to_string(),
            manager_name: "John".to_string(),
            db_filename: "existing.db".to_string(),
            checksum: "abc".to_string(),
            created_at: "2026-01-01".to_string(),
            last_played_at: "2026-01-01".to_string(),
        });
        write_index(&index_path, &index).unwrap();

        let (loaded, validations) = load_or_rebuild_index(&index_path, &saves_dir).unwrap();
        assert_eq!(loaded.saves.len(), 1);
        assert_eq!(loaded.saves[0].id, "existing");
        assert!(validations.is_empty()); // No rebuild needed
    }

    #[test]
    fn test_load_or_rebuild_rebuilds_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let index_path = dir.path().join("save_index.json");
        let saves_dir = dir.path().join("saves");
        fs::create_dir(&saves_dir).unwrap();

        // Create a valid DB
        let db_path = saves_dir.join("my-save.db");
        let db = GameDatabase::open(&db_path).unwrap();
        upsert_meta(
            db.conn(),
            &GameMeta {
                save_id: "my-save".to_string(),
                save_name: "My Career".to_string(),
                manager_id: "mgr-001".to_string(),
                start_date: "2026-07-01".to_string(),
                game_date: "2026-08-01".to_string(),
                created_at: "2026-01-01".to_string(),
                last_played_at: "2026-01-02".to_string(),
                vacant_team_days_json: "{}".to_string(),
            },
        )
        .unwrap();
        drop(db);

        let (loaded, validations) = load_or_rebuild_index(&index_path, &saves_dir).unwrap();
        assert_eq!(loaded.saves.len(), 1);
        assert_eq!(loaded.saves[0].id, "my-save");
        assert_eq!(validations.len(), 1);

        // Index file should now exist
        assert!(index_path.exists());
    }
}
