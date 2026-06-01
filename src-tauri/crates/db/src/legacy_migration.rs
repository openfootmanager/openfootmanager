use rusqlite::Connection;
use std::fs;
use std::path::Path;

use ofm_core::game::Game;
use ofm_core::player_identity;

use crate::save_manager::{SaveManager, canonicalize_game_starting_xi_ids};

const LEGACY_MIGRATION_FAILED_ERROR: &str = "be.error.gameDatabase.migrationFailed";

/// A row extracted from the legacy `saves.db` file.
#[derive(Debug)]
#[allow(dead_code)]
struct LegacySaveRow {
    id: String,
    name: String,
    manager_name: String,
    game_data: String,
    created_at: String,
    last_played_at: String,
}

/// Result of migrating one legacy save.
#[derive(Debug)]
pub enum LegacyMigrationResult {
    /// Successfully migrated to a new per-save DB.
    Success {
        old_id: String,
        new_id: String,
        name: String,
    },
    /// Failed to migrate (corrupt JSON, etc).
    Failed {
        old_id: String,
        name: String,
        reason: String,
    },
}

/// Check if a legacy `saves.db` exists at the given path.
pub fn has_legacy_db(app_data_dir: &Path) -> bool {
    app_data_dir.join("saves.db").exists()
}

/// Migrate all saves from the legacy `saves.db` into per-save databases
/// managed by the SaveManager. Returns results for each save attempted.
///
/// After successful migration, renames `saves.db` to `saves.db.migrated`.
pub fn migrate_legacy_saves(
    app_data_dir: &Path,
    save_manager: &mut SaveManager,
) -> Result<Vec<LegacyMigrationResult>, String> {
    let legacy_path = app_data_dir.join("saves.db");
    if !legacy_path.exists() {
        return Ok(Vec::new());
    }

    let rows = extract_legacy_rows(&legacy_path)?;

    let mut results = Vec::new();

    for row in &rows {
        match migrate_single_save(row, save_manager) {
            Ok(new_id) => {
                results.push(LegacyMigrationResult::Success {
                    old_id: row.id.clone(),
                    new_id,
                    name: row.name.clone(),
                });
            }
            Err(reason) => {
                results.push(LegacyMigrationResult::Failed {
                    old_id: row.id.clone(),
                    name: row.name.clone(),
                    reason,
                });
            }
        }
    }

    // Rename the old database to prevent re-migration
    let migrated_path = app_data_dir.join("saves.db.migrated");
    fs::rename(&legacy_path, &migrated_path)
        .map_err(|_| LEGACY_MIGRATION_FAILED_ERROR.to_string())?;

    Ok(results)
}

/// Extract all save rows from the legacy database.
fn extract_legacy_rows(legacy_path: &Path) -> Result<Vec<LegacySaveRow>, String> {
    let conn =
        Connection::open(legacy_path).map_err(|_| LEGACY_MIGRATION_FAILED_ERROR.to_string())?;

    // Check if the saves table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='saves'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .map_err(|_| LEGACY_MIGRATION_FAILED_ERROR.to_string())?;

    if !table_exists {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, name, manager_name, game_data, created_at, last_played_at
             FROM saves ORDER BY last_played_at DESC",
        )
        .map_err(|_| LEGACY_MIGRATION_FAILED_ERROR.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(LegacySaveRow {
                id: row.get(0)?,
                name: row.get(1)?,
                manager_name: row.get(2)?,
                game_data: row.get(3)?,
                created_at: row.get(4)?,
                last_played_at: row.get(5)?,
            })
        })
        .map_err(|_| LEGACY_MIGRATION_FAILED_ERROR.to_string())?;

    let mut saves = Vec::new();
    for row in rows {
        saves.push(row.map_err(|_| LEGACY_MIGRATION_FAILED_ERROR.to_string())?);
    }
    Ok(saves)
}

