use chrono::Utc;
use domain::stats::StatsState;
use log::info;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use domain::player::{Player, Position};
use ofm_core::game::Game;
use ofm_core::generator;
use ofm_core::player_identity;
use ofm_core::player_rating::{
    effective_rating_for_assignment, formation_slots, refresh_player_derived,
};

use crate::game_database::GameDatabase;
use crate::game_persistence::{GamePersistenceReader, GamePersistenceWriter};
use crate::repositories::league_repo;
use crate::save_index::{SaveEntry, compute_checksum, save_entry_metadata_from_game};
use crate::save_index_manager::SaveIndexManager;

/// Manages save sessions: creating, loading, saving, deleting, and listing.
pub struct SaveManager {
    saves_dir: PathBuf,
    save_index: SaveIndexManager,
}

const SAVE_MANAGER_UNAVAILABLE_ERROR: &str = "be.error.saveManagerUnavailable";
const SAVE_DELETE_ERROR: &str = "be.error.saveDeleteFailed";

fn backend_error_with_param(key: &str, param_name: &str, param_value: &str) -> String {
    let mut message = String::with_capacity(key.len() + param_name.len() + param_value.len() + 2);
    message.push_str(key);
    message.push('?');
    message.push_str(param_name);
    message.push('=');
    message.push_str(param_value);
    message
}

fn save_not_found_error(save_id: &str) -> String {
    backend_error_with_param("be.error.saveNotFound", "saveId", save_id)
}

impl SaveManager {
    /// Initialize the SaveManager without blocking startup on a missing save index.
    pub fn init(saves_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(saves_dir).map_err(|_| SAVE_MANAGER_UNAVAILABLE_ERROR.to_string())?;
        let save_index = SaveIndexManager::init(saves_dir)?;

        Ok(Self {
            saves_dir: saves_dir.to_path_buf(),
            save_index,
        })
    }

    fn ensure_save_index_ready(&mut self) -> Result<(), String> {
        self.save_index.ensure_loaded()
    }

    /// List all save entries.
    pub fn list_saves(&self) -> &[SaveEntry] {
        self.save_index.list_saves()
    }

    pub fn load_saves(&mut self) -> Result<Vec<SaveEntry>, String> {
        self.ensure_save_index_ready()?;
        let mut saves = self.save_index.list_saves().to_vec();
        for save in saves.iter_mut() {
            if !save.team_name.is_empty() {
                continue;
            }

            let db_path = self.saves_dir.join(&save.db_filename);
            let Ok(db) = GameDatabase::open(&db_path) else {
                continue;
            };
            let Ok(game) = GamePersistenceReader::read_game(&db) else {
                continue;
            };

            let (manager_name, team_name) = save_entry_metadata_from_game(&game);
            let can_backfill_team = !team_name.is_empty();
            let can_backfill_manager = save.manager_name.is_empty() && !manager_name.is_empty();
            if !can_backfill_team && !can_backfill_manager {
                continue;
            }

            save.team_name = team_name;
            if save.manager_name.is_empty() {
                save.manager_name = manager_name;
            }

            if let Err(error) = self.save_index.update_save(save.clone()) {
                log::warn!(
                    "[save_manager] failed to persist metadata backfill for save {}: {}",
                    save.id,
                    error
                );
            }
        }

        Ok(saves)
    }

    /// Create a new save from the current in-memory Game state.
    /// Returns the save_id.
    pub fn create_save(&mut self, game: &Game, save_name: &str) -> Result<String, String> {
        self.ensure_save_index_ready()?;

        let save_id = uuid::Uuid::new_v4().to_string();
        let db_filename = format!("{}.db", save_id);
        let db_path = self.saves_dir.join(&db_filename);
        let mut persisted_game = game.clone();

        canonicalize_game_starting_xi_ids(&mut persisted_game);

        let db = GameDatabase::open(&db_path)?;
        GamePersistenceWriter::write_game(&db, &persisted_game, &save_id, save_name)?;
        drop(db);

        let checksum = compute_checksum(&db_path)?;
        let now = Utc::now().to_rfc3339();
        let (manager_name, team_name) = save_entry_metadata_from_game(game);

        let entry = SaveEntry {
            id: save_id.clone(),
            name: save_name.to_string(),
            manager_name,
            team_name,
            db_filename,
            checksum,
            created_at: now.clone(),
            last_played_at: now,
        };

        if let Err(error) = self.save_index.record_new_save(entry) {
            let _ = fs::remove_file(&db_path);
            return Err(error);
        }

        Ok(save_id)
    }

    pub fn create_save_with_stats(
        &mut self,
        game: &Game,
        stats: &StatsState,
        save_name: &str,
    ) -> Result<String, String> {
        self.ensure_save_index_ready()?;

        let save_id = uuid::Uuid::new_v4().to_string();
        let db_filename = format!("{}.db", save_id);
        let db_path = self.saves_dir.join(&db_filename);
        let total_timer = Instant::now();

        let clone_timer = Instant::now();
        let mut persisted_game = game.clone();
        canonicalize_game_starting_xi_ids(&mut persisted_game);
        let clone_ms = clone_timer.elapsed().as_millis();

        let db_open_timer = Instant::now();
        let db = GameDatabase::open(&db_path)?;
        let db_open_ms = db_open_timer.elapsed().as_millis();

        let write_timer = Instant::now();
        GamePersistenceWriter::write_game_and_stats(
            &db,
            &persisted_game,
            stats,
            &save_id,
            save_name,
        )?;
        let write_ms = write_timer.elapsed().as_millis();
        drop(db);

        let checksum_timer = Instant::now();
        let checksum = compute_checksum(&db_path)?;
        let checksum_ms = checksum_timer.elapsed().as_millis();

        let now = Utc::now().to_rfc3339();
        let (manager_name, team_name) = save_entry_metadata_from_game(game);

        let index_timer = Instant::now();
        let entry = SaveEntry {
            id: save_id.clone(),
            name: save_name.to_string(),
            manager_name,
            team_name,
            db_filename,
            checksum,
            created_at: now.clone(),
            last_played_at: now,
        };
        if let Err(error) = self.save_index.record_new_save(entry) {
            let _ = fs::remove_file(&db_path);
            return Err(error);
        }
        let index_ms = index_timer.elapsed().as_millis();

        info!(
            "[save_manager] create_save_with_stats save_id={} clone_ms={} db_open_ms={} write_ms={} checksum_ms={} index_ms={} total_ms={}",
            save_id,
            clone_ms,
            db_open_ms,
            write_ms,
            checksum_ms,
            index_ms,
            total_timer.elapsed().as_millis()
        );

        Ok(save_id)
    }

