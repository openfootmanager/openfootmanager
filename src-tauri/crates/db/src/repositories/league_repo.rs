use domain::league::{
    CompletedTransfer, Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry,
    TransferRumour,
};
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

/// Insert or replace the league row and its fixtures + standings.
pub fn upsert_league(conn: &Connection, league: &League) -> Result<(), String> {
    conn.execute_batch("SAVEPOINT league_upsert")
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

    let result = (|| {
        conn.execute("DELETE FROM fixtures", [])
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        conn.execute("DELETE FROM standings", [])
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        conn.execute("DELETE FROM transfer_log", [])
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        conn.execute("DELETE FROM transfer_rumours", [])
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        conn.execute("DELETE FROM league", [])
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

        conn.execute(
            "INSERT OR REPLACE INTO league (id, name, season) VALUES (?1, ?2, ?3)",
            params![league.id, league.name, league.season],
        )
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

        for f in &league.fixtures {
            let competition_str = format!("{:?}", f.competition);
            let status_str = format!("{:?}", f.status);
            let result_json = f
                .result
                .as_ref()
                .map(|r| serde_json::to_string(r).unwrap_or_default());
            conn.execute(
                "INSERT INTO fixtures (id, league_id, matchday, date, home_team_id, away_team_id, competition, status, result)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    f.id,
                    league.id,
                    f.matchday,
                    f.date,
                    f.home_team_id,
                    f.away_team_id,
                    competition_str,
                    status_str,
                    result_json,
                ],
            )
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        }

        for s in &league.standings {
            conn.execute(
                "INSERT INTO standings (league_id, team_id, played, won, drawn, lost, goals_for, goals_against, points)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    league.id,
                    s.team_id,
                    s.played,
                    s.won,
                    s.drawn,
                    s.lost,
                    s.goals_for,
                    s.goals_against,
                    s.points,
                ],
            )
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        }

        for transfer in &league.transfer_log {
            conn.execute(
                "INSERT INTO transfer_log (league_id, date, from_team_id, to_team_id, player_id, fee)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    league.id,
                    transfer.date,
                    transfer.from_team_id,
                    transfer.to_team_id,
                    transfer.player_id,
                    transfer.fee,
                ],
            )
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        }

        for rumour in &league.transfer_rumours {
            conn.execute(
                "INSERT INTO transfer_rumours (league_id, rumour_id, date, player_id, player_name, team_id, team_name)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    league.id,
                    rumour.id,
                    rumour.date,
                    rumour.player_id,
                    rumour.player_name,
                    rumour.team_id,
                    rumour.team_name,
                ],
            )
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        }

        Ok(())
    })();

    match result {
        Ok(()) => conn
            .execute_batch("RELEASE SAVEPOINT league_upsert")
            .map_err(|error| {
                format!(
                    "{GAME_PERSISTENCE_WRITE_ERROR}; cleanup=release_savepoint: {error}"
                )
            }),
        Err(error) => {
            conn.execute_batch(
                "ROLLBACK TO SAVEPOINT league_upsert; RELEASE SAVEPOINT league_upsert",
            )
            .map_err(|cleanup_error| {
                format!(
                    "{error}; cleanup=rollback_release_savepoint: {cleanup_error}"
                )
            })?;
            Err(error)
        }
    }
}

fn parse_fixture_status(s: &str) -> FixtureStatus {
    match s {
        "InProgress" => FixtureStatus::InProgress,
        "Completed" => FixtureStatus::Completed,
        _ => FixtureStatus::Scheduled,
    }
}

fn parse_fixture_competition(s: &str) -> FixtureCompetition {
    match s {
        "Friendly" => FixtureCompetition::Friendly,
        "PreseasonTournament" => FixtureCompetition::PreseasonTournament,
        _ => FixtureCompetition::League,
    }
}

