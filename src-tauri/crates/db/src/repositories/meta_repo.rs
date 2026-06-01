use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Game metadata stored as a singleton row in `game_meta`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMeta {
    pub save_id: String,
    pub save_name: String,
    pub manager_id: String,
    pub start_date: String,
    pub game_date: String,
    pub created_at: String,
    pub last_played_at: String,
    #[serde(default = "default_vacant_team_days_json")]
    pub vacant_team_days_json: String,
    #[serde(default = "default_world_history_json")]
    pub world_history_json: String,
}

fn default_vacant_team_days_json() -> String {
    "{}".to_string()
}

fn default_world_history_json() -> String {
    "{}".to_string()
}

/// Insert or replace the singleton game_meta row.
pub fn upsert_meta(conn: &Connection, meta: &GameMeta) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO game_meta (id, save_id, save_name, manager_id, start_date, game_date, created_at, last_played_at, vacant_team_days_json, world_history_json)
         VALUES ('singleton', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            meta.save_id,
            meta.save_name,
            meta.manager_id,
            meta.start_date,
            meta.game_date,
            meta.created_at,
            meta.last_played_at,
            meta.vacant_team_days_json,
            meta.world_history_json,
        ],
    )
    .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    Ok(())
}

/// Load the singleton game_meta row. Returns None if no meta exists.
pub fn load_meta(conn: &Connection) -> Result<Option<GameMeta>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT save_id, save_name, manager_id, start_date, game_date, created_at, last_played_at, vacant_team_days_json, world_history_json
             FROM game_meta WHERE id = 'singleton'",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut rows = stmt
        .query_map([], |row| {
            Ok(GameMeta {
                save_id: row.get(0)?,
                save_name: row.get(1)?,
                manager_id: row.get(2)?,
                start_date: row.get(3)?,
                game_date: row.get(4)?,
                created_at: row.get(5)?,
                last_played_at: row.get(6)?,
                vacant_team_days_json: row.get(7)?,
                world_history_json: row.get(8)?,
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    match rows.next() {
        Some(Ok(meta)) => Ok(Some(meta)),
        Some(Err(_)) => Err(GAME_PERSISTENCE_LOAD_ERROR.to_string()),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;

    fn test_db() -> GameDatabase {
        GameDatabase::open_in_memory().unwrap()
    }

    #[test]
    fn test_upsert_and_load_meta() {
        let db = test_db();
        let meta = GameMeta {
            save_id: "save-001".to_string(),
            save_name: "Test Career".to_string(),
            manager_id: "mgr_user".to_string(),
            start_date: "2026-07-01T00:00:00Z".to_string(),
            game_date: "2026-07-15T00:00:00Z".to_string(),
            created_at: "2026-03-05T18:00:00Z".to_string(),
            last_played_at: "2026-03-05T19:00:00Z".to_string(),
            vacant_team_days_json: "{}".to_string(),
            world_history_json: "{}".to_string(),
        };

        upsert_meta(db.conn(), &meta).unwrap();
        let loaded = load_meta(db.conn()).unwrap().unwrap();

        assert_eq!(loaded.save_id, "save-001");
        assert_eq!(loaded.save_name, "Test Career");
        assert_eq!(loaded.manager_id, "mgr_user");
        assert_eq!(loaded.game_date, "2026-07-15T00:00:00Z");
        assert_eq!(loaded.world_history_json, "{}");
    }

    #[test]
    fn test_load_meta_empty() {
        let db = test_db();
        let loaded = load_meta(db.conn()).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_upsert_meta_overwrites() {
        let db = test_db();
        let meta1 = GameMeta {
            save_id: "save-001".to_string(),
            save_name: "Career v1".to_string(),
            manager_id: "mgr_user".to_string(),
            start_date: "2026-07-01T00:00:00Z".to_string(),
            game_date: "2026-07-15T00:00:00Z".to_string(),
            created_at: "2026-03-05T18:00:00Z".to_string(),
            last_played_at: "2026-03-05T19:00:00Z".to_string(),
            vacant_team_days_json: "{}".to_string(),
            world_history_json: "{}".to_string(),
        };
        upsert_meta(db.conn(), &meta1).unwrap();

        let meta2 = GameMeta {
            save_id: "save-001".to_string(),
            save_name: "Career v2".to_string(),
            manager_id: "mgr_user".to_string(),
            start_date: "2026-07-01T00:00:00Z".to_string(),
            game_date: "2026-08-01T00:00:00Z".to_string(),
            created_at: "2026-03-05T18:00:00Z".to_string(),
            last_played_at: "2026-03-06T10:00:00Z".to_string(),
            vacant_team_days_json: "{}".to_string(),
            world_history_json: r#"{"rivalries":[{"team_a_id":"team-1","team_b_id":"team-2","intensity":80}],"season_awards":[]}"#.to_string(),
        };
        upsert_meta(db.conn(), &meta2).unwrap();

        let loaded = load_meta(db.conn()).unwrap().unwrap();
        assert_eq!(loaded.save_name, "Career v2");
        assert_eq!(loaded.game_date, "2026-08-01T00:00:00Z");
        assert!(loaded.world_history_json.contains("rivalries"));
    }

    #[test]
    fn test_upsert_meta_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let meta = GameMeta {
            save_id: "save-001".to_string(),
            save_name: "Test Career".to_string(),
            manager_id: "mgr_user".to_string(),
            start_date: "2026-07-01T00:00:00Z".to_string(),
            game_date: "2026-07-15T00:00:00Z".to_string(),
            created_at: "2026-03-05T18:00:00Z".to_string(),
            last_played_at: "2026-03-05T19:00:00Z".to_string(),
            vacant_team_days_json: "{}".to_string(),
            world_history_json: "{}".to_string(),
        };

        let result = upsert_meta(&conn, &meta);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_meta_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_meta(&conn);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }
}
