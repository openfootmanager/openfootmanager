use domain::stats::{PlayerMatchStatsRecord, StatsState, TeamMatchStatsRecord};
use rusqlite::{Connection, OptionalExtension, params};

use super::fixture_competition::{fixture_competition_to_string, parse_fixture_competition};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

pub fn replace_stats_state(conn: &Connection, stats: &StatsState) -> Result<(), String> {
    conn.execute("DELETE FROM player_match_stats", [])
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    conn.execute("DELETE FROM team_match_stats", [])
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

    insert_player_records(conn, &stats.player_matches)?;
    insert_team_records(conn, &stats.team_matches)
}

fn insert_player_records(
    conn: &Connection,
    records: &[PlayerMatchStatsRecord],
) -> Result<(), String> {
    for record in records {
        conn.execute(
            "INSERT INTO player_match_stats (
                fixture_id, season, matchday, date, competition, player_id, team_id,
                opponent_team_id, home_team_id, away_team_id, home_goals, away_goals,
                minutes_played, goals, assists, shots, shots_on_target, passes_completed,
                passes_attempted, tackles_won, interceptions, fouls_committed,
                yellow_cards, red_cards, rating
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
            params![
                record.fixture_id,
                record.season,
                record.matchday,
                record.date,
                fixture_competition_to_string(&record.competition),
                record.player_id,
                record.team_id,
                record.opponent_team_id,
                record.home_team_id,
                record.away_team_id,
                record.home_goals,
                record.away_goals,
                record.minutes_played,
                record.goals,
                record.assists,
                record.shots,
                record.shots_on_target,
                record.passes_completed,
                record.passes_attempted,
                record.tackles_won,
                record.interceptions,
                record.fouls_committed,
                record.yellow_cards,
                record.red_cards,
                record.rating,
            ],
        )
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    }

    Ok(())
}

fn insert_team_records(conn: &Connection, records: &[TeamMatchStatsRecord]) -> Result<(), String> {
    for record in records {
        conn.execute(
            "INSERT INTO team_match_stats (
                fixture_id, season, matchday, date, competition, team_id, opponent_team_id,
                home_team_id, away_team_id, goals_for, goals_against, possession_pct,
                shots, shots_on_target, passes_completed, passes_attempted, tackles_won,
                interceptions, fouls_committed, yellow_cards, red_cards
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                record.fixture_id,
                record.season,
                record.matchday,
                record.date,
                fixture_competition_to_string(&record.competition),
                record.team_id,
                record.opponent_team_id,
                record.home_team_id,
                record.away_team_id,
                record.goals_for,
                record.goals_against,
                record.possession_pct,
                record.shots,
                record.shots_on_target,
                record.passes_completed,
                record.passes_attempted,
                record.tackles_won,
                record.interceptions,
                record.fouls_committed,
                record.yellow_cards,
                record.red_cards,
            ],
        )
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    }

    Ok(())
}

/// Writes the match records the file does not hold yet.
///
/// The history only ever grows by appending, and rows are written in order, so the file's newest
/// row (`MAX(rowid)`) says how many records it already holds: those are skipped and only the tail
/// is inserted. Before trusting that, the newest row must be the record it should be; a file that
/// holds more than memory, or a different history, is rewritten in full instead of appended to.
pub fn persist_stats_state(conn: &Connection, stats: &StatsState) -> Result<(), String> {
    let players_held = held_prefix(
        conn,
        "player_match_stats",
        "player_id",
        &stats.player_matches,
        |r| (r.fixture_id.as_str(), r.player_id.as_str()),
    )?;
    let teams_held = held_prefix(
        conn,
        "team_match_stats",
        "team_id",
        &stats.team_matches,
        |r| (r.fixture_id.as_str(), r.team_id.as_str()),
    )?;
    match (players_held, teams_held) {
        (Some(players), Some(teams)) => {
            insert_player_records(conn, &stats.player_matches[players..])?;
            insert_team_records(conn, &stats.team_matches[teams..])
        }
        _ => replace_stats_state(conn, stats),
    }
}