/// Migrate a single legacy save by deserializing the JSON blob and
/// creating a new save via SaveManager.
fn migrate_single_save(
    row: &LegacySaveRow,
    save_manager: &mut SaveManager,
) -> Result<String, String> {
    let mut game: Game = serde_json::from_str(&row.game_data)
        .map_err(|_| LEGACY_MIGRATION_FAILED_ERROR.to_string())?;

    canonicalize_game_starting_xi_ids(&mut game);
    player_identity::upgrade_game_player_identities(&mut game);
    ofm_core::football_identity::upgrade_game_football_identities(&mut game);

    save_manager.create_save(&game, &row.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::fs;

    /// Create a legacy saves.db with the old schema and some test data.
    fn create_legacy_db(path: &Path, saves: &[(&str, &str, &str, &str)]) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE saves (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                manager_name    TEXT NOT NULL,
                game_data       TEXT NOT NULL,
                created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                last_played_at  TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();

        for (id, name, manager_name, game_data) in saves {
            conn.execute(
                "INSERT INTO saves (id, name, manager_name, game_data) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, manager_name, game_data],
            )
            .unwrap();
        }
    }

    /// Generate a minimal valid Game JSON.
    fn minimal_game_json() -> String {
        use chrono::{TimeZone, Utc};
        use domain::player::{PlayerAttributes, Position};
        use domain::staff::{StaffAttributes, StaffRole};
        use ofm_core::clock::GameClock;
        use ofm_core::game::Game;

        let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let clock = GameClock::new(start);
        let manager = domain::manager::Manager::new(
            "mgr-001".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1990-01-01".to_string(),
            "GB".to_string(),
        );
        let team = domain::team::Team::new(
            "team-001".to_string(),
            "Test FC".to_string(),
            "TFC".to_string(),
            "GB".to_string(),
            "London".to_string(),
            "Test Stadium".to_string(),
            30000,
        );
        let player = domain::player::Player::new(
            "p-001".to_string(),
            "J. Test".to_string(),
            "John Test".to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            PlayerAttributes {
                pace: 50,
                stamina: 50,
                strength: 50,
                agility: 50,
                passing: 50,
                shooting: 50,
                tackling: 50,
                dribbling: 50,
                defending: 50,
                positioning: 50,
                vision: 50,
                decisions: 50,
                composure: 50,
                aggression: 50,
                teamwork: 50,
                leadership: 50,
                handling: 50,
                reflexes: 50,
                aerial: 50,
            },
        );
        let staff = domain::staff::Staff::new(
            "staff-001".to_string(),
            "A".to_string(),
            "Coach".to_string(),
            "1980-01-01".to_string(),
            StaffRole::Coach,
            StaffAttributes {
                coaching: 50,
                judging_ability: 50,
                judging_potential: 50,
                physiotherapy: 50,
            },
        );

        let game = Game::new(
            clock,
            manager,
            vec![team],
            vec![player],
            vec![staff],
            vec![],
        );
        serde_json::to_string(&game).unwrap()
    }

    fn legacy_game_json_with_partial_morale_core() -> String {
        let mut json: serde_json::Value =
            serde_json::from_str(&minimal_game_json()).expect("minimal game json should parse");

        json["players"][0]["morale_core"] = serde_json::json!({
            "manager_trust": 63,
            "unresolved_issue": {
                "category": "PlayingTime",
                "severity": 55
            },
            "recent_treatment": null
        });

        serde_json::to_string(&json).expect("legacy game json should serialize")
    }

    fn legacy_game_json_with_mirrored_starting_xi() -> String {
        use chrono::{TimeZone, Utc};
        use domain::player::{Footedness, Player, PlayerAttributes, Position};
        use ofm_core::clock::GameClock;

        let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let clock = GameClock::new(start);
        let mut manager = domain::manager::Manager::new(
            "mgr-001".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1990-01-01".to_string(),
            "GB".to_string(),
        );
        manager.hire("team-001".to_string());

        let mut team = domain::team::Team::new(
            "team-001".to_string(),
            "Test FC".to_string(),
            "TFC".to_string(),
            "GB".to_string(),
            "London".to_string(),
            "Test Stadium".to_string(),
            30000,
        );
        team.formation = "4-4-2".to_string();
        team.starting_xi_ids = vec![
            "gk", "rb", "cb1", "cb2", "lb", "rm", "cm1", "cm2", "lm", "st1", "st2",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();

        let make_player = |id: &str, position: Position, footedness: Footedness| {
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
        };

        let players = vec![
            make_player("gk", Position::Goalkeeper, Footedness::Right),
            make_player("lb", Position::LeftBack, Footedness::Left),
            make_player("cb1", Position::CenterBack, Footedness::Right),
            make_player("cb2", Position::CenterBack, Footedness::Right),
            make_player("rb", Position::RightBack, Footedness::Right),
            make_player("lm", Position::LeftMidfielder, Footedness::Left),
            make_player("cm1", Position::CentralMidfielder, Footedness::Right),
            make_player("cm2", Position::CentralMidfielder, Footedness::Right),
            make_player("rm", Position::RightMidfielder, Footedness::Right),
            make_player("st1", Position::Striker, Footedness::Right),
            make_player("st2", Position::Striker, Footedness::Right),
        ];

        let game = Game::new(clock, manager, vec![team], players, vec![], vec![]);
        serde_json::to_string(&game).expect("legacy mirrored xi game json should serialize")
    }

    fn legacy_game_json_for_position_identity_upgrade() -> String {
        let mut json: serde_json::Value =
            serde_json::from_str(&minimal_game_json()).expect("minimal game json should parse");

        json["teams"][0]["formation"] = serde_json::json!("4-4-2");
        json["teams"][0]["starting_xi_ids"] = serde_json::json!(["legacy-gk", "p-001"]);
        json["players"][0]["position"] = serde_json::json!("Defender");
        json["players"][0]["natural_position"] = serde_json::json!("Defender");
        json["players"][0]["alternate_positions"] = serde_json::json!([]);
        json["players"][0]["attributes"] = serde_json::json!({
            "pace": 84,
            "stamina": 82,
            "strength": 63,
            "agility": 72,
            "passing": 64,
            "shooting": 40,
            "tackling": 77,
            "dribbling": 62,
            "defending": 72,
            "positioning": 66,
            "vision": 58,
            "decisions": 64,
            "composure": 60,
            "aggression": 64,
            "teamwork": 74,
            "leadership": 44,
            "handling": 20,
            "reflexes": 20,
            "aerial": 46
        });

        serde_json::to_string(&json).expect("legacy game json should serialize")
    }

    fn legacy_game_json_with_partial_transfer_offer() -> String {
        let mut json: serde_json::Value =
            serde_json::from_str(&minimal_game_json()).expect("minimal game json should parse");

        json["players"][0]["transfer_offers"] = serde_json::json!([
            {
                "id": "offer-legacy-1",
                "from_team_id": "team-999",
                "fee": 900000,
                "wage_offered": 0
            }
        ]);

        serde_json::to_string(&json).expect("legacy game json should serialize")
    }

    fn legacy_game_json_with_partial_facilities() -> String {
        let mut json: serde_json::Value =
            serde_json::from_str(&minimal_game_json()).expect("minimal game json should parse");

        json["teams"][0]["facilities"] = serde_json::json!({
            "training": 3
        });

        serde_json::to_string(&json).expect("legacy game json should serialize")
    }

    fn legacy_game_json_with_partial_sponsorship() -> String {
        let mut json: serde_json::Value =
            serde_json::from_str(&minimal_game_json()).expect("minimal game json should parse");

        json["teams"][0]["sponsorship"] = serde_json::json!({
            "sponsor_name": "Acme Corp"
        });

        serde_json::to_string(&json).expect("legacy game json should serialize")
    }

    fn legacy_game_json_with_partial_recent_treatment() -> String {
        let mut json: serde_json::Value =
            serde_json::from_str(&minimal_game_json()).expect("minimal game json should parse");

        json["players"][0]["morale_core"] = serde_json::json!({
            "manager_trust": 63,
            "recent_treatment": {
                "action_key": "praise"
            }
        });

        serde_json::to_string(&json).expect("legacy game json should serialize")
    }

    fn legacy_game_json_with_partial_pending_promise() -> String {
        let mut json: serde_json::Value =
            serde_json::from_str(&minimal_game_json()).expect("minimal game json should parse");

        json["players"][0]["morale_core"] = serde_json::json!({
            "manager_trust": 63,
            "pending_promise": {
                "kind": "PlayingTime"
            }
        });

        serde_json::to_string(&json).expect("legacy game json should serialize")
    }

    #[test]
    fn test_has_legacy_db() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!has_legacy_db(dir.path()));

        fs::write(dir.path().join("saves.db"), "").unwrap();
        assert!(has_legacy_db(dir.path()));
    }

    #[test]
    fn test_migrate_no_legacy_db() {
        let dir = tempfile::tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        let mut sm = SaveManager::init(&saves_dir).unwrap();

        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_migrate_single_save() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");
        let json = minimal_game_json();

        create_legacy_db(
            &legacy_path,
            &[("old-save-1", "Test Career", "Test Manager", &json)],
        );

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], LegacyMigrationResult::Success { .. }));

        // saves.db should be renamed
        assert!(!legacy_path.exists());
        assert!(dir.path().join("saves.db.migrated").exists());

        // New save should be loadable
        assert_eq!(sm.list_saves().len(), 1);
        let save_id = sm.list_saves()[0].id.clone();
        let loaded = sm.load_game(&save_id).unwrap();
        assert_eq!(loaded.manager.first_name, "Test");
    }

    #[test]
    fn test_migrate_multiple_saves() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");
        let json = minimal_game_json();

        create_legacy_db(
            &legacy_path,
            &[
                ("old-1", "Career 1", "Manager A", &json),
                ("old-2", "Career 2", "Manager B", &json),
                ("old-3", "Career 3", "Manager C", &json),
            ],
        );

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(matches!(r, LegacyMigrationResult::Success { .. }));
        }
        assert_eq!(sm.list_saves().len(), 3);
    }

    #[test]
    fn test_migrate_with_corrupt_json() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");
        let json = minimal_game_json();

        create_legacy_db(
            &legacy_path,
            &[
                ("good-1", "Good Save", "Manager A", &json),
                ("bad-1", "Bad Save", "Manager B", "not valid json"),
            ],
        );

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert_eq!(results.len(), 2);

        let successes: Vec<_> = results
            .iter()
            .filter(|r| matches!(r, LegacyMigrationResult::Success { .. }))
            .collect();
        let failures: Vec<_> = results
            .iter()
            .filter(|r| matches!(r, LegacyMigrationResult::Failed { .. }))
            .collect();

        assert_eq!(successes.len(), 1);
        assert_eq!(failures.len(), 1);

        // Good save should still be loadable
        assert_eq!(sm.list_saves().len(), 1);
    }

    #[test]
    fn test_migrate_empty_legacy_db() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");

        create_legacy_db(&legacy_path, &[]);

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert!(results.is_empty());
        assert!(!legacy_path.exists()); // Renamed even if empty
        assert!(dir.path().join("saves.db.migrated").exists());
    }

    #[test]
    fn test_migrate_legacy_save_with_partial_morale_core_defaults_new_fields() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");
        let json = legacy_game_json_with_partial_morale_core();

        create_legacy_db(
            &legacy_path,
            &[("old-save-1", "Legacy Morale Save", "Test Manager", &json)],
        );

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], LegacyMigrationResult::Success { .. }));

        let save_id = sm.list_saves()[0].id.clone();
        let loaded = sm.load_game(&save_id).unwrap();
        let player = loaded
            .players
            .iter()
            .find(|player| player.id == "p-001")
            .unwrap();

        assert_eq!(player.morale_core.manager_trust, 63);
        assert_eq!(
            player
                .morale_core
                .unresolved_issue
                .as_ref()
                .map(|issue| issue.severity),
            Some(55)
        );
        assert_eq!(player.morale_core.pending_promise, None);
        assert_eq!(player.morale_core.talk_cooldown_until, None);
        assert_eq!(player.morale_core.renewal_state, None);
    }

    #[test]
    fn test_migrate_legacy_save_with_partial_transfer_offer_defaults_negotiation_fields() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");
        let json = legacy_game_json_with_partial_transfer_offer();

        create_legacy_db(
            &legacy_path,
            &[("old-save-2", "Legacy Transfer Save", "Test Manager", &json)],
        );

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], LegacyMigrationResult::Success { .. }));

        let save_id = sm.list_saves()[0].id.clone();
        let loaded = sm.load_game(&save_id).unwrap();
        let player = loaded
            .players
            .iter()
            .find(|player| player.id == "p-001")
            .unwrap();

        assert_eq!(player.transfer_offers.len(), 1);
        assert_eq!(player.transfer_offers[0].id, "offer-legacy-1");
        assert_eq!(player.transfer_offers[0].from_team_id, "team-999");
        assert_eq!(player.transfer_offers[0].fee, 900_000);
        assert_eq!(format!("{:?}", player.transfer_offers[0].status), "Pending");
        assert_eq!(player.transfer_offers[0].date, "");
    }

    #[test]
    fn test_migrate_legacy_save_with_partial_facilities_defaults_missing_levels() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");
        let json = legacy_game_json_with_partial_facilities();

        create_legacy_db(
            &legacy_path,
            &[(
                "old-save-3",
                "Legacy Facilities Save",
                "Test Manager",
                &json,
            )],
        );

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], LegacyMigrationResult::Success { .. }));

        let save_id = sm.list_saves()[0].id.clone();
        let loaded = sm.load_game(&save_id).unwrap();
        let team = loaded
            .teams
            .iter()
            .find(|team| team.id == "team-001")
            .unwrap();

        assert_eq!(team.facilities.training, 3);
        assert_eq!(team.facilities.medical, 1);
        assert_eq!(team.facilities.scouting, 1);
    }

    #[test]
    fn test_migrate_legacy_save_with_partial_sponsorship_defaults_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");
        let json = legacy_game_json_with_partial_sponsorship();

        create_legacy_db(
            &legacy_path,
            &[(
                "old-save-4",
                "Legacy Sponsorship Save",
                "Test Manager",
                &json,
            )],
        );

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], LegacyMigrationResult::Success { .. }));

        let save_id = sm.list_saves()[0].id.clone();
        let loaded = sm.load_game(&save_id).unwrap();
        let team = loaded
            .teams
            .iter()
            .find(|team| team.id == "team-001")
            .unwrap();
        let sponsorship = team
            .sponsorship
            .as_ref()
            .expect("sponsorship should be present");

        assert_eq!(sponsorship.sponsor_name, "Acme Corp");
        assert_eq!(sponsorship.base_value, 0);
        assert_eq!(sponsorship.remaining_weeks, 0);
        assert!(sponsorship.bonus_criteria.is_empty());
    }

    #[test]
    fn test_migrate_legacy_save_with_partial_recent_treatment_defaults_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");
        let json = legacy_game_json_with_partial_recent_treatment();

        create_legacy_db(
            &legacy_path,
            &[(
                "old-save-5",
                "Legacy Recent Treatment Save",
                "Test Manager",
                &json,
            )],
        );

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], LegacyMigrationResult::Success { .. }));

        let save_id = sm.list_saves()[0].id.clone();
        let loaded = sm.load_game(&save_id).unwrap();
        let player = loaded
            .players
            .iter()
            .find(|player| player.id == "p-001")
            .unwrap();
        let recent_treatment = player
            .morale_core
            .recent_treatment
            .as_ref()
            .expect("recent treatment should be present");

        assert_eq!(recent_treatment.action_key, "praise");
        assert_eq!(recent_treatment.times_recently_used, 0);
    }

    #[test]
    fn test_migrate_legacy_save_with_partial_pending_promise_defaults_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");
        let json = legacy_game_json_with_partial_pending_promise();

        create_legacy_db(
            &legacy_path,
            &[(
                "old-save-6",
                "Legacy Pending Promise Save",
                "Test Manager",
                &json,
            )],
        );

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], LegacyMigrationResult::Success { .. }));

        let save_id = sm.list_saves()[0].id.clone();
        let loaded = sm.load_game(&save_id).unwrap();
        let player = loaded
            .players
            .iter()
            .find(|player| player.id == "p-001")
            .unwrap();
        let pending_promise = player
            .morale_core
            .pending_promise
            .as_ref()
            .expect("pending promise should be present");

        assert_eq!(format!("{:?}", pending_promise.kind), "PlayingTime");
        assert_eq!(pending_promise.matches_remaining, 0);
    }

    #[test]
    fn test_migrate_legacy_save_upgrades_player_identity_fields() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");
        let json = legacy_game_json_for_position_identity_upgrade();

        create_legacy_db(
            &legacy_path,
            &[("old-save-7", "Legacy Identity Save", "Test Manager", &json)],
        );

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], LegacyMigrationResult::Success { .. }));

        let save_id = sm.list_saves()[0].id.clone();
        let loaded = sm.load_game(&save_id).unwrap();
        let player = loaded
            .players
            .iter()
            .find(|player| player.id == "p-001")
            .unwrap();

        assert_eq!(player.natural_position, domain::player::Position::LeftBack);
        assert_eq!(player.footedness, domain::player::Footedness::Left);
        assert!(player.weak_foot >= 2);
        assert!(
            player
                .alternate_positions
                .contains(&domain::player::Position::LeftWingBack)
        );
    }

    #[test]
    fn test_migrate_legacy_save_canonicalizes_mirrored_starting_xi_order() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");
        let json = legacy_game_json_with_mirrored_starting_xi();

        create_legacy_db(
            &legacy_path,
            &[(
                "old-save-8",
                "Legacy Mirrored XI Save",
                "Test Manager",
                &json,
            )],
        );

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], LegacyMigrationResult::Success { .. }));

        let save_entry = &sm.list_saves()[0];
        let db_path = saves_dir.join(&save_entry.db_filename);
        let db = Connection::open(&db_path).unwrap();
        let starting_xi_json: String = db
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
    fn test_migrate_legacy_db_no_saves_table() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("saves.db");
        let saves_dir = dir.path().join("saves");

        // Create a DB with no saves table
        let conn = Connection::open(&legacy_path).unwrap();
        conn.execute_batch("CREATE TABLE other (id TEXT);").unwrap();
        drop(conn);

        let mut sm = SaveManager::init(&saves_dir).unwrap();
        let results = migrate_legacy_saves(dir.path(), &mut sm).unwrap();

        assert!(results.is_empty());
        assert!(!legacy_path.exists());
        assert!(dir.path().join("saves.db.migrated").exists());
    }
}
