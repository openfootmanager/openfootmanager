use crate::json_column::{self, Cell};
use crate::repositories::{competition_repo, league_repo};
use domain::player::PlayerSeasonStats;
use ofm_core::end_of_season::club_league_table_in;
use rusqlite::{Connection, params};

const MIGRATION_FAILED: &str = "be.error.gameDatabase.migrationFailed";

/// Finish v066 once, including when a previous metadata read applied the schema migration.
/// The old counter cannot reveal past divisions, so its total belongs to the current club's
/// league. Normal repository loading preserves and quarantines malformed legacy JSON.
pub(crate) fn backfill(conn: &Connection) -> Result<(), String> {
    let mut pending = conn.prepare(
        "SELECT id, team_id, stats, league_stats FROM players WHERE league_stats_by_competition IS NULL",
    ).map_err(|_| MIGRATION_FAILED.to_string())?;
    let rows = pending
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|_| MIGRATION_FAILED.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| MIGRATION_FAILED.to_string())?;
    if rows.is_empty() {
        return Ok(());
    }
    let tx = conn
        .unchecked_transaction()
        .map_err(|_| MIGRATION_FAILED.to_string())?;
    let mut competitions = competition_repo::load_competitions(&tx)?;
    if competitions.is_empty()
        && let Some(league) = league_repo::load_league(&tx)?
    {
        competitions.push(league);
    }
    for (id, club, stats_json, league_json) in rows {
        let mut by_competition = std::collections::BTreeMap::new();
        if let Some(league) = club
            .as_deref()
            .and_then(|club| club_league_table_in(&competitions, club))
        {
            let stats = json_column::read_optional::<PlayerSeasonStats>(
                &tx,
                Cell::new("players", &id, "league_stats"),
                league_json.as_deref(),
            )
            .unwrap_or_else(|| {
                json_column::read(&tx, Cell::new("players", &id, "stats"), &stats_json)
            });
            by_competition.insert(league.id.clone(), stats);
        }
        let saved =
            serde_json::to_string(&by_competition).map_err(|_| MIGRATION_FAILED.to_string())?;
        tx.execute(
            "UPDATE players SET league_stats_by_competition = ?1 WHERE id = ?2 AND league_stats_by_competition IS NULL",
            params![saved, id],
        )
        .map_err(|_| MIGRATION_FAILED.to_string())?;
    }
    tx.commit().map_err(|_| MIGRATION_FAILED.to_string())
}

#[cfg(test)]
mod tests {
    use crate::game_database::GameDatabase;
    use rusqlite::{Connection, params};

    /// Given a v065 save with malformed participants but a readable league table,
    /// when the save upgrades, then it retains its totals and quarantines the original JSON.
    #[test]
    fn upgrading_league_stats_preserves_a_recoverable_competition() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recoverable.db");
        let mut conn = Connection::open(&path).unwrap();
        crate::migrations::all_migrations()
            .to_version(&mut conn, 65)
            .unwrap();
        conn.execute("INSERT INTO competitions (id, name, season, participant_ids_json, standings_json)
            VALUES ('division', 'Division', 2025, 'broken json', ?1)",
            params![r#"[{"team_id":"club","played":10,"won":5,"drawn":1,"lost":4,"goals_for":15,"goals_against":12,"points":16}]"#],
        ).unwrap();
        conn.execute("INSERT INTO players (id, match_name, full_name, date_of_birth, nationality,
            position, attributes, team_id, league_stats)
            VALUES ('p', 'P', 'Player', '2000-01-01', 'ENG', 'Forward', '{}', 'club', '{\"goals\":10}')", [],).unwrap();
        drop(conn);

        let reader = GameDatabase::open_save(&path).unwrap();
        let saved: String = reader
            .conn()
            .query_row(
                "SELECT league_stats_by_competition FROM players WHERE id = 'p'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(value["division"]["goals"], 10);
        let raw: String = reader
            .conn()
            .query_row(
                "SELECT raw FROM unreadable_records WHERE table_name = 'competitions'
             AND row_id = 'division' AND column_name = 'participant_ids_json'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(raw, "broken json");
        drop(reader);
        let reader = GameDatabase::open_save(&path).unwrap();
        let reloaded: String = reader
            .conn()
            .query_row(
                "SELECT league_stats_by_competition FROM players WHERE id = 'p'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(reloaded, saved);
    }
}
