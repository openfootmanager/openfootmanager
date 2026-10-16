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

/// The archive, with each record's `display_name` recomputed: a manager archived before they
/// could be known by a short name was stored without one, and serde would hand back `""`.
pub fn load_retired_managers(conn: &Connection) -> Result<Vec<Manager>, String> {
    let mut stmt = conn
        .prepare("SELECT record_json FROM retired_managers ORDER BY manager_id")
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
    stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?
        .map(|row| {
            let json = row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
            let mut manager: Manager =
                serde_json::from_str(&json).map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
            manager.refresh_display_name();
            Ok(manager)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;

    /// Given a manager archived before v057, whose record has no `display_name`
    /// When the archive is loaded
    /// Then they come back called by their first and last name.
    #[test]
    fn a_manager_archived_before_display_names_is_called_by_their_name() {
        let db = GameDatabase::open_in_memory().unwrap();
        let mut stored = serde_json::to_value(Manager::new(
            "mgr-old".into(),
            "Ada".into(),
            "Lovelace".into(),
            "1950-01-01".into(),
            "England".into(),
        ))
        .unwrap();
        stored.as_object_mut().unwrap().remove("display_name");
        db.conn()
            .execute(
                "INSERT INTO retired_managers (manager_id, record_json) VALUES (?1, ?2)",
                params!["mgr-old", stored.to_string()],
            )
            .unwrap();

        let loaded = load_retired_managers(db.conn()).unwrap();

        assert_eq!(loaded[0].display_name, "Ada Lovelace");
    }

    /// Given an archived manager whose record carries a blank known-as name
    /// When the archive is loaded
    /// Then they are called by their full name and keep no known-as.
    #[test]
    fn an_archived_manager_with_a_blank_known_as_is_called_by_their_name() {
        let db = GameDatabase::open_in_memory().unwrap();
        let mut stored = serde_json::to_value(Manager::new(
            "mgr-old".into(),
            "Ada".into(),
            "Lovelace".into(),
            "1950-01-01".into(),
            "England".into(),
        ))
        .unwrap();
        stored["known_as"] = serde_json::json!("   ");
        db.conn()
            .execute(
                "INSERT INTO retired_managers (manager_id, record_json) VALUES (?1, ?2)",
                params!["mgr-old", stored.to_string()],
            )
            .unwrap();

        let loaded = load_retired_managers(db.conn()).unwrap();

        assert_eq!(loaded[0].display_name, "Ada Lovelace");
        assert_eq!(loaded[0].known_as, None);
    }
}
