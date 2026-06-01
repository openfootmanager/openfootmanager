use chrono::Utc;
use domain::player::Position;
use domain::stats::StatsState;
use domain::world_history::WorldHistoryArchive;
use rusqlite::Connection;

use ofm_core::clock::GameClock;
use ofm_core::game::{
    BoardObjective, Game, ObjectiveType, ScoutingAssignment, YouthScoutingAssignment,
    YouthScoutingObjective, YouthScoutingRegion,
};

use crate::game_database::GameDatabase;
use crate::repositories::{
    league_repo, manager_repo, message_repo, meta_repo, news_repo, objective_repo, player_repo,
    scouting_repo, staff_repo, stats_repo, team_repo,
};

pub struct GamePersistenceWriter;

fn game_persistence_write_error() -> String {
    "be.error.gamePersistence.writeFailed".to_string()
}

impl GamePersistenceWriter {
    pub fn write_game(
        db: &GameDatabase,
        game: &Game,
        save_id: &str,
        save_name: &str,
    ) -> Result<(), String> {
        let transaction = db
            .conn()
            .unchecked_transaction()
            .map_err(|_| game_persistence_write_error())?;
        write_game_to_connection(&transaction, game, save_id, save_name)?;
        transaction
            .commit()
            .map_err(|_| game_persistence_write_error())?;
        Ok(())
    }

    pub fn write_game_and_stats(
        db: &GameDatabase,
        game: &Game,
        stats: &StatsState,
        save_id: &str,
        save_name: &str,
    ) -> Result<(), String> {
        let transaction = db
            .conn()
            .unchecked_transaction()
            .map_err(|_| game_persistence_write_error())?;
        write_game_to_connection(&transaction, game, save_id, save_name)?;
        stats_repo::replace_stats_state(&transaction, stats)?;
        transaction
            .commit()
            .map_err(|_| game_persistence_write_error())?;
        Ok(())
    }

    pub fn write_stats_state(db: &GameDatabase, stats: &StatsState) -> Result<(), String> {
        let transaction = db
            .conn()
            .unchecked_transaction()
            .map_err(|_| game_persistence_write_error())?;
        stats_repo::replace_stats_state(&transaction, stats)?;
        transaction
            .commit()
            .map_err(|_| game_persistence_write_error())?;
        Ok(())
    }
}

fn write_game_to_connection(
    conn: &Connection,
    game: &Game,
    save_id: &str,
    save_name: &str,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    let vacant_team_days_json = serde_json::to_string(&game.vacant_team_days)
        .map_err(|_| game_persistence_write_error())?;
    let world_history_json = serde_json::to_string(&game.world_history)
        .map_err(|_| game_persistence_write_error())?;
    let manager_id = if game.manager_id.is_empty() {
        game.manager.id.clone()
    } else {
        game.manager_id.clone()
    };
    let mut managers = game.managers.clone();
    if let Some(existing) = managers.iter_mut().find(|manager| manager.id == manager_id) {
        *existing = game.manager.clone();
    } else {
        managers.push(game.manager.clone());
    }

    meta_repo::upsert_meta(
        conn,
        &meta_repo::GameMeta {
            save_id: save_id.to_string(),
            save_name: save_name.to_string(),
            manager_id: manager_id.clone(),
            start_date: game.clock.start_date.to_rfc3339(),
            game_date: game.clock.current_date.to_rfc3339(),
            created_at: now.clone(),
            last_played_at: now,
            vacant_team_days_json,
            world_history_json,
        },
    )?;

    for manager in &managers {
        manager_repo::upsert_manager(conn, manager)?;
    }
    team_repo::upsert_teams(conn, &game.teams)?;
    player_repo::upsert_players(conn, &game.players)?;
    staff_repo::upsert_staff_list(conn, &game.staff)?;
    message_repo::upsert_messages(conn, &game.messages)?;
    news_repo::upsert_news_list(conn, &game.news)?;

    if let Some(ref league) = game.league {
        league_repo::upsert_league(conn, league)?;
    }

    let objective_rows: Vec<objective_repo::BoardObjectiveRow> = game
        .board_objectives
        .iter()
        .map(|objective| objective_repo::BoardObjectiveRow {
            id: objective.id.clone(),
            description: objective.description.clone(),
            target: objective.target,
            objective_type: format!("{:?}", objective.objective_type),
            met: objective.met,
        })
        .collect();
    objective_repo::upsert_objectives(conn, &objective_rows)?;

    let scouting_rows: Vec<scouting_repo::ScoutingAssignmentRow> = game
        .scouting_assignments
        .iter()
        .map(|assignment| scouting_repo::ScoutingAssignmentRow {
            id: assignment.id.clone(),
            scout_id: assignment.scout_id.clone(),
            player_id: assignment.player_id.clone(),
            days_remaining: assignment.days_remaining,
        })
        .collect();
    scouting_repo::upsert_scouting_list(conn, &scouting_rows)?;

    let youth_scouting_rows: Vec<scouting_repo::YouthScoutingAssignmentRow> = game
        .youth_scouting_assignments
        .iter()
        .map(|assignment| scouting_repo::YouthScoutingAssignmentRow {
            id: assignment.id.clone(),
            scout_id: assignment.scout_id.clone(),
            region: format!("{:?}", assignment.region),
            objective: format!("{:?}", assignment.objective),
            target_position: assignment
                .target_position
                .as_ref()
                .map(|position| format!("{:?}", position)),
            days_remaining: assignment.days_remaining,
        })
        .collect();
    scouting_repo::upsert_youth_scouting_list(conn, &youth_scouting_rows)?;

    Ok(())
}

