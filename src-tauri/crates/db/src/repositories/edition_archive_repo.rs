use domain::edition_archive::CompletedEdition;
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Insert-only: a recorded edition is frozen, so an existing row is never rewritten or deleted.
/// Serializes every record before the first insert and relies on the caller's transaction
/// (`write_game`) for all-or-nothing writes.
pub(crate) fn persist_new_editions(
    conn: &Connection,
    editions: &[CompletedEdition],
) -> Result<(), String> {
    let records = editions
        .iter()
        .map(|edition| {
            serde_json::to_string(edition).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (edition, record_json) in editions.iter().zip(records) {
        conn.execute(
            "INSERT OR IGNORE INTO competition_edition_archive (competition_id, season, record_json)
             VALUES (?1, ?2, ?3)",
            params![edition.competition_id, edition.season, record_json],
        )
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    }
    Ok(())
}

pub fn load_editions(conn: &Connection) -> Result<Vec<CompletedEdition>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT record_json FROM competition_edition_archive ORDER BY competition_id, season",
        )
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
    use crate::repositories::competition_repo;
    use domain::league::{League, StandingEntry};

    fn edition(competition_id: &str, season: u32, champion: &str) -> CompletedEdition {
        CompletedEdition {
            competition_id: competition_id.into(),
            season,
            completed_on: "2033-06-30".into(),
            champion_id: champion.into(),
            participant_ids: vec!["a".into(), "b".into()],
            standings: vec![StandingEntry {
                points: 7,
                goals_for: 5,
                ..StandingEntry::new(champion.into())
            }],
            groups: Vec::new(),
            knockout_rounds: Vec::new(),
            fixtures: Vec::new(),
        }
    }

    fn json(editions: &[CompletedEdition]) -> serde_json::Value {
        serde_json::to_value(editions).unwrap()
    }

    /// Given non-default editions, when saved and reloaded, then every field survives.
    #[test]
    fn editions_survive_sqlite_reload() {
        let db = GameDatabase::open_in_memory().unwrap();
        let saved = vec![
            edition("ar-d1-apertura", 2033, "a"),
            edition("ar-cup", 2033, "b"),
        ];
        persist_new_editions(db.conn(), &saved).unwrap();
        let mut loaded = load_editions(db.conn()).unwrap();
        loaded.sort_by(|x, y| x.competition_id.cmp(&y.competition_id));
        let mut expected = saved;
        expected.sort_by(|x, y| x.competition_id.cmp(&y.competition_id));
        assert_eq!(json(&loaded), json(&expected));
    }

    /// Given a recorded edition, when competitions are replaced, then the archive remains.
    #[test]
    fn replacing_competitions_does_not_delete_the_archive() {
        let db = GameDatabase::open_in_memory().unwrap();
        persist_new_editions(db.conn(), &[edition("ar-d1-apertura", 2033, "a")]).unwrap();
        competition_repo::replace_competitions(
            db.conn(),
            &[League::new(
                "ar-d1-apertura".into(),
                "Opening".into(),
                2034,
                &["a".into(), "b".into()],
            )],
        )
        .unwrap();
        assert_eq!(load_editions(db.conn()).unwrap().len(), 1);
    }

    /// Given a recorded edition, when a different record claims its key, then the first record wins.
    #[test]
    fn a_recorded_edition_is_never_rewritten() {
        let db = GameDatabase::open_in_memory().unwrap();
        persist_new_editions(db.conn(), &[edition("ar-d1-apertura", 2033, "a")]).unwrap();
        persist_new_editions(db.conn(), &[edition("ar-d1-apertura", 2033, "b")]).unwrap();
        let loaded = load_editions(db.conn()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].champion_id, "a");
    }

    /// Given a corrupt record, when loaded, then load fails through the translated error.
    #[test]
    fn corrupt_archive_record_is_not_silently_dropped() {
        let db = GameDatabase::open_in_memory().unwrap();
        db.conn()
            .execute(
                "INSERT INTO competition_edition_archive VALUES ('x', 1, '{not json')",
                [],
            )
            .unwrap();
        assert_eq!(
            load_editions(db.conn()).unwrap_err(),
            "be.error.gamePersistence.loadFailed"
        );
    }
}
