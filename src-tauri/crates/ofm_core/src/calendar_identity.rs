//! Durable calendar identity, separate from scheduling and edition renewal.

use crate::generator::CompetitionDefinition;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use domain::competition_calendar::{CalendarMetadata, EditionBasis, SeasonPhase};
use domain::league::{CompetitionFormat, CompetitionScope, FixtureCompetition, League};

pub(crate) const LEAGUE_MATCHDAY_GAP_DAYS: u32 = 7;

pub(crate) fn invalid_definition_calendar(def: &CompetitionDefinition) -> bool {
    let Some(calendar) = &def.calendar else {
        return false;
    };
    let invalid_end = calendar.window_end.is_some_and(|end| {
        NaiveDate::from_ymd_opt(2000, end.month.into(), end.day.into()).is_none()
    });
    let invalid_division = calendar.division.as_ref().is_some_and(|division| {
        division.family_id.trim().is_empty()
            || division.tier == 0
            || def.format.kind != CompetitionFormat::LeagueTable
            || def.scope != CompetitionScope::Domestic
            || def
                .country_id
                .as_ref()
                .is_none_or(|country| country.trim().is_empty())
            || (division.phase != SeasonPhase::Annual && calendar.window_end.is_none())
    });
    invalid_end || invalid_division
}

fn calendar_year(season: u32) -> bool {
    (1000..=9999).contains(&season)
}

pub(crate) fn attach_definition_calendar(
    league: &mut League,
    def: &CompetitionDefinition,
    start: DateTime<Utc>,
) {
    league.calendar = Some(CalendarMetadata {
        definition_id: def.id.clone(),
        edition_basis: if calendar_year(league.season) {
            EditionBasis::CalendarYear
        } else if league.season > 0 {
            EditionBasis::LegacyOrdinal {
                first_season: league.season,
                opener_year: start.year(),
            }
        } else {
            EditionBasis::Unresolved
        },
        league_legs: (def.format.kind == CompetitionFormat::LeagueTable)
            .then(|| def.format.legs.unwrap_or(2)),
        matchday_gap_days: (def.format.kind == CompetitionFormat::LeagueTable)
            .then_some(LEAGUE_MATCHDAY_GAP_DAYS),
        season: def.calendar.clone().unwrap_or_default(),
    });
}

/// Adopt only provenance supported by the saved edition, never by today's clock,
/// display names, club overlap or a guess about the old authoring format.
/// Returns whether data changed, for persistence callers.
pub fn backfill_competition_calendar(league: &mut League) -> bool {
    if league.calendar.is_some() {
        return false;
    }
    let basis = if calendar_year(league.season) {
        EditionBasis::CalendarYear
    } else {
        verified_ordinal_basis(league).unwrap_or_default()
    };
    league.calendar = Some(CalendarMetadata {
        definition_id: league.id.clone(),
        edition_basis: basis,
        ..Default::default()
    });
    true
}

