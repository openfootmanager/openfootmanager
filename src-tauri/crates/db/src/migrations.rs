use rusqlite_migration::{M, Migrations};

/// Number of migrations defined.
pub const MIGRATION_COUNT: usize = MIGRATIONS.len();

/// The number of migrations, pinned.
///
/// Adding a migration breaks the build here until this is raised, which is the point: the pin is
/// what makes *lowering* it a deliberate act. Every other check in this module compares the
/// registration list, the count and the `src/sql` directory against each other, so deleting a
/// migration from all three at once leaves them agreeing — only a number that does not move
/// notices, and a `>=` floor nobody is forced to maintain would quietly stop covering anything
/// added after it.
///
/// **Lowering this is almost always wrong.** A save written by a release with N migrations
/// reports `user_version = N` and expects every column those migrations added; a build with
/// fewer can neither open it nor recreate it.
const EXPECTED_MIGRATION_COUNT: usize = 51;

// Compile-time rather than a test: adding or removing a migration should fail the build, not
// merely turn a suite red.
const _: () = assert!(
    MIGRATION_COUNT == EXPECTED_MIGRATION_COUNT,
    "the migration count changed. If you added a migration, raise EXPECTED_MIGRATION_COUNT to \
     match. If you removed one, put it back: saves written by the release that shipped it report \
     a higher user_version than this build can produce, and expect columns it never creates."
);

