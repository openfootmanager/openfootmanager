use chrono::Utc;
use domain::stats::StatsState;

use ofm_core::clock::GameClock;
use ofm_core::game::{BoardObjective, Game, ObjectiveType, ScoutingAssignment};

use crate::game_database::GameDatabase;
use crate::repositories::{
    league_repo, manager_repo, message_repo, meta_repo, news_repo, objective_repo, player_repo,
    scouting_repo, staff_repo, stats_repo, team_repo,
};

pub struct GamePersistenceWriter;

impl GamePersistenceWriter {
    pub fn write_game(
        db: &GameDatabase,
        game: &Game,
        save_id: &str,
        save_name: &str,
    ) -> Result<(), String> {
        let conn = db.conn();
        let now = Utc::now().to_rfc3339();
        let vacant_team_days_json = serde_json::to_string(&game.vacant_team_days)
            .map_err(|error| format!("Failed to serialize vacant team days: {}", error))?;
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

        Ok(())
    }
}

impl GamePersistenceWriter {
    pub fn write_stats_state(db: &GameDatabase, stats: &StatsState) -> Result<(), String> {
        stats_repo::replace_stats_state(db.conn(), stats)
    }
}

pub struct GamePersistenceReader;

impl GamePersistenceReader {
    pub fn read_game(db: &GameDatabase) -> Result<Game, String> {
        let conn = db.conn();

        let meta = meta_repo::load_meta(conn)?
            .ok_or_else(|| "No game_meta found in database".to_string())?;

        let start_date = chrono::DateTime::parse_from_rfc3339(&meta.start_date)
            .map_err(|error| format!("Invalid start_date: {}", error))?
            .with_timezone(&Utc);
        let game_date = chrono::DateTime::parse_from_rfc3339(&meta.game_date)
            .map_err(|error| format!("Invalid game_date: {}", error))?
            .with_timezone(&Utc);

        let mut clock = GameClock::new(start_date);
        clock.current_date = game_date;

        let manager = manager_repo::load_manager(conn, &meta.manager_id)?
            .ok_or_else(|| format!("Manager '{}' not found", meta.manager_id))?;
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
            .map(|objective| BoardObjective {
                id: objective.id,
                description: objective.description,
                target: objective.target,
                objective_type: parse_objective_type(&objective.objective_type),
                met: objective.met,
            })
            .collect();

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
            board_objectives,
            season_context: domain::season::SeasonContext::default(),
            days_since_last_job_offer: None,
            vacant_team_days: serde_json::from_str(&meta.vacant_team_days_json).unwrap_or_default(),
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

fn parse_objective_type(value: &str) -> ObjectiveType {
    match value {
        "LeaguePosition" => ObjectiveType::LeaguePosition,
        "Wins" => ObjectiveType::Wins,
        "GoalsScored" => ObjectiveType::GoalsScored,
        _ => ObjectiveType::Wins,
    }
}
