use rusqlite_migration::{M, Migrations};

/// Number of migrations defined. Keep in sync with the vec in `all_migrations`.
pub const MIGRATION_COUNT: usize = 42;

/// All migrations for a per-save game database.
/// Each save `.db` file gets this schema applied via `rusqlite_migration`.
pub fn all_migrations() -> Migrations<'static> {
    Migrations::new(vec![
        // V1: Initial schema — all game entity tables
        M::up(include_str!("sql/v001_initial_schema.sql")),
        // V2: Training groups per team
        M::up(include_str!("sql/v002_training_groups.sql")),
        // V3: Alternate positions per player
        M::up(include_str!("sql/v003_alternate_positions.sql")),
        // V4: Natural/preferred position per player
        M::up(include_str!("sql/v004_natural_position.sql")),
        // V5: Per-player training focus override
        M::up(include_str!("sql/v005_player_training_focus.sql")),
        // V6: Team match roles defaults
        M::up(include_str!("sql/v006_team_match_roles.sql")),
        // V7: Team financial ledger
        M::up(include_str!("sql/v007_team_financial_ledger.sql")),
        // V8: Team sponsorship state
        M::up(include_str!("sql/v008_team_sponsorship.sql")),
        // V9: Team facilities state
        M::up(include_str!("sql/v009_team_facilities.sql")),
        // V10: Hidden per-player morale architecture state
        M::up(include_str!("sql/v010_player_morale_core.sql")),
        // V11: Player footedness identity fields
        M::up(include_str!("sql/v011_player_footedness.sql")),
        // V12: Fixture competition metadata
        M::up(include_str!("sql/v012_fixture_competition.sql")),
        // V13: Player long-term fitness value
        M::up(include_str!("sql/v013_player_fitness.sql")),
        // V14: Explicit football identity fields for teams and people
        M::up(include_str!("sql/v014_football_identity.sql")),
        // V15: Historical player and team match stats
        M::up(include_str!("sql/v015_match_stats_history.sql")),
        // V16: Manager board-warning stage tracking (per-club, resets on hire)
        M::up(include_str!("sql/v016_manager_warning_stage.sql")),
        // V17: Persist vacancy-age tracking for delayed AI manager replacements
        M::up(include_str!("sql/v017_vacant_team_days.sql")),
        // V18: Completed transfer log for world transfer-centre views
        M::up(include_str!("sql/v018_transfer_log.sql")),
        // V19: Explicit senior versus youth squad assignment for players
        M::up(include_str!("sql/v019_player_squad_role.sql")),
        // V20: Persist computed OVR and potential so they survive save/load
        M::up(include_str!("sql/v020_player_ovr_potential.sql")),
        // V21: Persist youth recruitment scouting assignments separately from player scouting
        M::up(include_str!("sql/v021_youth_scouting_assignments.sql")),
        // V22: Persist target position for youth recruitment scouting assignments
        M::up(include_str!("sql/v022_youth_scouting_target_position.sql")),
        // V23: Persist region and objective for youth recruitment scouting assignments
        M::up(include_str!("sql/v023_youth_scouting_search_profile.sql")),
        // V24: Persist structured transfer rumours for the world transfer centre
        M::up(include_str!("sql/v024_transfer_rumours.sql")),
        // V25: Persist retired player state for seasonal aging and hall-of-fame work
        M::up(include_str!("sql/v025_player_retired.sql")),
        // V26: Persist world-history archives for rivalries and historical season awards
        M::up(include_str!("sql/v026_world_history_archive.sql")),
        // V27: Persist available staff market activity for monthly rotation
        M::up(include_str!("sql/v027_available_staff_market_activity.sql")),
        // V28: Persist optional local media paths for teams and players
        M::up(include_str!("sql/v028_entity_media.sql")),
        // V29: Save metadata for world package versions and active simulation scope
        M::up(include_str!("sql/v029_competition_save_metadata.sql")),
        // V30: Persist multi-competition and national-team state
        M::up(include_str!("sql/v030_competitions_and_national_teams.sql")),
        // V31: Persist group stages for group-and-knockout competitions
        M::up(include_str!("sql/v031_competition_groups.sql")),
        // V32: Persist qualification berths on competitions
        M::up(include_str!("sql/v032_competition_berths.sql")),
        // V33: Season start month and day per competition for hemisphere-aware scheduling
        M::up(include_str!("sql/v033_competition_season_start.sql")),
        // V34: Optional jersey/squad number per player (1-99, NULL = unassigned)
        M::up(include_str!("sql/v034_player_jersey_number.sql")),
        // V35: Kit pattern for team jersey visual (Solid, Stripes, Hoops, HalfAndHalf, Diagonal)
        M::up(include_str!("sql/v035_team_kit_pattern.sql")),
        // V36: Enforce per-team jersey number uniqueness at DB level
        M::up(include_str!("sql/v036_player_jersey_number_unique.sql")),
        // V37: Per-player tactical roles and phase blueprint settings
        M::up(include_str!("sql/v037_team_tactics.sql")),
        // V38: i18n name key for national teams
        M::up(include_str!("sql/v038_national_team_name_key.sql")),
        // V39: Persist extra translations bundle from world packages
        M::up(include_str!("sql/v039_game_extra_translations.sql")),
        // V40: Persist loan offer history and active loan contracts
        M::up(include_str!("sql/v040_player_loan_state.sql")),
        // V41: Persist per-player transfer and loan movement history
        M::up(include_str!("sql/v041_player_movement_history.sql")),
        // V42: Persist installed-package lockfile (id, version, hash) for save reproducibility
        M::up(include_str!("sql/v042_game_package_lockfile.sql")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Every `vNNN_*.sql` file in `src/sql/`, as (version number, file name).
    fn sql_files_on_disk() -> Vec<(usize, String)> {
        let sql_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sql");

        let mut files: Vec<(usize, String)> = std::fs::read_dir(&sql_dir)
            .expect("src/sql should be readable")
            .map(|entry| entry.expect("readable directory entry").file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".sql"))
            .map(|name| {
                let version = name
                    .strip_prefix('v')
                    .and_then(|rest| rest.split('_').next())
                    // Exactly three digits: `v1_x.sql` and `v01_x.sql` both parse as 1, so
                    // without this the directory could hold two files claiming one version and
                    // the contiguity check below would still be satisfied.
                    .filter(|digits| digits.len() == 3 && digits.bytes().all(|b| b.is_ascii_digit()))
                    .and_then(|digits| digits.parse::<usize>().ok())
                    .unwrap_or_else(|| {
                        panic!("migration file `{name}` does not follow the vNNN_description.sql naming")
                    });
                (version, name)
            })
            .collect();

        files.sort();
        files
    }

    /// `MIGRATION_COUNT` and the length of the `all_migrations()` vec are already tied together by
    /// `test_schema_version_after_migration`, which checks the `user_version` the migrations
    /// actually write. What nothing checked was the third side of the triangle: the `src/sql`
    /// directory. Adding `v043_something.sql` and forgetting to register it left the constant, the
    /// vec and the directory disagreeing with no test to say so — the file simply never ran.
    #[test]
    fn test_migration_count_matches_sql_directory() {
        let files = sql_files_on_disk();

        assert_eq!(
            files.len(),
            MIGRATION_COUNT,
            "src/sql holds {} migration files but MIGRATION_COUNT is {}. Every .sql file must be \
             registered in all_migrations() and counted here.",
            files.len(),
            MIGRATION_COUNT,
        );
    }

    /// The file names registered in `all_migrations()`, in the order they are registered.
    ///
    /// Read out of this module's own source because `Migrations` does not expose its scripts.
    /// Only `M::up(include_str!("sql/…"))` is matched, so a name mentioned in a comment — or in
    /// this very test — is not mistaken for a registration.
    fn registered_file_names() -> Vec<String> {
        // `include_str!` resolves relative to this file, so the bare name is required:
        // `include_str!(file!())` expands to a crate-root-relative path and does not compile.
        let source = include_str!("migrations.rs");
        const OPENING: &str = "M::up(include_str!(\"sql/";

        source
            .lines()
            .map(str::trim)
            .filter_map(|line| line.strip_prefix(OPENING))
            .filter_map(|rest| rest.split('"').next())
            .map(String::from)
            .collect()
    }

    /// Guards the parser below: if the registration style changes, this stops matching and every
    /// name silently disappears. Pinning the count against `MIGRATION_COUNT` turns that into a
    /// failure rather than a vacuous pass over an empty list.
    #[test]
    fn test_registrations_are_all_recognised() {
        assert_eq!(
            registered_file_names().len(),
            MIGRATION_COUNT,
            "recognised {} `M::up(include_str!(\"sql/…\"))` registrations but MIGRATION_COUNT is \
             {MIGRATION_COUNT}. A migration was registered in a style this test cannot read — \
             teach it the new style rather than deleting the assertion, or the ordering check \
             below starts passing for migrations it never examined.",
            registered_file_names().len(),
        );
    }

    /// The registration order **is** the schema version: `rusqlite_migration` applies the vec in
    /// order and stores the applied count as `user_version`, so entry N is what "version N" means
    /// for every save file ever written. Swapping two entries, or registering a file that is not
    /// on disk, therefore changes what an existing database's version number refers to.
    ///
    /// Comparing the ordered lists catches all of it: a missing registration, a duplicate, a file
    /// with no registration, and a reordering — which a per-name occurrence count cannot see,
    /// because swapping two entries leaves every name present exactly once.
    #[test]
    fn test_registrations_match_the_sql_directory_in_order() {
        let registered = registered_file_names();
        let on_disk: Vec<String> = sql_files_on_disk()
            .into_iter()
            .map(|(_, name)| name)
            .collect();

        assert_eq!(
            registered, on_disk,
            "all_migrations() does not register src/sql in version order. A file with no \
             registration never runs; a registration out of order changes which script a given \
             user_version means, and no existing save can be corrected afterwards.",
        );
    }

    /// Versions must be contiguous from 1. A gap means a migration was deleted after shipping —
    /// which silently changes what `user_version` means for every existing save — and a duplicate
    /// means two files claim the same slot.
    #[test]
    fn test_sql_versions_are_contiguous_from_one() {
        let files = sql_files_on_disk();

        for (index, (version, name)) in files.iter().enumerate() {
            let expected = index + 1;
            assert_eq!(
                *version, expected,
                "expected migration v{expected:03} at this position but found `{name}`. \
                 Migration versions must run 1..={MIGRATION_COUNT} with no gaps or duplicates: \
                 rusqlite_migration stores the applied count as the schema version, so renumbering \
                 or removing one reinterprets every existing save file.",
            );
        }
    }

    #[test]
    fn test_migrations_are_valid() {
        let migrations = all_migrations();
        migrations.validate().expect("migrations should be valid");
    }

    #[test]
    fn test_apply_migrations_to_empty_db() {
        let mut conn = Connection::open_in_memory().unwrap();
        let migrations = all_migrations();
        migrations
            .to_latest(&mut conn)
            .expect("migrations should apply cleanly");

        // Verify all expected tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(
            tables.contains(&"game_meta".to_string()),
            "missing game_meta"
        );
        assert!(tables.contains(&"managers".to_string()), "missing managers");
        assert!(tables.contains(&"teams".to_string()), "missing teams");
        assert!(tables.contains(&"players".to_string()), "missing players");
        assert!(
            tables.contains(&"player_match_stats".to_string()),
            "missing player_match_stats"
        );
        assert!(tables.contains(&"staff".to_string()), "missing staff");
        assert!(
            tables.contains(&"team_match_stats".to_string()),
            "missing team_match_stats"
        );
        assert!(tables.contains(&"league".to_string()), "missing league");
        assert!(tables.contains(&"fixtures".to_string()), "missing fixtures");
        assert!(
            tables.contains(&"standings".to_string()),
            "missing standings"
        );
        assert!(tables.contains(&"messages".to_string()), "missing messages");
        assert!(
            tables.contains(&"transfer_log".to_string()),
            "missing transfer_log"
        );
        assert!(
            tables.contains(&"transfer_rumours".to_string()),
            "missing transfer_rumours"
        );
        assert!(tables.contains(&"news".to_string()), "missing news");
        assert!(
            tables.contains(&"board_objectives".to_string()),
            "missing board_objectives"
        );
        assert!(
            tables.contains(&"scouting_assignments".to_string()),
            "missing scouting_assignments"
        );
        assert!(
            tables.contains(&"youth_scouting_assignments".to_string()),
            "missing youth_scouting_assignments"
        );

        let game_meta_columns: Vec<String> = conn
            .prepare("PRAGMA table_info(game_meta)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .filter_map(|row| row.ok())
            .collect();
        assert!(
            game_meta_columns.contains(&"world_history_json".to_string()),
            "missing game_meta.world_history_json"
        );
        assert!(
            game_meta_columns.contains(&"available_staff_market_last_activity_date".to_string()),
            "missing game_meta.available_staff_market_last_activity_date"
        );
        assert!(
            game_meta_columns.contains(&"extra_translations_json".to_string()),
            "missing game_meta.extra_translations_json"
        );

        let national_team_columns: Vec<String> = conn
            .prepare("PRAGMA table_info(national_teams)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .filter_map(|row| row.ok())
            .collect();
        assert!(
            national_team_columns.contains(&"name_key".to_string()),
            "missing national_teams.name_key"
        );

        let team_columns: Vec<String> = conn
            .prepare("PRAGMA table_info(teams)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .filter_map(|row| row.ok())
            .collect();
        assert!(
            team_columns.contains(&"media_json".to_string()),
            "missing teams.media_json"
        );

        let player_columns: Vec<String> = conn
            .prepare("PRAGMA table_info(players)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .filter_map(|row| row.ok())
            .collect();
        assert!(
            player_columns.contains(&"media_json".to_string()),
            "missing players.media_json"
        );
        assert!(
            player_columns.contains(&"loan_offers".to_string()),
            "missing players.loan_offers"
        );
        assert!(
            player_columns.contains(&"active_loan".to_string()),
            "missing players.active_loan"
        );
        assert!(
            player_columns.contains(&"movement_history".to_string()),
            "missing players.movement_history"
        );
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        let migrations = all_migrations();
        migrations
            .to_latest(&mut conn)
            .expect("first apply should succeed");
        // Applying again should be a no-op (already at latest)
        migrations
            .to_latest(&mut conn)
            .expect("second apply should succeed (idempotent)");
    }

    #[test]
    fn test_schema_version_after_migration() {
        let mut conn = Connection::open_in_memory().unwrap();
        let migrations = all_migrations();
        migrations.to_latest(&mut conn).unwrap();

        let version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        // rusqlite_migration sets user_version to the number of applied migrations
        assert_eq!(
            version, MIGRATION_COUNT as i64,
            "expected schema version {} after migrations",
            MIGRATION_COUNT
        );
    }
}
