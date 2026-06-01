use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Mirrors ofm_core::game::BoardObjective but avoids coupling db to ofm_core.
/// Conversion happens in the save_manager layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardObjectiveRow {
    pub id: String,
    pub description: String,
    pub target: u32,
    pub objective_type: String,
    pub met: bool,
}

/// Insert or replace a board objective row.
pub fn upsert_objective(conn: &Connection, obj: &BoardObjectiveRow) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO board_objectives (id, description, target, objective_type, met)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            obj.id,
            obj.description,
            obj.target,
            obj.objective_type,
            obj.met as i32
        ],
    )
    .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    Ok(())
}

/// Insert or replace multiple objectives.
pub fn upsert_objectives(
    conn: &Connection,
    objectives: &[BoardObjectiveRow],
) -> Result<(), String> {
    // Clear existing then re-insert for clean state
    conn.execute("DELETE FROM board_objectives", [])
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    for obj in objectives {
        upsert_objective(conn, obj)?;
    }
    Ok(())
}

/// Load all board objectives.
pub fn load_all_objectives(conn: &Connection) -> Result<Vec<BoardObjectiveRow>, String> {
    let mut stmt = conn
        .prepare("SELECT id, description, target, objective_type, met FROM board_objectives")
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let met_int: i32 = row.get(4)?;
            Ok(BoardObjectiveRow {
                id: row.get(0)?,
                description: row.get(1)?,
                target: row.get(2)?,
                objective_type: row.get(3)?,
                met: met_int != 0,
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut objectives = Vec::new();
    for row in rows {
        objectives.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }
    Ok(objectives)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use rusqlite::Connection;

    fn test_db() -> GameDatabase {
        GameDatabase::open_in_memory().unwrap()
    }

    #[test]
    fn test_upsert_and_load_objectives() {
        let db = test_db();
        let objs = vec![
            BoardObjectiveRow {
                id: "obj-001".to_string(),
                description: "Finish top 4".to_string(),
                target: 4,
                objective_type: "LeaguePosition".to_string(),
                met: false,
            },
            BoardObjectiveRow {
                id: "obj-002".to_string(),
                description: "Win 15 matches".to_string(),
                target: 15,
                objective_type: "Wins".to_string(),
                met: true,
            },
        ];

        upsert_objectives(db.conn(), &objs).unwrap();
        let loaded = load_all_objectives(db.conn()).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "obj-001");
        assert!(!loaded[0].met);
        assert!(loaded[1].met);
    }

    #[test]
    fn test_upsert_objectives_clears_old() {
        let db = test_db();
        let objs1 = vec![BoardObjectiveRow {
            id: "obj-001".to_string(),
            description: "Old".to_string(),
            target: 1,
            objective_type: "Wins".to_string(),
            met: false,
        }];
        upsert_objectives(db.conn(), &objs1).unwrap();

        let objs2 = vec![BoardObjectiveRow {
            id: "obj-002".to_string(),
            description: "New".to_string(),
            target: 2,
            objective_type: "Wins".to_string(),
            met: false,
        }];
        upsert_objectives(db.conn(), &objs2).unwrap();

        let loaded = load_all_objectives(db.conn()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "obj-002");
    }

    #[test]
    fn test_load_empty_objectives() {
        let db = test_db();
        let loaded = load_all_objectives(db.conn()).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_upsert_objectives_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let objectives = vec![BoardObjectiveRow {
            id: "obj-001".to_string(),
            description: "Finish top 4".to_string(),
            target: 4,
            objective_type: "LeaguePosition".to_string(),
            met: false,
        }];

        let result = upsert_objectives(&conn, &objectives);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_objectives_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_all_objectives(&conn);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }
}
