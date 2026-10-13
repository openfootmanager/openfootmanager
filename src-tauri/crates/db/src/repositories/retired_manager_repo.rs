use domain::manager::Manager;
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Archives retired managers and removes their active rows, in the caller's transaction so the
/// two halves commit together or not at all. Insert-only by id: a record already archived is
/// never rewritten, whatever the payload now says.
pub fn archive_retired_managers(conn: &Connection, retired: &[Manager]) -> Result<(), String> {
    let records = retired
        .iter()
        .map(|manager| {
            serde_json::to_string(manager).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (manager, record_json) in retired.iter().zip(records) {
        conn.execute(
            "INSERT OR IGNORE INTO retired_managers (manager_id, record_json) VALUES (?1, ?2)",
            params![manager.id, record_json],
        )
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        conn.execute("DELETE FROM managers WHERE id = ?1", params![manager.id])
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    }
    Ok(())
}

pub fn load_retired_managers(conn: &Connection) -> Result<Vec<Manager>, String> {
    let mut stmt = conn
        .prepare("SELECT record_json FROM retired_managers ORDER BY manager_id")
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
    stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?
        .map(|row| {
            let json = row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
            serde_json::from_str(&json).map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())
        })
        .collect()
}
