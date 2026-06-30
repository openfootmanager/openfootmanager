//! Career-start configuration: startup-options parsing/validation and the date
//! math that anchors a new or loaded game to its season. Unit-tested in
//! `game::tests`.

use chrono::{Datelike, Duration, TimeZone, Utc};
use ofm_core::game::Game;

pub(crate) const DEFAULT_GENERATED_HISTORY_DEPTH_YEARS: u32 = 12;
pub(crate) const MAX_GENERATED_HISTORY_DEPTH_YEARS: u32 = 24;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawStartupOptions {
    #[serde(default)]
    pub(crate) start_year: Option<i32>,
    #[serde(default)]
    pub(crate) start_phase: Option<String>,
    #[serde(default)]
    pub(crate) history_depth_years: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StartPhase {
    SeasonStart,
    MidSeason,
}

impl StartPhase {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "seasonStart" => Some(Self::SeasonStart),
            "midSeason" => Some(Self::MidSeason),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SeasonStart => "seasonStart",
            Self::MidSeason => "midSeason",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartupOptions {
    pub(crate) start_year: i32,
    pub(crate) start_phase: StartPhase,
    pub(crate) history_depth_years: u32,
}

fn default_start_year() -> i32 {
    chrono::Utc::now().year().max(2020)
}

fn default_history_depth_years() -> u32 {
    DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
}

pub(crate) fn start_date_for_year(start_year: i32) -> Result<chrono::DateTime<Utc>, String> {
    // Use June 1 in World Cup years so a fresh career opens just before the
    // tournament, keeping the WC in June rather than scheduling it in July.
    let month = if ofm_core::world_cup::is_world_cup_summer(start_year) {
        6
    } else {
        7
    };
    Utc.with_ymd_and_hms(start_year, month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| "be.error.createManager.invalidStartYear".to_string())
}

pub(crate) fn current_date_for_phase(
    start_year: i32,
    start_phase: StartPhase,
) -> Result<chrono::DateTime<Utc>, String> {
    let start_date = start_date_for_year(start_year)?;
    Ok(match start_phase {
        StartPhase::SeasonStart => start_date,
        StartPhase::MidSeason => start_date + Duration::days(120),
    })
}

pub(crate) fn age_on_date(birth_date: chrono::NaiveDate, reference_date: chrono::NaiveDate) -> i64 {
    let mut age = i64::from(reference_date.year() - birth_date.year());
    let has_had_birthday =
        (reference_date.month(), reference_date.day()) >= (birth_date.month(), birth_date.day());
    if !has_had_birthday {
        age -= 1;
    }
    age
}

pub(crate) fn start_phase_for_game(game: &Game) -> StartPhase {
    if game.clock.current_date > game.clock.start_date {
        StartPhase::MidSeason
    } else {
        StartPhase::SeasonStart
    }
}

pub(crate) fn normalize_startup_options(
    raw: Option<RawStartupOptions>,
) -> Result<StartupOptions, String> {
    let raw = raw.unwrap_or_default();
    let start_year = raw.start_year.unwrap_or_else(default_start_year);
    if start_year < 2020 {
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
