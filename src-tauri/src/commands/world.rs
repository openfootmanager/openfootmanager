use log::info;
use tauri::Manager as TauriManager;
use tauri::State;

use ofm_core::state::StateManager;

fn export_world_database_internal(
    state: &StateManager,
    export_path: &std::path::Path,
) -> Result<String, String> {
    let game = state
        .get_game(|g| g.clone())
        .ok_or("No active game session".to_string())?;

    let club_count = game
        .teams
        .iter()
        .map(|team| {
            if team.club_id.trim().is_empty() {
                team.id.clone()
            } else {
                team.club_id.clone()
            }
        })
        .collect::<std::collections::HashSet<_>>()
        .len();
    let world = ofm_core::generator::WorldData {
        name: "Exported World".to_string(),
        description: format!(
            "World with {} clubs and {} teams exported from saved game",
            club_count,
            game.teams.len()
        ),
        countries: vec![],
        clubs: vec![],
        teams: game.teams.clone(),
        players: game.players.clone(),
        staff: game.staff.clone(),
    };

    let json = ofm_core::generator::export_world_to_json(&world)?;
    std::fs::write(export_path, json).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(export_path.to_string_lossy().to_string())
}

fn write_database_json_to_dir(db_dir: &std::path::Path, json: &str) -> Result<String, String> {
    std::fs::create_dir_all(db_dir).map_err(|e| e.to_string())?;

    let world = ofm_core::generator::load_world_from_json(json)?;
    let normalized_json = ofm_core::generator::export_world_to_json(&world)?;

    let filename = format!(
        "imported_{}.json",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    let path = db_dir.join(filename);
    std::fs::write(&path, normalized_json)
        .map_err(|e| format!("Failed to write database: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}

/// List available world databases (built-in random + any user JSON files).
#[tauri::command]
pub fn list_world_databases(
    app_handle: tauri::AppHandle,
) -> Result<Vec<ofm_core::generator::WorldDatabaseInfo>, String> {
    info!("[cmd] list_world_databases");
    use ofm_core::generator::WorldDatabaseInfo;

    // Always include the built-in random option
    let mut databases = vec![WorldDatabaseInfo {
        id: "random".to_string(),
        name: "Random World".to_string(),
        description: "Randomly generated world with countries, clubs, and teams".to_string(),
        country_count: 4,
        club_count: 80,
        team_count: 80,
        player_count: 1760,
        source: "builtin".to_string(),
        path: String::new(),
    }];

    // Scan bundled databases directory (next to the executable / in resources)
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let bundled_dir = resource_dir.join("databases");
        let mut bundled = ofm_core::generator::scan_world_databases(&bundled_dir);
        for db in &mut bundled {
            db.source = "builtin".to_string();
        }
        databases.extend(bundled);
    }

    // Scan user databases directory in app data
    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        let user_dir = app_data_dir.join("databases");
        let user_dbs = ofm_core::generator::scan_world_databases(&user_dir);
        databases.extend(user_dbs);
    }

    Ok(databases)
}

/// Export the current world data to a JSON file so it can be shared/reused.
#[tauri::command]
pub fn export_world_database(
    state: State<'_, StateManager>,
    export_path: String,
) -> Result<String, String> {
    info!("[cmd] export_world_database: path={}", export_path);
    export_world_database_internal(&state, std::path::Path::new(&export_path))
}

/// Write imported world database JSON to the user's databases directory.
/// Returns the full path so the frontend can pass it to start_new_game.
#[tauri::command]
pub fn write_temp_database(app_handle: tauri::AppHandle, json: String) -> Result<String, String> {
    info!("[cmd] write_temp_database: json_len={}", json.len());
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let db_dir = app_data_dir.join("databases");
    write_database_json_to_dir(&db_dir, &json)
}

#[cfg(test)]
mod tests {
    use super::{export_world_database_internal, write_database_json_to_dir};
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::generator::WorldData;
    use ofm_core::state::StateManager;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempCommandDir {
        path: PathBuf,
    }

    impl TempCommandDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ofm-world-command-tests-{}", unique));
            fs::create_dir_all(&path).expect("temporary command dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempCommandDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn sample_attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 65,
            stamina: 65,
            strength: 65,
            agility: 65,
            passing: 65,
            shooting: 65,
            tackling: 65,
            dribbling: 65,
            defending: 65,
            positioning: 65,
            vision: 65,
            decisions: 65,
            composure: 65,
            aggression: 50,
            teamwork: 65,
            leadership: 50,
            handling: 20,
            reflexes: 20,
            aerial: 60,
        }
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr-1".to_string(),
            "Ada".to_string(),
            "Lovelace".to_string(),
            "1980-01-01".to_string(),
            "British".to_string(),
        );
        manager.hire("team-1".to_string());

        let mut team = Team::new(
            "team-1".to_string(),
            "London FC".to_string(),
            "LFC".to_string(),
            "GB".to_string(),
            "London".to_string(),
            "London Arena".to_string(),
            50_000,
        );
        team.football_nation.clear();

        let mut player = Player::new(
            "player-1".to_string(),
            "J. Doe".to_string(),
            "John Doe".to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            sample_attrs(),
        );
        player.team_id = Some("team-1".to_string());
        player.football_nation.clear();
        player.birth_country = None;

        Game::new(clock, manager, vec![team], vec![player], vec![], vec![])
    }

    #[test]
    fn export_world_database_internal_writes_canonicalized_world_json() {
        let temp_dir = TempCommandDir::new();
        let export_path = temp_dir.path().join("world-export.json");
        let state = StateManager::new();
        let mut game = make_game();
        game.teams[0].football_nation.clear();
        game.players[0].football_nation.clear();
        game.players[0].birth_country = None;
        state.set_game(game);

        let written_path = export_world_database_internal(&state, &export_path).unwrap();
        let json = fs::read_to_string(&written_path).unwrap();
        let world: WorldData = serde_json::from_str(&json).unwrap();

        assert_eq!(world.teams[0].football_nation, "ENG");
        assert_eq!(world.players[0].football_nation, "ENG");
    }

    #[test]
    fn write_database_json_to_dir_normalizes_imported_world_json() {
        let temp_dir = TempCommandDir::new();
        let json = r##"
        {
          "name": "Legacy Import",
          "description": "Old GB import",
          "teams": [
            {
              "id": "team-1",
              "name": "London FC",
              "short_name": "LFC",
              "country": "GB",
              "city": "London",
              "stadium_name": "London Arena",
              "stadium_capacity": 50000,
              "finance": 1000000,
              "manager_id": null,
              "reputation": 500,
              "wage_budget": 100000,
              "transfer_budget": 250000,
              "season_income": 0,
              "season_expenses": 0,
              "formation": "4-4-2",
              "play_style": "Balanced",
              "training_focus": "Physical",
              "training_intensity": "Medium",
              "training_schedule": "Balanced",
              "founded_year": 1900,
              "colors": { "primary": "#ffffff", "secondary": "#000000" },
              "starting_xi_ids": [],
              "match_roles": { "captain": null, "vice_captain": null, "penalty_taker": null, "free_kick_taker": null, "corner_taker": null },
              "form": [],
              "history": []
            }
          ],
          "players": [
            {
              "id": "player-1",
              "match_name": "J. Doe",
              "full_name": "John Doe",
              "date_of_birth": "2000-01-01",
              "nationality": "GB",
              "position": "Midfielder",
              "natural_position": "Midfielder",
              "alternate_positions": [],
              "footedness": "Right",
              "weak_foot": 2,
              "attributes": {
                "pace": 70, "stamina": 70, "strength": 70, "agility": 70,
                "passing": 70, "shooting": 70, "tackling": 70, "dribbling": 70,
                "defending": 70, "positioning": 70, "vision": 70, "decisions": 70,
                "composure": 70, "aggression": 70, "teamwork": 70, "leadership": 70,
                "handling": 20, "reflexes": 20, "aerial": 60
              },
              "condition": 100,
              "morale": 100,
              "fitness": 75,
              "injury": null,
              "team_id": "team-1",
              "traits": [],
              "contract_end": null,
              "wage": 0,
              "market_value": 0,
              "stats": { "appearances": 0, "goals": 0, "assists": 0, "clean_sheets": 0, "yellow_cards": 0, "red_cards": 0, "avg_rating": 0.0, "minutes_played": 0 },
              "career": [],
              "training_focus": null,
              "transfer_listed": false,
              "loan_listed": false,
              "transfer_offers": [],
              "morale_core": { "manager_trust": 50, "unresolved_issue": null, "recent_treatment": null, "pending_promise": null, "talk_cooldown_until": null, "renewal_state": null }
            }
          ],
          "staff": []
        }
        "##;

        let written_path = write_database_json_to_dir(temp_dir.path(), json).unwrap();
        let stored_json = fs::read_to_string(&written_path).unwrap();
        let world: WorldData = serde_json::from_str(&stored_json).unwrap();

        assert_eq!(world.teams[0].football_nation, "ENG");
        assert_eq!(world.players[0].football_nation, "ENG");
    }

    #[test]
    fn write_database_json_to_dir_rejects_invalid_json() {
        let temp_dir = TempCommandDir::new();
        let result = write_database_json_to_dir(temp_dir.path(), "not valid json");

        assert!(result.is_err());
        let written_files = fs::read_dir(temp_dir.path()).unwrap().count();
        assert_eq!(written_files, 0);
    }
}
