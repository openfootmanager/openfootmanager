use rusqlite_migration::{M, Migrations};

/// Number of migrations defined. Keep in sync with the vec in `all_migrations`.
pub const MIGRATION_COUNT: usize = 23;

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
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

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
