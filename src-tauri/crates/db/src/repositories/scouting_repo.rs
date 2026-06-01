use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Mirrors ofm_core::game::ScoutingAssignment but avoids coupling db to ofm_core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoutingAssignmentRow {
    pub id: String,
    pub scout_id: String,
    pub player_id: String,
    pub days_remaining: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouthScoutingAssignmentRow {
    pub id: String,
    pub scout_id: String,
    pub region: String,
    pub objective: String,
    pub target_position: Option<String>,
    pub days_remaining: u32,
}

/// Insert or replace a scouting assignment row.
pub fn upsert_scouting(conn: &Connection, sa: &ScoutingAssignmentRow) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO scouting_assignments (id, scout_id, player_id, days_remaining)
         VALUES (?1, ?2, ?3, ?4)",
        params![sa.id, sa.scout_id, sa.player_id, sa.days_remaining],
    )
    .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    Ok(())
}

/// Replace all scouting assignments (clear + re-insert).
pub fn upsert_scouting_list(
    conn: &Connection,
    assignments: &[ScoutingAssignmentRow],
) -> Result<(), String> {
    conn.execute("DELETE FROM scouting_assignments", [])
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    for sa in assignments {
        upsert_scouting(conn, sa)?;
    }
    Ok(())
}

/// Load all scouting assignments.
pub fn load_all_scouting(conn: &Connection) -> Result<Vec<ScoutingAssignmentRow>, String> {
    let mut stmt = conn
        .prepare("SELECT id, scout_id, player_id, days_remaining FROM scouting_assignments")
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ScoutingAssignmentRow {
                id: row.get(0)?,
                scout_id: row.get(1)?,
                player_id: row.get(2)?,
                days_remaining: row.get(3)?,
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut assignments = Vec::new();
    for row in rows {
        assignments.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }
    Ok(assignments)
}

pub fn upsert_youth_scouting(
    conn: &Connection,
    assignment: &YouthScoutingAssignmentRow,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO youth_scouting_assignments (id, scout_id, region, objective, target_position, days_remaining)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            assignment.id,
            assignment.scout_id,
            assignment.region,
            assignment.objective,
            assignment.target_position,
            assignment.days_remaining,
        ],
    )
    .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    Ok(())
}

pub fn upsert_youth_scouting_list(
    conn: &Connection,
    assignments: &[YouthScoutingAssignmentRow],
) -> Result<(), String> {
    conn.execute("DELETE FROM youth_scouting_assignments", [])
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    for assignment in assignments {
        upsert_youth_scouting(conn, assignment)?;
    }
    Ok(())
}

pub fn load_all_youth_scouting(
    conn: &Connection,
) -> Result<Vec<YouthScoutingAssignmentRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, scout_id, region, objective, target_position, days_remaining FROM youth_scouting_assignments",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(YouthScoutingAssignmentRow {
                id: row.get(0)?,
                scout_id: row.get(1)?,
                region: row.get(2)?,
                objective: row.get(3)?,
                target_position: row.get(4)?,
                days_remaining: row.get(5)?,
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut assignments = Vec::new();
    for row in rows {
        assignments.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }
    Ok(assignments)
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
    fn test_upsert_and_load_scouting() {
        let db = test_db();
        let assignments = vec![
            ScoutingAssignmentRow {
                id: "sa-001".to_string(),
                scout_id: "scout-001".to_string(),
                player_id: "p-001".to_string(),
                days_remaining: 7,
            },
            ScoutingAssignmentRow {
                id: "sa-002".to_string(),
                scout_id: "scout-001".to_string(),
                player_id: "p-002".to_string(),
                days_remaining: 14,
            },
        ];

        upsert_scouting_list(db.conn(), &assignments).unwrap();
        let loaded = load_all_scouting(db.conn()).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].days_remaining, 7);
    }

    #[test]
    fn test_upsert_scouting_clears_old() {
        let db = test_db();
        let old = vec![ScoutingAssignmentRow {
            id: "sa-old".to_string(),
            scout_id: "s-1".to_string(),
            player_id: "p-1".to_string(),
            days_remaining: 3,
        }];
        upsert_scouting_list(db.conn(), &old).unwrap();

        let new = vec![ScoutingAssignmentRow {
            id: "sa-new".to_string(),
            scout_id: "s-2".to_string(),
            player_id: "p-2".to_string(),
            days_remaining: 10,
        }];
        upsert_scouting_list(db.conn(), &new).unwrap();

        let loaded = load_all_scouting(db.conn()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "sa-new");
    }

    #[test]
    fn test_load_empty_scouting() {
        let db = test_db();
        let loaded = load_all_scouting(db.conn()).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_upsert_and_load_youth_scouting() {
        let db = test_db();
        let assignments = vec![YouthScoutingAssignmentRow {
            id: "ysa-001".to_string(),
            scout_id: "scout-001".to_string(),
            region: "Domestic".to_string(),
            objective: "Balanced".to_string(),
            target_position: Some("Defender".to_string()),
            days_remaining: 5,
        }];

        upsert_youth_scouting_list(db.conn(), &assignments).unwrap();
        let loaded = load_all_youth_scouting(db.conn()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].region, "Domestic");
        assert_eq!(loaded[0].objective, "Balanced");
        assert_eq!(loaded[0].target_position.as_deref(), Some("Defender"));
        assert_eq!(loaded[0].days_remaining, 5);
    }

    #[test]
    fn test_upsert_scouting_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let assignments = vec![ScoutingAssignmentRow {
            id: "sa-001".to_string(),
            scout_id: "scout-001".to_string(),
            player_id: "p-001".to_string(),
            days_remaining: 7,
        }];

        let result = upsert_scouting_list(&conn, &assignments);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_scouting_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_all_scouting(&conn);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }

    #[test]
    fn test_upsert_youth_scouting_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let assignments = vec![YouthScoutingAssignmentRow {
            id: "ysa-001".to_string(),
            scout_id: "scout-001".to_string(),
            region: "Domestic".to_string(),
            objective: "Balanced".to_string(),
            target_position: Some("Defender".to_string()),
            days_remaining: 5,
        }];

        let result = upsert_youth_scouting_list(&conn, &assignments);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_youth_scouting_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_all_youth_scouting(&conn);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }
}
