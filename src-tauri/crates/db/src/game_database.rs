use log::{debug, error, info};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

use crate::migrations::{MIGRATION_COUNT, all_migrations};
use crate::save_load_error::SaveLoadError;

const GAME_DATABASE_OPEN_FAILED: &str = "be.error.gameDatabase.openFailed";
const GAME_DATABASE_MIGRATION_FAILED: &str = "be.error.gameDatabase.migrationFailed";
const GAME_DATABASE_SCHEMA_VERSION_READ_FAILED: &str =
    "be.error.gameDatabase.schemaVersionReadFailed";
const GAME_DATABASE_CURRENT_VERSION_READ_FAILED: &str =
    "be.error.gameDatabase.currentVersionReadFailed";

/// Represents an open per-save game database with migrations applied.
pub struct GameDatabase {
    conn: Connection,
    path: Option<PathBuf>,
}

impl GameDatabase {
    /// Open (or create) a game database at the given path and apply all migrations.
    pub fn open(path: &Path) -> Result<Self, String> {
        debug!("[game_db] opening database at {:?}", path);
        let mut conn = Connection::open(path).map_err(|e| {
            error!("[game_db] failed to open database at {:?}: {}", path, e);
            GAME_DATABASE_OPEN_FAILED.to_string()
        })?;

        let migrations = all_migrations();
        migrations.to_latest(&mut conn).map_err(|e| {
            error!("[game_db] migration failed for {:?}: {}", path, e);
            GAME_DATABASE_MIGRATION_FAILED.to_string()
        })?;

        info!("[game_db] database ready at {:?}", path);
        Ok(Self {
            conn,
            path: Some(path.to_path_buf()),
        })
    }

    /// Open an existing save with structured error classification, so the UI
    /// can distinguish a corrupted file from one created by a newer build. Used
    /// by the save-loading path; other callers use [`GameDatabase::open`].
    pub fn open_save(path: &Path) -> Result<Self, SaveLoadError> {
        debug!("[game_db] opening save at {:?}", path);
        let mut conn = Connection::open(path).map_err(|e| {
            error!("[game_db] failed to open save at {:?}: {}", path, e);
            SaveLoadError::Corrupted
        })?;

        // A save from a newer build carries a higher schema version than we can
        // apply; detect it before migrating so the failure is explainable rather
        // than a generic migration error.
        let save_version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap_or(0);
        if save_version > MIGRATION_COUNT as i64 {
            return Err(SaveLoadError::IncompatibleVersion {
                save_version,
                supported: MIGRATION_COUNT as i64,
            });
        }

        let migrations = all_migrations();
        migrations.to_latest(&mut conn).map_err(|e| {
            error!("[game_db] migration failed for save {:?}: {}", path, e);
            SaveLoadError::Corrupted
        })?;

        info!("[game_db] save ready at {:?}", path);
        Ok(Self {
            conn,
            path: Some(path.to_path_buf()),
        })
    }

    /// Create an in-memory game database (useful for tests and pre-save state).
    pub fn open_in_memory() -> Result<Self, String> {
        debug!("[game_db] opening in-memory database");
        let mut conn = Connection::open_in_memory().map_err(|e| {
            error!("[game_db] failed to open in-memory database: {}", e);
            GAME_DATABASE_OPEN_FAILED.to_string()
        })?;

        let migrations = all_migrations();
        migrations.to_latest(&mut conn).map_err(|e| {
            error!("[game_db] migration failed for in-memory db: {}", e);
            GAME_DATABASE_MIGRATION_FAILED.to_string()
        })?;

        Ok(Self { conn, path: None })
    }

