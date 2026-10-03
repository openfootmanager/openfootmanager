use domain::league::{
    CompetitionRules, CompetitionScope, CompetitionState, CompetitionType, Fixture, GroupState,
    KnockoutRoundState, StandingEntry,
};
use rusqlite::{Connection, params};

const GAME_PERSISTENCE_LOAD_ERROR: &str = "be.error.gamePersistence.loadFailed";
const GAME_PERSISTENCE_WRITE_ERROR: &str = "be.error.gamePersistence.writeFailed";

fn competition_type_to_string(kind: &CompetitionType) -> &'static str {
    match kind {
        CompetitionType::League => "League",
        CompetitionType::Cup => "Cup",
        CompetitionType::ContinentalClub => "ContinentalClub",
        CompetitionType::InternationalClub => "InternationalClub",
        CompetitionType::InternationalNation => "InternationalNation",
        CompetitionType::FriendlyCup => "FriendlyCup",
    }
}

fn competition_scope_to_string(scope: &CompetitionScope) -> &'static str {
    match scope {
        CompetitionScope::Domestic => "Domestic",
        CompetitionScope::Regional => "Regional",
        CompetitionScope::Continental => "Continental",
        CompetitionScope::International => "International",
    }
}

fn parse_competition_type(value: &str) -> CompetitionType {
    match value {
        "Cup" => CompetitionType::Cup,
        "ContinentalClub" => CompetitionType::ContinentalClub,
        "InternationalClub" => CompetitionType::InternationalClub,
        "InternationalNation" => CompetitionType::InternationalNation,
        "FriendlyCup" => CompetitionType::FriendlyCup,
        _ => CompetitionType::League,
    }
}

fn parse_competition_scope(value: &str) -> CompetitionScope {
    match value {
        "Regional" => CompetitionScope::Regional,
        "Continental" => CompetitionScope::Continental,
        "International" => CompetitionScope::International,
        _ => CompetitionScope::Domestic,
    }
}

pub fn replace_competitions(
    conn: &Connection,
    competitions: &[CompetitionState],
) -> Result<(), String> {
    conn.execute("DELETE FROM competitions", [])
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

    for competition in competitions {
        let required_region_ids_json = serde_json::to_string(&competition.required_region_ids)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let participant_ids_json = serde_json::to_string(&competition.participant_ids)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let rules_json = serde_json::to_string(&competition.rules)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let fixtures_json = serde_json::to_string(&competition.fixtures)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let standings_json = serde_json::to_string(&competition.standings)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let groups_json = serde_json::to_string(&competition.groups)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let knockout_rounds_json = serde_json::to_string(&competition.knockout_rounds)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let transfer_log_json = serde_json::to_string(&competition.transfer_log)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let transfer_rumours_json = serde_json::to_string(&competition.transfer_rumours)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
        let berths_json = serde_json::to_string(&competition.berths)
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

        let calendar_json = competition
            .calendar
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;

        conn.execute(
            "INSERT INTO competitions (id, name, kind, scope, season, region_id, country_id, required_region_ids_json, participant_ids_json, rules_json, fixtures_json, standings_json, groups_json, knockout_rounds_json, transfer_log_json, transfer_rumours_json, priority, berths_json, season_start_month, season_start_day, calendar_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                competition.id,
                competition.name,
                competition_type_to_string(&competition.kind),
                competition_scope_to_string(&competition.scope),
                competition.season,
                competition.region_id,
                competition.country_id,
                required_region_ids_json,
                participant_ids_json,
                rules_json,
                fixtures_json,
                standings_json,
                groups_json,
                knockout_rounds_json,
                transfer_log_json,
                transfer_rumours_json,
                competition.priority,
                berths_json,
                competition.season_start_month as i64,
                competition.season_start_day as i64,
                calendar_json,
            ],
        )
        .map_err(|_| GAME_PERSISTENCE_WRITE_ERROR.to_string())?;
    }

    Ok(())
}