    /// Save the current Game state to an existing save.
    pub fn save_game(&mut self, game: &Game, save_id: &str) -> Result<(), String> {
        self.ensure_save_index_ready()?;

        let entry = self
            .save_index
            .find(save_id)
            .ok_or_else(|| save_not_found_error(save_id))?;

        let db_path = self.saves_dir.join(&entry.db_filename);
        let save_name = entry.name.clone();
        let mut persisted_game = game.clone();

        canonicalize_game_starting_xi_ids(&mut persisted_game);

        let db = GameDatabase::open(&db_path)?;
        GamePersistenceWriter::write_game(&db, &persisted_game, save_id, &save_name)?;
        drop(db);

        let checksum = compute_checksum(&db_path)?;
        let now = Utc::now().to_rfc3339();
        let (manager_name, team_name) = save_entry_metadata_from_game(game);

        self.save_index.update_save(SaveEntry {
            id: save_id.to_string(),
            name: save_name,
            manager_name,
            team_name,
            db_filename: entry.db_filename.clone(),
            checksum,
            created_at: entry.created_at.clone(),
            last_played_at: now,
        })?;
        Ok(())
    }

    pub fn save_stats_state(&mut self, stats: &StatsState, save_id: &str) -> Result<(), String> {
        self.ensure_save_index_ready()?;

        let entry = self
            .save_index
            .find(save_id)
            .ok_or_else(|| save_not_found_error(save_id))?
            .clone();

        let db_path = self.saves_dir.join(&entry.db_filename);
        let db = GameDatabase::open(&db_path)?;
        GamePersistenceWriter::write_stats_state(&db, stats)?;
        drop(db);

        let checksum = compute_checksum(&db_path)?;
        let now = Utc::now().to_rfc3339();
        self.save_index.update_save(SaveEntry {
            id: save_id.to_string(),
            name: entry.name,
            manager_name: entry.manager_name,
            team_name: entry.team_name.clone(),
            db_filename: entry.db_filename,
            checksum,
            created_at: entry.created_at,
            last_played_at: now,
        })?;

        Ok(())
    }

    pub fn save_game_with_stats(
        &mut self,
        game: &Game,
        stats: &StatsState,
        save_id: &str,
    ) -> Result<(), String> {
        self.ensure_save_index_ready()?;

        let entry = self
            .save_index
            .find(save_id)
            .ok_or_else(|| save_not_found_error(save_id))?
            .clone();

        let db_path = self.saves_dir.join(&entry.db_filename);
        let total_timer = Instant::now();

        let clone_timer = Instant::now();
        let mut persisted_game = game.clone();
        canonicalize_game_starting_xi_ids(&mut persisted_game);
        let clone_ms = clone_timer.elapsed().as_millis();

        let db_open_timer = Instant::now();
        let db = GameDatabase::open(&db_path)?;
        let db_open_ms = db_open_timer.elapsed().as_millis();

        let write_timer = Instant::now();
        GamePersistenceWriter::write_game_and_stats(
            &db,
            &persisted_game,
            stats,
            save_id,
            &entry.name,
        )?;
        let write_ms = write_timer.elapsed().as_millis();
        drop(db);

        let checksum_timer = Instant::now();
        let checksum = compute_checksum(&db_path)?;
        let checksum_ms = checksum_timer.elapsed().as_millis();

        let now = Utc::now().to_rfc3339();
        let (manager_name, team_name) = save_entry_metadata_from_game(game);

        let index_timer = Instant::now();
        self.save_index.update_save(SaveEntry {
            id: save_id.to_string(),
            name: entry.name,
            manager_name,
            team_name,
            db_filename: entry.db_filename,
            checksum,
            created_at: entry.created_at,
            last_played_at: now,
        })?;
        let index_ms = index_timer.elapsed().as_millis();

        info!(
            "[save_manager] save_game_with_stats save_id={} clone_ms={} db_open_ms={} write_ms={} checksum_ms={} index_ms={} total_ms={}",
            save_id,
            clone_ms,
            db_open_ms,
            write_ms,
            checksum_ms,
            index_ms,
            total_timer.elapsed().as_millis()
        );

        Ok(())
    }

    pub fn load_stats_state(&mut self, save_id: &str) -> Result<StatsState, String> {
        self.ensure_save_index_ready()?;

        let entry = self
            .save_index
            .find(save_id)
            .ok_or_else(|| save_not_found_error(save_id))?
            .clone();

        let db_path = self.saves_dir.join(&entry.db_filename);
        let db = GameDatabase::open(&db_path)?;
        GamePersistenceReader::read_stats_state(&db)
    }

    /// Load a Game from a save database.
    pub fn load_game(&mut self, save_id: &str) -> Result<Game, String> {
        self.ensure_save_index_ready()?;

        let entry = self
            .save_index
            .find(save_id)
            .ok_or_else(|| save_not_found_error(save_id))?
            .clone();

        let db_path = self.saves_dir.join(&entry.db_filename);
        let save_name = entry.name.clone();

        let db = GameDatabase::open(&db_path)?;
        let mut game = GamePersistenceReader::read_game(&db)?;
        let mut needs_resave = false;
        let manager_count_before = game.managers.len();
        let assigned_manager_count_before = game
            .teams
            .iter()
            .filter(|team| team.manager_id.is_some())
            .count();

        ofm_core::ai_hiring::seed_ai_managers(&mut game);
        if game.managers.len() != manager_count_before
            || game
                .teams
                .iter()
                .filter(|team| team.manager_id.is_some())
                .count()
                != assigned_manager_count_before
        {
            needs_resave = true;
        }

        if canonicalize_game_starting_xi_ids(&mut game) {
            needs_resave = true;
        }

        if player_identity::upgrade_game_player_identities(&mut game) {
            needs_resave = true;
        }

        if ofm_core::football_identity::upgrade_game_football_identities(&mut game) {
            needs_resave = true;
        }

        // Backfill OVR/potential for players from older saves that don't have them yet.
        // We use the game clock year so age is accurate.
        let current_year = game
            .clock
            .current_date
            .format("%Y")
            .to_string()
            .parse::<u32>()
            .unwrap_or(2026);
        let backfill_count = game.players.iter().filter(|p| p.ovr == 0).count();
        if backfill_count > 0 {
            for player in game.players.iter_mut() {
                if player.ovr == 0 {
                    refresh_player_derived(player, current_year);
                }
            }
            info!(
                "[save_manager] backfilled OVR/potential for {} players in save {}",
                backfill_count, save_id
            );
            needs_resave = true;
        }

        if generator::repair_opening_youth_academies(&mut game) {
            info!(
                "[save_manager] backfilled opening youth academy players for save {}",
                save_id
            );
            needs_resave = true;
        }

        if league_repo::needs_cleanup(
            db.conn(),
            game.league.as_ref().map(|league| league.id.as_str()),
        )? {
            needs_resave = true;
        }

        drop(db);

        if needs_resave {
            let db = GameDatabase::open(&db_path)?;
            GamePersistenceWriter::write_game(&db, &game, save_id, &save_name)?;
            drop(db);

            let checksum = compute_checksum(&db_path)?;
            let now = Utc::now().to_rfc3339();
            let (manager_name, team_name) = save_entry_metadata_from_game(&game);

            self.save_index.update_save(SaveEntry {
                id: save_id.to_string(),
                name: save_name,
                manager_name,
                team_name,
                db_filename: entry.db_filename.clone(),
                checksum,
                created_at: entry.created_at.clone(),
                last_played_at: now,
            })?;
        }

        Ok(game)
    }

