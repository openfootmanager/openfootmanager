use domain::finance::{
    CashKind, CashPost, CashPostMeta, ReservationKind, ReservationStatus, TransferReservation,
};
use ofm_core::game::Game;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

const WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";
const LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";

fn encode_enum<T: Serialize>(value: &T) -> Result<String, String> {
    match serde_json::to_value(value).map_err(|_| WRITE_ERROR.to_string())? {
        serde_json::Value::String(text) => Ok(text),
        _ => Err(WRITE_ERROR.to_string()),
    }
}

fn decode_enum<T: DeserializeOwned>(text: String, column: usize) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(text)).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Text,
            Box::new(err),
        )
    })
}

/// Insert posts whose ids are not already stored. Collision is a no-op.
pub fn insert_cash_posts(conn: &Connection, posts: &[CashPost]) -> Result<usize, String> {
    if posts.is_empty() {
        return Ok(0);
    }
    // The caller (`write_game_to_connection`) already holds a transaction.
    let mut stmt = conn
        .prepare(
            "INSERT OR IGNORE INTO cash_journal
             (id, club_id, amount, kind, date, envelope_generation, meta_json, reverses_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .map_err(|_| WRITE_ERROR.to_string())?;
    let mut inserted = 0;
    for post in posts {
        let kind = encode_enum(&post.kind)?;
        let meta_json = serde_json::to_string(&post.meta).map_err(|_| WRITE_ERROR.to_string())?;
        inserted += stmt
            .execute(params![
                post.id,
                post.club_id,
                post.amount,
                kind,
                post.date,
                post.envelope_generation,
                meta_json,
                post.reverses_id,
            ])
            .map_err(|_| WRITE_ERROR.to_string())?;
    }
    Ok(inserted)
}

/// Persist the dirty-id hint from `game`. Does not clear the hint (`&Game`).
pub fn insert_dirty_cash_posts(conn: &Connection, game: &Game) -> Result<usize, String> {
    if game.cash_journal_dirty_ids.is_empty() {
        return Ok(0);
    }
    let wanted: std::collections::HashSet<&str> = game
        .cash_journal_dirty_ids
        .iter()
        .map(String::as_str)
        .collect();
    let posts: Vec<&CashPost> = game
        .cash_journal
        .iter()
        .filter(|post| wanted.contains(post.id.as_str()))
        .collect();
    let owned: Vec<CashPost> = posts.into_iter().cloned().collect();
    insert_cash_posts(conn, &owned)
}

pub fn load_cash_journal(conn: &Connection) -> Result<Vec<CashPost>, String> {
    if !table_exists(conn, "cash_journal")? {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, club_id, amount, kind, date, envelope_generation, meta_json, reverses_id
             FROM cash_journal
             ORDER BY date, id",
        )
        .map_err(|_| LOAD_ERROR.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let kind_str: String = row.get(3)?;
            let kind: CashKind = decode_enum(kind_str, 3)?;
            let meta_json: String = row.get(6)?;
            let meta: CashPostMeta = serde_json::from_str(&meta_json).unwrap_or_default();
            Ok(CashPost {
                id: row.get(0)?,
                club_id: row.get(1)?,
                amount: row.get(2)?,
                kind,
                date: row.get(4)?,
                envelope_generation: row.get(5)?,
                meta,
                reverses_id: row.get(7)?,
            })
        })
        .map_err(|_| LOAD_ERROR.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| LOAD_ERROR.to_string())
}

pub fn upsert_transfer_reservations(
    conn: &Connection,
    rows: &[TransferReservation],
) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut stmt = conn
        .prepare(
            "INSERT OR REPLACE INTO transfer_reservations
             (id, club_id, player_id, offer_id, amount, created_on, settles_on, kind, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .map_err(|_| WRITE_ERROR.to_string())?;
    for row in rows {
        stmt.execute(params![
            row.id,
            row.club_id,
            row.player_id,
            row.offer_id,
            row.amount,
            row.created_on,
            row.settles_on,
            encode_enum(&row.kind)?,
            encode_enum(&row.status)?,
        ])
        .map_err(|_| WRITE_ERROR.to_string())?;
    }
    Ok(())
}

pub fn load_transfer_reservations(conn: &Connection) -> Result<Vec<TransferReservation>, String> {
    if !table_exists(conn, "transfer_reservations")? {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, club_id, player_id, offer_id, amount, created_on, settles_on, kind, status
             FROM transfer_reservations",
        )
        .map_err(|_| LOAD_ERROR.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let kind_str: String = row.get(7)?;
            let status_str: String = row.get(8)?;
            Ok(TransferReservation {
                id: row.get(0)?,
                club_id: row.get(1)?,
                player_id: row.get(2)?,
                offer_id: row.get(3)?,
                amount: row.get(4)?,
                created_on: row.get(5)?,
                settles_on: row.get(6)?,
                kind: decode_enum::<ReservationKind>(kind_str, 7)?,
                status: decode_enum::<ReservationStatus>(status_str, 8)?,
            })
        })
        .map_err(|_| LOAD_ERROR.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| LOAD_ERROR.to_string())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| LOAD_ERROR.to_string())?;
    Ok(exists.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use domain::finance::CashKind;

    fn sample_post(id: &str, amount: i64) -> CashPost {
        CashPost {
            id: id.to_string(),
            club_id: "club-a".to_string(),
            amount,
            kind: CashKind::Matchday,
            date: "2026-02-16".to_string(),
            envelope_generation: 0,
            meta: CashPostMeta::default(),
            reverses_id: None,
        }
    }

    #[test]
    fn insert_or_ignore_does_not_duplicate() {
        let db = GameDatabase::open_in_memory().unwrap();
        let post = sample_post("post-1", 12_000);
        assert_eq!(
            insert_cash_posts(db.conn(), std::slice::from_ref(&post)).unwrap(),
            1
        );
        assert_eq!(
            insert_cash_posts(db.conn(), std::slice::from_ref(&post)).unwrap(),
            0
        );
        let loaded = load_cash_journal(db.conn()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].amount, 12_000);
        assert_eq!(loaded[0].kind, CashKind::Matchday);
    }

    #[test]
    fn reservations_round_trip() {
        let db = GameDatabase::open_in_memory().unwrap();
        let row = TransferReservation {
            id: "res-1".to_string(),
            club_id: "club-a".to_string(),
            player_id: "p-1".to_string(),
            offer_id: "off-1".to_string(),
            amount: 2_000_000,
            created_on: "2026-02-16".to_string(),
            settles_on: Some("2026-07-01".to_string()),
            kind: ReservationKind::TransferFee,
            status: ReservationStatus::Active,
        };
        upsert_transfer_reservations(db.conn(), std::slice::from_ref(&row)).unwrap();
        let loaded = load_transfer_reservations(db.conn()).unwrap();
        assert_eq!(loaded, vec![row]);
    }
}