pub fn load_competitions(conn: &Connection) -> Result<Vec<CompetitionState>, String> {
    let mut stmt = match conn.prepare(
        "SELECT id, name, kind, scope, season, region_id, country_id, required_region_ids_json, participant_ids_json, rules_json, fixtures_json, standings_json, groups_json, knockout_rounds_json, transfer_log_json, transfer_rumours_json, priority, berths_json, season_start_month, season_start_day, calendar_json
         FROM competitions
         ORDER BY priority ASC, season DESC, name ASC",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Ok(Vec::new()),
    };

    let rows = stmt
        .query_map([], |row| {
            let required_region_ids_json: String = row.get(7)?;
            let participant_ids_json: String = row.get(8)?;
            let rules_json: String = row.get(9)?;
            let fixtures_json: String = row.get(10)?;
            let standings_json: String = row.get(11)?;
            let groups_json: String = row.get(12)?;
            let knockout_rounds_json: String = row.get(13)?;
            let transfer_log_json: String = row.get(14)?;
            let transfer_rumours_json: String = row.get(15)?;
            let berths_json: String = row.get(17)?;

            Ok(CompetitionState {
                calendar:
                    row.get::<_, Option<String>>(20)?
                        .map(|raw| {
                            serde_json::from_str::<
                                Option<domain::competition_calendar::CalendarMetadata>,
                            >(&raw)
                        })
                        .transpose()
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                20,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?
                        .flatten(),
                id: row.get(0)?,
                name: row.get(1)?,
                kind: parse_competition_type(&row.get::<_, String>(2)?),
                scope: parse_competition_scope(&row.get::<_, String>(3)?),
                season: row.get(4)?,
                region_id: row.get(5)?,
                country_id: row.get(6)?,
                required_region_ids: serde_json::from_str(&required_region_ids_json)
                    .unwrap_or_default(),
                participant_ids: serde_json::from_str(&participant_ids_json).unwrap_or_default(),
                rules: serde_json::from_str::<CompetitionRules>(&rules_json).unwrap_or_default(),
                fixtures: serde_json::from_str::<Vec<Fixture>>(&fixtures_json).unwrap_or_default(),
                standings: serde_json::from_str::<Vec<StandingEntry>>(&standings_json)
                    .unwrap_or_default(),
                groups: serde_json::from_str::<Vec<GroupState>>(&groups_json).unwrap_or_default(),
                knockout_rounds: serde_json::from_str::<Vec<KnockoutRoundState>>(
                    &knockout_rounds_json,
                )
                .unwrap_or_default(),
                transfer_log: serde_json::from_str(&transfer_log_json).unwrap_or_default(),
                transfer_rumours: serde_json::from_str(&transfer_rumours_json).unwrap_or_default(),
                priority: row.get::<_, i64>(16).unwrap_or_default() as u32,
                berths: serde_json::from_str(&berths_json).unwrap_or_default(),
                season_start_month: row.get::<_, i64>(18).unwrap_or(8) as u8,
                season_start_day: row.get::<_, i64>(19).unwrap_or(1) as u8,
                // name_key is not stored in the DB; re-derive it from type so
                // WC competitions display their translated name after a load.
                name_key: {
                    let id: String = row.get(0)?;
                    if id.starts_with("world-cup-qualifying-") {
                        Some("tournaments.competitions.worldCupQualifying".to_string())
                    } else if id.starts_with("world-cup-playoff-") {
                        Some("tournaments.competitions.worldCupPlayoff".to_string())
                    } else if parse_competition_type(&row.get::<_, String>(2)?)
                        == CompetitionType::InternationalNation
                    {
                        Some("tournaments.competitions.worldCup".to_string())
                    } else {
                        None
                    }
                },
            })
        })
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;

    let mut competitions = Vec::new();
    for row in rows {
        let mut competition = row.map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())?;
        ofm_core::calendar_identity::backfill_competition_calendar(&mut competition);
        competitions.push(competition);
    }
    Ok(competitions)
}

