use domain::custom_tactic::CustomTactic;
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Makes the stored tactics exactly `tactics`, in order: the player can delete and reorder, so
/// the table is rewritten rather than appended to. Runs in the caller's transaction.
///
/// The id is the table's primary key, so one id twice in the list would fail the whole save.
/// Saving through `ofm_core::custom_tactics` already replaces by id; a list that arrives with a
/// repeat (an edited or merged save) keeps its last copy at the place it last stood.
pub fn replace_custom_tactics(conn: &Connection, tactics: &[CustomTactic]) -> Result<(), String> {
    let tactics: Vec<&CustomTactic> = tactics
        .iter()
        .enumerate()
        .filter(|(at, tactic)| !tactics[at + 1..].iter().any(|later| later.id == tactic.id))
        .map(|(_, tactic)| tactic)
        .collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;

    fn tactic(id: &str, name: &str) -> CustomTactic {
        CustomTactic {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            formation: "4-4-2".into(),
            play_style: "Balanced".into(),
            source_preset_name: None,
        }
    }

    /// Given a list that holds one id twice
    /// When it is stored
    /// Then the save succeeds and keeps the last copy, in the place it last stood.
    #[test]
    fn a_repeated_id_is_stored_once_as_its_last_copy() {
        let db = GameDatabase::open_in_memory().unwrap();
        let list = [tactic("a", "old"), tactic("b", "Other"), tactic("a", "new")];

        replace_custom_tactics(db.conn(), &list).unwrap();

        let names: Vec<String> = load_custom_tactics(db.conn())
            .unwrap()
            .into_iter()
            .map(|t| t.name)
            .collect();
        assert_eq!(names, ["Other", "new"]);
    }
}
