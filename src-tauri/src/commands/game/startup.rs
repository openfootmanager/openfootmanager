//! What the player chooses before a career exists: the start year, the phase
//! they join in, and how much past history to generate behind them.
//!
//! `RawStartupOptions` is the shape the frontend sends; `StartupOptions` is
//! the validated shape everything after this point may rely on, and
//! `normalize_startup_options` is the only bridge between them.

use chrono::{Datelike, Duration, Utc};

use ofm_core::career::StartPhase;
use ofm_core::world::{start_date_for_year, MIN_START_YEAR};

/// A manager must be at least this old on the day the career opens.
const MANAGER_MINIMUM_AGE: i32 = 30;
const MANAGER_MAXIMUM_AGE: i32 = 99;
pub(super) const DEFAULT_GENERATED_HISTORY_DEPTH_YEARS: u32 = 12;
pub(super) const MAX_GENERATED_HISTORY_DEPTH_YEARS: u32 = 24;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawStartupOptions {
    #[serde(default)]
    start_year: Option<i32>,
    #[serde(default)]
    start_phase: Option<String>,
    #[serde(default)]
    history_depth_years: Option<u32>,
}

impl RawStartupOptions {
    #[cfg(feature = "mcp")]
    pub(super) fn new(start_year: Option<i32>, start_phase: Option<String>) -> Self {
        Self {
            start_year,
            start_phase,
            history_depth_years: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartupOptions {
    pub(super) start_year: i32,
    pub(super) start_phase: StartPhase,
    pub(super) history_depth_years: u32,
}

fn default_start_year() -> i32 {
    chrono::Utc::now().year().max(MIN_START_YEAR)
}

fn default_history_depth_years() -> u32 {
    DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
}

pub(super) fn current_date_for_phase(
    start_year: i32,
    start_phase: StartPhase,
) -> Result<chrono::DateTime<Utc>, String> {
    let start_date = start_date_for_year(start_year)?;
    Ok(match start_phase {
        StartPhase::SeasonStart => start_date,
        StartPhase::MidSeason => start_date + Duration::days(120),
    })
}

/// The translation key of what is wrong with a manager's date of birth on the day a career opens,
/// or `None` when it will do. The one rule: `start_new_game` and the create-manager form both ask it.
pub(super) fn manager_age_error(
    birth_date: chrono::NaiveDate,
    opening: chrono::NaiveDate,
) -> Option<&'static str> {
    let age = domain::age::age_between(birth_date, opening);
    if age < MANAGER_MINIMUM_AGE {
        Some("be.error.createManager.minAge")
    } else if age > MANAGER_MAXIMUM_AGE {
        Some("be.error.createManager.invalidDob")
    } else {
        None
    }
}

/// Refuses a manager the age rule rejects on the day this world's career opens: the clock
/// `start_new_game` builds, which a historical snapshot sets itself. This is the last word; the
/// form can only check the generated-world date (`manager_dob_error`).
pub(super) fn check_manager_age(
    birth_date: chrono::NaiveDate,
    clock: &ofm_core::clock::GameClock,
) -> Result<(), String> {
    match manager_age_error(birth_date, clock.current_date.date_naive()) {
        Some(key) => Err(key.to_string()),
        None => Ok(()),
    }
}

/// What the create-manager form asks before a world has been chosen: the date of birth against the
/// day a generated world's career opens on (`current_date_for_phase`: 1 June in a World Cup summer,
/// 1 July otherwise, plus 120 days mid-season). A historical package can set its own opening date,
/// so `start_new_game` still has the last word. `Err` is a bad year or phase, which is the caller's
/// mistake; `Ok(Some(key))` is a finding about the date of birth.
pub(super) fn manager_dob_error(
    dob: &str,
    start_year: i32,
    start_phase: &str,
) -> Result<Option<String>, String> {
    let start_phase = StartPhase::parse(start_phase)
        .ok_or_else(|| "be.error.createManager.invalidStartPhase".to_string())?;
    let opening = current_date_for_phase(start_year, start_phase)?.date_naive();
    let Ok(birth_date) = chrono::NaiveDate::parse_from_str(dob, "%Y-%m-%d") else {
        return Ok(Some("be.error.createManager.invalidDobFormat".to_string()));
    };
    Ok(manager_age_error(birth_date, opening).map(str::to_string))
}

pub(super) fn normalize_startup_options(
    raw: Option<RawStartupOptions>,
) -> Result<StartupOptions, String> {
    let raw = raw.unwrap_or_default();
    let start_year = raw.start_year.unwrap_or_else(default_start_year);
    if start_year < MIN_START_YEAR {
        return Err("be.error.createManager.startYearMin".to_string());
    }

    let start_phase = match raw.start_phase.as_deref() {
        None | Some("") => StartPhase::SeasonStart,
        Some(value) => StartPhase::parse(value)
            .ok_or_else(|| "be.error.createManager.invalidStartPhase".to_string())?,
    };
    let history_depth_years = raw
        .history_depth_years
        .unwrap_or_else(default_history_depth_years);
    if history_depth_years > MAX_GENERATED_HISTORY_DEPTH_YEARS {
        return Err("be.error.createManager.historyDepthMax".to_string());
    }

    Ok(StartupOptions {
        start_year,
        start_phase,
        history_depth_years,
    })
}

/// What a career-start request asked for, rendered for the log.
///
/// `start_new_game` had no log line at all, so every early `Err` it returned — an empty name, a
/// DOB that will not parse, a package that will not load — produced no output whatsoever. Combined
/// with a frontend that reported the failure through `window.alert` and wrote nothing down, a
/// failed career start was indistinguishable from a click that did nothing. See #558.
///
/// **The manager's name is deliberately absent.** It is free text the player typed, it is very
/// often their own name, and this line is written on every start — so it would end up in the log
/// folder that slice 3 packages into a bug report. Lengths are what the validation actually turns
/// on (empty, or over 30), and lengths are enough to explain the rejection without carrying the
/// name off the machine. The same reasoning keeps the date of birth out: its *shape* decides
/// whether parsing fails, and the shape is all a diagnosis needs.
pub(super) struct StartRequestSummary<'a> {
    pub(super) first_name: &'a str,
    pub(super) last_name: &'a str,
    pub(super) dob: &'a str,
    pub(super) nationality: &'a str,
    pub(super) startup_options: Option<&'a RawStartupOptions>,
    pub(super) world_source: Option<&'a str>,
    pub(super) package_ids: Option<&'a [String]>,
}

/// `YYYY-MM-DD`, or a description of what arrived instead.
fn dob_shape(dob: &str) -> &'static str {
    if dob.is_empty() {
        return "empty";
    }
    let mut parts = dob.split('-');
    let ok = matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(y), Some(m), Some(d), None)
            if y.len() == 4
                && m.len() == 2
                && d.len() == 2
                && [y, m, d].iter().all(|p| p.bytes().all(|b| b.is_ascii_digit()))
    );
    if ok {
        "yyyy-mm-dd"
    } else {
        "malformed"
    }
}

