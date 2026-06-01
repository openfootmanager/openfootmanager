use super::definitions::{WorldData, WorldDatabaseInfo};

const WORLD_PARSE_FAILED_ERROR: &str = "be.error.worldParseFailed";
const WORLD_SERIALIZE_FAILED_ERROR: &str = "be.error.worldSerializeFailed";
const RANDOM_WORLD_NAME_KEY: &str = "be.msg.world.randomName";
const RANDOM_WORLD_DESCRIPTION_KEY: &str = "be.msg.world.randomDescription";

fn backend_text_with_param(key: &str, param_name: &str, param_value: usize) -> String {
    let param_value = param_value.to_string();
    let mut message = String::with_capacity(
        key.len() + param_name.len() + param_value.len() + 2,
    );
    message.push_str(key);
    message.push('?');
    message.push_str(param_name);
    message.push('=');
    message.push_str(&param_value);
    message
}

/// Generate a random world and wrap it in a `WorldData`.
/// If `data_dir` is provided, tries to load definition files from that directory.
pub fn generate_world_data(data_dir: Option<&std::path::Path>) -> WorldData {
    let (mut teams, mut players, mut staff) = super::generate_world(data_dir);
    crate::football_identity::upgrade_world_football_identities(
        &mut teams,
        &mut players,
        &mut staff,
    );

    WorldData {
        name: RANDOM_WORLD_NAME_KEY.to_string(),
        description: backend_text_with_param(
            RANDOM_WORLD_DESCRIPTION_KEY,
            "teamCount",
            teams.len(),
        ),
        teams,
        players,
        staff,
        managers: vec![],
        league: None,
        news: vec![],
        stats: domain::stats::StatsState::default(),
        world_history: domain::world_history::WorldHistoryArchive::default(),
        metadata: super::definitions::WorldDataMetadata::default(),
    }
}

/// Parse a JSON string into a `WorldData`.
pub fn load_world_from_json(json: &str) -> Result<WorldData, String> {
    let mut world: WorldData =
        serde_json::from_str(json).map_err(|_| WORLD_PARSE_FAILED_ERROR.to_string())?;
    crate::football_identity::upgrade_world_football_identities(
        &mut world.teams,
        &mut world.players,
        &mut world.staff,
    );
    crate::football_identity::upgrade_world_manager_identities(&world.teams, &mut world.managers);
    Ok(world)
}

/// Serialise a `WorldData` to a pretty-printed JSON string.
pub fn export_world_to_json(world: &WorldData) -> Result<String, String> {
    let mut normalized = world.clone();
    crate::football_identity::upgrade_world_football_identities(
        &mut normalized.teams,
        &mut normalized.players,
        &mut normalized.staff,
    );
    crate::football_identity::upgrade_world_manager_identities(
        &normalized.teams,
        &mut normalized.managers,
    );
    serde_json::to_string_pretty(&normalized).map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())
}