/// A load backfill must reach disk even when every other repair is already settled.
pub(crate) fn needs_calendar_backfill(conn: &Connection) -> Result<bool, String> {
    conn.query_row("SELECT EXISTS(SELECT 1 FROM competitions WHERE calendar_json IS NULL OR trim(calendar_json) = 'null') OR (NOT EXISTS(SELECT 1 FROM competitions) AND EXISTS(SELECT 1 FROM league))", [], |row| row.get(0))
        .map_err(|_| GAME_PERSISTENCE_LOAD_ERROR.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;
    use domain::league::League;

    /// Pins the stored name of every competition type.
    /// The case list also generates an exhaustive match for new variants.
    #[test]
    fn competition_types_are_stored_by_name() {
        use CompetitionType as T;
        crate::stored_text::assert_stored_as(
            &crate::stored_text::stored_text_cases!(
                T::League; [
                    ("League", T::League),
                    ("Cup", T::Cup),
                    ("ContinentalClub", T::ContinentalClub),
                    ("InternationalClub", T::InternationalClub),
                    ("InternationalNation", T::InternationalNation),
                    ("FriendlyCup", T::FriendlyCup),
                ]
            ),
            |value| competition_type_to_string(value).to_string(),
            parse_competition_type,
        );
    }

    /// Pins the stored name of every competition scope.
    /// The case list also generates an exhaustive match for new variants.
    #[test]
    fn competition_scopes_are_stored_by_name() {
        use CompetitionScope as S;
        crate::stored_text::assert_stored_as(
            &crate::stored_text::stored_text_cases!(
                S::Domestic; [
                    ("Domestic", S::Domestic),
                    ("Regional", S::Regional),
                    ("Continental", S::Continental),
                    ("International", S::International),
                ]
            ),
            |value| competition_scope_to_string(value).to_string(),
            parse_competition_scope,
        );
    }

    fn division(id: &str, priority: u32, clubs: &[&str]) -> CompetitionState {
        let team_ids: Vec<String> = clubs.iter().map(|club| club.to_string()).collect();
        let mut league = League::new(id.to_string(), id.to_string(), 2035, &team_ids);
        league.priority = priority;
        league.country_id = Some("ENG".to_string());
        league
    }

    /// Promotion and relegation are written back as nothing but a new
    /// `participant_ids` list, so this column is the whole of what a rollover
    /// persists. It is stored as JSON in a hand-written positional INSERT,
    /// which is exactly the shape that loses a field silently.
    #[test]
    fn a_promoted_roster_survives_a_round_trip() {
        let database = GameDatabase::open_in_memory().expect("an in-memory database");
        let connection = database.conn();

        let mut first = division("eng-d1", 0, &["a1", "a2", "a3", "a4"]);
        let mut second = division("eng-d2", 1, &["b1", "b2", "b3", "b4"]);
        replace_competitions(connection, &[first.clone(), second.clone()])
            .expect("the initial write");

        // The rollover swaps a club each way and writes the whole set again.
        first.participant_ids = vec![
            "a1".to_string(),
            "a2".to_string(),
            "a3".to_string(),
            "b1".to_string(),
        ];
        second.participant_ids = vec![
            "b2".to_string(),
            "b3".to_string(),
            "b4".to_string(),
            "a4".to_string(),
        ];
        replace_competitions(connection, &[first.clone(), second.clone()])
            .expect("the rollover write");

        let loaded = load_competitions(connection).expect("the reload");
        let by_id = |id: &str| {
            loaded
                .iter()
                .find(|competition| competition.id == id)
                .unwrap_or_else(|| panic!("{id} should have been stored"))
        };
        assert_eq!(by_id("eng-d1").participant_ids, first.participant_ids);
        assert_eq!(by_id("eng-d2").participant_ids, second.participant_ids);
        assert_eq!(by_id("eng-d1").priority, 0, "tier rank must survive too");
        assert_eq!(by_id("eng-d2").priority, 1);
    }

    /// A competition retired at rollover — a World Cup edition, a cup that no
    /// longer exists — must not come back on the next load. `replace_` is a
    /// delete-then-insert, and this is what says so.
    #[test]
    fn a_retired_competition_does_not_come_back() {
        let database = GameDatabase::open_in_memory().expect("an in-memory database");
        let connection = database.conn();

        let keep = division("eng-d1", 0, &["a1", "a2"]);
        let retire = division("world-cup-2038", 9, &["a1", "a2"]);
        replace_competitions(connection, &[keep.clone(), retire]).expect("the initial write");
        assert_eq!(load_competitions(connection).expect("reload").len(), 2);

        replace_competitions(connection, &[keep]).expect("the rollover write");

        let loaded = load_competitions(connection).expect("the reload");
        assert_eq!(loaded.len(), 1, "the retired edition is gone: {loaded:?}");
        assert_eq!(loaded[0].id, "eng-d1");
    }
    #[test]
    fn authored_group_size_survives_sqlite_and_next_season() {
        use chrono::TimeZone;
        let database = GameDatabase::open_in_memory().unwrap();
        let connection = database.conn();
        let ids: Vec<String> = (0..8).map(|i| format!("club-{i}")).collect();
        let start = chrono::Utc.with_ymd_and_hms(2031, 8, 1, 0, 0, 0).unwrap();
        let definition: ofm_core::generator::CompetitionDefinition = serde_json::from_value(
            serde_json::json!({"id":"authored-cup", "name":"Authored Cup", "type":"Cup", "scope":"Domestic",
                "format":{"kind":"GroupAndKnockout","groupSize":2}, "participants":{"explicit":ids}})
        ).unwrap();
        let cup =
            ofm_core::generator::build_explicit_competition(&definition, 2031, start).unwrap();
        replace_competitions(connection, &[cup]).unwrap();
        let stored: String = connection
            .query_row("SELECT rules_json FROM competitions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&stored).unwrap()["group_size"],
            2
        );
        let mut loaded = load_competitions(connection).unwrap().remove(0);
        assert_eq!(
            serde_json::to_value(&loaded.rules).unwrap()["group_size"],
            2
        );
        ofm_core::group_stage::regenerate_for_season(
            &mut loaded,
            2032,
            start + chrono::Duration::days(366),
        );
        assert_eq!(loaded.groups.len(), 4);
        assert!(loaded.groups.iter().all(|group| group.team_ids.len() == 2));
        assert_eq!(loaded.fixtures.len(), 8);
        replace_competitions(connection, &[loaded]).unwrap();
        let loaded = load_competitions(connection).unwrap().remove(0);
        assert_eq!(loaded.season, 2032);
        assert_eq!(loaded.groups.len(), 4);

        // A shipped rules_json without the new field keeps the old shape.
        let mut old_rules: serde_json::Value = serde_json::from_str(&stored).unwrap();
        old_rules.as_object_mut().unwrap().remove("group_size");
        connection
            .execute(
                "UPDATE competitions SET rules_json = ?1",
                [old_rules.to_string()],
            )
            .unwrap();
        let mut legacy = load_competitions(connection).unwrap().remove(0);
        assert_eq!(
            serde_json::to_value(&legacy.rules).unwrap()["group_size"],
            4
        );
        ofm_core::group_stage::regenerate_for_season(&mut legacy, 2033, start);
        assert_eq!(legacy.groups.len(), 2);
        assert_eq!(legacy.fixtures.len(), 24);
    }
    fn calendar_sample() -> League {
        let raw = serde_json::json!({"id":"authored","name":"Authored","season":2033,
            "calendar":{"definition_id":"authored","edition_basis":{"kind":"calendarYear"},
              "league_legs":1,"matchday_gap_days":7,
              "season":{"division":{"familyId":"ar-league","tier":1,"phase":"closing"},"windowEnd":{"month":1,"day":31}}}});
        let mut league: League = serde_json::from_value(raw).unwrap();
        league.participant_ids = vec!["a".into(), "b".into()];
        league.fixtures = legacy_division(2033, "2033-07-01").fixtures;
        for fixture in &mut league.fixtures {
            fixture.competition_id = league.id.clone();
        }
        league.fixtures[0].status = domain::league::FixtureStatus::Completed;
        league.fixtures[0].result = Some(domain::league::MatchResult {
            home_goals: 3,
            away_goals: 1,
            ..Default::default()
        });
        league
    }
    #[test]
    fn sqlite_calendar_metadata_survives_replace_and_reload() {
        let db = GameDatabase::open_in_memory().unwrap();
        for _ in 0..2 {
            let original = calendar_sample();
            let original_fixtures = serde_json::to_value(&original.fixtures).unwrap();
            let original_calendar = serde_json::to_value(&original.calendar).unwrap();
            replace_competitions(db.conn(), &[original]).unwrap();
            let loaded = load_competitions(db.conn()).unwrap();
            let value = serde_json::to_value(&loaded[0]).unwrap();
            assert_eq!(value["fixtures"], original_fixtures);
            assert_eq!(value["calendar"], original_calendar);
            assert_eq!(value["calendar"]["league_legs"], 1);
            assert_eq!(value["calendar"]["season"]["division"]["phase"], "closing");
        }
    }
    fn legacy_division(season: u32, date: &str) -> League {
        let mut league = League::new(
            "legacy".into(),
            "Apertura".into(),
            season,
            &["a".into(), "b".into()],
        );
        league.fixtures = ofm_core::schedule::build_round_robin_fixtures(
            &league.id,
            &league.participant_ids,
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc(),
            domain::league::FixtureCompetition::League,
        );
        league
    }
    #[test]
    fn legacy_ordinal_uses_verified_opener_provenance() {
        let db = GameDatabase::open_in_memory().unwrap();
        for (date, month, day, year) in [
            ("2030-08-01", 8, 1, 2030),
            ("2033-02-28", 2, 29, 2033),
            ("2032-02-29", 2, 29, 2032),
        ] {
            let mut league = legacy_division(5, date);
            league.season_start_month = month;
            league.season_start_day = day;
            let mut friendly = league.fixtures[0].clone();
            friendly.id = "preseason".into();
            friendly.date = "2029-01-01".into();
            friendly.competition = domain::league::FixtureCompetition::Friendly;
            league.fixtures.push(friendly);
            let fixtures = serde_json::to_value(&league.fixtures).unwrap();
            replace_competitions(db.conn(), &[league]).unwrap();
            let loaded = load_competitions(db.conn()).unwrap();
            let value = serde_json::to_value(&loaded[0]).unwrap();
            assert_eq!(
                value["calendar"]["edition_basis"],
                serde_json::json!({"kind":"legacyOrdinal","first_season":5,"opener_year":year})
            );
            assert_eq!(loaded[0].season, 5);
            assert_eq!(serde_json::to_value(&loaded[0].fixtures).unwrap(), fixtures);
        }
    }
    #[test]
    fn ambiguous_legacy_edition_is_recorded_as_unresolved() {
        let db = GameDatabase::open_in_memory().unwrap();
        for date in ["2030-05-16", "2030-08-01"] {
            let mut league = legacy_division(5, date);
            if date.ends_with("08-01") {
                league.fixtures[0].date = "broken".into();
            }
            replace_competitions(db.conn(), &[league]).unwrap();
            let loaded = load_competitions(db.conn()).unwrap();
            assert_eq!(
                serde_json::to_value(&loaded[0]).unwrap()["calendar"]["edition_basis"],
                serde_json::json!({"kind":"unresolved"})
            );
        }
    }
    #[test]
    fn legacy_same_roster_does_not_invent_phase_identity() {
        let db = GameDatabase::open_in_memory().unwrap();
        let a = legacy_division(2033, "2033-08-01");
        let mut b = a.clone();
        b.id = "clausura".into();
        b.name = "Clausura".into();
        replace_competitions(db.conn(), &[a, b]).unwrap();
        let loaded = load_competitions(db.conn()).unwrap();
        for league in loaded {
            let value = serde_json::to_value(&league).unwrap();
            assert_eq!(value["calendar"]["definition_id"], league.id);
            assert!(value["calendar"]["season"]["division"].is_null());
        }
    }
    #[test]
    fn corrupt_calendar_metadata_is_not_silently_discarded() {
        let db = GameDatabase::open_in_memory().unwrap();
        replace_competitions(db.conn(), &[calendar_sample()]).unwrap();
        db.conn()
            .execute("UPDATE competitions SET calendar_json = '{broken'", [])
            .unwrap();
        assert_eq!(
            load_competitions(db.conn()).unwrap_err(),
            "be.error.gamePersistence.loadFailed"
        );
    }
}