/// Every migration, in the order `rusqlite_migration` applies them.
///
/// The order *is* the schema version: the applied count is stored as `user_version`, so entry
/// N is what "version N" means for every save file already written. Append only — reordering
/// or removing an entry after release points existing databases at a different script, and no
/// already-written save can be corrected afterwards.
///
/// The file name is carried beside the SQL rather than left implicit in the `include_str!`
/// path so the tests can check the registration list against `src/sql` as data. They used to
/// recover it by parsing this file as text, which could not tell a real registration from one
/// inside a block comment.
// One line per migration on purpose: this is a table, and rustfmt's four-line-per-entry form
// makes it four times longer and much harder to scan for a version number.
#[rustfmt::skip]
const MIGRATIONS: &[(&str, &str)] = &[
    // V1: Initial schema — all game entity tables
    ("v001_initial_schema.sql", include_str!("sql/v001_initial_schema.sql")),
    // V2: Training groups per team
    ("v002_training_groups.sql", include_str!("sql/v002_training_groups.sql")),
    // V3: Alternate positions per player
    ("v003_alternate_positions.sql", include_str!("sql/v003_alternate_positions.sql")),
    // V4: Natural/preferred position per player
    ("v004_natural_position.sql", include_str!("sql/v004_natural_position.sql")),
    // V5: Per-player training focus override
    ("v005_player_training_focus.sql", include_str!("sql/v005_player_training_focus.sql")),
    // V6: Team match roles defaults
    ("v006_team_match_roles.sql", include_str!("sql/v006_team_match_roles.sql")),
    // V7: Team financial ledger
    ("v007_team_financial_ledger.sql", include_str!("sql/v007_team_financial_ledger.sql")),
    // V8: Team sponsorship state
    ("v008_team_sponsorship.sql", include_str!("sql/v008_team_sponsorship.sql")),
    // V9: Team facilities state
    ("v009_team_facilities.sql", include_str!("sql/v009_team_facilities.sql")),
    // V10: Hidden per-player morale architecture state
    ("v010_player_morale_core.sql", include_str!("sql/v010_player_morale_core.sql")),
    // V11: Player footedness identity fields
    ("v011_player_footedness.sql", include_str!("sql/v011_player_footedness.sql")),
    // V12: Fixture competition metadata
    ("v012_fixture_competition.sql", include_str!("sql/v012_fixture_competition.sql")),
    // V13: Player long-term fitness value
    ("v013_player_fitness.sql", include_str!("sql/v013_player_fitness.sql")),
    // V14: Explicit football identity fields for teams and people
    ("v014_football_identity.sql", include_str!("sql/v014_football_identity.sql")),
    // V15: Historical player and team match stats
    ("v015_match_stats_history.sql", include_str!("sql/v015_match_stats_history.sql")),
    // V16: Manager board-warning stage tracking (per-club, resets on hire)
    ("v016_manager_warning_stage.sql", include_str!("sql/v016_manager_warning_stage.sql")),
    // V17: Persist vacancy-age tracking for delayed AI manager replacements
    ("v017_vacant_team_days.sql", include_str!("sql/v017_vacant_team_days.sql")),
    // V18: Completed transfer log for world transfer-centre views
    ("v018_transfer_log.sql", include_str!("sql/v018_transfer_log.sql")),
    // V19: Explicit senior versus youth squad assignment for players
    ("v019_player_squad_role.sql", include_str!("sql/v019_player_squad_role.sql")),
    // V20: Persist computed OVR and potential so they survive save/load
    ("v020_player_ovr_potential.sql", include_str!("sql/v020_player_ovr_potential.sql")),
    // V21: Persist youth recruitment scouting assignments separately from player scouting
    ("v021_youth_scouting_assignments.sql", include_str!("sql/v021_youth_scouting_assignments.sql")),
    // V22: Persist target position for youth recruitment scouting assignments
    ("v022_youth_scouting_target_position.sql", include_str!("sql/v022_youth_scouting_target_position.sql")),
    // V23: Persist region and objective for youth recruitment scouting assignments
    ("v023_youth_scouting_search_profile.sql", include_str!("sql/v023_youth_scouting_search_profile.sql")),
    // V24: Persist structured transfer rumours for the world transfer centre
    ("v024_transfer_rumours.sql", include_str!("sql/v024_transfer_rumours.sql")),
    // V25: Persist retired player state for seasonal aging and hall-of-fame work
    ("v025_player_retired.sql", include_str!("sql/v025_player_retired.sql")),
    // V26: Persist world-history archives for rivalries and historical season awards
    ("v026_world_history_archive.sql", include_str!("sql/v026_world_history_archive.sql")),
    // V27: Persist available staff market activity for monthly rotation
    ("v027_available_staff_market_activity.sql", include_str!("sql/v027_available_staff_market_activity.sql")),
    // V28: Persist optional local media paths for teams and players
    ("v028_entity_media.sql", include_str!("sql/v028_entity_media.sql")),
    // V29: Save metadata for world package versions and active simulation scope
    ("v029_competition_save_metadata.sql", include_str!("sql/v029_competition_save_metadata.sql")),
    // V30: Persist multi-competition and national-team state
    ("v030_competitions_and_national_teams.sql", include_str!("sql/v030_competitions_and_national_teams.sql")),
    // V31: Persist group stages for group-and-knockout competitions
    ("v031_competition_groups.sql", include_str!("sql/v031_competition_groups.sql")),
    // V32: Persist qualification berths on competitions
    ("v032_competition_berths.sql", include_str!("sql/v032_competition_berths.sql")),
    // V33: Season start month and day per competition for hemisphere-aware scheduling
    ("v033_competition_season_start.sql", include_str!("sql/v033_competition_season_start.sql")),
    // V34: Optional jersey/squad number per player (1-99, NULL = unassigned)
    ("v034_player_jersey_number.sql", include_str!("sql/v034_player_jersey_number.sql")),
    // V35: Kit pattern for team jersey visual (Solid, Stripes, Hoops, HalfAndHalf, Diagonal)
    ("v035_team_kit_pattern.sql", include_str!("sql/v035_team_kit_pattern.sql")),
    // V36: Enforce per-team jersey number uniqueness at DB level
    ("v036_player_jersey_number_unique.sql", include_str!("sql/v036_player_jersey_number_unique.sql")),
    // V37: Per-player tactical roles and phase blueprint settings
    ("v037_team_tactics.sql", include_str!("sql/v037_team_tactics.sql")),
    // V38: i18n name key for national teams
    ("v038_national_team_name_key.sql", include_str!("sql/v038_national_team_name_key.sql")),
    // V39: Persist extra translations bundle from world packages
    ("v039_game_extra_translations.sql", include_str!("sql/v039_game_extra_translations.sql")),
    // V40: Persist loan offer history and active loan contracts
    ("v040_player_loan_state.sql", include_str!("sql/v040_player_loan_state.sql")),
    // V41: Persist per-player transfer and loan movement history
    ("v041_player_movement_history.sql", include_str!("sql/v041_player_movement_history.sql")),
    // V42: Persist installed-package lockfile (id, version, hash) for save reproducibility
    ("v042_game_package_lockfile.sql", include_str!("sql/v042_game_package_lockfile.sql")),
    // V43: Persist the inbox sent-ledger so deleting a message can't re-send the event
    ("v043_emitted_events.sql", include_str!("sql/v043_emitted_events.sql")),
    // V44: Append-only cash journal
    ("v044_cash_journal.sql", include_str!("sql/v044_cash_journal.sql")),
    // V45: A contract is an interval — players gain a start date beside the end
    (
        "v045_player_contract_start.sql",
        include_str!("sql/v045_player_contract_start.sql"),
    ),
    // V46: The seed the game's own dice are derived from
    ("v046_game_seed.sql", include_str!("sql/v046_game_seed.sql")),
    // V47: Whether World Cups are drawn the pre-seed way
    ("v047_legacy_world_cup_draw.sql", include_str!("sql/v047_legacy_world_cup_draw.sql")),
    // V48: Competition calendar identity and edition provenance
    ("v048_competition_calendar.sql", include_str!("sql/v048_competition_calendar.sql")),
    // V49: Frozen outcome of each finished competition edition
    ("v049_competition_edition_archive.sql", include_str!("sql/v049_competition_edition_archive.sql")),
    // V50: Scouted players: what a finished scouting job revealed, and when
    (
        "v050_scouted_players.sql",
        include_str!("sql/v050_scouted_players.sql"),
    ),
    // V51: Persisted competition name key
    ("v051_competition_name_key.sql", include_str!("sql/v051_competition_name_key.sql")),
];

