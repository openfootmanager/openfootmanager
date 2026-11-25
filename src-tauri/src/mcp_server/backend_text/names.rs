//! The names the screen works out before it resolves a message or an article, in English: a
//! country's name, a division's or competition's display name, and the club a season award names.
//! They follow `countryName` and `normaliseNationality` (`src/lib/countries.ts`),
//! `competitionDisplayName` (`src/lib/competitionName.ts`), `storedTeamLabel` (`src/lib/team.ts`)
//! and the normalisers in `src/utils/backendI18n.ts` that call them.
//! `src/utils/backendTextGolden.json` holds the two sides to the same text.
//!
//! The Rust side has no country library. `src/lib/countryNames.en.json` is the English side of the
//! screen's one, written by `englishCountryTable()` and checked against `countryName` and
//! `normaliseNationality` by `src/lib/countryNames.test.ts`; to change it, regenerate it as that
//! test's header says. Only English is carried, because MCP output is English. So a value the
//! screen reads through another language's country name (`argentine`, through the French name) or
//! through an ISO alpha-3 code (`IRL`) is shown as the backend wrote it. The backend writes ISO
//! alpha-2 codes, football identity codes and English names, and an old save its demonyms as they
//! were, and the table covers all of those.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use super::{english, interpolate, resolve, Params};

const COUNTRY_NAMES: &str = include_str!("../../../../src/lib/countryNames.en.json");

/// The key a winner at no club carries, and the English literal older articles stored instead.
const FREE_AGENT_KEY: &str = "common.freeAgent";
const LEGACY_FREE_AGENT_NAME: &str = "Free Agent";

const SEASON_AWARDS: &str = "be.news.seasonAwards.";
const REPORT_COMPETITION_BODY: &str = "be.news.matchReport.reportCompetition.body";
const DIVISION_CHANGE_SUBJECTS: [&str; 2] =
    ["be.msg.promotion.subject", "be.msg.relegation.subject"];

/// Params that hold a country: the screen names these (`COUNTRY_PARAM_KEYS`).
const COUNTRY_PARAMS: [&str; 2] = ["country", "nationality"];

#[derive(Deserialize, Default)]
struct CountryTable {
    /// Every ISO alpha-2 code and football identity code, with its English name.
    codes: HashMap<String, String>,
    /// Lower case; read without regard to the case written.
    aliases: HashMap<String, String>,
    /// As an old save wrote them: read exactly.
    demonyms: HashMap<String, String>,
    /// Lower case English names; read without regard to the case written.
    names: HashMap<String, String>,
}

fn table() -> &'static CountryTable {
    static TABLE: OnceLock<CountryTable> = OnceLock::new();
    TABLE.get_or_init(|| serde_json::from_str(COUNTRY_NAMES).unwrap_or_default())
}

/// A `country` or `nationality` param as the country's English name; any other param as it is.
pub(super) fn country_param(key: &str, value: &str) -> String {
    if !COUNTRY_PARAMS.contains(&key) {
        return value.to_string();
    }
    country_name(value)
}

/// `countryName(value, "en")`: the name of the country `value` stands for, or `value` itself.
fn country_name(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    let code = normalised_nationality(value).to_uppercase();
    match table().codes.get(&code) {
        Some(name) if !name.is_empty() => name.clone(),
        _ => value.to_string(),
    }
}

/// `normaliseNationality(value)`, in the order the screen tries its readings: a code, an alias, a
/// demonym exactly as written, three letters left as they are, then an English country name.
fn normalised_nationality(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let table = table();
    let upper = trimmed.to_uppercase();
    if table.codes.contains_key(&upper) {
        return upper;
    }
    let lower = trimmed.to_lowercase();
    if let Some(code) = table.aliases.get(&lower) {
        return code.clone();
    }
    if let Some(code) = table.demonyms.get(trimmed) {
        return code.clone();
    }
    if trimmed.len() == 3 && trimmed.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return trimmed.to_string();
    }
    match table.names.get(&lower) {
        Some(code) => code.clone(),
        None => trimmed.to_string(),
    }
}

/// What `competitionDisplayName` is given: the stored name and the key it is named from, and the
/// parts a template can read.
struct NamedCompetition<'a> {
    stored: &'a str,
    name_key: &'a str,
    country_id: Option<&'a str>,
    season: Option<String>,
    /// A division's tier, when the notice carries one (a whole number above zero).
    tier: Option<i64>,
    /// `opening` or `closing` for a split-season division; any other phase names nothing.
    phase: Option<&'a str>,
}

/// The names with a tier in them, which a split-season division's phase is appended to.
const TIER_TEMPLATES: [&str; 4] = [
    "tournaments.competitions.league",
    "tournaments.competitions.firstDivision",
    "tournaments.competitions.secondDivision",
    "tournaments.competitions.tierDivision",
];
const PHASE_OPENING: &str = "tournaments.competitions.phaseOpening";
const PHASE_CLOSING: &str = "tournaments.competitions.phaseClosing";