/// How many of `records` the table already holds, or `None` when the table is not a prefix of them.
fn held_prefix<T>(
    conn: &Connection,
    table: &str,
    subject_column: &str,
    records: &[T],
    key: impl Fn(&T) -> (&str, &str),
) -> Result<Option<usize>, String> {
    let newest: Option<i64> = conn
        .query_row(&format!("SELECT MAX(rowid) FROM {table}"), [], |row| {
            row.get(0)
        })
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let Some(held) = newest else {
        return Ok(Some(0));
    };
    let held = usize::try_from(held).map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let Some(last_held) = held.checked_sub(1).and_then(|index| records.get(index)) else {
        return Ok(None);
    };
    let stored: Option<(String, String)> = conn
        .query_row(
            &format!("SELECT fixture_id, {subject_column} FROM {table} WHERE rowid = ?1"),
            [held as i64],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    let (fixture, subject) = key(last_held);
    Ok(
        (stored.as_ref().map(|(f, s)| (f.as_str(), s.as_str())) == Some((fixture, subject)))
            .then_some(held),
    )
}

pub fn load_stats_state(conn: &Connection) -> Result<StatsState, String> {
    let mut player_stmt = conn
        .prepare(
            "SELECT fixture_id, season, matchday, date, competition, player_id, team_id,
                    opponent_team_id, home_team_id, away_team_id, home_goals, away_goals,
                    minutes_played, goals, assists, shots, shots_on_target, passes_completed,
                    passes_attempted, tackles_won, interceptions, fouls_committed,
                    yellow_cards, red_cards, rating
             FROM player_match_stats
             ORDER BY date, matchday, fixture_id, player_id",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
    let player_rows = player_stmt
        .query_map([], |row| {
            Ok(PlayerMatchStatsRecord {
                fixture_id: row.get(0)?,
                season: row.get(1)?,
                matchday: row.get(2)?,
                date: row.get(3)?,
                competition: parse_fixture_competition(&row.get::<_, String>(4)?),
                player_id: row.get(5)?,
                team_id: row.get(6)?,
                opponent_team_id: row.get(7)?,
                home_team_id: row.get(8)?,
                away_team_id: row.get(9)?,
                home_goals: row.get(10)?,
                away_goals: row.get(11)?,
                minutes_played: row.get(12)?,
                goals: row.get(13)?,
                assists: row.get(14)?,
                shots: row.get(15)?,
                shots_on_target: row.get(16)?,
                passes_completed: row.get(17)?,
                passes_attempted: row.get(18)?,
                tackles_won: row.get(19)?,
                interceptions: row.get(20)?,
                fouls_committed: row.get(21)?,
                yellow_cards: row.get(22)?,
                red_cards: row.get(23)?,
                rating: row.get(24)?,
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut player_matches = Vec::new();
    for row in player_rows {
        player_matches.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }

    let mut team_stmt = conn
        .prepare(
            "SELECT fixture_id, season, matchday, date, competition, team_id, opponent_team_id,
                    home_team_id, away_team_id, goals_for, goals_against, possession_pct,
                    shots, shots_on_target, passes_completed, passes_attempted, tackles_won,
                    interceptions, fouls_committed, yellow_cards, red_cards
             FROM team_match_stats
             ORDER BY date, matchday, fixture_id, team_id",
        )
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
    let team_rows = team_stmt
        .query_map([], |row| {
            Ok(TeamMatchStatsRecord {
                fixture_id: row.get(0)?,
                season: row.get(1)?,
                matchday: row.get(2)?,
                date: row.get(3)?,
                competition: parse_fixture_competition(&row.get::<_, String>(4)?),
                team_id: row.get(5)?,
                opponent_team_id: row.get(6)?,
                home_team_id: row.get(7)?,
                away_team_id: row.get(8)?,
                goals_for: row.get(9)?,
                goals_against: row.get(10)?,
                possession_pct: row.get(11)?,
                shots: row.get(12)?,
                shots_on_target: row.get(13)?,
                passes_completed: row.get(14)?,
                passes_attempted: row.get(15)?,
                tackles_won: row.get(16)?,
                interceptions: row.get(17)?,
                fouls_committed: row.get(18)?,
                yellow_cards: row.get(19)?,
                red_cards: row.get(20)?,
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut team_matches = Vec::new();
    for row in team_rows {
        team_matches.push(row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?);
    }

    Ok(StatsState {
        player_matches,
        team_matches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use domain::league::FixtureCompetition;
    use rusqlite::Connection;

    fn test_db() -> GameDatabase {
        GameDatabase::open_in_memory().unwrap()
    }

    fn sample_stats_state() -> StatsState {
        StatsState {
            player_matches: vec![PlayerMatchStatsRecord {
                fixture_id: "fixture-001".to_string(),
                season: 2025,
                matchday: 1,
                date: "2025-08-10".to_string(),
                competition: FixtureCompetition::League,
                player_id: "player-001".to_string(),
                team_id: "team-001".to_string(),
                opponent_team_id: "team-002".to_string(),
                home_team_id: "team-001".to_string(),
                away_team_id: "team-002".to_string(),
                home_goals: 2,
                away_goals: 1,
                minutes_played: 90,
                goals: 1,
                assists: 0,
                shots: 3,
                shots_on_target: 2,
                passes_completed: 18,
                passes_attempted: 22,
                tackles_won: 1,
                interceptions: 0,
                fouls_committed: 1,
                yellow_cards: 0,
                red_cards: 0,
                rating: 7.4,
            }],
            team_matches: vec![TeamMatchStatsRecord {
                fixture_id: "fixture-001".to_string(),
                season: 2025,
                matchday: 1,
                date: "2025-08-10".to_string(),
                competition: FixtureCompetition::League,
                team_id: "team-001".to_string(),
                opponent_team_id: "team-002".to_string(),
                home_team_id: "team-001".to_string(),
                away_team_id: "team-002".to_string(),
                goals_for: 2,
                goals_against: 1,
                possession_pct: 54,
                shots: 12,
                shots_on_target: 5,
                passes_completed: 310,
                passes_attempted: 360,
                tackles_won: 14,
                interceptions: 8,
                fouls_committed: 10,
                yellow_cards: 1,
                red_cards: 0,
            }],
        }
    }

    #[test]
    fn test_replace_stats_state_roundtrip() {
        let db = test_db();
        let stats = sample_stats_state();

        replace_stats_state(db.conn(), &stats).unwrap();
        let loaded = load_stats_state(db.conn()).unwrap();

        assert_eq!(loaded.player_matches, stats.player_matches);
        assert_eq!(loaded.team_matches, stats.team_matches);
    }

    fn player_row(fixture: &str, player: &str) -> PlayerMatchStatsRecord {
        PlayerMatchStatsRecord {
            fixture_id: fixture.to_string(),
            player_id: player.to_string(),
            ..sample_stats_state().player_matches[0].clone()
        }
    }

    fn team_row(fixture: &str, team: &str) -> TeamMatchStatsRecord {
        TeamMatchStatsRecord {
            fixture_id: fixture.to_string(),
            team_id: team.to_string(),
            ..sample_stats_state().team_matches[0].clone()
        }
    }

    /// One played match: 22 players and the two teams.
    fn a_match(fixture: &str) -> StatsState {
        StatsState {
            player_matches: (0..22)
                .map(|n| player_row(fixture, &format!("{fixture}-p{n}")))
                .collect(),
            team_matches: vec![
                team_row(fixture, &format!("{fixture}-home")),
                team_row(fixture, &format!("{fixture}-away")),
            ],
        }
    }

    fn history_of(matches: usize) -> StatsState {
        let mut stats = StatsState::default();
        for n in 0..matches {
            stats.append(a_match(&format!("fixture-{n:04}")));
        }
        stats
    }

    /// The records of two states, whatever order a load returns them in.
    fn assert_same_records(left: &StatsState, right: &StatsState) {
        let players = |s: &StatsState| {
            let mut keys: Vec<_> = s
                .player_matches
                .iter()
                .map(|r| (r.fixture_id.clone(), r.player_id.clone()))
                .collect();
            keys.sort();
            keys
        };
        let teams = |s: &StatsState| {
            let mut keys: Vec<_> = s
                .team_matches
                .iter()
                .map(|r| (r.fixture_id.clone(), r.team_id.clone()))
                .collect();
            keys.sort();
            keys
        };
        assert_eq!(players(left), players(right));
        assert_eq!(teams(left), teams(right));
    }

    fn rows_written_by(conn: &Connection, save: impl FnOnce()) -> u64 {
        let before = conn.total_changes();
        save();
        conn.total_changes() - before
    }

    /// Given a history already saved, when the same history is saved again,
    /// then nothing is written.
    #[test]
    fn saving_an_unchanged_history_writes_nothing() {
        let db = test_db();
        let stats = history_of(40);
        persist_stats_state(db.conn(), &stats).unwrap();

        let written = rows_written_by(db.conn(), || {
            persist_stats_state(db.conn(), &stats).unwrap();
        });

        assert_eq!(written, 0);
    }

    /// Given a saved history and one more match played, when it is saved, then only that
    /// match's 22 player rows and 2 team rows are written, however long the history is.
    #[test]
    fn saving_one_more_match_writes_only_that_match() {
        let db = test_db();
        let mut stats = history_of(60);
        persist_stats_state(db.conn(), &stats).unwrap();
        stats.append(a_match("fixture-new"));

        let written = rows_written_by(db.conn(), || {
            persist_stats_state(db.conn(), &stats).unwrap();
        });

        assert_eq!(written, 24);
        assert_same_records(&load_stats_state(db.conn()).unwrap(), &stats);
    }

    /// Given a history saved, loaded, extended and saved again, when the file is read through a
    /// second connection, then every record is there exactly once.
    #[test]
    fn nothing_is_lost_or_duplicated_across_save_reload_and_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("save.db");
        let writer = GameDatabase::open(&path).unwrap();
        persist_stats_state(writer.conn(), &history_of(10)).unwrap();

        let mut reloaded = load_stats_state(writer.conn()).unwrap();
        reloaded.append(a_match("fixture-after-reload"));
        persist_stats_state(writer.conn(), &reloaded).unwrap();
        persist_stats_state(writer.conn(), &reloaded).unwrap();

        let reader = Connection::open(&path).unwrap();
        let count = |sql: &str| {
            reader
                .query_row(sql, [], |row| row.get::<_, i64>(0))
                .unwrap()
        };
        assert_eq!(count("SELECT COUNT(*) FROM player_match_stats"), 11 * 22);
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM (SELECT DISTINCT fixture_id, player_id FROM player_match_stats)"
            ),
            11 * 22
        );
        assert_eq!(count("SELECT COUNT(*) FROM team_match_stats"), 11 * 2);
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM (SELECT DISTINCT fixture_id, team_id FROM team_match_stats)"
            ),
            11 * 2
        );
    }

    /// Given a file holding different records than memory, when memory is saved, then the file
    /// ends up holding exactly memory: a file that disagrees is rewritten, not appended to.
    #[test]
    fn a_file_that_disagrees_with_memory_is_rewritten_in_full() {
        let db = test_db();
        persist_stats_state(db.conn(), &history_of(5)).unwrap();
        let mut elsewhere = StatsState::default();
        for n in 0..8 {
            elsewhere.append(a_match(&format!("other-{n}")));
        }

        persist_stats_state(db.conn(), &elsewhere).unwrap();

        let loaded = load_stats_state(db.conn()).unwrap();
        assert_eq!(loaded.player_matches.len(), elsewhere.player_matches.len());
        assert!(
            loaded
                .player_matches
                .iter()
                .all(|r| r.fixture_id.starts_with("other-"))
        );
    }

    /// Given a file holding more than memory, when memory is saved, then the extra is removed.
    #[test]
    fn a_shorter_history_removes_what_it_no_longer_holds() {
        let db = test_db();
        persist_stats_state(db.conn(), &history_of(9)).unwrap();
        let shorter = history_of(4);

        persist_stats_state(db.conn(), &shorter).unwrap();

        assert_same_records(&load_stats_state(db.conn()).unwrap(), &shorter);
    }

    #[test]
    fn test_replace_stats_state_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();
        let stats = StatsState::default();

        let result = replace_stats_state(&conn, &stats);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_WRITE_ERROR);
    }

    #[test]
    fn test_load_stats_state_returns_backend_key_when_schema_is_missing() {
        let conn = Connection::open_in_memory().unwrap();

        let result = load_stats_state(&conn);

        assert_eq!(result.unwrap_err(), GAME_PERSISTENCE_LOAD_ERROR);
    }
}