/// All migrations for a per-save game database.
/// Each save `.db` file gets this schema applied via `rusqlite_migration`.
/// Table name -> column names, as the first `applied` migrations create them.
pub(crate) type Layout = std::collections::BTreeMap<String, std::collections::BTreeSet<String>>;

/// The layout a save at schema version `applied` must have, built by running the
/// first `applied` migrations on an empty in-memory database.
pub(crate) fn expected_layout(applied: usize) -> Result<Layout, String> {
    if applied == 0 {
        // `Migrations` refuses an empty list, and a fresh file has no tables.
        return Ok(Layout::new());
    }
    let mut reference = rusqlite::Connection::open_in_memory().map_err(|e| e.to_string())?;
    Migrations::new(
        MIGRATIONS
            .iter()
            .take(applied)
            .map(|(_, sql)| M::up(sql))
            .collect(),
    )
    .to_latest(&mut reference)
    .map_err(|e| e.to_string())?;
    layout_of(&reference)
}

/// Every user table of `conn` with its columns (SQLite's own tables excluded).
pub(crate) fn layout_of(conn: &rusqlite::Connection) -> Result<Layout, String> {
    let mut layout = Layout::new();
    let mut tables = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .map_err(|e| e.to_string())?;
    let names: Vec<String> = tables
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    for name in names {
        // Bound, not interpolated: a foreign table may be named anything, quotes included.
        let mut info = conn
            .prepare("SELECT name FROM pragma_table_info(?1)")
            .map_err(|e| e.to_string())?;
        let columns = info
            .query_map([&name], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        layout.insert(name, columns);
    }
    Ok(layout)
}

/// The first table or column on which `found` disagrees with `expected`, if any. Tables the
/// migrations never created are ignored.
pub(crate) fn first_layout_difference(expected: &Layout, found: &Layout) -> Option<String> {
    for (table, columns) in expected {
        let Some(found_columns) = found.get(table) else {
            return Some(format!("missing table {table}"));
        };
        if let Some(column) = columns.difference(found_columns).next() {
            return Some(format!("missing column {table}.{column}"));
        }
        if let Some(column) = found_columns.difference(columns).next() {
            return Some(format!("unexpected column {table}.{column}"));
        }
    }
    // A table no migration made is somebody's own (a debugging probe, a tool's cache) and
    // harms nothing; only a table or column the version promises, or a stray column on one of
    // ours, makes the next migration fail halfway.
    None
}

pub fn all_migrations() -> Migrations<'static> {
    Migrations::new(MIGRATIONS.iter().map(|(_, sql)| M::up(sql)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// The version a `vNNN_description.sql` file name claims, or `None` if it is not that shape.
    ///
    /// The three-digit rule matters: `v1_x.sql` and `v01_x.sql` both parse as 1, so a looser
    /// reading would let the directory hold two files claiming one version while the contiguity
    /// check still agreed. The description must be non-empty because `v001.sql` and `v001_.sql`
    /// carry no hint of what they do, and the naming is the only documentation a migration has.
    fn migration_version(name: &str) -> Option<usize> {
        let (digits, description) = name
            .strip_suffix(".sql")?
            .strip_prefix('v')?
            .split_once('_')?;

        if digits.len() != 3
            || !digits.bytes().all(|b| b.is_ascii_digit())
            || description.is_empty()
        {
            return None;
        }

        digits.parse::<usize>().ok()
    }

    #[test]
    fn test_migration_version_requires_the_documented_shape() {
        assert_eq!(migration_version("v001_initial_schema.sql"), Some(1));
        assert_eq!(
            migration_version("v042_game_package_lockfile.sql"),
            Some(42)
        );

        for rejected in [
            "v1_short.sql",      // one digit: would collide with v001
            "v01_short.sql",     // two digits: likewise
            "v0001_long.sql",    // four digits
            "v001.sql",          // no description
            "v001_.sql",         // empty description
            "vabc_letters.sql",  // not a number
            "001_no_prefix.sql", // no `v`
            "v001_no_extension", // not a .sql file
        ] {
            assert_eq!(
                migration_version(rejected),
                None,
                "should reject `{rejected}`"
            );
        }
    }

    /// Every `vNNN_*.sql` file in `src/sql/`, as (version number, file name).
    fn sql_files_on_disk() -> Vec<(usize, String)> {
        let sql_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sql");

        let mut files: Vec<(usize, String)> = std::fs::read_dir(&sql_dir)
            .expect("src/sql should be readable")
            .map(|entry| entry.expect("readable directory entry").file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".sql"))
            .map(|name| {
                let version = migration_version(&name).unwrap_or_else(|| {
                    panic!(
                        "migration file `{name}` does not follow the vNNN_description.sql naming"
                    )
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

    /// The file names registered in `MIGRATIONS`, in the order they are registered.
    fn registered_file_names() -> Vec<String> {
        MIGRATIONS
            .iter()
            .map(|(name, _)| String::from(*name))
            .collect()
    }

    /// The name in each entry is what ties a registration to a file on disk, so it has to be the
    /// file that entry actually embeds — otherwise the ordering test below compares labels while
    /// the SQL underneath them says something else.
    #[test]
    fn test_each_registration_embeds_the_file_it_names() {
        let sql_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sql");

        for (name, embedded) in MIGRATIONS {
            let on_disk = std::fs::read_to_string(sql_dir.join(name))
                .unwrap_or_else(|e| panic!("migration `{name}` is registered but unreadable: {e}"));

            assert_eq!(
                *embedded, on_disk,
                "the entry named `{name}` does not embed `src/sql/{name}`. The name is what every \
                 other check in this module matches against, so a mismatched pair makes them all \
                 agree about the wrong thing.",
            );
        }
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

        let cash_journal_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'cash_journal'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cash_journal_exists, 1, "missing cash_journal table");

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
    fn calendar_v048_upgrades_a_seeded_v047_save_without_reinterpreting_it() {
        for draw_policy in [0_i64, 1] {
            let mut conn = Connection::open_in_memory().unwrap();
            let old = Migrations::new(
                MIGRATIONS
                    .iter()
                    .take(47)
                    .map(|(_, sql)| M::up(sql))
                    .collect(),
            );
            old.to_latest(&mut conn).unwrap();
            conn.execute(
                "INSERT INTO game_meta (save_id, save_name, manager_id, start_date, game_date, save_format_version, legacy_world_cup_draw) VALUES ('save', 'Seeded save', 'manager', '2033-02-01', '2033-06-30', 8, ?1)",
                [draw_policy],
            ).unwrap();
            conn.execute("INSERT INTO competitions (id, name, season) VALUES ('ar-d1-apertura', 'Opening', 2033)", []).unwrap();
            let version: i64 = conn
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .unwrap();
            assert_eq!(version, 47);
            assert!(
                conn.prepare("SELECT calendar_json FROM competitions")
                    .is_err()
            );

            for _ in 0..2 {
                all_migrations().to_latest(&mut conn).unwrap();
                let row: (String, u32, Option<String>) = conn
                    .query_row(
                        "SELECT id, season, calendar_json FROM competitions",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .expect("v048 must add the calendar column to a real v047 save");
                assert_eq!(row, ("ar-d1-apertura".into(), 2033, None));
                let meta: (u32, i64) = conn
                    .query_row(
                        "SELECT save_format_version, legacy_world_cup_draw FROM game_meta",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .unwrap();
                assert_eq!(meta, (8, draw_policy));
                let version: i64 = conn
                    .pragma_query_value(None, "user_version", |row| row.get(0))
                    .unwrap();
                assert_eq!(version, MIGRATION_COUNT as i64);
            }
        }
    }

    /// Given a real v048 save, when migrated twice, then the archive table exists and is empty.
    #[test]
    fn edition_archive_v049_upgrades_a_v048_save_without_touching_competitions() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(
            MIGRATIONS
                .iter()
                .take(48)
                .map(|(_, sql)| M::up(sql))
                .collect(),
        )
        .to_latest(&mut conn)
        .unwrap();
        conn.execute("INSERT INTO competitions (id, name, season) VALUES ('ar-d1-apertura', 'Opening', 2033)", []).unwrap();
        assert!(
            conn.prepare("SELECT 1 FROM competition_edition_archive")
                .is_err()
        );

        for _ in 0..2 {
            all_migrations().to_latest(&mut conn).unwrap();
            let archived: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM competition_edition_archive",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            let competitions: i64 = conn
                .query_row("SELECT COUNT(*) FROM competitions", [], |row| row.get(0))
                .unwrap();
            assert_eq!((archived, competitions), (0, 1));
        }
    }

    /// A real v050 row for the World Cup competitions the game itself generates, so the migration is
    /// checked against the generator and not against a hand-copied shape.
    fn generated_world_cup_rows() -> Vec<(String, String, String, String, i64)> {
        use chrono::TimeZone;
        let clock = ofm_core::clock::GameClock::new(
            chrono::Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap(),
        );
        let manager = domain::manager::Manager::new(
            "mgr".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        let mut game = ofm_core::game::Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        let windows = ofm_core::national_team::international_window_dates(
            chrono::Utc.with_ymd_and_hms(2025, 8, 1, 0, 0, 0).unwrap(),
        );
        ofm_core::world_cup::schedule_world_cup_qualifying(&mut game, 2026, &windows);
        ofm_core::world_cup::schedule_world_cup(
            &mut game,
            chrono::Utc.with_ymd_and_hms(2026, 6, 10, 0, 0, 0).unwrap(),
            &ofm_core::world_cup::FORMAT_48,
        );
        game.competitions
            .iter()
            .filter(|c| ofm_core::world_cup::is_world_cup_competition(c))
            .map(|c| {
                (
                    c.id.clone(),
                    c.name.clone(),
                    "InternationalNation".to_string(),
                    "International".to_string(),
                    i64::from(c.priority),
                )
            })
            .collect()
    }

    /// Given a real v050 save holding the World Cup competitions the game generates, a keyless
    /// authored competition of the very same kind and scope, other authored rows and a club league,
    /// when migrated twice, then only the generated World Cup rows get their key.
    #[test]
    fn name_key_v051_derives_world_cup_keys_once_and_leaves_other_rows_null() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(
            MIGRATIONS
                .iter()
                .take(50)
                .map(|(_, sql)| M::up(sql))
                .collect(),
        )
        .to_latest(&mut conn)
        .unwrap();
        let generated = generated_world_cup_rows();
        assert!(
            generated
                .iter()
                .any(|row| row.0.starts_with("world-cup-qualifying-"))
                && generated.iter().any(|row| row.1.starts_with("World Cup ")),
            "the generator must have produced qualifying and finals"
        );
        let mut rows = generated.clone();
        rows.extend(
            [
                // The playoff is only staged after qualifying is played; its id prefix is the frozen contract.
                (
                    "world-cup-playoff-2026",
                    "Playoff",
                    "InternationalNation",
                    "International",
                    10_000,
                ),
                // Authored by a package: same kind and scope as the finals, but not generated.
                (
                    "pkg-nations-cup",
                    "Nations Cup",
                    "InternationalNation",
                    "International",
                    0,
                ),
                (
                    "pkg-world-cup-lookalike",
                    "World Cup 2030",
                    "InternationalNation",
                    "International",
                    0,
                ),
                (
                    "pkg-domestic-nations",
                    "Nations League",
                    "InternationalNation",
                    "Domestic",
                    0,
                ),
                (
                    "pkg-high-priority",
                    "Nations Cup",
                    "InternationalNation",
                    "International",
                    10_000,
                ),
                (
                    "pkg-domestic-lookalike",
                    "World Cup 2031",
                    "InternationalNation",
                    "Domestic",
                    10_000,
                ),
                (
                    "pkg-league-lookalike",
                    "World Cup 2032",
                    "League",
                    "International",
                    10_000,
                ),
                ("ar-d1-apertura", "Opening", "League", "Domestic", 0),
            ]
            .map(|(id, name, kind, scope, priority)| {
                (
                    id.to_string(),
                    name.to_string(),
                    kind.to_string(),
                    scope.to_string(),
                    priority,
                )
            }),
        );
        for (id, name, kind, scope, priority) in &rows {
            conn.execute(
                "INSERT INTO competitions (id, name, kind, scope, season, priority) VALUES (?1, ?2, ?3, ?4, 2033, ?5)",
                rusqlite::params![id, name, kind, scope, priority],
            )
            .unwrap();
        }
        assert!(conn.prepare("SELECT name_key FROM competitions").is_err());

        for _ in 0..2 {
            all_migrations().to_latest(&mut conn).unwrap();
            let mut stmt = conn
                .prepare("SELECT id, name_key FROM competitions ORDER BY id")
                .unwrap();
            let stored: std::collections::BTreeMap<String, Option<String>> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            let key = |k: &str| Some(format!("tournaments.competitions.{k}"));
            for (id, name, ..) in &generated {
                let expected = if id.starts_with("world-cup-qualifying-") {
                    key("worldCupQualifying")
                } else {
                    assert!(name.starts_with("World Cup "), "{id} is not a finals row");
                    key("worldCup")
                };
                assert_eq!(stored[id], expected, "generated row {id}");
            }
            assert_eq!(stored["world-cup-playoff-2026"], key("worldCupPlayoff"));
            for authored in [
                "pkg-nations-cup",
                "pkg-world-cup-lookalike",
                "pkg-domestic-nations",
                "pkg-high-priority",
                "pkg-domestic-lookalike",
                "pkg-league-lookalike",
                "ar-d1-apertura",
            ] {
                assert_eq!(
                    stored[authored], None,
                    "authored row {authored} must stay NULL"
                );
            }
        }
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

    /// FNV-1a 64 over the file with carriage returns removed, so a Windows checkout of the
    /// same migration hashes the same.
    fn fingerprint(sql: &str) -> u64 {
        sql.bytes()
            .filter(|byte| *byte != b'\r')
            .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
            })
    }

    /// Shipped migrations, frozen. A save at version N was built by exactly these statements,
    /// so editing one in place leaves existing saves at N with a layout the code no longer expects.
    /// Add a new migration instead; append its row here.
    #[rustfmt::skip]
    const SHIPPED: &[(&str, u64)] = &[
    ("v001_initial_schema.sql", 0xed06e3806a80b990),
    ("v002_training_groups.sql", 0x587e38819ef431d9),
    ("v003_alternate_positions.sql", 0xae3ea2435e3d379e),
    ("v004_natural_position.sql", 0xdf0a07b5a2d9c817),
    ("v005_player_training_focus.sql", 0x5d98ec87b69e464c),
    ("v006_team_match_roles.sql", 0x1d15390749d7d4a6),
    ("v007_team_financial_ledger.sql", 0x31ec6a65bba43a34),
    ("v008_team_sponsorship.sql", 0x2dec841687f8768c),
    ("v009_team_facilities.sql", 0x73e7cfdbe588a760),
    ("v010_player_morale_core.sql", 0x0241fb7805c6c2f5),
    ("v011_player_footedness.sql", 0x8f6f5dd143459cff),
    ("v012_fixture_competition.sql", 0xc54ab8e93659ddd1),
    ("v013_player_fitness.sql", 0x0cc110dadfa27314),
    ("v014_football_identity.sql", 0x478f5c28a51338ac),
    ("v015_match_stats_history.sql", 0x57db76c8adc2f71b),
    ("v016_manager_warning_stage.sql", 0x17e9fd6a796d799b),
    ("v017_vacant_team_days.sql", 0xc54c0d585e586259),
    ("v018_transfer_log.sql", 0x4e907c47cf4227bb),
    ("v019_player_squad_role.sql", 0xa79ca67590bea7c2),
    ("v020_player_ovr_potential.sql", 0x111e1fe83f07b2e9),
    ("v021_youth_scouting_assignments.sql", 0xf8ca0afb47d51b60),
    ("v022_youth_scouting_target_position.sql", 0x4f4ade988d8c15fa),
    ("v023_youth_scouting_search_profile.sql", 0xed647b5f2f4af448),
    ("v024_transfer_rumours.sql", 0x8ff846041844443b),
    ("v025_player_retired.sql", 0x8ab28eca23a856bf),
    ("v026_world_history_archive.sql", 0x253356d90eb53aab),
    ("v027_available_staff_market_activity.sql", 0x967da647d9e138f5),
    ("v028_entity_media.sql", 0x0fea8f4c1c27fd45),
    ("v029_competition_save_metadata.sql", 0x661e6f978b415c5c),
    ("v030_competitions_and_national_teams.sql", 0xb8865abfb0e6efe0),
    ("v031_competition_groups.sql", 0xb6d4975982237820),
    ("v032_competition_berths.sql", 0xa7e0bf974ee1bc6a),
    ("v033_competition_season_start.sql", 0xd6a4033e79226f72),
    ("v034_player_jersey_number.sql", 0xd7e28bd1d7122806),
    ("v035_team_kit_pattern.sql", 0x7a375c40d876005f),
    ("v036_player_jersey_number_unique.sql", 0x3b68eca99a6b9200),
    ("v037_team_tactics.sql", 0x5ba0aa051c92396f),
    ("v038_national_team_name_key.sql", 0xd6b369d67c57720a),
    ("v039_game_extra_translations.sql", 0xf9ce5e9d4fd53d9b),
    ("v040_player_loan_state.sql", 0xf430c87823e79d49),
    ("v041_player_movement_history.sql", 0x35b7a832c4c19d47),
    ("v042_game_package_lockfile.sql", 0x7427606582204a20),
    ("v043_emitted_events.sql", 0x7973ce887e85964c),
    ("v044_cash_journal.sql", 0x0f2cda907bb330f8),
    ("v045_player_contract_start.sql", 0xe82234613477d74d),
    ("v046_game_seed.sql", 0xd9cddb1327e8fb92),
    ("v047_legacy_world_cup_draw.sql", 0x45671a737bc8f6f9),
    ("v048_competition_calendar.sql", 0x24d66bf320f0a2dc),
    ("v049_competition_edition_archive.sql", 0xce105022e5bed8c0),
    ("v050_scouted_players.sql", 0x3536cbfaf1572f2b),
    ("v051_competition_name_key.sql", 0x834ad58e7ffc2f0a),
    ];

    /// Given the migrations already shipped
    /// When their SQL is compared with the pinned fingerprints
    /// Then none was edited in place, renamed or reordered; only new ones follow the pinned list.
    #[test]
    fn shipped_migrations_are_never_edited_in_place() {
        for (position, (name, pinned)) in SHIPPED.iter().enumerate() {
            let (registered, sql) = MIGRATIONS[position];
            assert_eq!(
                registered,
                *name,
                "migration {} moved or was renamed",
                position + 1
            );
            assert_eq!(
                fingerprint(sql),
                *pinned,
                "{name} was edited after shipping; write a new migration instead"
            );
        }
        assert!(MIGRATIONS.len() >= SHIPPED.len());
    }
}