    /// Delete a save (removes DB file and index entry).
    pub fn delete_save(&mut self, save_id: &str) -> Result<bool, String> {
        self.ensure_save_index_ready()?;

        let entry = match self.save_index.find(save_id) {
            Some(e) => e.clone(),
            None => return Ok(false),
        };

        let db_path = self.saves_dir.join(&entry.db_filename);
        if db_path.exists() {
            fs::remove_file(&db_path).map_err(|_| SAVE_DELETE_ERROR.to_string())?;
        }

        self.save_index.remove_save(save_id)?;
        Ok(true)
    }

    /// Create a new game by loading an existing save, stripping session data,
    /// and resetting the clock. Returns the loaded Game with clean session state.
    /// This does NOT create a new save — the caller should use `create_save` afterwards.
    pub fn new_game_from_save(&mut self, source_save_id: &str) -> Result<Game, String> {
        let mut game = self.load_game(source_save_id)?;

        // Strip session-specific data
        game.messages.clear();
        game.news.clear();
        game.scouting_assignments.clear();
        game.youth_scouting_assignments.clear();
        game.board_objectives.clear();

        // Reset clock to start date
        game.clock.current_date = game.clock.start_date;

        // Reset manager
        game.manager.satisfaction = 100;
        game.manager.fan_approval = 50;
        game.manager.career_stats = Default::default();
        game.manager.career_history.clear();
        game.sync_user_manager_record();

        // Reset team season data
        for team in &mut game.teams {
            team.form.clear();
            team.season_income = 0;
            team.season_expenses = 0;
        }

        // Reset player stats
        for player in &mut game.players {
            player.stats = Default::default();
            player.transfer_listed = false;
            player.loan_listed = false;
            player.transfer_offers.clear();
        }

        // Clear league (will be regenerated)
        game.league = None;
        Ok(game)
    }
}

pub(crate) fn canonicalize_game_starting_xi_ids(game: &mut Game) -> bool {
    let players_by_id: HashMap<String, Player> = game
        .players
        .iter()
        .cloned()
        .map(|player| (player.id.clone(), player))
        .collect();
    let mut changed = false;

    for team in &mut game.teams {
        changed |= canonicalize_team_starting_xi_ids(team, &players_by_id);
    }

    changed
}

fn canonicalize_team_starting_xi_ids(
    team: &mut domain::team::Team,
    players_by_id: &HashMap<String, Player>,
) -> bool {
    let row_lengths = formation_row_lengths(&team.formation);
    let slots = formation_slots(&team.formation);
    let mut row_start_index = 0;
    let mut changed = false;

    for row_length in row_lengths {
        if row_length < 2 {
            row_start_index += row_length;
            continue;
        }

        let left_index = row_start_index;
        let right_index = row_start_index + row_length - 1;
        let left_slot = slots.get(left_index);
        let right_slot = slots.get(right_index);

        row_start_index += row_length;

        let (Some(left_slot), Some(right_slot)) = (left_slot, right_slot) else {
            continue;
        };

        if !is_mirrored_side_pair(left_slot, right_slot) {
            continue;
        }

        let left_player = team
            .starting_xi_ids
            .get(left_index)
            .and_then(|id| players_by_id.get(id));
        let right_player = team
            .starting_xi_ids
            .get(right_index)
            .and_then(|id| players_by_id.get(id));

        let (Some(left_player), Some(right_player)) = (left_player, right_player) else {
            continue;
        };

        let current_fit = effective_rating_for_assignment(left_player, left_slot)
            + effective_rating_for_assignment(right_player, right_slot);
        let swapped_fit = effective_rating_for_assignment(left_player, right_slot)
            + effective_rating_for_assignment(right_player, left_slot);

        if swapped_fit > current_fit {
            team.starting_xi_ids.swap(left_index, right_index);
            changed = true;
        }
    }

    changed
}

fn formation_row_lengths(formation: &str) -> Vec<usize> {
    let parts: Vec<usize> = formation
        .split('-')
        .filter_map(|part| part.parse::<usize>().ok())
        .collect();

    match parts.as_slice() {
        [defenders, midfielders, forwards] => vec![1, *defenders, *midfielders, *forwards],
        [defenders, deep_midfielders, attacking_midfielders, forwards] => {
            vec![
                1,
                *defenders,
                *deep_midfielders,
                *attacking_midfielders,
                *forwards,
            ]
        }
        _ => formation_row_lengths("4-4-2"),
    }
}