pub struct GamePersistenceReader;

fn backend_error_with_param(key: &str, param_name: &str, param_value: &str) -> String {
    let mut message = String::with_capacity(key.len() + param_name.len() + param_value.len() + 2);
    message.push_str(key);
    message.push('?');
    message.push_str(param_name);
    message.push('=');
    message.push_str(param_value);
    message
}

fn game_meta_missing_error() -> String {
    "be.error.gamePersistence.gameMetaMissing".to_string()
}

fn invalid_start_date_error() -> String {
    "be.error.gamePersistence.invalidStartDate".to_string()
}

fn invalid_game_date_error() -> String {
    "be.error.gamePersistence.invalidGameDate".to_string()
}

fn manager_not_found_error(manager_id: &str) -> String {
    backend_error_with_param(
        "be.error.gamePersistence.managerNotFound",
        "managerId",
        manager_id,
    )
}

fn game_persistence_load_error() -> String {
    "be.error.gamePersistence.loadFailed".to_string()
}

impl GamePersistenceReader {
    pub fn read_game(db: &GameDatabase) -> Result<Game, String> {
        let conn = db.conn();

        let meta = meta_repo::load_meta(conn)?.ok_or_else(game_meta_missing_error)?;

        let start_date = chrono::DateTime::parse_from_rfc3339(&meta.start_date)
            .map_err(|_| invalid_start_date_error())?
            .with_timezone(&Utc);
        let game_date = chrono::DateTime::parse_from_rfc3339(&meta.game_date)
            .map_err(|_| invalid_game_date_error())?
            .with_timezone(&Utc);

        let mut clock = GameClock::new(start_date);
        clock.current_date = game_date;

        let manager = manager_repo::load_manager(conn, &meta.manager_id)?
            .ok_or_else(|| manager_not_found_error(&meta.manager_id))?;
        let mut managers = manager_repo::load_all_managers(conn)?;
        if managers.is_empty() {
            managers.push(manager.clone());
        }
        let teams = team_repo::load_all_teams(conn)?;
        let players = player_repo::load_all_players(conn)?;
        let staff = staff_repo::load_all_staff(conn)?;
        let messages = message_repo::load_all_messages(conn)?;
        let news = news_repo::load_all_news(conn)?;
        let league = league_repo::load_league(conn)?;

        let objective_rows = objective_repo::load_all_objectives(conn)?;
        let board_objectives: Vec<BoardObjective> = objective_rows
            .into_iter()
            .map(|objective| {
                Ok(BoardObjective {
                    id: objective.id,
                    description: objective.description,
                    target: objective.target,
                    objective_type: parse_objective_type(&objective.objective_type)?,
                    met: objective.met,
                })
            })
            .collect::<Result<_, String>>()?;

        let scouting_rows = scouting_repo::load_all_scouting(conn)?;
        let scouting_assignments: Vec<ScoutingAssignment> = scouting_rows
            .into_iter()
            .map(|assignment| ScoutingAssignment {
                id: assignment.id,
                scout_id: assignment.scout_id,
                player_id: assignment.player_id,
                days_remaining: assignment.days_remaining,
            })
            .collect();
        let youth_scouting_rows = scouting_repo::load_all_youth_scouting(conn)?;
        let youth_scouting_assignments: Vec<YouthScoutingAssignment> = youth_scouting_rows
            .into_iter()
            .map(|assignment| {
                Ok(YouthScoutingAssignment {
                    id: assignment.id,
                    scout_id: assignment.scout_id,
                    region: parse_youth_region(&assignment.region)?,
                    objective: parse_youth_objective(&assignment.objective)?,
                    target_position: assignment.target_position.as_deref().map(parse_position),
                    days_remaining: assignment.days_remaining,
                })
            })
            .collect::<Result<_, String>>()?;

        let mut game = Game {
            clock,
            manager_id: meta.manager_id.clone(),
            managers,
            manager,
            teams,
            players,
            staff,
            messages,
            news,
            league,
            scouting_assignments,
            youth_scouting_assignments,
            board_objectives,
            season_context: domain::season::SeasonContext::default(),
            days_since_last_job_offer: None,
            vacant_team_days: serde_json::from_str(&meta.vacant_team_days_json).unwrap_or_default(),
            world_history: serde_json::from_str(&meta.world_history_json)
                .unwrap_or_else(|_| WorldHistoryArchive::default()),
        };
        ofm_core::season_context::refresh_game_context(&mut game);

        Ok(game)
    }
}