/// `competitionDisplayName` for a name key, in English: the key's text with the season, the
/// country's name and the tier filled in, and the phase of a split-season division after it. A
/// stored name stands when there is no key, when a `...tierDivision` key has no tier to name, or
/// when the locale file has no text for the key.
fn competition_display_name(competition: &NamedCompetition) -> String {
    let NamedCompetition {
        stored,
        name_key,
        country_id,
        season,
        tier,
        phase,
    } = competition;
    if name_key.is_empty() {
        return stored.to_string();
    }
    if name_key.ends_with(".tierDivision") && tier.is_none() {
        return stored.to_string();
    }
    let country = match country_id.filter(|id| !id.is_empty()) {
        Some(id) => english()
            .get(&format!("nations.{}", id.to_lowercase()))
            .cloned()
            .unwrap_or_else(|| id.to_string()),
        None => String::new(),
    };
    let mut params = Params::new();
    params.insert("country".to_string(), country);
    // The screen passes the year to the translation even when there is none, and a value that is
    // not there reads as nothing ("World Cup"), unlike a param the call never names.
    params.insert("year".to_string(), season.clone().unwrap_or_default());
    params.insert(
        "tier".to_string(),
        tier.map(|tier| tier.to_string()).unwrap_or_default(),
    );
    let template = english().get(*name_key).map_or(*stored, String::as_str);
    let name = interpolate(template, &params, false).trim().to_string();
    let phase_key = match *phase {
        Some("opening") => PHASE_OPENING,
        Some("closing") => PHASE_CLOSING,
        _ => return name,
    };
    if !TIER_TEMPLATES.contains(name_key) {
        return name;
    }
    let phase_name = resolve(Some(phase_key), phase_key, &Params::new());
    format!("{name} {phase_name}").trim().to_string()
}

/// JavaScript's `Number(text)` for a param: nothing when the param is not there or is no number,
/// 0 for an empty text.
fn number_of(text: Option<&String>) -> Option<f64> {
    let number: f64 = match text?.trim() {
        "" => 0.0,
        digits => digits.parse().ok()?,
    };
    number.is_finite().then_some(number)
}

/// `Number(text)` kept when it is finite, written as `String(number)` would.
fn season_year(text: Option<&String>) -> Option<String> {
    let number = number_of(text)?;
    Some(if number.fract() == 0.0 && number.abs() < 1e15 {
        format!("{}", number as i64)
    } else {
        number.to_string()
    })
}

/// A tier the notice carries: a whole number above zero, else none.
fn division_tier(text: Option<&String>) -> Option<i64> {
    let number = number_of(text)?;
    (number.fract() == 0.0 && number > 0.0 && number < 1e15).then_some(number as i64)
}

/// The phase of a split-season division, when the notice names one the screen knows.
fn division_phase(text: Option<&String>) -> Option<&str> {
    text.map(String::as_str)
        .filter(|phase| ["annual", "opening", "closing"].contains(phase))
}

/// A promotion or relegation names the division the club moved to by its name key, country and
/// season (`normalizeDivisionName`); without a key the stored name stands.
pub(super) fn division_change_params(subject_key: Option<&str>, params: &Params) -> Params {
    let mut named = params.clone();
    let is_division_change = subject_key.is_some_and(|key| DIVISION_CHANGE_SUBJECTS.contains(&key));
    let Some(name_key) = params.get("divisionKey").filter(|key| !key.is_empty()) else {
        return named;
    };
    if !is_division_change {
        return named;
    }
    let stored = params.get("division").map_or("", String::as_str);
    named.insert(
        "division".to_string(),
        competition_display_name(&NamedCompetition {
            stored,
            name_key,
            country_id: params.get("divisionCountry").map(String::as_str),
            season: season_year(params.get("divisionSeason")),
            tier: division_tier(params.get("divisionTier")),
            phase: division_phase(params.get("divisionPhase")),
        }),
    );
    named
}

/// A competition's match report names its competition the same way (`normalizeReportCompetitionName`).
pub(super) fn report_competition(body_key: &str, params: &Params) -> Option<Params> {
    if body_key != REPORT_COMPETITION_BODY {
        return None;
    }
    let name_key = params.get("competitionKey").filter(|key| !key.is_empty())?;
    let stored = params.get("competition").map_or("", String::as_str);
    let name = competition_display_name(&NamedCompetition {
        stored,
        name_key,
        country_id: params.get("competitionCountry").map(String::as_str),
        season: season_year(params.get("competitionSeason")),
        tier: None,
        phase: None,
    });
    Some(Params::from([("competition".to_string(), name)]))
}

/// A season-awards article names its winners' clubs by param; a winner at no club (nothing, the
/// legacy literal, or the key itself) becomes the free-agent key, which the params pass then turns
/// into words (`normalizeSeasonAwardsParams`, `storedTeamLabel`).
pub(super) fn season_awards_teams(body_key: &str, params: &Params) -> Option<Params> {
    if !body_key.starts_with(SEASON_AWARDS) {
        return None;
    }
    let teams: Params = ["goldenBootTeam", "potyTeam"]
        .into_iter()
        .filter_map(|name| {
            let stored = params.get(name)?.trim();
            let no_club =
                stored.is_empty() || stored == LEGACY_FREE_AGENT_NAME || stored == FREE_AGENT_KEY;
            let label = if no_club { FREE_AGENT_KEY } else { stored };
            Some((name.to_string(), label.to_string()))
        })
        .collect();
    Some(teams)
}
