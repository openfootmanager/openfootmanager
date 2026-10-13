use domain::custom_tactic::CustomTactic;
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Makes the stored tactics exactly `tactics`, in order: the player can delete and reorder, so
/// the table is rewritten rather than appended to. Runs in the caller's transaction.
pub fn replace_custom_tactics(conn: &Connection, tactics: &[CustomTactic]) -> Result<(), String> {
    let records = tactics
        .iter()
        .map(|tactic| {
            serde_json::to_string(tactic).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    conn.execute("DELETE FROM custom_tactics", [])
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    for (position, (tactic, record_json)) in tactics.iter().zip(records).enumerate() {
        conn.execute(
            "INSERT INTO custom_tactics (tactic_id, position, record_json) VALUES (?1, ?2, ?3)",
            params![tactic.id, position as i64, record_json],
        )
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    }
    Ok(())
}

pub fn load_custom_tactics(conn: &Connection) -> Result<Vec<CustomTactic>, String> {
    let mut stmt = conn
        .prepare("SELECT record_json FROM custom_tactics ORDER BY position")
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
    stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?
        .map(|row| {
            let json = row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
            serde_json::from_str(&json).map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())
        })
        .collect()
}