    /// Get a reference to the underlying connection (for repositories).
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Get the file path, if this is a file-backed database.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Get the current schema version (number of applied migrations).
    pub fn schema_version(&self) -> Result<i64, String> {
        self.conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| GAME_DATABASE_SCHEMA_VERSION_READ_FAILED.to_string())
    }

    /// Validate that the database has the expected schema version.
    /// Returns Ok(true) if valid, Ok(false) if version mismatch.
    pub fn validate_schema(&self) -> Result<bool, String> {
        let migrations = all_migrations();
        let current: usize = migrations
            .current_version(&self.conn)
            .map_err(|_| GAME_DATABASE_CURRENT_VERSION_READ_FAILED.to_string())?
            .into();
        // We expect the version to equal the number of migrations (1 for V1)
        let expected = MIGRATION_COUNT;
        Ok(current == expected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = GameDatabase::open_in_memory().unwrap();
        assert!(db.path().is_none());
        assert_eq!(
            db.schema_version().unwrap(),
            crate::migrations::MIGRATION_COUNT as i64
        );
    }

    #[test]
    fn test_open_file_database() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_game.db");

        let db = GameDatabase::open(&db_path).unwrap();
        assert_eq!(db.path().unwrap(), db_path);
        assert_eq!(
            db.schema_version().unwrap(),
            crate::migrations::MIGRATION_COUNT as i64
        );
        assert!(db.validate_schema().unwrap());
    }

    #[test]
    fn test_reopen_existing_database() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_reopen.db");

        // Create and close
        {
            let db = GameDatabase::open(&db_path).unwrap();
            assert_eq!(
                db.schema_version().unwrap(),
                crate::migrations::MIGRATION_COUNT as i64
            );
        }

        // Reopen — migrations should be idempotent
        let db = GameDatabase::open(&db_path).unwrap();
        assert_eq!(
            db.schema_version().unwrap(),
            crate::migrations::MIGRATION_COUNT as i64
        );
        assert!(db.validate_schema().unwrap());
    }

    #[test]
    fn test_validate_schema_on_empty_db() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("empty.db");

        // Create a raw DB without migrations
        {
            let _conn = Connection::open(&db_path).unwrap();
        }

        // Opening via GameDatabase applies migrations, so it should be valid
        let db = GameDatabase::open(&db_path).unwrap();
        assert!(db.validate_schema().unwrap());
    }

    #[test]
    fn test_conn_is_usable() {
        let db = GameDatabase::open_in_memory().unwrap();
        // Verify we can query a table created by the migration
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM teams", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_open_directory_path_returns_backend_key() {
        let dir = tempfile::tempdir().unwrap();

        let result = GameDatabase::open(dir.path());

        match result {
            Err(error) => assert_eq!(error, GAME_DATABASE_OPEN_FAILED),
            Ok(_) => panic!("expected directory open to fail"),
        }
    }

    #[test]
    fn open_save_rejects_a_future_version() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("future.db");
        {
            // A save written by a newer build: schema version above ours.
            let conn = Connection::open(&db_path).unwrap();
            conn.pragma_update(None, "user_version", (MIGRATION_COUNT + 5) as i64)
                .unwrap();
        }

        match GameDatabase::open_save(&db_path) {
            Err(SaveLoadError::IncompatibleVersion {
                save_version,
                supported,
            }) => {
                assert_eq!(save_version, (MIGRATION_COUNT + 5) as i64);
                assert_eq!(supported, MIGRATION_COUNT as i64);
            }
            Err(other) => panic!("expected IncompatibleVersion, got {other:?}"),
            Ok(_) => panic!("expected IncompatibleVersion, got an open database"),
        }
    }

    #[test]
    fn open_save_reports_corruption_for_a_non_database_file() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("corrupt.db");
        std::fs::write(&db_path, b"this is not a sqlite database").unwrap();

        match GameDatabase::open_save(&db_path) {
            Err(SaveLoadError::Corrupted) => {}
            Err(other) => panic!("expected Corrupted, got {other:?}"),
            Ok(_) => panic!("expected Corrupted, got an open database"),
        }
    }

    #[test]
    fn open_save_loads_a_current_save() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("current.db");
        // Create a fully-migrated save, then reopen it through open_save.
        GameDatabase::open(&db_path).unwrap();

        let db = GameDatabase::open_save(&db_path).expect("a current save should open");
        assert!(db.validate_schema().unwrap());
    }
}