impl GamePersistenceReader {
    pub fn read_stats_state(db: &GameDatabase) -> Result<StatsState, String> {
        stats_repo::load_stats_state(db.conn())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::world_history::{
        HistoricalManagerAwardWinner, HistoricalPlayerAwardWinner, HistoricalSeasonAwardsRecord,
        WorldHistoryArchive,
    };
    use domain::{manager::Manager, team::Team};

    fn sample_meta(start_date: &str, game_date: &str, manager_id: &str) -> meta_repo::GameMeta {
        meta_repo::GameMeta {
            save_id: "save-1".to_string(),
            save_name: "Career".to_string(),
            manager_id: manager_id.to_string(),
            start_date: start_date.to_string(),
            game_date: game_date.to_string(),
            created_at: "2026-07-01T00:00:00+00:00".to_string(),
            last_played_at: "2026-07-01T00:00:00+00:00".to_string(),
            vacant_team_days_json: "{}".to_string(),
            world_history_json: "{}".to_string(),
        }
    }

    fn sample_world_history() -> WorldHistoryArchive {
        let mut archive = WorldHistoryArchive::default();
        archive.upsert_rivalry("team-1", "team-2", 84, Some(2027));
        archive.record_season_awards(HistoricalSeasonAwardsRecord {
            season: 2031,
            golden_boot: Some(HistoricalPlayerAwardWinner {
                player_id: "player-9".to_string(),
                player_name: "Goals McGee".to_string(),
                team_id: "team-1".to_string(),
                team_name: "Alpha FC".to_string(),
                value: 27.0,
            }),
            assist_king: None,
            player_of_year: None,
            clean_sheet_king: None,
            most_appearances: None,
            young_player: None,
            manager_of_season: Some(HistoricalManagerAwardWinner {
                manager_id: "mgr-1".to_string(),
                manager_name: "Alex Manager".to_string(),
                team_id: "team-1".to_string(),
                team_name: "Alpha FC".to_string(),
                value: 91.0,
                win_rate: 72.2,
            }),
        });
        archive
    }

    fn sample_game_with_clock(start_year: i32, current_day: u32) -> Game {
        let manager = Manager::new(
            "mgr-1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let team = Team::new(
            "team-1".to_string(),
            "Alpha FC".to_string(),
            "AFC".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Alpha Park".to_string(),
            20_000,
        );
        let mut game = Game::new(
            GameClock::new(Utc.with_ymd_and_hms(start_year, 7, 1, 0, 0, 0).unwrap()),
            manager,
            vec![team],
            vec![],
            vec![],
            vec![],
        );
        game.clock.current_date = Utc
            .with_ymd_and_hms(start_year, 7, current_day, 0, 0, 0)
            .unwrap();
        game
    }

    #[test]
    fn read_game_returns_backend_key_when_game_meta_is_missing() {
        let db = GameDatabase::open_in_memory().unwrap();

        let result = GamePersistenceReader::read_game(&db);

        assert_eq!(
            result.unwrap_err(),
            "be.error.gamePersistence.gameMetaMissing"
        );
    }

    #[test]
    fn read_game_returns_backend_key_when_start_date_is_invalid() {
        let db = GameDatabase::open_in_memory().unwrap();
        meta_repo::upsert_meta(
            db.conn(),
            &sample_meta("not-a-date", "2026-07-01T00:00:00+00:00", "mgr-1"),
        )
        .unwrap();

        let result = GamePersistenceReader::read_game(&db);

        assert_eq!(
            result.unwrap_err(),
            "be.error.gamePersistence.invalidStartDate"
        );
    }

    #[test]
    fn read_game_returns_backend_key_when_game_date_is_invalid() {
        let db = GameDatabase::open_in_memory().unwrap();
        meta_repo::upsert_meta(
            db.conn(),
            &sample_meta("2026-07-01T00:00:00+00:00", "not-a-date", "mgr-1"),
        )
        .unwrap();

        let result = GamePersistenceReader::read_game(&db);

        assert_eq!(
            result.unwrap_err(),
            "be.error.gamePersistence.invalidGameDate"
        );
    }

    #[test]
    fn read_game_returns_backend_key_when_manager_is_missing() {
        let db = GameDatabase::open_in_memory().unwrap();
        meta_repo::upsert_meta(
            db.conn(),
            &sample_meta(
                "2026-07-01T00:00:00+00:00",
                "2026-07-01T00:00:00+00:00",
                "mgr-missing",
            ),
        )
        .unwrap();

        let result = GamePersistenceReader::read_game(&db);

        assert_eq!(
            result.unwrap_err(),
            "be.error.gamePersistence.managerNotFound?managerId=mgr-missing"
        );
    }

    #[test]
    fn write_game_persists_arbitrary_year_clock_metadata() {
        let db = GameDatabase::open_in_memory().unwrap();
        let game = sample_game_with_clock(2032, 18);

        GamePersistenceWriter::write_game(&db, &game, "save-1", "Career").unwrap();

        let meta = meta_repo::load_meta(db.conn()).unwrap().unwrap();
        assert_eq!(meta.start_date, "2032-07-01T00:00:00+00:00");
        assert_eq!(meta.game_date, "2032-07-18T00:00:00+00:00");
    }

    #[test]
    fn write_and_read_game_preserves_world_history_archive() {
        let db = GameDatabase::open_in_memory().unwrap();
        let mut game = sample_game_with_clock(2032, 18);
        game.world_history = sample_world_history();

        GamePersistenceWriter::write_game(&db, &game, "save-1", "Career").unwrap();

        let loaded = GamePersistenceReader::read_game(&db).unwrap();
        assert_eq!(loaded.world_history, game.world_history);
    }

    #[test]
    fn parse_objective_type_returns_backend_key_when_value_is_unknown() {
        let result = parse_objective_type("UnknownObjective");

        assert_eq!(result.unwrap_err(), "be.error.gamePersistence.loadFailed");
    }

    #[test]
    fn parse_youth_region_returns_backend_key_when_value_is_unknown() {
        let result = parse_youth_region("MoonBase");

        assert_eq!(result.unwrap_err(), "be.error.gamePersistence.loadFailed");
    }

    #[test]
    fn parse_youth_objective_returns_backend_key_when_value_is_unknown() {
        let result = parse_youth_objective("SuperProspect");

        assert_eq!(result.unwrap_err(), "be.error.gamePersistence.loadFailed");
    }
}

fn parse_objective_type(value: &str) -> Result<ObjectiveType, String> {
    match value {
        "LeaguePosition" => Ok(ObjectiveType::LeaguePosition),
        "Wins" => Ok(ObjectiveType::Wins),
        "GoalsScored" => Ok(ObjectiveType::GoalsScored),
        "FinancialStability" => Ok(ObjectiveType::FinancialStability),
        _ => Err(game_persistence_load_error()),
    }
}

fn parse_position(value: &str) -> Position {
    match value {
        "Goalkeeper" => Position::Goalkeeper,
        "Defender" => Position::Defender,
        "Midfielder" => Position::Midfielder,
        "Forward" => Position::Forward,
        "RightBack" => Position::RightBack,
        "CenterBack" => Position::CenterBack,
        "LeftBack" => Position::LeftBack,
        "RightWingBack" => Position::RightWingBack,
        "LeftWingBack" => Position::LeftWingBack,
        "DefensiveMidfielder" => Position::DefensiveMidfielder,
        "CentralMidfielder" => Position::CentralMidfielder,
        "AttackingMidfielder" => Position::AttackingMidfielder,
        "RightMidfielder" => Position::RightMidfielder,
        "LeftMidfielder" => Position::LeftMidfielder,
        "RightWinger" => Position::RightWinger,
        "LeftWinger" => Position::LeftWinger,
        "Striker" => Position::Striker,
        _ => Position::Midfielder,
    }
}

fn parse_youth_region(value: &str) -> Result<YouthScoutingRegion, String> {
    match value {
        "Domestic" => Ok(YouthScoutingRegion::Domestic),
        "International" => Ok(YouthScoutingRegion::International),
        _ => Err(game_persistence_load_error()),
    }
}

fn parse_youth_objective(value: &str) -> Result<YouthScoutingObjective, String> {
    match value {
        "Balanced" => Ok(YouthScoutingObjective::Balanced),
        "HighPotential" => Ok(YouthScoutingObjective::HighPotential),
        "ReadySoon" => Ok(YouthScoutingObjective::ReadySoon),
        _ => Err(game_persistence_load_error()),
    }
}