/// Load the league (if any). Returns None if the league table is empty.
pub fn load_league(conn: &Connection) -> Result<Option<League>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, season FROM league ORDER BY season DESC, rowid DESC LIMIT 1")
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
            ))
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let (league_id, name, season) = match rows.next() {
        Some(Ok(tuple)) => tuple,
        Some(Err(_)) => return Err(GAME_PERSISTENCE_LOAD_ERROR.to_string()),
        None => return Ok(None),
    };

    // Load fixtures
    let mut fix_stmt = conn
        .prepare(
            "SELECT id, matchday, date, home_team_id, away_team_id, competition, status, result
             FROM fixtures WHERE league_id = ?1 ORDER BY matchday, id",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let fixture_rows = fix_stmt
        .query_map(params![league_id], |row| {
            let competition_str: String = row.get(5)?;
            let status_str: String = row.get(6)?;
            let result_json: Option<String> = row.get(7)?;
            Ok(Fixture {
                id: row.get(0)?,
                matchday: row.get(1)?,
                date: row.get(2)?,
                home_team_id: row.get(3)?,
                away_team_id: row.get(4)?,
                competition: parse_fixture_competition(&competition_str),
                status: parse_fixture_status(&status_str),
                result: result_json.and_then(|j| serde_json::from_str(&j).ok()),
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut fixtures = Vec::new();
    for row in fixture_rows {
        fixtures.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }

    // Load standings
    let mut stand_stmt = conn
        .prepare(
            "SELECT team_id, played, won, drawn, lost, goals_for, goals_against, points
             FROM standings WHERE league_id = ?1",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let standing_rows = stand_stmt
        .query_map(params![league_id], |row| {
            Ok(StandingEntry {
                team_id: row.get(0)?,
                played: row.get(1)?,
                won: row.get(2)?,
                drawn: row.get(3)?,
                lost: row.get(4)?,
                goals_for: row.get(5)?,
                goals_against: row.get(6)?,
                points: row.get(7)?,
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut standings = Vec::new();
    for row in standing_rows {
        standings.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }

    let mut transfer_stmt = conn
        .prepare(
            "SELECT date, from_team_id, to_team_id, player_id, fee
             FROM transfer_log WHERE league_id = ?1 ORDER BY date, id",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let transfer_rows = transfer_stmt
        .query_map(params![league_id], |row| {
            Ok(CompletedTransfer {
                date: row.get(0)?,
                from_team_id: row.get(1)?,
                to_team_id: row.get(2)?,
                player_id: row.get(3)?,
                fee: row.get(4)?,
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut transfer_log = Vec::new();
    for row in transfer_rows {
        transfer_log.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }

    let mut rumour_stmt = conn
        .prepare(
            "SELECT rumour_id, date, player_id, player_name, team_id, team_name
             FROM transfer_rumours WHERE league_id = ?1 ORDER BY date DESC, id DESC",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let rumour_rows = rumour_stmt
        .query_map(params![league_id.clone()], |row| {
            Ok(TransferRumour {
                id: row.get(0)?,
                date: row.get(1)?,
                player_id: row.get(2)?,
                player_name: row.get(3)?,
                team_id: row.get(4)?,
                team_name: row.get(5)?,
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut transfer_rumours = Vec::new();
    for row in rumour_rows {
        transfer_rumours.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }

    Ok(Some(League {
        id: league_id,
        name,
        season,
        fixtures,
        standings,
        transfer_log,
        transfer_rumours,
    }))
}

pub fn needs_cleanup(conn: &Connection, active_league_id: Option<&str>) -> Result<bool, String> {
    let league_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM league", [], |row| row.get(0))
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let Some(active_league_id) = active_league_id else {
        return Ok(league_count > 0);
    };

    if league_count != 1 {
        return Ok(true);
    }

    let stale_fixture_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM fixtures WHERE league_id != ?1",
            params![active_league_id],
            |row| row.get(0),
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
    if stale_fixture_count > 0 {
        return Ok(true);
    }

    let stale_standings_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM standings WHERE league_id != ?1",
            params![active_league_id],
            |row| row.get(0),
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
    if stale_standings_count > 0 {
        return Ok(true);
    }

    let stale_transfer_log_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transfer_log WHERE league_id != ?1",
            params![active_league_id],
            |row| row.get(0),
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    if stale_transfer_log_count > 0 {
        return Ok(true);
    }

    let stale_transfer_rumour_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transfer_rumours WHERE league_id != ?1",
            params![active_league_id],
            |row| row.get(0),
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
    if stale_transfer_rumour_count > 0 {
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use domain::league::{GoalEvent, MatchResult};
    use rusqlite::Connection;

    fn test_db() -> GameDatabase {
        GameDatabase::open_in_memory().unwrap()
    }

    fn sample_league() -> League {
        let team_ids = vec!["team-001".to_string(), "team-002".to_string()];
        let mut league = League::new(
            "league-1".to_string(),
            "Premier Division".to_string(),
            2026,
            &team_ids,
        );
        league.fixtures = vec![
            Fixture {
                id: "fix-001".to_string(),
                matchday: 1,
                date: "2026-08-15".to_string(),
                home_team_id: "team-001".to_string(),
                away_team_id: "team-002".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Scheduled,
                result: None,
            },
            Fixture {
                id: "fix-002".to_string(),
                matchday: 2,
                date: "2026-08-22".to_string(),
                home_team_id: "team-002".to_string(),
                away_team_id: "team-001".to_string(),
                competition: FixtureCompetition::Friendly,
                status: FixtureStatus::Completed,
                result: Some(MatchResult {
                    home_goals: 2,
                    away_goals: 1,
                    home_scorers: vec![GoalEvent {
                        player_id: "p-010".to_string(),
                        minute: 23,
                    }],
                    away_scorers: vec![],
                    report: None,
                }),
            },
        ];
        league.transfer_log = vec![CompletedTransfer {
            date: "2026-08-18".to_string(),
            from_team_id: "team-001".to_string(),
            to_team_id: "team-002".to_string(),
            player_id: "player-001".to_string(),
            fee: 1_250_000,
        }];
        league.transfer_rumours = vec![TransferRumour {
            id: "rumour_player-001_2026-W34".to_string(),
            date: "2026-08-18T12:00:00+00:00".to_string(),
            player_id: "player-001".to_string(),
            player_name: "Player One".to_string(),
            team_id: "team-001".to_string(),
            team_name: "Team One".to_string(),
        }];
        league
    }

    fn persisted_row_counts(conn: &Connection) -> (i64, i64, i64, i64, i64) {
        let league_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM league", [], |row| row.get(0))
            .unwrap();
        let fixture_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM fixtures", [], |row| row.get(0))
            .unwrap();
        let standings_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM standings", [], |row| row.get(0))
            .unwrap();
        let transfer_log_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transfer_log", [], |row| row.get(0))
            .unwrap();
        let transfer_rumour_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transfer_rumours", [], |row| row.get(0))
            .unwrap();

        (
            league_count,
            fixture_count,
            standings_count,
            transfer_log_count,
            transfer_rumour_count,
        )
    }

    #[test]
    fn test_upsert_and_load_league() {
        let db = test_db();
        let league = sample_league();

        upsert_league(db.conn(), &league).unwrap();
        let loaded = load_league(db.conn()).unwrap().unwrap();

        assert_eq!(loaded.id, "league-1");
        assert_eq!(loaded.name, "Premier Division");
        assert_eq!(loaded.season, 2026);
    }

    #[test]
    fn test_league_fixtures_roundtrip() {
        let db = test_db();
        let league = sample_league();

        upsert_league(db.conn(), &league).unwrap();
        let loaded = load_league(db.conn()).unwrap().unwrap();

        assert_eq!(loaded.fixtures.len(), 2);
        assert_eq!(loaded.fixtures[0].status, FixtureStatus::Scheduled);
        assert!(loaded.fixtures[0].result.is_none());
        assert_eq!(loaded.fixtures[1].status, FixtureStatus::Completed);
        assert_eq!(loaded.fixtures[1].competition, FixtureCompetition::Friendly);
        assert!(loaded.fixtures[1].result.is_some());
        let result = loaded.fixtures[1].result.as_ref().unwrap();
        assert_eq!(result.home_goals, 2);
        assert_eq!(result.away_goals, 1);
    }

    #[test]
    fn test_league_standings_roundtrip() {
        let db = test_db();
        let league = sample_league();

        upsert_league(db.conn(), &league).unwrap();
        let loaded = load_league(db.conn()).unwrap().unwrap();

        assert_eq!(loaded.standings.len(), 2);
    }

    #[test]
    fn test_league_transfer_log_roundtrip() {
        let db = test_db();
        let league = sample_league();

        upsert_league(db.conn(), &league).unwrap();
        let loaded = load_league(db.conn()).unwrap().unwrap();

        assert_eq!(loaded.transfer_log.len(), 1);
        assert_eq!(loaded.transfer_log[0].player_id, "player-001");
        assert_eq!(loaded.transfer_log[0].from_team_id, "team-001");
        assert_eq!(loaded.transfer_log[0].to_team_id, "team-002");
        assert_eq!(loaded.transfer_log[0].fee, 1_250_000);
    }

    #[test]
    fn test_league_transfer_rumours_roundtrip() {
        let db = test_db();
        let league = sample_league();

        upsert_league(db.conn(), &league).unwrap();
        let loaded = load_league(db.conn()).unwrap().unwrap();

        assert_eq!(loaded.transfer_rumours.len(), 1);
        assert_eq!(loaded.transfer_rumours[0].player_id, "player-001");
        assert_eq!(loaded.transfer_rumours[0].player_name, "Player One");
        assert_eq!(loaded.transfer_rumours[0].team_id, "team-001");
        assert_eq!(loaded.transfer_rumours[0].team_name, "Team One");
    }

    #[test]
    fn test_load_league_empty() {
        let db = test_db();
        let loaded = load_league(db.conn()).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_upsert_league_replaces_fixtures() {
        let db = test_db();
        let mut league = sample_league();
        upsert_league(db.conn(), &league).unwrap();

        // Modify and re-upsert — old fixtures should be replaced
        league.fixtures = vec![Fixture {
            id: "fix-003".to_string(),
            matchday: 3,
            date: "2026-08-29".to_string(),
            home_team_id: "team-001".to_string(),
            away_team_id: "team-002".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        }];
        upsert_league(db.conn(), &league).unwrap();

        let loaded = load_league(db.conn()).unwrap().unwrap();
        assert_eq!(loaded.fixtures.len(), 1);
        assert_eq!(loaded.fixtures[0].id, "fix-003");
    }

    #[test]
    fn test_upsert_league_clears_previous_season_rows() {
        let db = test_db();
        let league = sample_league();
        upsert_league(db.conn(), &league).unwrap();

        let replacement = League {
            id: "league-2".to_string(),
            name: "Premier Division".to_string(),
            season: 2027,
            fixtures: vec![Fixture {
                id: "fix-101".to_string(),
                matchday: 1,
                date: "2027-08-15".to_string(),
                home_team_id: "team-001".to_string(),
                away_team_id: "team-002".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Scheduled,
                result: None,
            }],
            standings: vec![
                StandingEntry::new("team-001".to_string()),
                StandingEntry::new("team-002".to_string()),
            ],
            transfer_log: vec![],
            transfer_rumours: vec![],
        };

        upsert_league(db.conn(), &replacement).unwrap();

        let league_count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM league", [], |row| row.get(0))
            .unwrap();
        let fixture_count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM fixtures", [], |row| row.get(0))
            .unwrap();
        let loaded = load_league(db.conn()).unwrap().unwrap();

        assert_eq!(league_count, 1);
        assert_eq!(fixture_count, 1);
        assert_eq!(loaded.id, "league-2");
        assert_eq!(loaded.season, 2027);
        assert_eq!(loaded.fixtures[0].id, "fix-101");
    }

    #[test]
    fn test_upsert_league_rolls_back_when_a_write_fails_midway() {
        let db = test_db();
        let league = sample_league();
        upsert_league(db.conn(), &league).unwrap();

        db.conn()
            .execute(
                "CREATE TRIGGER fail_bad_fixture BEFORE INSERT ON fixtures
                 WHEN NEW.id = 'fix-bad'
                 BEGIN
                   SELECT RAISE(FAIL, 'boom');
                 END;",
                [],
            )
            .unwrap();

        let mut replacement = sample_league();
        replacement.id = "league-rollback".to_string();
        replacement.season = 2027;
        replacement.fixtures = vec![Fixture {
            id: "fix-bad".to_string(),
            matchday: 1,
            date: "2027-08-15".to_string(),
            home_team_id: "team-001".to_string(),
            away_team_id: "team-002".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        }];

        assert_eq!(
            upsert_league(db.conn(), &replacement),
            Err(GAME_PERSISTENCE_WRITE_ERROR.to_string())
        );

        let loaded = load_league(db.conn()).unwrap().unwrap();
        assert_eq!(loaded.id, "league-1");
        assert_eq!(loaded.season, 2026);
        assert_eq!(loaded.fixtures.len(), 2);
    }

    #[test]
    fn test_upsert_league_respects_parent_transaction_boundaries() {
        let rollback_db = test_db();
        rollback_db.conn().execute_batch("BEGIN").unwrap();

        upsert_league(rollback_db.conn(), &sample_league()).unwrap();
        assert_eq!(persisted_row_counts(rollback_db.conn()), (1, 2, 2, 1, 1));

        rollback_db.conn().execute_batch("ROLLBACK").unwrap();

        assert_eq!(persisted_row_counts(rollback_db.conn()), (0, 0, 0, 0, 0));
        assert!(load_league(rollback_db.conn()).unwrap().is_none());

        let commit_db = test_db();
        commit_db.conn().execute_batch("BEGIN").unwrap();

        upsert_league(commit_db.conn(), &sample_league()).unwrap();
        commit_db.conn().execute_batch("COMMIT").unwrap();

        assert_eq!(persisted_row_counts(commit_db.conn()), (1, 2, 2, 1, 1));
        assert_eq!(load_league(commit_db.conn()).unwrap().unwrap().id, "league-1");
    }

    #[test]
    fn test_load_league_prefers_newest_season_when_stale_rows_exist() {
        let db = test_db();

        db.conn()
            .execute(
                "INSERT INTO league (id, name, season) VALUES (?1, ?2, ?3)",
                params!["league-old", "Premier Division", 2026],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO fixtures (id, league_id, matchday, date, home_team_id, away_team_id, status, result)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "fix-old",
                    "league-old",
                    1,
                    "2026-08-15",
                    "team-001",
                    "team-002",
                    "Completed",
                    None::<String>,
                ],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO league (id, name, season) VALUES (?1, ?2, ?3)",
                params!["league-new", "Premier Division", 2027],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO fixtures (id, league_id, matchday, date, home_team_id, away_team_id, status, result)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "fix-new",
                    "league-new",
                    1,
                    "2027-08-15",
                    "team-001",
                    "team-002",
                    "Scheduled",
                    None::<String>,
                ],
            )
            .unwrap();

        let loaded = load_league(db.conn()).unwrap().unwrap();

        assert_eq!(loaded.id, "league-new");
        assert_eq!(loaded.season, 2027);
        assert_eq!(loaded.fixtures.len(), 1);
        assert_eq!(loaded.fixtures[0].id, "fix-new");
    }

    #[test]
    fn test_needs_cleanup_detects_multiple_league_rows() {
        let db = test_db();

        db.conn()
            .execute(
                "INSERT INTO league (id, name, season) VALUES (?1, ?2, ?3)",
                params!["league-old", "Premier Division", 2026],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO league (id, name, season) VALUES (?1, ?2, ?3)",
                params!["league-new", "Premier Division", 2027],
            )
            .unwrap();

        assert!(needs_cleanup(db.conn(), Some("league-new")).unwrap());
    }

    #[test]
    fn test_upsert_league_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let league = sample_league();

        let result = upsert_league(&conn, &league);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_league_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_league(&conn);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }

    #[test]
    fn test_needs_cleanup_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = needs_cleanup(&conn, Some("league-1"));

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }
}
