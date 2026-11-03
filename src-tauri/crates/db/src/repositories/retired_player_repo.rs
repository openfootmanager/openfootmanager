use domain::retired_player::RetiredPlayer;
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Archives retirees and removes their active rows, in the caller's transaction so the two
/// halves commit together or not at all. Insert-only by id: a record already archived is
/// never rewritten, whatever the payload now says.
pub fn archive_retired_players(conn: &Connection, retired: &[RetiredPlayer]) -> Result<(), String> {
    let records = retired
        .iter()
        .map(|player| {
            serde_json::to_string(player).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (player, record_json) in retired.iter().zip(records) {
        conn.execute(
            "INSERT OR IGNORE INTO retired_players (player_id, record_json) VALUES (?1, ?2)",
            params![player.id, record_json],
        )
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        conn.execute("DELETE FROM players WHERE id = ?1", params![player.id])
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    }
    Ok(())
}

/// Reads the whole archive back, ordered by id so a load is deterministic. A record that no
/// longer parses fails the load rather than silently dropping a person.
pub fn load_retired_players(conn: &Connection) -> Result<Vec<RetiredPlayer>, String> {
    let mut stmt = conn
        .prepare("SELECT record_json FROM retired_players ORDER BY player_id")
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
    stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?
        .map(|row| {
            let json = row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
            serde_json::from_str(&json).map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())
        })
        .collect()
}