fn is_mirrored_side_pair(left_position: &Position, right_position: &Position) -> bool {
    matches!(
        (left_position, right_position),
        (Position::LeftBack, Position::RightBack)
            | (Position::LeftWingBack, Position::RightWingBack)
            | (Position::LeftMidfielder, Position::RightMidfielder)
            | (Position::LeftWinger, Position::RightWinger)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
    use domain::player::{Footedness, Player, PlayerAttributes, Position, SquadRole};
    use domain::staff::{StaffAttributes, StaffRole};
    use domain::stats::{PlayerMatchStatsRecord, StatsState, TeamMatchStatsRecord};
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::{
        BoardObjective, ObjectiveType, ScoutingAssignment, YouthScoutingAssignment,
    };
    use rusqlite::params;

    fn db_file_count(dir: &std::path::Path) -> usize {
        fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("db"))
            .count()
    }

    fn sample_game() -> Game {
        let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let mut clock = GameClock::new(start);
        clock.current_date = Utc.with_ymd_and_hms(2026, 8, 15, 0, 0, 0).unwrap();

        let mut manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "John".to_string(),
            "Smith".to_string(),
            "1990-01-15".to_string(),
            "British".to_string(),
        );
        manager.hire("team-001".to_string());

        let team = Team::new(
            "team-001".to_string(),
            "London FC".to_string(),
            "LFC".to_string(),
            "GB".to_string(),
            "London".to_string(),
            "London Stadium".to_string(),
            50000,
        );

        let player = domain::player::Player::new(
            "p-001".to_string(),
            "J. Doe".to_string(),
            "John Doe".to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            PlayerAttributes {
                pace: 70,
                stamina: 75,
                strength: 65,
                agility: 72,
                passing: 80,
                shooting: 60,
                tackling: 55,
                dribbling: 68,
                defending: 50,
                positioning: 65,
                vision: 78,
                decisions: 70,
                composure: 60,
                aggression: 55,
                teamwork: 80,
                leadership: 45,
                handling: 20,
                reflexes: 25,
                aerial: 40,
            },
        );

        let staff = domain::staff::Staff::new(
            "staff-001".to_string(),
            "Alice".to_string(),
            "Coach".to_string(),
            "1980-05-10".to_string(),
            StaffRole::Coach,
            StaffAttributes {
                coaching: 75,
                judging_ability: 60,
                judging_potential: 55,
                physiotherapy: 40,
            },
        );

        Game::new(
            clock,
            manager,
            vec![team],
            vec![player],
            vec![staff],
            vec![],
        )
    }

    fn sample_game_with_league() -> Game {
        let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let clock = GameClock::new(start);
        let mut manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "John".to_string(),
            "Smith".to_string(),
            "1990-01-15".to_string(),
            "British".to_string(),
        );
        manager.hire("team-001".to_string());

        let team_one = Team::new(
            "team-001".to_string(),
            "London FC".to_string(),
            "LFC".to_string(),
            "GB".to_string(),
            "London".to_string(),
            "London Stadium".to_string(),
            50000,
        );
        let team_two = Team::new(
            "team-002".to_string(),
            "Rivals FC".to_string(),
            "RFC".to_string(),
            "GB".to_string(),
            "Manchester".to_string(),
            "Rivals Stadium".to_string(),
            42000,
        );

        let league = League {
            id: "league-current".to_string(),
            name: "Premier Division".to_string(),
            season: 2027,
            fixtures: vec![Fixture {
                id: "fix-current".to_string(),
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

        let mut game = Game::new(
            clock,
            manager,
            vec![team_one, team_two],
            vec![],
            vec![],
            vec![],
        );
        game.league = Some(league);
        game
    }

    fn sample_stats_state() -> StatsState {
        StatsState {
            player_matches: vec![PlayerMatchStatsRecord {
                fixture_id: "fix-current".to_string(),
                season: 2027,
                matchday: 1,
                date: "2027-08-15".to_string(),
                competition: FixtureCompetition::League,
                player_id: "p-001".to_string(),
                team_id: "team-001".to_string(),
                opponent_team_id: "team-002".to_string(),
                home_team_id: "team-001".to_string(),
                away_team_id: "team-002".to_string(),
                home_goals: 2,
                away_goals: 1,
                minutes_played: 90,
                goals: 1,
                assists: 1,
                shots: 4,
                shots_on_target: 2,
                passes_completed: 38,
                passes_attempted: 44,
                tackles_won: 3,
                interceptions: 2,
                fouls_committed: 1,
                yellow_cards: 0,
                red_cards: 0,
                rating: 7.8,
            }],
            team_matches: vec![TeamMatchStatsRecord {
                fixture_id: "fix-current".to_string(),
                season: 2027,
                matchday: 1,
                date: "2027-08-15".to_string(),
                competition: FixtureCompetition::League,
                team_id: "team-001".to_string(),
                opponent_team_id: "team-002".to_string(),
                home_team_id: "team-001".to_string(),
                away_team_id: "team-002".to_string(),
                goals_for: 2,
                goals_against: 1,
                possession_pct: 54,
                shots: 12,
                shots_on_target: 6,
                passes_completed: 410,
                passes_attempted: 470,
                tackles_won: 15,
                interceptions: 9,
                fouls_committed: 11,
                yellow_cards: 2,
                red_cards: 0,
            }],
        }
    }

    fn make_lineup_player(id: &str, position: Position, footedness: Footedness) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_uppercase(),
            format!("Player {}", id),
            "2000-01-01".to_string(),
            "GB".to_string(),
            position.clone(),
            PlayerAttributes {
                pace: 70,
                stamina: 70,
                strength: 70,
                agility: 70,
                passing: 70,
                shooting: 70,
                tackling: 70,
                dribbling: 70,
                defending: 70,
                positioning: 70,
                vision: 70,
                decisions: 70,
                composure: 70,
                aggression: 70,
                teamwork: 70,
                leadership: 70,
                handling: 20,
                reflexes: 20,
                aerial: 70,
            },
        );
        player.natural_position = position;
        player.footedness = footedness;
        player.weak_foot = 1;
        player.team_id = Some("team-001".to_string());
        player
    }

    fn sample_game_with_side_specific_starting_xi(mirrored: bool) -> Game {
        let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let clock = GameClock::new(start);
        let mut manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "John".to_string(),
            "Smith".to_string(),
            "1990-01-15".to_string(),
            "British".to_string(),
        );
        manager.hire("team-001".to_string());

        let mut team = Team::new(
            "team-001".to_string(),
            "London FC".to_string(),
            "LFC".to_string(),
            "GB".to_string(),
            "London".to_string(),
            "London Stadium".to_string(),
            50000,
        );
        team.formation = "4-4-2".to_string();
        team.starting_xi_ids = if mirrored {
            vec![
                "gk", "rb", "cb1", "cb2", "lb", "rm", "cm1", "cm2", "lm", "st1", "st2",
            ]
        } else {
            vec![
                "gk", "lb", "cb1", "cb2", "rb", "lm", "cm1", "cm2", "rm", "st1", "st2",
            ]
        }
        .into_iter()
        .map(str::to_string)
        .collect();

        let players = vec![
            make_lineup_player("gk", Position::Goalkeeper, Footedness::Right),
            make_lineup_player("lb", Position::LeftBack, Footedness::Left),
            make_lineup_player("cb1", Position::CenterBack, Footedness::Right),
            make_lineup_player("cb2", Position::CenterBack, Footedness::Right),
            make_lineup_player("rb", Position::RightBack, Footedness::Right),
            make_lineup_player("lm", Position::LeftMidfielder, Footedness::Left),
            make_lineup_player("cm1", Position::CentralMidfielder, Footedness::Right),
            make_lineup_player("cm2", Position::CentralMidfielder, Footedness::Right),
            make_lineup_player("rm", Position::RightMidfielder, Footedness::Right),
            make_lineup_player("st1", Position::Striker, Footedness::Right),
            make_lineup_player("st2", Position::Striker, Footedness::Right),
        ];

        Game::new(clock, manager, vec![team], players, vec![], vec![])
    }

    fn make_opening_repair_player(id: &str, position: Position, date_of_birth: &str) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_uppercase(),
            format!("Player {}", id),
            date_of_birth.to_string(),
            "GB".to_string(),
            position,
            PlayerAttributes {
                pace: 70,
                stamina: 70,
                strength: 70,
                agility: 70,
                passing: 70,
                shooting: 70,
                tackling: 70,
                dribbling: 70,
                defending: 70,
                positioning: 70,
                vision: 70,
                decisions: 70,
                composure: 70,
                aggression: 70,
                teamwork: 70,
                leadership: 70,
                handling: 20,
                reflexes: 20,
                aerial: 70,
            },
        );
        player.team_id = Some("team-001".to_string());
        player.squad_role = SquadRole::Senior;
        player
    }

    fn sample_opening_save_without_youth_academy() -> Game {
        let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let clock = GameClock::new(start);
        let mut manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "John".to_string(),
            "Smith".to_string(),
            "1990-01-15".to_string(),
            "British".to_string(),
        );
        manager.hire("team-001".to_string());

        let team = Team::new(
            "team-001".to_string(),
            "London FC".to_string(),
            "LFC".to_string(),
            "GB".to_string(),
            "London".to_string(),
            "London Stadium".to_string(),
            50000,
        );

        let players = vec![
            make_opening_repair_player("gk", Position::Goalkeeper, "2002-01-01"),
            make_opening_repair_player("def", Position::Defender, "2008-01-01"),
            make_opening_repair_player("mid", Position::Midfielder, "2007-01-01"),
            make_opening_repair_player("fwd", Position::Forward, "2006-01-01"),
            make_opening_repair_player("senior", Position::Defender, "2000-01-01"),
        ];

        Game::new(clock, manager, vec![team], players, vec![], vec![])
    }

    #[test]
    fn test_load_saves_backfills_legacy_index_missing_team_name() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        let index_path = saves_dir.join("save_index.json");

        let save_id = {
            let mut sm = SaveManager::init(&saves_dir).unwrap();
            let game = sample_game();
            sm.create_save(&game, "Legacy Career").unwrap()
        };

        let legacy_index = format!(
            r#"{{
            "version": 1,
            "saves": [{{
                "id": "{save_id}",
                "name": "Legacy Career",
                "manager_name": "John Smith",
                "db_filename": "{save_id}.db",
                "checksum": "stale",
                "created_at": "2026-01-01",
                "last_played_at": "2026-01-02"
            }}]
        }}"#
        );
        fs::write(&index_path, legacy_index).unwrap();

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let saves = sm.load_saves().unwrap();

        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].team_name, "London FC");
    }

    #[test]
    fn test_load_saves_backfills_manager_name_when_unemployed() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        let index_path = saves_dir.join("save_index.json");

        let save_id = {
            let mut sm = SaveManager::init(&saves_dir).unwrap();
            let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
            let clock = GameClock::new(start);
            let manager = domain::manager::Manager::new(
                "mgr-user".to_string(),
                "Jane".to_string(),
                "Doe".to_string(),
                "1990-01-15".to_string(),
                "British".to_string(),
            );
            let game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
            sm.create_save(&game, "Unemployed Career").unwrap()
        };

        let legacy_index = format!(
            r#"{{
            "version": 1,
            "saves": [{{
                "id": "{save_id}",
                "name": "Unemployed Career",
                "manager_name": "",
                "db_filename": "{save_id}.db",
                "checksum": "stale",
                "created_at": "2026-01-01",
                "last_played_at": "2026-01-02"
            }}]
        }}"#
        );
        fs::write(&index_path, legacy_index).unwrap();

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let saves = sm.load_saves().unwrap();

        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].manager_name, "Jane Doe");
        assert_eq!(saves[0].team_name, "");
    }

    #[test]
    fn test_init_creates_directory() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let sm = SaveManager::init(&saves_dir).unwrap();
        assert!(saves_dir.exists());
        assert!(sm.list_saves().is_empty());
    }

    #[test]
    fn test_init_returns_backend_key_when_saves_path_is_file() {
        let dir = tempfile::tempdir().unwrap();
        let saves_path = dir.path().join("saves");
        fs::write(&saves_path, "not a directory").unwrap();

        let result = SaveManager::init(&saves_path);

        match result {
            Err(error) => assert_eq!(error, SAVE_MANAGER_UNAVAILABLE_ERROR),
            Ok(_) => panic!("expected save manager init to fail for a file path"),
        }
    }

    #[test]
    fn test_missing_index_rebuilds_lazily_on_first_save_query() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        let index_path = saves_dir.join("save_index.json");

        {
            let mut sm = SaveManager::init(&saves_dir).unwrap();
            let game = sample_game();
            sm.create_save(&game, "Deferred Index Career").unwrap();
        }

        assert!(index_path.exists());
        fs::remove_file(&index_path).unwrap();
        assert!(!index_path.exists());

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        assert!(sm.list_saves().is_empty());
        assert!(!index_path.exists());

        let saves = sm.load_saves().unwrap();
        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].name, "Deferred Index Career");
        assert!(index_path.exists());
    }

    #[test]
    fn test_create_and_list_save() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game();

        let save_id = sm.create_save(&game, "John's Career").unwrap();
        assert!(!save_id.is_empty());

        let saves = sm.list_saves();
        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].name, "John's Career");
        assert_eq!(saves[0].manager_name, "John Smith");
        assert_eq!(saves[0].team_name, "London FC");
        assert!(!saves[0].checksum.is_empty());
    }

    #[test]
    fn test_create_save_removes_db_when_index_write_fails() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        let index_path = saves_dir.join("save_index.json");
        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game();

        assert!(sm.load_saves().unwrap().is_empty());
        fs::remove_file(&index_path).unwrap();
        fs::create_dir(&index_path).unwrap();

        let result = sm.create_save(&game, "Broken Index Career");

        assert_eq!(result.unwrap_err(), "be.error.saveIndex.writeFailed");
        assert_eq!(db_file_count(&saves_dir), 0);
    }

    #[test]
    fn test_create_save_with_stats_removes_db_when_index_write_fails() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        let index_path = saves_dir.join("save_index.json");
        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game();
        let stats = sample_stats_state();

        assert!(sm.load_saves().unwrap().is_empty());
        fs::remove_file(&index_path).unwrap();
        fs::create_dir(&index_path).unwrap();

        let result = sm.create_save_with_stats(&game, &stats, "Broken Stats Career");

        assert_eq!(result.unwrap_err(), "be.error.saveIndex.writeFailed");
        assert_eq!(db_file_count(&saves_dir), 0);
    }

    #[test]
    fn test_create_and_load_game() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game();

        let save_id = sm.create_save(&game, "Test Career").unwrap();
        let loaded = sm.load_game(&save_id).unwrap();

        assert_eq!(loaded.manager.id, "mgr-user");
        assert_eq!(loaded.manager.first_name, "John");
        assert_eq!(loaded.manager.last_name, "Smith");
        assert_eq!(loaded.teams.len(), 1);
        assert_eq!(loaded.teams[0].name, "London FC");
        assert_eq!(loaded.players.len(), 1);
        assert_eq!(loaded.staff.len(), 1);
        assert_eq!(loaded.clock.start_date, game.clock.start_date);
        assert_eq!(loaded.clock.current_date, game.clock.current_date);
    }

    #[test]
    fn test_create_and_load_game_preserves_retired_player_state() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let mut game = sample_game();
        game.players[0].retired = true;
        game.players[0].team_id = None;

        let save_id = sm.create_save(&game, "Retired Career").unwrap();
        let loaded = sm.load_game(&save_id).unwrap();

        assert!(loaded.players[0].retired);
        assert_eq!(loaded.players[0].team_id, None);
    }

    #[test]
    fn test_load_game_upgrades_football_identity_fields() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let mut game = sample_game();
        game.manager.nationality = "British".to_string();
        game.manager.football_nation.clear();
        game.manager.birth_country = None;
        game.teams[0].football_nation.clear();
        game.players[0].football_nation.clear();
        game.players[0].birth_country = None;

        let save_id = sm.create_save(&game, "Legacy Identity Career").unwrap();
        let loaded = sm.load_game(&save_id).unwrap();

        assert_eq!(loaded.manager.nationality, "ENG");
        assert_eq!(loaded.manager.football_nation, "ENG");
        assert_eq!(loaded.manager.birth_country, None);
        assert_eq!(loaded.teams[0].football_nation, "ENG");
        assert_eq!(loaded.players[0].football_nation, "GB");
        assert_eq!(loaded.players[0].birth_country, None);
    }

    #[test]
    fn test_save_game_updates_existing() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let mut game = sample_game();

        let save_id = sm.create_save(&game, "Career").unwrap();
        let old_checksum = sm.list_saves()[0].checksum.clone();

        // Advance the game
        game.clock.advance_days(7);
        game.manager.reputation = 999;

        sm.save_game(&game, &save_id).unwrap();

        let saves = sm.list_saves();
        assert_eq!(saves.len(), 1);
        // Checksum should change since data changed
        assert_ne!(saves[0].checksum, old_checksum);

        // Reload and verify
        let loaded = sm.load_game(&save_id).unwrap();
        assert_eq!(loaded.manager.reputation, 999);
    }

    #[test]
    fn test_save_and_load_roundtrips_all_managers() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let mut game = sample_game_with_league();

        let mut rival_team = Team::new(
            "team-003".to_string(),
            "Rivals City".to_string(),
            "RIV".to_string(),
            "GB".to_string(),
            "Manchester".to_string(),
            "Rivals Park".to_string(),
            42000,
        );
        rival_team.manager_id = Some("mgr-ai".to_string());
        game.teams.push(rival_team);

        let mut ai_manager = domain::manager::Manager::new(
            "mgr-ai".to_string(),
            "Marco".to_string(),
            "Rossi".to_string(),
            "1978-03-12".to_string(),
            "Italy".to_string(),
        );
        ai_manager.hire("team-003".to_string());
        game.managers.push(ai_manager);

        let save_id = sm.create_save(&game, "Manager World").unwrap();
        let loaded = sm.load_game(&save_id).unwrap();

        assert_eq!(loaded.managers.len(), 2);
        assert!(
            loaded
                .managers
                .iter()
                .any(|manager| manager.id == "mgr-user")
        );
        assert!(loaded.managers.iter().any(|manager| {
            manager.id == "mgr-ai" && manager.team_id.as_deref() == Some("team-003")
        }));
    }

    #[test]
    fn test_load_game_backfills_missing_ai_managers_from_staff() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let mut game = sample_game_with_league();

        let rival_team = Team::new(
            "team-003".to_string(),
            "Rivals City".to_string(),
            "RIV".to_string(),
            "GB".to_string(),
            "Manchester".to_string(),
            "Rivals Park".to_string(),
            42000,
        );
        game.teams.push(rival_team);

        let mut assistant = domain::staff::Staff::new(
            "staff-ai".to_string(),
            "Marco".to_string(),
            "Rossi".to_string(),
            "1978-03-12".to_string(),
            StaffRole::AssistantManager,
            StaffAttributes {
                coaching: 70,
                judging_ability: 60,
                judging_potential: 60,
                physiotherapy: 30,
            },
        );
        assistant.nationality = "Italy".to_string();
        assistant.team_id = Some("team-003".to_string());
        game.staff.push(assistant);

        let save_id = sm.create_save(&game, "Legacy Single Manager").unwrap();
        let loaded = sm.load_game(&save_id).unwrap();

        assert!(loaded.managers.len() >= 2);
        assert!(loaded.managers.iter().any(|manager| {
            manager.team_id.as_deref() == Some("team-003") && manager.full_name() == "Marco Rossi"
        }));
        assert_eq!(
            loaded
                .teams
                .iter()
                .find(|team| team.id == "team-003")
                .and_then(|team| team.manager_id.clone())
                .is_some(),
            true
        );
    }

    #[test]
    fn test_save_and_load_roundtrips_vacant_team_days() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let mut game = sample_game_with_league();
        game.vacant_team_days.insert("team-002".to_string(), 4);
        game.vacant_team_days.insert("team-003".to_string(), 2);

        let save_id = sm.create_save(&game, "Vacancy Tracker").unwrap();
        let loaded = sm.load_game(&save_id).unwrap();

        assert_eq!(loaded.vacant_team_days.get("team-002"), Some(&4));
        assert_eq!(loaded.vacant_team_days.get("team-003"), Some(&2));
    }

    #[test]
    fn test_save_and_load_stats_state_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game_with_league();
        let stats = sample_stats_state();

        let save_id = sm
            .create_save_with_stats(&game, &stats, "Stats Career")
            .unwrap();

        let loaded_stats = sm.load_stats_state(&save_id).unwrap();

        assert_eq!(loaded_stats.player_matches.len(), 1);
        assert_eq!(loaded_stats.team_matches.len(), 1);
        assert_eq!(loaded_stats.player_matches[0].player_id, "p-001");
        assert_eq!(loaded_stats.player_matches[0].shots, 4);
        assert_eq!(loaded_stats.team_matches[0].team_id, "team-001");
        assert_eq!(loaded_stats.team_matches[0].shots_on_target, 6);
    }

    #[test]
    fn test_save_game_with_stats_updates_existing_and_roundtrips_stats() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let mut game = sample_game_with_league();
        let save_id = sm.create_save(&game, "Combined Save Career").unwrap();
        let old_checksum = sm.list_saves()[0].checksum.clone();

        game.clock.advance_days(3);
        game.manager.reputation = 777;

        let stats = sample_stats_state();
        sm.save_game_with_stats(&game, &stats, &save_id).unwrap();

        let saves = sm.list_saves();
        assert_eq!(saves.len(), 1);
        assert_ne!(saves[0].checksum, old_checksum);

        let loaded = sm.load_game(&save_id).unwrap();
        assert_eq!(loaded.manager.reputation, 777);

        let loaded_stats = sm.load_stats_state(&save_id).unwrap();
        assert_eq!(loaded_stats.player_matches.len(), 1);
        assert_eq!(loaded_stats.team_matches.len(), 1);
        assert_eq!(loaded_stats.player_matches[0].player_id, "p-001");
        assert_eq!(loaded_stats.team_matches[0].team_id, "team-001");
    }

    #[test]
    fn test_load_stats_state_without_saved_history_returns_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game();
        let save_id = sm.create_save(&game, "Legacy Style Career").unwrap();

        let loaded_stats = sm.load_stats_state(&save_id).unwrap();

        assert!(loaded_stats.player_matches.is_empty());
        assert!(loaded_stats.team_matches.is_empty());
    }

    #[test]
    fn test_create_save_canonicalizes_mirrored_starting_xi_order_on_write() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game_with_side_specific_starting_xi(true);

        let save_id = sm.create_save(&game, "Mirrored XI Career").unwrap();
        let db_path = saves_dir.join(format!("{}.db", save_id));
        let db = GameDatabase::open(&db_path).unwrap();
        let starting_xi_json: String = db
            .conn()
            .query_row(
                "SELECT starting_xi_ids FROM teams WHERE id = ?1",
                params!["team-001"],
                |row| row.get(0),
            )
            .unwrap();
        let starting_xi_ids: Vec<String> = serde_json::from_str(&starting_xi_json).unwrap();

        assert_eq!(
            starting_xi_ids,
            vec![
                "gk", "lb", "cb1", "cb2", "rb", "lm", "cm1", "cm2", "rm", "st1", "st2"
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_load_game_repairs_existing_mirrored_starting_xi_order() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game_with_side_specific_starting_xi(false);
        let save_id = sm.create_save(&game, "Repair XI Career").unwrap();
        let db_path = saves_dir.join(format!("{}.db", save_id));

        {
            let db = GameDatabase::open(&db_path).unwrap();
            let mirrored_xi_json = serde_json::to_string(&vec![
                "gk", "rb", "cb1", "cb2", "lb", "rm", "cm1", "cm2", "lm", "st1", "st2",
            ])
            .unwrap();
            db.conn()
                .execute(
                    "UPDATE teams SET starting_xi_ids = ?1 WHERE id = ?2",
                    params![mirrored_xi_json, "team-001"],
                )
                .unwrap();
        }

        let loaded = sm.load_game(&save_id).unwrap();
        let team = loaded
            .teams
            .iter()
            .find(|team| team.id == "team-001")
            .unwrap();

        assert_eq!(
            team.starting_xi_ids,
            vec![
                "gk", "lb", "cb1", "cb2", "rb", "lm", "cm1", "cm2", "rm", "st1", "st2"
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
        );

        let db = GameDatabase::open(&db_path).unwrap();
        let starting_xi_json: String = db
            .conn()
            .query_row(
                "SELECT starting_xi_ids FROM teams WHERE id = ?1",
                params!["team-001"],
                |row| row.get(0),
            )
            .unwrap();
        let starting_xi_ids: Vec<String> = serde_json::from_str(&starting_xi_json).unwrap();

        assert_eq!(starting_xi_ids, team.starting_xi_ids);
    }

    #[test]
    fn test_load_game_backfills_opening_youth_academy_for_legacy_save() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_opening_save_without_youth_academy();
        let save_id = sm
            .create_save(&game, "Legacy Youth Academy Career")
            .unwrap();
        let db_path = saves_dir.join(format!("{}.db", save_id));

        let loaded = sm.load_game(&save_id).unwrap();
        let youth_players: Vec<_> = loaded
            .players
            .iter()
            .filter(|player| player.squad_role == SquadRole::Youth)
            .collect();

        assert_eq!(youth_players.len(), 3);
        assert!(
            youth_players
                .iter()
                .all(|player| player.position != Position::Goalkeeper)
        );

        let db = GameDatabase::open(&db_path).unwrap();
        let persisted = GamePersistenceReader::read_game(&db).unwrap();
        assert_eq!(
            persisted
                .players
                .iter()
                .filter(|player| player.squad_role == SquadRole::Youth)
                .count(),
            3
        );
    }

    #[test]
    fn test_load_game_does_not_backfill_opening_youth_academy_after_opening_window() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let mut game = sample_opening_save_without_youth_academy();
        game.clock.advance_days(31);

        let save_id = sm
            .create_save(&game, "Late Legacy Youth Academy Career")
            .unwrap();
        let loaded = sm.load_game(&save_id).unwrap();

        assert_eq!(
            loaded
                .players
                .iter()
                .filter(|player| player.squad_role == SquadRole::Youth)
                .count(),
            0
        );
    }

    #[test]
    fn test_delete_save() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game();

        let save_id = sm.create_save(&game, "To Delete").unwrap();
        assert_eq!(sm.list_saves().len(), 1);

        let deleted = sm.delete_save(&save_id).unwrap();
        assert!(deleted);
        assert!(sm.list_saves().is_empty());

        // File should be gone
        let db_path = saves_dir.join(format!("{}.db", save_id));
        assert!(!db_path.exists());
    }

    #[test]
    fn test_delete_nonexistent_save() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let deleted = sm.delete_save("nonexistent").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_load_nonexistent_save_uses_backend_key() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let result = sm.load_game("nonexistent");
        assert_eq!(
            result.unwrap_err(),
            "be.error.saveNotFound?saveId=nonexistent"
        );
    }

    #[test]
    fn test_save_to_nonexistent_save_uses_backend_key() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game();
        let result = sm.save_game(&game, "nonexistent");
        assert_eq!(
            result.unwrap_err(),
            "be.error.saveNotFound?saveId=nonexistent"
        );
    }

    #[test]
    fn test_load_stats_for_nonexistent_save_uses_backend_key() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let result = sm.load_stats_state("nonexistent");

        assert_eq!(
            result.unwrap_err(),
            "be.error.saveNotFound?saveId=nonexistent"
        );
    }

    #[test]
    fn test_multiple_saves() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game();

        let id1 = sm.create_save(&game, "Career 1").unwrap();
        let id2 = sm.create_save(&game, "Career 2").unwrap();
        let id3 = sm.create_save(&game, "Career 3").unwrap();

        assert_eq!(sm.list_saves().len(), 3);
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);

        // Delete one
        sm.delete_save(&id2).unwrap();
        assert_eq!(sm.list_saves().len(), 2);

        // Others still loadable
        sm.load_game(&id1).unwrap();
        sm.load_game(&id3).unwrap();
    }

    #[test]
    fn test_index_persists_across_reinit() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        // Create a save
        {
            let mut sm = SaveManager::init(&saves_dir).unwrap();
            let game = sample_game();
            sm.create_save(&game, "Persistent Career").unwrap();
        }

        // Re-init — should find the save in the index
        let sm = SaveManager::init(&saves_dir).unwrap();
        assert_eq!(sm.list_saves().len(), 1);
        assert_eq!(sm.list_saves()[0].name, "Persistent Career");
    }

    #[test]
    fn test_game_with_objectives_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let mut game = sample_game();
        game.board_objectives.push(BoardObjective {
            id: "obj-001".to_string(),
            description: "Finish top 4".to_string(),
            target: 4,
            objective_type: ObjectiveType::LeaguePosition,
            met: false,
        });

        let save_id = sm.create_save(&game, "With Objectives").unwrap();
        let loaded = sm.load_game(&save_id).unwrap();

        assert_eq!(loaded.board_objectives.len(), 1);
        assert_eq!(loaded.board_objectives[0].description, "Finish top 4");
    }

    #[test]
    fn test_game_with_scouting_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let mut game = sample_game();
        game.scouting_assignments.push(ScoutingAssignment {
            id: "sa-001".to_string(),
            scout_id: "staff-001".to_string(),
            player_id: "p-001".to_string(),
            days_remaining: 7,
        });
        game.youth_scouting_assignments
            .push(YouthScoutingAssignment {
                id: "ysa-001".to_string(),
                scout_id: "staff-001".to_string(),
                region: ofm_core::game::YouthScoutingRegion::Domestic,
                objective: ofm_core::game::YouthScoutingObjective::Balanced,
                target_position: Some(domain::player::Position::Defender),
                days_remaining: 5,
            });

        let save_id = sm.create_save(&game, "With Scouting").unwrap();
        let loaded = sm.load_game(&save_id).unwrap();

        assert_eq!(loaded.scouting_assignments.len(), 1);
        assert_eq!(loaded.scouting_assignments[0].days_remaining, 7);
        assert_eq!(loaded.youth_scouting_assignments.len(), 1);
        assert_eq!(
            loaded.youth_scouting_assignments[0].target_position,
            Some(domain::player::Position::Defender)
        );
        assert_eq!(loaded.youth_scouting_assignments[0].days_remaining, 5);
    }

    #[test]
    fn test_new_game_from_save_strips_session_data() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let mut game = sample_game();

        // Add session-specific data
        game.clock.advance_days(30);
        game.board_objectives.push(BoardObjective {
            id: "obj-1".to_string(),
            description: "Win".to_string(),
            target: 10,
            objective_type: ObjectiveType::Wins,
            met: false,
        });
        game.scouting_assignments.push(ScoutingAssignment {
            id: "sa-1".to_string(),
            scout_id: "staff-001".to_string(),
            player_id: "p-001".to_string(),
            days_remaining: 5,
        });
        game.youth_scouting_assignments
            .push(YouthScoutingAssignment {
                id: "ysa-1".to_string(),
                scout_id: "staff-001".to_string(),
                region: ofm_core::game::YouthScoutingRegion::Domestic,
                objective: ofm_core::game::YouthScoutingObjective::Balanced,
                target_position: Some(domain::player::Position::Forward),
                days_remaining: 6,
            });
        game.manager.reputation = 999;

        let save_id = sm.create_save(&game, "Source Save").unwrap();

        // Create new game from this save
        let new_game = sm.new_game_from_save(&save_id).unwrap();

        // Session data should be stripped
        assert!(new_game.messages.is_empty());
        assert!(new_game.news.is_empty());
        assert!(new_game.scouting_assignments.is_empty());
        assert!(new_game.youth_scouting_assignments.is_empty());
        assert!(new_game.board_objectives.is_empty());
        assert!(new_game.league.is_none());

        // Clock should be reset
        assert_eq!(new_game.clock.current_date, new_game.clock.start_date);

        // World data should be preserved
        assert_eq!(new_game.teams.len(), 1);
        assert_eq!(new_game.teams[0].name, "London FC");
        assert_eq!(new_game.players.len(), 1);
        assert_eq!(new_game.staff.len(), 1);

        // Manager should be reset
        assert_eq!(new_game.manager.satisfaction, 100);
        assert_eq!(new_game.manager.fan_approval, 50);

        // Player stats should be reset
        assert!(!new_game.players[0].transfer_listed);
        assert!(!new_game.players[0].loan_listed);
    }

    #[test]
    fn test_new_game_from_nonexistent_save() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let result = sm.new_game_from_save("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_game_cleans_stale_league_rows() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let game = sample_game_with_league();
        let save_id = sm.create_save(&game, "League Cleanup Career").unwrap();
        let db_path = saves_dir.join(format!("{}.db", save_id));

        {
            let db = GameDatabase::open(&db_path).unwrap();
            db.conn()
                .execute(
                    "INSERT INTO league (id, name, season) VALUES (?1, ?2, ?3)",
                    rusqlite::params!["league-stale", "Premier Division", 2026],
                )
                .unwrap();
            db.conn()
                .execute(
                    "INSERT INTO fixtures (id, league_id, matchday, date, home_team_id, away_team_id, status, result)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        "fix-stale",
                        "league-stale",
                        1,
                        "2026-08-15",
                        "team-001",
                        "team-002",
                        "Completed",
                        None::<String>,
                    ],
                )
                .unwrap();
        }

        let loaded = sm.load_game(&save_id).unwrap();
        let loaded_league = loaded.league.expect("league should load");

        assert_eq!(loaded_league.id, "league-current");
        assert_eq!(loaded_league.season, 2027);
        assert_eq!(loaded_league.fixtures.len(), 1);
        assert_eq!(loaded_league.fixtures[0].id, "fix-current");

        let db = GameDatabase::open(&db_path).unwrap();
        let league_count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM league", [], |row| row.get(0))
            .unwrap();
        let fixture_count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM fixtures", [], |row| row.get(0))
            .unwrap();

        assert_eq!(league_count, 1);
        assert_eq!(fixture_count, 1);
    }
}