impl StartRequestSummary<'_> {
    pub(super) fn describe(&self) -> String {
        let (start_year, start_phase, history_depth) = match self.startup_options {
            Some(options) => (
                options.start_year,
                options.start_phase.as_deref(),
                options.history_depth_years,
            ),
            None => (None, None, None),
        };
        format!(
            "name_len={}/{} nationality={:?} dob={} start_year={:?} phase={:?} history_depth={:?} \
world_source={:?} packages={}",
            self.first_name.trim().chars().count(),
            self.last_name.trim().chars().count(),
            self.nationality,
            dob_shape(self.dob),
            start_year,
            start_phase,
            history_depth,
            self.world_source,
            self.package_ids.map_or(0, <[String]>::len),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ofm_core::game::Game;

    fn summary_for<'a>(
        first: &'a str,
        last: &'a str,
        dob: &'a str,
        options: Option<&'a RawStartupOptions>,
    ) -> StartRequestSummary<'a> {
        StartRequestSummary {
            first_name: first,
            last_name: last,
            dob,
            nationality: "Brazil",
            startup_options: options,
            world_source: None,
            package_ids: None,
        }
    }

    #[test]
    fn start_summary_never_carries_the_manager_name() {
        // This line is written on every start, and slice 3 packages the log folder into a bug
        // report. A name the player typed must not ride along.
        let described = summary_for("Sturdy", "Robot", "1990-05-02", None).describe();

        assert!(!described.contains("Sturdy"), "{described}");
        assert!(!described.contains("Robot"), "{described}");
        assert!(described.contains("name_len=6/5"), "{described}");
    }

    #[test]
    fn start_summary_reports_the_dob_shape_not_the_date() {
        let described = summary_for("A", "B", "1990-05-02", None).describe();

        assert!(described.contains("dob=yyyy-mm-dd"), "{described}");
        assert!(!described.contains("1990"), "{described}");
    }

    #[test]
    fn start_summary_distinguishes_the_three_dob_failures() {
        assert!(summary_for("A", "B", "", None)
            .describe()
            .contains("dob=empty"));
        assert!(summary_for("A", "B", "02/05/1990", None)
            .describe()
            .contains("dob=malformed"));
        assert!(summary_for("A", "B", "1990-5-2", None)
            .describe()
            .contains("dob=malformed"));
    }

    #[test]
    fn start_summary_reports_blank_names_as_zero_length() {
        // The validation rejects an empty name after trimming, so whitespace has to read as empty
        // here too — otherwise the log says a name was supplied and the error says it was not.
        let described = summary_for("   ", "", "1990-05-02", None).describe();

        assert!(described.contains("name_len=0/0"), "{described}");
    }

    #[test]
    fn start_summary_includes_the_startup_options_that_were_asked_for() {
        let options = RawStartupOptions {
            start_year: Some(1962),
            start_phase: Some("midSeason".to_owned()),
            history_depth_years: Some(8),
        };
        let described = summary_for("A", "B", "1990-05-02", Some(&options)).describe();

        assert!(described.contains("start_year=Some(1962)"), "{described}");
        assert!(
            described.contains("phase=Some(\"midSeason\")"),
            "{described}"
        );
        assert!(described.contains("history_depth=Some(8)"), "{described}");
    }

    #[test]
    fn start_summary_survives_absent_startup_options() {
        let described = summary_for("A", "B", "1990-05-02", None).describe();

        assert!(described.contains("start_year=None"), "{described}");
        assert!(described.contains("packages=0"), "{described}");
    }
    use crate::commands::game::game_clock_for_world;
    use crate::commands::game::testkit::make_historical_snapshot_world;
    use ofm_core::clock::GameClock;
    use ofm_core::season_context::refresh_game_context;
    use ofm_core::world::{preseason_league_year, preseason_season_start};

    #[test]
    fn normalize_startup_options_defaults_to_current_year_and_season_start() {
        use chrono::Datelike;

        // Bracket the call rather than reading the clock a second time
        // afterwards: `default_start_year` has its own `Utc::now()`, so a run
        // that straddles a UTC new year would otherwise fail with no defect.
        let before = chrono::Utc::now().year();
        let options = normalize_startup_options(None).unwrap();
        let after = chrono::Utc::now().year();

        // Defaulting still means "today", not the floor — a fresh career should
        // open in the current year even though historical years are now legal.
        assert!(
            options.start_year == before || options.start_year == after,
            "expected the current year ({before} or {after}), got {}",
            options.start_year
        );
        assert_eq!(options.start_phase, StartPhase::SeasonStart);
        assert_eq!(
            options.history_depth_years,
            DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
        );
    }

    #[test]
    fn normalize_startup_options_rejects_years_before_the_floor() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(MIN_START_YEAR - 1),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: None,
        }));

        assert_eq!(result.unwrap_err(), "be.error.createManager.startYearMin");
    }

    #[test]
    fn normalize_startup_options_accepts_historical_start_years() {
        // A 1962 career is the motivating case: historical world packages need a
        // clock that predates the modern era by decades.
        let options = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(1962),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: None,
        }))
        .expect("1962 is above the floor and must be accepted");

        assert_eq!(options.start_year, 1962);
    }

    #[test]
    fn start_date_for_historical_world_cup_year_opens_in_june() {
        use chrono::Datelike;

        let start = start_date_for_year(1962).expect("1962 is a valid start year");

        assert_eq!(start.year(), 1962);
        assert_eq!(start.month(), 6);
    }

    #[test]
    fn normalize_startup_options_rejects_unknown_start_phase() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("playoffs".to_string()),
            history_depth_years: None,
        }));

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.invalidStartPhase"
        );
    }

    #[test]
    fn normalize_startup_options_rejects_history_depths_above_maximum() {
        let result = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: Some(MAX_GENERATED_HISTORY_DEPTH_YEARS + 1),
        }));

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.historyDepthMax"
        );
    }

    #[test]
    fn normalize_startup_options_accepts_custom_history_depth() {
        let options = normalize_startup_options(Some(RawStartupOptions {
            start_year: Some(2026),
            start_phase: Some("seasonStart".to_string()),
            history_depth_years: Some(24),
        }))
        .unwrap();

        assert_eq!(options.history_depth_years, 24);
    }

    #[test]
    fn start_date_for_year_uses_selected_july_first() {
        let start_date = start_date_for_year(2032).unwrap();

        assert_eq!(start_date.to_rfc3339(), "2032-07-01T00:00:00+00:00");
    }

    #[test]
    fn start_date_for_year_rejects_out_of_range_years() {
        let result = start_date_for_year(i32::MAX);

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.invalidStartYear"
        );
    }

    #[test]
    fn current_date_for_midseason_phase_is_after_start_date() {
        let current_date = current_date_for_phase(2032, StartPhase::MidSeason).unwrap();

        assert_eq!(current_date.to_rfc3339(), "2032-10-29T00:00:00+00:00");
    }

    #[test]
    fn age_on_date_uses_selected_start_year() {
        let birth_date = chrono::NaiveDate::from_ymd_opt(2008, 1, 1).unwrap();
        let reference_date = current_date_for_phase(2038, StartPhase::SeasonStart)
            .unwrap()
            .date_naive();

        assert_eq!(
            i64::from(domain::age::age_between(birth_date, reference_date)),
            30
        );
    }

    /// Given a manager born on 15 June 1996 and a career opening in 2026, a World Cup summer
    ///       (the clock starts on 1 June, not 1 July)
    /// When the form asks whether the date of birth will do
    /// Then he is 29 on the day the career opens, so he is too young; and in 2027, which opens on
    ///      1 July, a manager born on 15 June 1997 is 30 and passes.
    #[test]
    fn manager_dob_is_checked_against_the_day_the_career_really_opens() {
        assert_eq!(
            manager_dob_error("1996-06-15", 2026, "seasonStart").unwrap(),
            Some("be.error.createManager.minAge".to_string())
        );
        assert_eq!(
            manager_dob_error("1997-06-15", 2027, "seasonStart").unwrap(),
            None
        );
    }

    /// Given a 2026 career, which opens on 1 June
    /// When managers born on 1 June and on 2 June of the years that make them 30 and 99 are checked
    /// Then exactly 30 and exactly 99 pass, and a day short of 30 or a day past 99 do not: the age
    ///      limits are inclusive at 30 and at 99.
    #[test]
    fn manager_age_limits_are_inclusive_to_the_day() {
        let age_30_today = manager_dob_error("1996-06-01", 2026, "seasonStart").unwrap();
        let almost_30 = manager_dob_error("1996-06-02", 2026, "seasonStart").unwrap();
        let age_99_today = manager_dob_error("1927-06-01", 2026, "seasonStart").unwrap();
        let age_100_today = manager_dob_error("1926-06-01", 2026, "seasonStart").unwrap();

        assert_eq!(age_30_today, None);
        assert_eq!(almost_30, Some("be.error.createManager.minAge".to_string()));
        assert_eq!(age_99_today, None);
        assert_eq!(
            age_100_today,
            Some("be.error.createManager.invalidDob".to_string())
        );
    }

    /// Given a manager born on 1 August 1996 and a 2026 career
    /// When the career opens at the start of the season (1 June) and mid-season (29 September)
    /// Then he is 29 at the start and 30 mid-season: he turns 30 in between, so the phase decides.
    #[test]
    fn manager_dob_follows_the_start_phase() {
        assert_eq!(
            manager_dob_error("1996-08-01", 2026, "seasonStart").unwrap(),
            Some("be.error.createManager.minAge".to_string())
        );
        assert_eq!(
            manager_dob_error("1996-08-01", 2026, "midSeason").unwrap(),
            None
        );
    }

    /// Given dates of birth that make a manager 100, that are not dates, and that are fine
    /// When the form asks
    /// Then they answer invalidDob, invalidDobFormat and nothing; an unknown start phase or year is
    ///      the caller's error, not a finding about the date of birth.
    #[test]
    fn manager_dob_names_each_kind_of_problem() {
        assert_eq!(
            manager_dob_error("1900-01-01", 2026, "seasonStart").unwrap(),
            Some("be.error.createManager.invalidDob".to_string())
        );
        assert_eq!(
            manager_dob_error("not-a-date", 2026, "seasonStart").unwrap(),
            Some("be.error.createManager.invalidDobFormat".to_string())
        );
        assert_eq!(
            manager_dob_error("1980-01-01", 2026, "seasonStart").unwrap(),
            None
        );
        assert_eq!(
            manager_dob_error("1980-01-01", 2026, "nextWeek").unwrap_err(),
            "be.error.createManager.invalidStartPhase"
        );
        assert_eq!(
            manager_dob_error("1980-01-01", i32::MAX, "seasonStart").unwrap_err(),
            "be.error.createManager.invalidStartYear"
        );
    }

    #[test]
    fn age_on_date_changes_between_season_start_and_midseason() {
        let birth_date = chrono::NaiveDate::from_ymd_opt(2008, 8, 1).unwrap();
        let season_start = current_date_for_phase(2038, StartPhase::SeasonStart)
            .unwrap()
            .date_naive();
        let midseason = current_date_for_phase(2038, StartPhase::MidSeason)
            .unwrap()
            .date_naive();

        assert_eq!(
            i64::from(domain::age::age_between(birth_date, season_start)),
            29
        );
        assert_eq!(
            i64::from(domain::age::age_between(birth_date, midseason)),
            30
        );
    }

    #[test]
    fn age_on_date_uses_world_snapshot_date_over_startup_phase() {
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let world = make_historical_snapshot_world();
        let reference_date = game_clock_for_world(&startup_options, &world.metadata)
            .unwrap()
            .current_date
            .date_naive();
        let birth_date = chrono::NaiveDate::from_ymd_opt(2001, 12, 15).unwrap();

        assert_eq!(reference_date.to_string(), "2031-11-20");
        assert_eq!(
            i64::from(domain::age::age_between(birth_date, reference_date)),
            29
        );
    }

    /// Given a historical-snapshot package whose career opens on 20 November 2031 whatever the start
    ///       year says, and managers born on 15 December 2001 (29 that day) and on 20 November 2001
    ///       (30 that day)
    /// When `check_manager_age`, the check `start_new_game` makes, runs against the world's own clock
    /// Then the first is refused with minAge and the second passes, although the form, which only
    ///      knows a generated world's date, lets the first through for the same 2032 start: the
    ///      backend has the last word.
    #[test]
    fn check_manager_age_refuses_a_manager_too_young_on_the_snapshot_date() {
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let world = make_historical_snapshot_world();
        let clock = game_clock_for_world(&startup_options, &world.metadata).unwrap();
        let too_young = chrono::NaiveDate::from_ymd_opt(2001, 12, 15).unwrap();
        let just_old_enough = chrono::NaiveDate::from_ymd_opt(2001, 11, 20).unwrap();

        assert_eq!(
            check_manager_age(too_young, &clock),
            Err("be.error.createManager.minAge".to_string())
        );
        assert_eq!(check_manager_age(just_old_enough, &clock), Ok(()));
        assert_eq!(manager_dob_error("2001-12-15", 2032, "midSeason"), Ok(None));
    }

    #[test]
    fn preseason_league_setup_uses_selected_start_year_for_context() {
        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = vec![
            domain::team::Team::new(
                "team1".to_string(),
                "Alpha FC".to_string(),
                "AFC".to_string(),
                "England".to_string(),
                "London".to_string(),
                "Alpha Park".to_string(),
                20_000,
            ),
            domain::team::Team::new(
                "team2".to_string(),
                "Beta FC".to_string(),
                "BFC".to_string(),
                "England".to_string(),
                "Manchester".to_string(),
                "Beta Park".to_string(),
                22_000,
            ),
        ];
        let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

        let season_start = preseason_season_start(&game.clock);
        let team_ids = game
            .teams
            .iter()
            .map(|team| team.id.clone())
            .collect::<Vec<_>>();
        game.league = Some(ofm_core::schedule::generate_league(
            "Premier Division",
            preseason_league_year(&game.clock),
            &team_ids,
            season_start,
        ));
        refresh_game_context(&mut game);

        assert_eq!(
            game.clock.start_date.to_rfc3339(),
            "2032-07-01T00:00:00+00:00"
        );
        assert_eq!(game.league.as_ref().map(|league| league.season), Some(2032));
        assert_eq!(
            game.season_context.season_start.as_deref(),
            Some("2032-07-31")
        );
        assert_eq!(game.season_context.days_until_season_start, Some(30));
    }

    #[test]
    fn generating_past_history_populates_default_twelve_prior_seasons() {
        let clock = GameClock::new(start_date_for_year(2032).unwrap());
        let manager = domain::manager::Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = vec![
            domain::team::Team::new(
                "team1".to_string(),
                "Alpha FC".to_string(),
                "AFC".to_string(),
                "England".to_string(),
                "London".to_string(),
                "Alpha Park".to_string(),
                20_000,
            ),
            domain::team::Team::new(
                "team2".to_string(),
                "Beta FC".to_string(),
                "BFC".to_string(),
                "England".to_string(),
                "Manchester".to_string(),
                "Beta Park".to_string(),
                22_000,
            ),
        ];
        let staff = vec![
            {
                let mut staff = domain::staff::Staff::new(
                    "staff1".to_string(),
                    "Pat".to_string(),
                    "Coach".to_string(),
                    "1978-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 70,
                        judging_ability: 65,
                        judging_potential: 64,
                        physiotherapy: 40,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team1".to_string());
                staff
            },
            {
                let mut staff = domain::staff::Staff::new(
                    "staff2".to_string(),
                    "Lee".to_string(),
                    "Coach".to_string(),
                    "1979-01-01".to_string(),
                    domain::staff::StaffRole::AssistantManager,
                    domain::staff::StaffAttributes {
                        coaching: 72,
                        judging_ability: 66,
                        judging_potential: 65,
                        physiotherapy: 39,
                    },
                );
                staff.nationality = "England".to_string();
                staff.team_id = Some("team2".to_string());
                staff
            },
        ];
        let players = vec![
            {
                let mut player = domain::player::Player::new(
                    "player1".to_string(),
                    "A. Keeper".to_string(),
                    "Alex Keeper".to_string(),
                    "1994-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Goalkeeper,
                    domain::player::PlayerAttributes {
                        pace: 48,
                        stamina: 62,
                        strength: 64,
                        agility: 66,
                        passing: 50,
                        shooting: 20,
                        tackling: 18,
                        dribbling: 32,
                        defending: 24,
                        positioning: 68,
                        vision: 48,
                        decisions: 63,
                        composure: 61,
                        aggression: 38,
                        teamwork: 64,
                        leadership: 58,
                        handling: 76,
                        reflexes: 77,
                        aerial: 72,
                    },
                );
                player.team_id = Some("team1".to_string());
                player.ovr = 68;
                player.potential = 73;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player2".to_string(),
                    "A. Striker".to_string(),
                    "Alex Striker".to_string(),
                    "1996-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Striker,
                    domain::player::PlayerAttributes {
                        pace: 72,
                        stamina: 68,
                        strength: 70,
                        agility: 71,
                        passing: 60,
                        shooting: 79,
                        tackling: 34,
                        dribbling: 73,
                        defending: 28,
                        positioning: 74,
                        vision: 62,
                        decisions: 68,
                        composure: 69,
                        aggression: 52,
                        teamwork: 64,
                        leadership: 47,
                        handling: 18,
                        reflexes: 18,
                        aerial: 61,
                    },
                );
                player.team_id = Some("team1".to_string());
                player.ovr = 74;
                player.potential = 80;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player3".to_string(),
                    "B. Keeper".to_string(),
                    "Ben Keeper".to_string(),
                    "1993-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Goalkeeper,
                    domain::player::PlayerAttributes {
                        pace: 47,
                        stamina: 61,
                        strength: 63,
                        agility: 65,
                        passing: 49,
                        shooting: 19,
                        tackling: 18,
                        dribbling: 30,
                        defending: 23,
                        positioning: 67,
                        vision: 47,
                        decisions: 62,
                        composure: 60,
                        aggression: 39,
                        teamwork: 63,
                        leadership: 57,
                        handling: 75,
                        reflexes: 76,
                        aerial: 71,
                    },
                );
                player.team_id = Some("team2".to_string());
                player.ovr = 67;
                player.potential = 72;
                player
            },
            {
                let mut player = domain::player::Player::new(
                    "player4".to_string(),
                    "B. Striker".to_string(),
                    "Ben Striker".to_string(),
                    "1995-01-01".to_string(),
                    "England".to_string(),
                    domain::player::Position::Striker,
                    domain::player::PlayerAttributes {
                        pace: 71,
                        stamina: 67,
                        strength: 69,
                        agility: 70,
                        passing: 59,
                        shooting: 78,
                        tackling: 33,
                        dribbling: 72,
                        defending: 27,
                        positioning: 73,
                        vision: 61,
                        decisions: 67,
                        composure: 68,
                        aggression: 51,
                        teamwork: 63,
                        leadership: 46,
                        handling: 18,
                        reflexes: 18,
                        aerial: 60,
                    },
                );
                player.team_id = Some("team2".to_string());
                player.ovr = 73;
                player.potential = 79;
                player
            },
        ];
        let mut game = Game::new(clock, manager, teams, players, staff, vec![]);

        ofm_core::history_generation::generate_past_world_history(
            &mut game,
            2032,
            DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        );

        assert!(game.teams.iter().all(|team| team.history.len() == 12));
        assert_eq!(game.world_history.season_awards.len(), 12);
        assert!(game.players.iter().any(|player| player.career.len() == 12));
        assert!(game
            .managers
            .iter()
            .any(|manager| !manager.career_history.is_empty()));
    }

    /// A new-game request for a world read from `world_file`, as the screens would send it: a 2032
    /// mid-season start (which a historical snapshot overrides with its own date).
    fn request_for_world(
        dob: &str,
        world_file: &std::path::Path,
    ) -> crate::commands::game::NewGameRequest {
        crate::commands::game::NewGameRequest {
            first_name: "Alex".to_string(),
            last_name: "Manager".to_string(),
            dob: dob.to_string(),
            nationality: "GB".to_string(),
            startup_options: Some(
                serde_json::from_str(r#"{"startYear":2032,"startPhase":"midSeason"}"#).unwrap(),
            ),
            world_source: Some(world_file.to_string_lossy().into_owned()),
            competition_definitions_json: None,
            package_ids: None,
        }
    }

    /// Given a historical-snapshot world on disk whose career opens on 20 November 2031 (a 2032
    ///       mid-season start would be 29 October 2032), and managers born on 15 December 2001
    ///       (29 on the snapshot date, 30 on the form's date) and on 20 November 2001 (30 on it)
    /// When `start_new_game_internal` starts a career for each
    /// Then the first is refused with minAge and no game is installed, and the second gets his
    ///      career: the age refusal is the command's own, against the clock the world sets.
    #[test]
    fn start_new_game_internal_refuses_a_manager_too_young_on_the_snapshot_clock() {
        let dir = crate::commands::game::testkit::temp_pkg_dir("sng-snapshot");
        let world_file = dir.join("snapshot-world.json");
        std::fs::write(
            &world_file,
            serde_json::to_string(&make_historical_snapshot_world()).unwrap(),
        )
        .unwrap();
        let environment = crate::commands::game::StartEnvironment {
            packages_dir: Err("no packages in this test".to_string()),
            asset_root: None,
            sources: ofm_core::generator::DefinitionSources::embedded_only(),
        };
        let state = ofm_core::state::StateManager::new();

        let refused = crate::commands::game::start_new_game_internal(
            &state,
            request_for_world("2001-12-15", &world_file),
            &environment,
        );
        let nothing_installed = state.get_game(|_| ()).is_none();
        let accepted = crate::commands::game::start_new_game_internal(
            &state,
            request_for_world("2001-11-20", &world_file),
            &environment,
        );
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(
            refused.err().as_deref(),
            Some("be.error.createManager.minAge")
        );
        assert!(nothing_installed);
        assert_eq!(accepted.unwrap().manager.date_of_birth, "2001-11-20");
    }
}