fn verified_ordinal_basis(league: &League) -> Option<EditionBasis> {
    if league.season == 0 || league.season >= 1000 {
        return None;
    }
    let dates = league
        .fixtures
        .iter()
        .filter(|fixture| fixture.competition != FixtureCompetition::Friendly)
        .map(|fixture| NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d"))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let opener = dates.into_iter().min()?;
    // Reject invalid authored values rather than treating their broad runtime clamp
    // as provenance. A valid leap-day opener uses the existing calendar helper.
    NaiveDate::from_ymd_opt(
        2000,
        league.season_start_month.into(),
        league.season_start_day.into(),
    )?;
    let year_start = NaiveDate::from_ymd_opt(opener.year(), 1, 1)?
        .and_hms_opt(0, 0, 0)?
        .and_utc();
    let authored = crate::generator::next_season_start(
        year_start,
        league.season_start_month,
        league.season_start_day,
    )
    .date_naive();
    (opener == authored).then_some(EditionBasis::LegacyOrdinal {
        first_season: league.season,
        opener_year: opener.year(),
    })
}

#[cfg(test)]
mod tests {
    use crate::generator::{CompetitionDefinition, build_explicit_competition};
    use chrono::{TimeZone, Utc};
    use domain::league::League;
    use serde_json::{Value, json};

    fn definition(kind: &str) -> CompetitionDefinition {
        serde_json::from_value(json!({
            "id":"authored", "name":"Authored", "type":"League", "scope":"Domestic",
            "countryId":"AR", "format":{"kind":kind,"legs":1},
            "participants":{"explicit":["a","b","c","d"]},
            "seasonStartMonth":2,"seasonStartDay":1,
            "calendar":{"division":{"familyId":"ar-league","tier":1,"phase":"opening"},
                        "windowEnd":{"month":6,"day":30}}
        }))
        .unwrap()
    }
    fn build(def: &CompetitionDefinition, season: u32) -> League {
        build_explicit_competition(
            def,
            season,
            Utc.with_ymd_and_hms(2033, 2, 1, 0, 0, 0).unwrap(),
        )
        .unwrap()
    }
    fn metadata() -> Value {
        json!({"definition_id":"authored","edition_basis":{"kind":"calendarYear"},
            "league_legs":1,"matchday_gap_days":7,
            "season":{"division":{"familyId":"ar-league","tier":1,"phase":"opening"},
                      "windowEnd":{"month":6,"day":30}}})
    }
    #[test]
    fn authored_league_calendar_retains_identity_and_single_leg_shape() {
        let competition = build(&definition("LeagueTable"), 2033);
        assert_eq!(competition.fixtures.len(), 6);
        let mut dates: Vec<_> = competition
            .fixtures
            .iter()
            .map(|fixture| fixture.date.as_str())
            .collect();
        dates.sort();
        assert_eq!(
            dates,
            vec![
                "2033-02-01",
                "2033-02-01",
                "2033-02-08",
                "2033-02-08",
                "2033-02-15",
                "2033-02-15"
            ]
        );
        assert_eq!(
            serde_json::to_value(&competition).unwrap()["calendar"],
            metadata()
        );
        assert_eq!(competition.season, 2033);
    }
    #[test]
    fn future_edition_identity_does_not_follow_the_clock() {
        let competition = build(&definition("LeagueTable"), 2038);
        assert_eq!(competition.season, 2038);
        assert_eq!(competition.fixtures[0].date, "2033-02-01");
        assert_eq!(
            serde_json::to_value(&competition).unwrap()["calendar"]["edition_basis"],
            json!({"kind":"calendarYear"})
        );
    }
    #[test]
    fn calendar_metadata_survives_core_serialization() {
        let mut raw =
            serde_json::to_value(League::new("authored".into(), "Authored".into(), 2033, &[]))
                .unwrap();
        raw["calendar"] = metadata();
        let league: League = serde_json::from_value(raw).unwrap();
        assert_eq!(
            serde_json::to_value(league).unwrap()["calendar"],
            metadata()
        );
    }
    #[test]
    fn missing_calendar_json_remains_backward_compatible() {
        let mut raw = serde_json::to_value(League::default()).unwrap();
        raw.as_object_mut().unwrap().remove("calendar");
        let league: League = serde_json::from_value(raw).unwrap();
        assert!(serde_json::to_value(league).unwrap()["calendar"].is_null());
        let mut raw = serde_json::to_value(definition("LeagueTable")).unwrap();
        raw.as_object_mut().unwrap().remove("calendar");
        let def: CompetitionDefinition = serde_json::from_value(raw).unwrap();
        assert!(def.calendar.is_none());
    }
    #[test]
    fn cup_calendar_keeps_its_opener_without_inventing_league_shape() {
        for kind in ["Knockout", "GroupAndKnockout"] {
            let mut raw = serde_json::to_value(definition(kind)).unwrap();
            raw["type"] = json!("Cup");
            raw["calendar"] = Value::Null;
            raw["seasonStartMonth"] = json!(3);
            raw["seasonStartDay"] = json!(8);
            let def = serde_json::from_value(raw).unwrap();
            let cup = build_explicit_competition(
                &def,
                2033,
                Utc.with_ymd_and_hms(2033, 3, 8, 0, 0, 0).unwrap(),
            )
            .unwrap();
            assert_eq!(
                cup.fixtures
                    .iter()
                    .map(|fixture| fixture.date.as_str())
                    .min(),
                Some("2033-03-08")
            );
            let saved = serde_json::to_value(&cup).unwrap();
            assert_eq!(saved["calendar"]["definition_id"], "authored");
            assert!(saved["calendar"]["league_legs"].is_null());
            assert!(saved["calendar"]["matchday_gap_days"].is_null());
            assert_eq!((cup.season_start_month, cup.season_start_day), (3, 8));
        }
    }
    #[test]
    fn leap_opener_keeps_authored_day_and_existing_clamp() {
        let mut def = definition("LeagueTable");
        def.season_start_day = Some(29);
        let start = crate::generator::next_season_start(
            Utc.with_ymd_and_hms(2033, 1, 1, 0, 0, 0).unwrap(),
            2,
            29,
        );
        let league = build_explicit_competition(&def, 2033, start).unwrap();
        assert_eq!(league.fixtures[0].date, "2033-02-28");
        assert_eq!(league.season_start_day, 29);
        assert_eq!(
            serde_json::to_value(league).unwrap()["calendar"]["definition_id"],
            "authored"
        );
    }
    #[test]
    fn invalid_calendar_definition_is_rejected_before_building() {
        for calendar in [
            json!({"division":{"familyId":"","tier":1,"phase":"opening"},"windowEnd":{"month":6,"day":30}}),
            json!({"division":{"familyId":"family","tier":0,"phase":"opening"},"windowEnd":{"month":6,"day":30}}),
            json!({"windowEnd":{"month":2,"day":30}}),
        ] {
            let mut raw = serde_json::to_value(definition("LeagueTable")).unwrap();
            raw["calendar"] = calendar;
            let def: CompetitionDefinition = serde_json::from_value(raw).unwrap();
            let file = crate::generator::CompetitionDefinitionFile {
                format_version: 1,
                competitions: vec![def],
            };
            let ctx = crate::generator::WorldValidationContext {
                team_ids: ["a", "b", "c", "d"].into(),
                country_codes: ["AR"].into(),
                region_ids: Default::default(),
            };
            let errors = crate::generator::validate_definitions(&file, &ctx);
            assert!(
                errors
                    .iter()
                    .any(|e| e.code == "be.error.competitionDef.invalidCalendar"
                        && e.competition_index == Some(0)),
                "{errors:?}"
            );
        }
    }
    #[test]
    fn selector_calendar_uses_the_same_metadata_rule() {
        let mut raw = serde_json::to_value(definition("LeagueTable")).unwrap();
        raw["participants"] = json!({"selector":{"kind":"allInCountry","country":"AR"}});
        let def = serde_json::from_value(raw).unwrap();
        let teams = ["a", "b", "c", "d"]
            .into_iter()
            .map(|id| {
                domain::team::Team::new(
                    id.into(),
                    id.into(),
                    id.into(),
                    "AR".into(),
                    "City".into(),
                    "Stadium".into(),
                    1000,
                )
            })
            .collect();
        let world = crate::generator::WorldData {
            teams,
            ..Default::default()
        };
        let file = crate::generator::CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![def],
        };
        let leagues = crate::generator::resolve_definitions(
            &file,
            &world,
            2033,
            Utc.with_ymd_and_hms(2033, 7, 1, 0, 0, 0).unwrap(),
        );
        assert_eq!(leagues.len(), 1);
        assert_eq!(
            serde_json::to_value(&leagues[0]).unwrap()["calendar"],
            metadata()
        );
    }
}