/// Scan a directory for `.json` world database files and return their metadata.
pub fn scan_world_databases(dir: &std::path::Path) -> Vec<WorldDatabaseInfo> {
    let mut results = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return results;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        // Parse just enough to get metadata — try full parse
        if let Ok(world) = load_world_from_json(&contents) {
            let file_stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let history_mode = match world.metadata.kind {
                crate::generator::WorldDataKind::HistoricalSnapshot => "reference",
                crate::generator::WorldDataKind::RosterBaseline => "hybrid",
            };
            results.push(WorldDatabaseInfo {
                id: format!("file:{}", path.display()),
                name: world.name,
                description: world.description,
                team_count: world.teams.len(),
                player_count: world.players.len(),
                history_mode: history_mode.to_string(),
                base_year: world.metadata.base_year,
                snapshot_date: world.metadata.snapshot_date,
                source: "user".to_string(),
                path: path.to_string_lossy().to_string(),
            });
            // suppress unused variable warning
            let _ = file_stem;
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempWorldDir {
        path: PathBuf,
    }

    impl TempWorldDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ofm-world-io-tests-{}", unique));
            fs::create_dir_all(&path).expect("temporary world dir should be created");
            Self { path }
        }

        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TempWorldDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn load_world_from_json_normalizes_legacy_english_world_data() {
        let json = r##"
                {
                    "name": "Legacy World",
                    "description": "Old GB world",
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

        let world = load_world_from_json(json).unwrap();

        assert_eq!(world.teams[0].football_nation, "ENG");
        assert_eq!(world.players[0].football_nation, "ENG");
        assert_eq!(world.players[0].birth_country, None);
        assert!(world.managers.is_empty());
        assert!(world.league.is_none());
        assert!(world.news.is_empty());
        assert!(world.stats.player_matches.is_empty());
        assert_eq!(world.metadata.kind, crate::generator::WorldDataKind::RosterBaseline);
    }

    #[test]
    fn export_world_to_json_writes_canonical_football_identity_fields() {
        let mut world = generate_world_data(None);
        world.teams[0].country = "GB".to_string();
        world.teams[0].football_nation.clear();

        if let Some(player) = world
            .players
            .iter_mut()
            .find(|player| player.team_id.as_deref() == Some(world.teams[0].id.as_str()))
        {
            player.nationality = "GB".to_string();
            player.football_nation.clear();
            player.birth_country = None;
        }

        let json = export_world_to_json(&world).unwrap();
        let reparsed: WorldData = serde_json::from_str(&json).unwrap();

        assert_eq!(reparsed.name, RANDOM_WORLD_NAME_KEY);
        assert!(
            reparsed
                .description
                .starts_with("be.msg.world.randomDescription?teamCount=")
        );
        assert_eq!(reparsed.teams[0].football_nation, "ENG");
        assert_eq!(
            reparsed.metadata.kind,
            crate::generator::WorldDataKind::RosterBaseline
        );
    }

    #[test]
    fn load_world_from_json_preserves_historical_snapshot_fields() {
        let json = r##"
                {
                    "name": "Snapshot World",
                    "description": "Rich snapshot",
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
                            "manager_id": "mgr-1",
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
                    "players": [],
                    "staff": [],
                    "managers": [
                        {
                            "id": "mgr-1",
                            "first_name": "Ada",
                            "last_name": "Lovelace",
                            "date_of_birth": "1980-01-01",
                            "nationality": "GB",
                            "football_nation": "",
                            "birth_country": null,
                            "reputation": 600,
                            "satisfaction": 75,
                            "fan_approval": 55,
                            "team_id": "team-1",
                            "warning_stage": 0,
                            "career_stats": {
                                "matches_managed": 10,
                                "wins": 4,
                                "draws": 3,
                                "losses": 3,
                                "trophies": 0,
                                "best_finish": 5
                            },
                            "career_history": []
                        }
                    ],
                    "league": {
                        "id": "league-1",
                        "name": "Open League",
                        "season": 2024,
                        "fixtures": [],
                        "standings": []
                    },
                    "news": [
                        {
                            "id": "news-1",
                            "headline": "Season underway",
                            "body": "The campaign has begun.",
                            "source": "World Feed",
                            "date": "2024-08-15",
                            "category": "SeasonPreview",
                            "team_ids": ["team-1"],
                            "player_ids": [],
                            "match_score": null,
                            "read": false,
                            "i18n_params": {}
                        }
                    ],
                    "stats": {
                        "player_matches": [],
                        "team_matches": []
                    },
                    "world_history": {
                        "rivalries": [
                            {
                                "team_a_id": "team-1",
                                "team_b_id": "team-2",
                                "intensity": 66,
                                "started_season": 2023
                            }
                        ],
                        "season_awards": []
                    },
                    "metadata": {
                        "kind": "historicalSnapshot",
                        "base_year": 2024,
                        "snapshot_date": "2024-08-15T00:00:00Z"
                    }
                }
                "##;

        let world = load_world_from_json(json).unwrap();

        assert_eq!(world.managers.len(), 1);
        assert_eq!(world.managers[0].football_nation, "ENG");
        assert_eq!(world.league.as_ref().map(|league| league.season), Some(2024));
        assert_eq!(world.news.len(), 1);
        assert_eq!(world.world_history.rivalries.len(), 1);
        assert_eq!(
            world.metadata.kind,
            crate::generator::WorldDataKind::HistoricalSnapshot
        );
        assert_eq!(world.metadata.base_year, Some(2024));
    }

    #[test]
    fn export_world_to_json_preserves_historical_snapshot_fields() {
        let mut world = generate_world_data(None);
        world.managers.push(domain::manager::Manager::new(
            "mgr-1".to_string(),
            "Ada".to_string(),
            "Lovelace".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        ));
        world.managers[0].team_id = Some(world.teams[0].id.clone());
        world.league = Some(domain::league::League::new(
            "league-1".to_string(),
            "Open League".to_string(),
            2028,
            &[world.teams[0].id.clone()],
        ));
        world.news.push(domain::news::NewsArticle::new(
            "news-1".to_string(),
            "Season underway".to_string(),
            "The campaign has begun.".to_string(),
            "World Feed".to_string(),
            "2028-08-15".to_string(),
            domain::news::NewsCategory::SeasonPreview,
        ));
        world
            .world_history
            .upsert_rivalry("team-1", "team-2", 72, Some(2027));
        world.metadata = crate::generator::WorldDataMetadata {
            kind: crate::generator::WorldDataKind::HistoricalSnapshot,
            base_year: Some(2028),
            snapshot_date: Some("2028-08-15T00:00:00Z".to_string()),
        };

        let json = export_world_to_json(&world).unwrap();
        let reparsed: WorldData = serde_json::from_str(&json).unwrap();

        assert_eq!(reparsed.managers.len(), 1);
        assert_eq!(reparsed.managers[0].football_nation, "ENG");
        assert_eq!(reparsed.league.as_ref().map(|league| league.season), Some(2028));
        assert_eq!(reparsed.news.len(), 1);
        assert_eq!(reparsed.world_history.rivalries.len(), 1);
        assert_eq!(
            reparsed.metadata.kind,
            crate::generator::WorldDataKind::HistoricalSnapshot
        );
    }

    #[test]
    fn load_world_from_json_returns_backend_key_when_invalid_json() {
        let result = load_world_from_json("not valid json");

        assert_eq!(result.unwrap_err(), WORLD_PARSE_FAILED_ERROR);
    }

    #[test]
    fn scan_world_databases_exposes_history_mode_metadata() {
        let temp_dir = TempWorldDir::new();
        let path = temp_dir.path().join("snapshot.json");
        fs::write(
            &path,
            r#"
            {
                "name": "Historical Snapshot",
                "description": "Season already underway",
                "teams": [],
                "players": [],
                "staff": [],
                "metadata": {
                    "kind": "historicalSnapshot",
                    "base_year": 2031,
                    "snapshot_date": "2031-11-20T00:00:00+00:00"
                }
            }
            "#,
        )
        .expect("world json should be written");

        let databases = scan_world_databases(temp_dir.path());
        let database = databases
            .iter()
            .find(|database| database.id == format!("file:{}", path.display()))
            .expect("snapshot database should be scanned");

        assert_eq!(database.history_mode, "reference");
        assert_eq!(database.base_year, Some(2031));
        assert_eq!(
            database.snapshot_date.as_deref(),
            Some("2031-11-20T00:00:00+00:00")
        );
    }
}
