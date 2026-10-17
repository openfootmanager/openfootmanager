use domain::custom_tactic::CustomTactic;
use ofm_core::custom_tactics::one_tactic_per_id;
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Makes the stored tactics exactly `tactics`, in order: the player can delete and reorder, so
/// the table is rewritten rather than appended to. Runs in the caller's transaction.
///
/// The id is the table's primary key, so one id twice in the list would fail the whole save.
/// Saving through `ofm_core::custom_tactics` already replaces by id; a list that arrives with a
/// repeat (an edited or merged save, or ids kept before they were trimmed) is reduced by
/// `one_tactic_per_id`, which keeps the last copy at the place it last stood.
pub fn replace_custom_tactics(conn: &Connection, tactics: &[CustomTactic]) -> Result<(), String> {
    let tactics = one_tactic_per_id(tactics.to_vec());
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
    // A save written before ids were trimmed may hold "a" and "a " as two rows.
    stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?
        .map(|row| {
            let json = row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
            serde_json::from_str(&json).map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())
        })
        .collect::<Result<Vec<_>, _>>()
        .map(one_tactic_per_id)
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

    /// Given a save whose table holds the tactic "a" and a padded twin "a "
    ///       (rows written before ids were trimmed)
    /// When it is loaded
    /// Then the player has one tactic, under the trimmed id, as the core treats them as one.
    #[test]
    fn rows_whose_ids_trim_to_the_same_id_load_as_one_tactic() {
        let db = GameDatabase::open_in_memory().unwrap();
        for (position, t) in [
            tactic("a", "old"),
            tactic("b", "Other"),
            tactic("a ", "new"),
        ]
        .iter()
        .enumerate()
        {
            db.conn()
                .execute(
                    "INSERT INTO custom_tactics (tactic_id, position, record_json) VALUES (?1, ?2, ?3)",
                    params![t.id, position as i64, serde_json::to_string(t).unwrap()],
                )
                .unwrap();
        }

        let loaded = load_custom_tactics(db.conn()).unwrap();

        let seen: Vec<(&str, &str)> = loaded
            .iter()
            .map(|t| (t.id.as_str(), t.name.as_str()))
            .collect();
        assert_eq!(seen, [("b", "Other"), ("a", "new")]);
    }

    /// Given a list holding "a" and a padded twin "a "
    /// When it is stored
    /// Then one row is written, under the trimmed id, so the table never holds both.
    #[test]
    fn ids_that_trim_to_the_same_id_are_stored_once() {
        let db = GameDatabase::open_in_memory().unwrap();

        replace_custom_tactics(db.conn(), &[tactic("a", "old"), tactic("a ", "new")]).unwrap();

        let ids: Vec<String> = db
            .conn()
            .prepare("SELECT tactic_id FROM custom_tactics ORDER BY position")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(ids, ["a"]);
    }
}
