use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Insert or replace a message row.
pub fn upsert_message(conn: &Connection, msg: &InboxMessage) -> Result<(), String> {
    let actions_json =
        serde_json::to_string(&msg.actions).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let context_json =
        serde_json::to_string(&msg.context).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

    // Pack all i18n fields into a single JSON object
    let i18n = serde_json::json!({
        "subject_key": msg.subject_key,
        "body_key": msg.body_key,
        "sender_key": msg.sender_key,
        "sender_role_key": msg.sender_role_key,
        "i18n_params": msg.i18n_params,
    });
    let i18n_json =
        serde_json::to_string(&i18n).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

    let category_str = format!("{:?}", msg.category);
    let priority_str = format!("{:?}", msg.priority);

    conn.execute(
        "INSERT OR REPLACE INTO messages
         (id, subject, body, sender, sender_role, date, read, category, priority, actions, context, i18n)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            msg.id,
            msg.subject,
            msg.body,
            msg.sender,
            msg.sender_role,
            msg.date,
            msg.read as i32,
            category_str,
            priority_str,
            actions_json,
            context_json,
            i18n_json,
        ],
    )
    .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    Ok(())
}

/// Insert or replace multiple messages.
pub fn upsert_messages(conn: &Connection, messages: &[InboxMessage]) -> Result<(), String> {
    for msg in messages {
        upsert_message(conn, msg)?;
    }
    Ok(())
}

fn parse_category(s: &str) -> MessageCategory {
    match s {
        "Welcome" => MessageCategory::Welcome,
        "LeagueInfo" => MessageCategory::LeagueInfo,
        "MatchPreview" => MessageCategory::MatchPreview,
        "MatchResult" => MessageCategory::MatchResult,
        "Transfer" => MessageCategory::Transfer,
        "BoardDirective" => MessageCategory::BoardDirective,
        "PlayerMorale" => MessageCategory::PlayerMorale,
        "Injury" => MessageCategory::Injury,
        "Training" => MessageCategory::Training,
        "Finance" => MessageCategory::Finance,
        "Contract" => MessageCategory::Contract,
        "ScoutReport" => MessageCategory::ScoutReport,
        "Media" => MessageCategory::Media,
        _ => MessageCategory::System,
    }
}

fn parse_priority(s: &str) -> MessagePriority {
    match s {
        "Low" => MessagePriority::Low,
        "High" => MessagePriority::High,
        "Urgent" => MessagePriority::Urgent,
        _ => MessagePriority::Normal,
    }
}

/// Load all messages ordered by date descending.
pub fn load_all_messages(conn: &Connection) -> Result<Vec<InboxMessage>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, subject, body, sender, sender_role, date, read, category, priority, actions, context, i18n
             FROM messages ORDER BY date DESC",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let rows = stmt
        .query_map([], row_to_message)
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut messages = Vec::new();
    for row in rows {
        messages.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }
    Ok(messages)
}

fn row_to_message(row: &rusqlite::Row) -> rusqlite::Result<InboxMessage> {
    let read_int: i32 = row.get(6)?;
    let category_str: String = row.get(7)?;
    let priority_str: String = row.get(8)?;
    let actions_json: String = row.get(9)?;
    let context_json: String = row.get(10)?;
    let i18n_json: String = row.get(11)?;

    let i18n: serde_json::Value = serde_json::from_str(&i18n_json).unwrap_or_default();

    Ok(InboxMessage {
        id: row.get(0)?,
        subject: row.get(1)?,
        body: row.get(2)?,
        sender: row.get(3)?,
        sender_role: row.get(4)?,
        date: row.get(5)?,
        read: read_int != 0,
        category: parse_category(&category_str),
        priority: parse_priority(&priority_str),
        actions: serde_json::from_str(&actions_json).unwrap_or_default(),
        context: serde_json::from_str(&context_json).unwrap_or_default(),
        subject_key: i18n
            .get("subject_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        body_key: i18n
            .get("body_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        sender_key: i18n
            .get("sender_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        sender_role_key: i18n
            .get("sender_role_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        i18n_params: i18n
            .get("i18n_params")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use std::collections::HashMap;

    fn test_db() -> GameDatabase {
        GameDatabase::open_in_memory().unwrap()
    }

    fn sample_message(id: &str) -> InboxMessage {
        InboxMessage::new(
            id.to_string(),
            "Welcome".to_string(),
            "Welcome to the club!".to_string(),
            "Board".to_string(),
            "2026-07-01".to_string(),
        )
        .with_category(MessageCategory::Welcome)
        .with_priority(MessagePriority::High)
    }

    #[test]
    fn test_upsert_and_load_message() {
        let db = test_db();
        let msg = sample_message("msg-001");

        upsert_message(db.conn(), &msg).unwrap();
        let all = load_all_messages(db.conn()).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "msg-001");
        assert_eq!(all[0].subject, "Welcome");
        assert_eq!(all[0].category, MessageCategory::Welcome);
        assert_eq!(all[0].priority, MessagePriority::High);
        assert!(!all[0].read);
    }

    #[test]
    fn test_upsert_messages_batch() {
        let db = test_db();
        let msgs = vec![
            sample_message("msg-001"),
            sample_message("msg-002"),
            sample_message("msg-003"),
        ];

        upsert_messages(db.conn(), &msgs).unwrap();
        let all = load_all_messages(db.conn()).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_message_read_flag_roundtrip() {
        let db = test_db();
        let mut msg = sample_message("msg-001");
        msg.read = true;

        upsert_message(db.conn(), &msg).unwrap();
        let all = load_all_messages(db.conn()).unwrap();
        assert!(all[0].read);
    }

    #[test]
    fn test_message_i18n_roundtrip() {
        let db = test_db();
        let mut params = HashMap::new();
        params.insert("team".to_string(), "London FC".to_string());
        let msg =
            sample_message("msg-001").with_i18n("msg.welcome.subject", "msg.welcome.body", params);

        upsert_message(db.conn(), &msg).unwrap();
        let all = load_all_messages(db.conn()).unwrap();
        assert_eq!(all[0].subject_key.as_deref(), Some("msg.welcome.subject"));
        assert_eq!(all[0].body_key.as_deref(), Some("msg.welcome.body"));
        assert_eq!(
            all[0].i18n_params.get("team").map(|s| s.as_str()),
            Some("London FC")
        );
    }

    #[test]
    fn test_upsert_message_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let message = sample_message("msg-001");

        let result = upsert_message(&conn, &message);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_messages_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_all_messages(&conn);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }
}
