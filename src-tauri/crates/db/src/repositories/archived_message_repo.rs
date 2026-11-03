use domain::message::InboxMessage;
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Stores archived mail, in the caller's transaction. Insert-only by id: a message already
/// archived is never rewritten, whatever the payload now says.
pub fn archive_messages(conn: &Connection, archived: &[InboxMessage]) -> Result<(), String> {
    let mut insert = conn
        .prepare_cached(
            "INSERT OR IGNORE INTO archived_messages (message_id, date, record_json) VALUES (?1, ?2, ?3)",
        )
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    for message in archived {
        let record_json =
            serde_json::to_string(message).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        insert
            .execute(params![message.id, message.date, record_json])
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    }
    Ok(())
}

/// The whole archive, newest first.
pub fn load_archived_messages(conn: &Connection) -> Result<Vec<InboxMessage>, String> {
    let mut stmt = conn
        .prepare("SELECT record_json FROM archived_messages ORDER BY date DESC, message_id DESC")
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

    fn mail(id: &str, date: &str, body: &str) -> InboxMessage {
        InboxMessage::new(
            id.into(),
            "Subject".into(),
            body.into(),
            "Board".into(),
            date.into(),
        )
    }

    /// Given archived mail
    /// When it is stored and loaded
    /// Then it comes back whole and newest first.
    #[test]
    fn archived_mail_round_trips_newest_first() {
        let db = GameDatabase::open_in_memory().unwrap();
        archive_messages(
            db.conn(),
            &[
                mail("a", "2030-01-01", "old"),
                mail("b", "2030-06-01", "new"),
            ],
        )
        .unwrap();

        let loaded = load_archived_messages(db.conn()).unwrap();

        let ids: Vec<&str> = loaded.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["b", "a"]);
        assert_eq!(loaded[1].body, "old");
    }

    /// Given a message already archived
    /// When it is archived again with a different body
    /// Then the first record stands.
    #[test]
    fn an_archived_message_is_never_rewritten() {
        let db = GameDatabase::open_in_memory().unwrap();
        archive_messages(db.conn(), &[mail("a", "2030-01-01", "first")]).unwrap();

        archive_messages(db.conn(), &[mail("a", "2030-01-01", "second")]).unwrap();

        let loaded = load_archived_messages(db.conn()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].body, "first");
    }
}
