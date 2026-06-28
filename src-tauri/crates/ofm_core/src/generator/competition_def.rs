//! Authoring format for user-defined competitions.
//!
//! A `CompetitionDefinitionFile` describes leagues, cups, and tournaments that
//! the game resolves into runnable competitions. Definitions can ship inside a
//! world package or be imported as standalone files at new-game time. They are
//! validated strictly: an invalid file is rejected with a structured list of
//! errors so modders get precise, localizable feedback.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use domain::league::{
    CompetitionFormat, CompetitionScope, CompetitionType, FixtureCompetition, League, StandingEntry,
};
use domain::team::Team;

// Qualification berths live in `domain` so the runtime `League` can carry them;
// re-export here so authored definitions and `ofm_core::generator` consumers
// keep referring to them through this module.
pub use domain::league::{Berth, BerthRule};

/// A bundle of competition definitions, as authored in a JSON file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionDefinitionFile {
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    #[serde(default)]
    pub competitions: Vec<CompetitionDefinition>,
}

fn default_format_version() -> u32 {
    1
}

/// The highest schema version this build understands.
pub const SUPPORTED_DEFINITION_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionDefinition {
    pub id: String,
    pub name: String,
    pub r#type: CompetitionType,
    pub scope: CompetitionScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_region_ids: Vec<String>,
    #[serde(default)]
    pub priority: u32,
    pub format: FormatDef,
    pub participants: ParticipantSpec,
    /// Qualification berths this competition awards into other competitions,
    /// evaluated from the season's real results at rollover (Phase C).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub berths: Vec<Berth>,
    /// Calendar month the season starts (1–12). Defaults to 8 (August).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season_start_month: Option<u8>,
    /// Day of month the season starts (1–31). Defaults to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season_start_day: Option<u8>,
    /// Optional i18n key for the competition name. When set, the frontend
    /// translates via `t(nameKey, { year })` instead of displaying `name` raw.
    /// Package authors can set this to a built-in key or to a custom key whose
    /// translation is provided in the package's `translations.{locale}.json` file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_key: Option<String>,
    /// Optional path to a logo/badge image, relative to the package root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
}

/// Format-specific configuration. `kind` selects the shape; the other fields
/// apply only to the relevant kinds and otherwise fall back to sensible
/// defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatDef {
    pub kind: CompetitionFormat,
    /// Round-robin legs (LeagueTable / GroupAndKnockout groups). Default 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legs: Option<u8>,
    /// Clubs per group (GroupAndKnockout). Default 4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_size: Option<u32>,
    /// Clubs advancing per group (GroupAndKnockout). Default 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualifiers_per_group: Option<u32>,
    /// Best next-placed finishers that also advance (GroupAndKnockout).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_third_qualifiers: Option<u32>,
}

/// How a competition's participants are chosen. Exactly one variant must be
/// supplied.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantSpec {
    /// Explicit team ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explicit: Option<Vec<String>>,
    /// A rule that resolves to team ids against the world.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<SelectorSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectorSpec {
    pub kind: SelectorKind,
    /// Country code (for country-scoped selectors).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Region id (for region-scoped selectors).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Maximum teams to take (for ranked / capped selectors).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// Other competition ids whose participants are excluded (e.g. a second
    /// division excludes the clubs already placed in the first).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_competitions: Vec<String>,
    /// Competition id this selector draws champions/qualifiers from
    /// (`championsOf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_competition: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectorKind {
    /// Strongest `count` clubs of a country by reputation.
    TopByReputation,
    /// Every club of a country.
    AllInCountry,
    /// Every club of a region.
    AllInRegion,
    /// Top `count` finishers of another competition (continental qualification).
    ChampionsOf,
}

/// A single validation problem. `code` is an i18n key; `params` fills its
/// placeholders. `competition_id` locates the offending entry (empty for
/// file-level problems).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionError {
    pub code: String,
    pub competition_id: String,
    pub params: Vec<(String, String)>,
}

impl DefinitionError {
    fn new(code: &str, competition_id: &str) -> Self {
        Self {
            code: code.to_string(),
            competition_id: competition_id.to_string(),
            params: Vec::new(),
        }
    }

    fn with(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.push((key.to_string(), value.into()));
        self
    }
}

/// The world facts a definition is validated against.
pub struct WorldValidationContext<'a> {
    pub team_ids: HashSet<&'a str>,
    pub country_codes: HashSet<&'a str>,
    pub region_ids: HashSet<&'a str>,
}

impl<'a> WorldValidationContext<'a> {
    pub fn from_world(world: &'a super::WorldData) -> Self {
        let team_ids = world.teams.iter().map(|team| team.id.as_str()).collect();
        let mut country_codes: HashSet<&'a str> = world
            .teams
            .iter()
            .map(|team| {
                if team.football_nation.is_empty() {
                    team.country.as_str()
                } else {
                    team.football_nation.as_str()
                }
            })
            .collect();
        // Include builtin nations so competition selectors that reference them
        // pass validation here, matching what validate_competition_references
        // uses at build/install time.
        for nation in crate::nations::NATION_CATALOG {
            country_codes.insert(nation.code);
        }
        let mut region_ids: HashSet<&'a str> = world
            .regions
            .iter()
            .map(|region| region.id.as_str())
            .collect();
        for nation in crate::nations::NATION_CATALOG {
            region_ids.insert(nation.region_id);
        }
        Self {
            team_ids,
            country_codes,
            region_ids,
        }
    }
}

/// Validate a definition file against the world. Returns every problem found
/// (not just the first), so the UI can present a complete list.
pub fn validate_definitions(
    file: &CompetitionDefinitionFile,
    ctx: &WorldValidationContext,
) -> Vec<DefinitionError> {
    let mut errors = Vec::new();

    if file.format_version > SUPPORTED_DEFINITION_FORMAT_VERSION {
        errors.push(
            DefinitionError::new("be.error.competitionDef.unsupportedVersion", "")
                .with("version", file.format_version.to_string())
                .with("supported", SUPPORTED_DEFINITION_FORMAT_VERSION.to_string()),
        );
    }

    let known_ids: HashSet<&str> = file.competitions.iter().map(|c| c.id.as_str()).collect();
    let mut seen_ids: HashSet<&str> = HashSet::new();

    for competition in &file.competitions {
        let id = competition.id.as_str();
        if id.is_empty() {
            errors.push(DefinitionError::new("be.error.competitionDef.emptyId", ""));
        } else if !seen_ids.insert(id) {
            errors.push(
                DefinitionError::new("be.error.competitionDef.duplicateId", id).with("id", id),
            );
        }
        if competition.name.trim().is_empty() {
            errors.push(DefinitionError::new(
                "be.error.competitionDef.emptyName",
                id,
            ));
        }

        validate_region_and_country(competition, ctx, &mut errors);
        validate_format(competition, &mut errors);
        validate_participants(competition, ctx, &known_ids, &mut errors);
        validate_berths(competition, &known_ids, &mut errors);
    }

    detect_selector_cycles(file, &mut errors);
    errors
}

fn validate_berths(
    competition: &CompetitionDefinition,
    known_ids: &HashSet<&str>,
    errors: &mut Vec<DefinitionError>,
) {
    let is_league = competition.format.kind == CompetitionFormat::LeagueTable;
    let is_knockout = matches!(
        competition.format.kind,
        CompetitionFormat::Knockout | CompetitionFormat::GroupAndKnockout
    );

    for berth in &competition.berths {
        check_berth_target(competition, &berth.target, known_ids, errors);
        if let Some(fallback) = &berth.fallback_to {
            check_berth_target(competition, fallback, known_ids, errors);
        }

        match &berth.rule {
            BerthRule::PositionRange { from, to } => {
                validate_range(competition, *from, *to, errors);
                require_league(competition, is_league, errors);
            }
            BerthRule::PlayoffWinner { from, to } => {
                validate_range(competition, *from, *to, errors);
                require_league(competition, is_league, errors);
                if to.saturating_sub(*from) + 1 < 2 {
                    errors.push(DefinitionError::new(
                        "be.error.competitionDef.berthPlayoffTooSmall",
                        &competition.id,
                    ));
                }
            }
            BerthRule::CupWinner => {
                if !is_knockout {
                    errors.push(DefinitionError::new(
                        "be.error.competitionDef.berthRequiresCup",
                        &competition.id,
                    ));
                }
            }
        }
    }
}

fn check_berth_target(
    competition: &CompetitionDefinition,
    target: &str,
    known_ids: &HashSet<&str>,
    errors: &mut Vec<DefinitionError>,
) {
    if !known_ids.contains(target) {
        errors.push(
            DefinitionError::new(
                "be.error.competitionDef.berthUnknownTarget",
                &competition.id,
            )
            .with("target", target.to_string()),
        );
    }
}

fn validate_range(
    competition: &CompetitionDefinition,
    from: u32,
    to: u32,
    errors: &mut Vec<DefinitionError>,
) {
    if from < 1 || to < from {
        errors.push(
            DefinitionError::new("be.error.competitionDef.berthInvalidRange", &competition.id)
                .with("from", from.to_string())
                .with("to", to.to_string()),
        );
    }
}

fn require_league(
    competition: &CompetitionDefinition,
    is_league: bool,
    errors: &mut Vec<DefinitionError>,
) {
    if !is_league {
        errors.push(DefinitionError::new(
            "be.error.competitionDef.berthRequiresLeague",
            &competition.id,
        ));
    }
}

fn validate_region_and_country(
    competition: &CompetitionDefinition,
    ctx: &WorldValidationContext,
    errors: &mut Vec<DefinitionError>,
) {
    if let Some(region) = &competition.region_id
        && !ctx.region_ids.contains(region.as_str())
    {
        errors.push(
            DefinitionError::new("be.error.competitionDef.unknownRegion", &competition.id)
                .with("region", region.clone()),
        );
    }
    if let Some(country) = &competition.country_id
        && !ctx.country_codes.contains(country.as_str())
    {
        errors.push(
            DefinitionError::new("be.error.competitionDef.unknownCountry", &competition.id)
                .with("country", country.clone()),
        );
    }
    for region in &competition.required_region_ids {
        if !ctx.region_ids.contains(region.as_str()) {
            errors.push(
                DefinitionError::new("be.error.competitionDef.unknownRegion", &competition.id)
                    .with("region", region.clone()),
            );
        }
    }
}

fn validate_format(competition: &CompetitionDefinition, errors: &mut Vec<DefinitionError>) {
    let format = &competition.format;
    if format.kind != CompetitionFormat::GroupAndKnockout
        && (format.group_size.is_some()
            || format.qualifiers_per_group.is_some()
            || format.best_third_qualifiers.is_some())
    {
        errors.push(DefinitionError::new(
            "be.error.competitionDef.groupConfigOnNonGroupFormat",
            &competition.id,
        ));
    }
    if let Some(month) = competition.season_start_month
        && !(1..=12).contains(&month)
    {
        errors.push(
            DefinitionError::new(
                "be.error.competitionDef.invalidSeasonMonth",
                &competition.id,
            )
            .with("month", month.to_string()),
        );
    }
    if let Some(day) = competition.season_start_day
        && !(1..=31).contains(&day)
    {
        errors.push(
            DefinitionError::new("be.error.competitionDef.invalidSeasonDay", &competition.id)
                .with("day", day.to_string()),
        );
    }
    // Cross-check month + day using a leap year as probe so Feb 29 is accepted
    // (it clamps to Feb 28 at runtime). This rejects truly impossible dates
    // such as Apr 31 or Feb 30 that pass the independent range checks above.
    if let (Some(month), Some(day)) = (competition.season_start_month, competition.season_start_day)
        && (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && NaiveDate::from_ymd_opt(2000, month as u32, day as u32).is_none()
    {
        errors.push(
            DefinitionError::new("be.error.competitionDef.invalidSeasonDay", &competition.id)
                .with("day", day.to_string()),
        );
    }
    if let Some(group_size) = format.group_size
        && group_size < 2
    {
        errors.push(
            DefinitionError::new("be.error.competitionDef.groupSizeTooSmall", &competition.id)
                .with("groupSize", group_size.to_string()),
        );
    }
    if let Some(legs) = format.legs
        && legs == 0
    {
        errors.push(DefinitionError::new(
            "be.error.competitionDef.zeroLegs",
            &competition.id,
        ));
    }
}

fn validate_participants(
    competition: &CompetitionDefinition,
    ctx: &WorldValidationContext,
    known_ids: &HashSet<&str>,
    errors: &mut Vec<DefinitionError>,
) {
    let spec = &competition.participants;
    match (&spec.explicit, &spec.selector) {
        (None, None) => errors.push(DefinitionError::new(
            "be.error.competitionDef.noParticipants",
            &competition.id,
        )),
        (Some(_), Some(_)) => errors.push(DefinitionError::new(
            "be.error.competitionDef.bothParticipantSources",
            &competition.id,
        )),
        (Some(explicit), None) => {
            if explicit.len() < 2 {
                errors.push(DefinitionError::new(
                    "be.error.competitionDef.tooFewParticipants",
                    &competition.id,
                ));
            }
            for team_id in explicit {
                if !ctx.team_ids.contains(team_id.as_str()) {
                    errors.push(
                        DefinitionError::new(
                            "be.error.competitionDef.unknownTeam",
                            &competition.id,
                        )
                        .with("team", team_id.clone()),
                    );
                }
            }
        }
        (None, Some(selector)) => {
            validate_selector(competition, selector, ctx, known_ids, errors);
        }
    }
}

fn validate_selector(
    competition: &CompetitionDefinition,
    selector: &SelectorSpec,
    ctx: &WorldValidationContext,
    known_ids: &HashSet<&str>,
    errors: &mut Vec<DefinitionError>,
) {
    let require_country = |errors: &mut Vec<DefinitionError>| match &selector.country {
        Some(country) if ctx.country_codes.contains(country.as_str()) => {}
        Some(country) => errors.push(
            DefinitionError::new("be.error.competitionDef.unknownCountry", &competition.id)
                .with("country", country.clone()),
        ),
        None => errors.push(DefinitionError::new(
            "be.error.competitionDef.selectorMissingCountry",
            &competition.id,
        )),
    };

    match selector.kind {
        SelectorKind::TopByReputation => {
            require_country(errors);
            if selector.count.unwrap_or(0) < 2 {
                errors.push(DefinitionError::new(
                    "be.error.competitionDef.selectorCountTooSmall",
                    &competition.id,
                ));
            }
        }
        SelectorKind::AllInCountry => require_country(errors),
        SelectorKind::AllInRegion => match &selector.region {
            Some(region) if ctx.region_ids.contains(region.as_str()) => {}
            Some(region) => errors.push(
                DefinitionError::new("be.error.competitionDef.unknownRegion", &competition.id)
                    .with("region", region.clone()),
            ),
            None => errors.push(DefinitionError::new(
                "be.error.competitionDef.selectorMissingRegion",
                &competition.id,
            )),
        },
        SelectorKind::ChampionsOf => match &selector.source_competition {
            Some(source) if known_ids.contains(source.as_str()) => {}
            Some(source) => errors.push(
                DefinitionError::new(
                    "be.error.competitionDef.unknownSourceCompetition",
                    &competition.id,
                )
                .with("source", source.clone()),
            ),
            None => errors.push(DefinitionError::new(
                "be.error.competitionDef.selectorMissingSource",
                &competition.id,
            )),
        },
    }

    for excluded in &selector.exclude_competitions {
        if !known_ids.contains(excluded.as_str()) {
            errors.push(
                DefinitionError::new(
                    "be.error.competitionDef.unknownExcludedCompetition",
                    &competition.id,
                )
                .with("excluded", excluded.clone()),
            );
        }
    }
}

/// `championsOf` selectors create dependencies between competitions; a cycle
/// would make resolution impossible.
fn detect_selector_cycles(file: &CompetitionDefinitionFile, errors: &mut Vec<DefinitionError>) {
    let mut dependencies: HashMap<&str, &str> = HashMap::new();
    for competition in &file.competitions {
        if let Some(selector) = &competition.participants.selector
            && selector.kind == SelectorKind::ChampionsOf
            && let Some(source) = &selector.source_competition
        {
            dependencies.insert(competition.id.as_str(), source.as_str());
        }
    }

    for start in dependencies.keys().copied() {
        let mut visited: HashSet<&str> = HashSet::new();
        let mut current = start;
        while let Some(&next) = dependencies.get(current) {
            if next == start {
                errors.push(
                    DefinitionError::new("be.error.competitionDef.selectorCycle", start)
                        .with("competition", start),
                );
                break;
            }
            if !visited.insert(next) {
                break;
            }
            current = next;
        }
    }
}

// ---------------------------------------------------------------------------
// Season date helpers
// ---------------------------------------------------------------------------

/// Converts (year, month, day) to midnight UTC, clamping `day` down to the
/// last valid day of the month when necessary (e.g. Feb 29 in a non-leap year
/// becomes Feb 28). Month and day are also clamped into their valid ranges so
/// this function is infallible even for DB-sourced values.
fn date_utc(year: i32, month: u8, day: u8) -> DateTime<Utc> {
    let m = month.clamp(1, 12);
    let d = day.clamp(1, 31);
    for candidate in (1..=d).rev() {
        if let Some(naive) = NaiveDate::from_ymd_opt(year, m as u32, candidate as u32) {
            return naive.and_hms_opt(0, 0, 0).unwrap().and_utc();
        }
    }
    // m is 1..=12 after clamp, so day 1 always exists.
    NaiveDate::from_ymd_opt(year, m as u32, 1)
        .expect("month is 1–12")
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
}

/// At game-open: returns the competition's start date for the first season and
/// whether it is already mid-season at that point (i.e. the competition's
/// start date falls before `game_start`).
pub fn start_date_at_game_open(
    game_start: DateTime<Utc>,
    month: u8,
    day: u8,
) -> (DateTime<Utc>, bool) {
    // A late-year management anchor belongs to the following calendar season,
    // so Jan–Mar competitions use their next occurrence.
    let year = if game_start.month() >= 10 && month <= 3 {
        game_start.year() + 1
    } else {
        game_start.year()
    };
    let date = date_utc(year, month, day);
    let is_mid_season = date <= game_start;
    (date, is_mid_season)
}

/// At rollover: the next calendar occurrence of (month, day) strictly after
/// `after`. Used to compute each competition's own next-season start without a
/// single global rollover anchor.
pub fn next_season_start(after: DateTime<Utc>, month: u8, day: u8) -> DateTime<Utc> {
    let same_year = date_utc(after.year(), month, day);
    if same_year >= after {
        same_year
    } else {
        date_utc(after.year() + 1, month, day)
    }
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

fn fixture_competition_for(kind: &CompetitionType) -> FixtureCompetition {
    match kind {
        CompetitionType::ContinentalClub => FixtureCompetition::ContinentalClub,
        CompetitionType::InternationalClub => FixtureCompetition::InternationalClub,
        CompetitionType::InternationalNation => FixtureCompetition::InternationalNation,
        CompetitionType::FriendlyCup => FixtureCompetition::FriendlyCup,
        CompetitionType::League => FixtureCompetition::League,
        CompetitionType::Cup => FixtureCompetition::Cup,
    }
}

fn team_country(team: &Team) -> &str {
    if team.football_nation.is_empty() {
        team.country.as_str()
    } else {
        team.football_nation.as_str()
    }
}

/// Country code -> region id, taken from the world's region catalog.
fn country_to_region(world: &super::WorldData) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for region in &world.regions {
        for country in &region.country_codes {
            map.insert(country.clone(), region.id.clone());
        }
    }
    map
}

/// Rewrite a competition's id everywhere it is referenced, so a generated
/// competition can adopt its definition's stable id.
fn reassign_competition_id(competition: &mut League, new_id: &str) {
    let old = competition.id.clone();
    competition.id = new_id.to_string();
    for fixture in &mut competition.fixtures {
        fixture.competition_id = new_id.to_string();
    }
    for round in &mut competition.knockout_rounds {
        round.id = round.id.replace(&old, new_id);
    }
    for group in &mut competition.groups {
        group.id = group.id.replace(&old, new_id);
    }
}

/// The selector dependencies of a definition (other competitions it reads).
fn dependency_ids(def: &CompetitionDefinition) -> Vec<String> {
    let mut deps = Vec::new();
    if let Some(selector) = &def.participants.selector {
        if let Some(source) = &selector.source_competition {
            deps.push(source.clone());
        }
        deps.extend(selector.exclude_competitions.iter().cloned());
    }
    deps
}

fn resolve_participants(
    def: &CompetitionDefinition,
    world: &super::WorldData,
    region_by_country: &HashMap<String, String>,
    resolved: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let spec = &def.participants;
    if let Some(explicit) = &spec.explicit {
        return explicit.clone();
    }
    let Some(selector) = &spec.selector else {
        return Vec::new();
    };

    let excluded: HashSet<String> = selector
        .exclude_competitions
        .iter()
        .flat_map(|id| resolved.get(id).cloned().unwrap_or_default())
        .collect();

    let take = selector.count.map(|c| c as usize);

    match selector.kind {
        SelectorKind::TopByReputation => {
            let country = selector.country.clone().unwrap_or_default();
            let mut clubs: Vec<&Team> = world
                .teams
                .iter()
                .filter(|team| team_country(team) == country)
                .filter(|team| !excluded.contains(&team.id))
                .collect();
            clubs.sort_by(|a, b| b.reputation.cmp(&a.reputation).then(a.id.cmp(&b.id)));
            clubs
                .into_iter()
                .take(take.unwrap_or(usize::MAX))
                .map(|team| team.id.clone())
                .collect()
        }
        SelectorKind::AllInCountry => {
            let country = selector.country.clone().unwrap_or_default();
            world
                .teams
                .iter()
                .filter(|team| team_country(team) == country)
                .filter(|team| !excluded.contains(&team.id))
                .map(|team| team.id.clone())
                .collect()
        }
        SelectorKind::AllInRegion => {
            let region = selector.region.clone().unwrap_or_default();
            world
                .teams
                .iter()
                .filter(|team| {
                    region_by_country
                        .get(team_country(team))
                        .is_some_and(|r| r == &region)
                })
                .filter(|team| !excluded.contains(&team.id))
                .map(|team| team.id.clone())
                .collect()
        }
        SelectorKind::ChampionsOf => {
            // At world creation there is no prior season, so qualification is
            // seeded by the source competition's strongest clubs. Once seasons
            // are played, continental fields are re-qualified from real domestic
            // standings at rollover (see
            // `end_of_season::continental_qualified_entrants`).
            let source = selector.source_competition.clone().unwrap_or_default();
            let mut clubs: Vec<&Team> = resolved
                .get(&source)
                .map(|ids| {
                    ids.iter()
                        .filter_map(|id| world.teams.iter().find(|team| &team.id == id))
                        .collect()
                })
                .unwrap_or_default();
            clubs.sort_by(|a, b| b.reputation.cmp(&a.reputation).then(a.id.cmp(&b.id)));
            clubs
                .into_iter()
                .take(take.unwrap_or(usize::MAX))
                .map(|team| team.id.clone())
                .collect()
        }
    }
}

fn build_competition(
    def: &CompetitionDefinition,
    team_ids: &[String],
    season: u32,
    season_start: DateTime<Utc>,
) -> Option<League> {
    if team_ids.len() < 2 {
        return None;
    }
    let fixture_competition = fixture_competition_for(&def.r#type);

    let mut competition = match def.format.kind {
        CompetitionFormat::LeagueTable => {
            let mut league = League::new(def.id.clone(), def.name.clone(), season, team_ids);
            league.fixtures = crate::schedule::build_round_robin_fixtures_with(
                &def.id,
                team_ids,
                season_start,
                fixture_competition,
                def.format.legs.unwrap_or(2),
                7,
            );
            league
        }
        CompetitionFormat::Knockout => {
            let mut cup = League::new(def.id.clone(), def.name.clone(), season, team_ids);
            cup.standings.clear();
            cup.rules.format = CompetitionFormat::Knockout;
            crate::schedule::seed_knockout_round(
                &mut cup,
                team_ids,
                season_start,
                fixture_competition,
            );
            cup
        }
        CompetitionFormat::GroupAndKnockout => {
            let config = crate::group_stage::GroupStageConfig {
                legs: def.format.legs.unwrap_or(2),
                matchday_gap_days: 7,
                qualifiers_per_group: def.format.qualifiers_per_group.unwrap_or(2),
                best_third_qualifiers: def.format.best_third_qualifiers.unwrap_or(0),
                ..Default::default()
            };
            let mut cup = crate::group_stage::generate_group_knockout_cup_with(
                &def.name,
                season,
                team_ids,
                season_start,
                def.r#type.clone(),
                def.scope.clone(),
                &config,
            );
            reassign_competition_id(&mut cup, &def.id);
            cup
        }
    };

    competition.kind = def.r#type.clone();
    competition.scope = def.scope.clone();
    competition.region_id = def.region_id.clone();
    competition.country_id = def.country_id.clone();
    competition.required_region_ids = def.required_region_ids.clone();
    competition.participant_ids = team_ids.to_vec();
    competition.priority = def.priority;
    competition.berths = def.berths.clone();
    competition.season_start_month = def.season_start_month.unwrap_or(8);
    competition.season_start_day = def.season_start_day.unwrap_or(1);
    competition.name_key = def.name_key.clone();
    // Rebuild standings to match the resolved participants for table formats.
    if def.format.kind == CompetitionFormat::LeagueTable {
        competition.standings = team_ids
            .iter()
            .map(|id| StandingEntry::new(id.clone()))
            .collect();
    }
    Some(competition)
}

/// Turn a validated definition file into runnable competitions. Selectors are
/// resolved against the world (in dependency order), and competitions whose
/// participant list comes out below two clubs are skipped. Call only after
/// [`validate_definitions`] has returned no errors.
///
/// `game_start` is the game's anchor date (July 1 of the chosen start year).
/// Each competition derives its own season-start date from its
/// `season_start_month`/`season_start_day` fields.
pub fn resolve_definitions(
    file: &CompetitionDefinitionFile,
    world: &super::WorldData,
    season: u32,
    game_start: DateTime<Utc>,
) -> Vec<League> {
    let region_by_country = country_to_region(world);

    // Resolve participant lists in dependency order (selectors that read other
    // competitions resolve after their sources). Validation has already ruled
    // out cycles, so this terminates.
    let mut resolved: HashMap<String, Vec<String>> = HashMap::new();
    let mut pending: Vec<&CompetitionDefinition> = file.competitions.iter().collect();
    loop {
        let mut progressed = false;
        pending.retain(|def| {
            let ready = dependency_ids(def)
                .iter()
                .all(|dep| resolved.contains_key(dep));
            if ready {
                let ids = resolve_participants(def, world, &region_by_country, &resolved);
                resolved.insert(def.id.clone(), ids);
                progressed = true;
                false
            } else {
                true
            }
        });
        if pending.is_empty() || !progressed {
            break;
        }
    }
    // Any unresolved (e.g. a dependency pointing outside the file) resolve
    // ignoring the missing dependency, so nothing is silently dropped.
    for def in pending {
        let ids = resolve_participants(def, world, &region_by_country, &resolved);
        resolved.insert(def.id.clone(), ids);
    }

    // Build in authoring order so priorities line up predictably.
    file.competitions
        .iter()
        .filter_map(|def| {
            let team_ids = resolved.get(&def.id).cloned().unwrap_or_default();
            let month = def.season_start_month.unwrap_or(8);
            let day = def.season_start_day.unwrap_or(1);
            let (comp_start, _) = start_date_at_game_open(game_start, month, day);
            build_competition(def, &team_ids, season, comp_start)
        })
        .collect()
}

/// Build a single competition from a definition whose participants are listed
/// explicitly (no selectors), at a caller-chosen start date. This is the
/// construction path the built-in foundation world uses so generated and
/// imported competitions flow through the same [`build_competition`] core; the
/// per-call start lets built-ins keep their staggered calendar. Returns `None`
/// when the explicit list has fewer than two clubs.
pub fn build_explicit_competition(
    def: &CompetitionDefinition,
    season: u32,
    season_start: DateTime<Utc>,
) -> Option<League> {
    let team_ids = def.participants.explicit.clone().unwrap_or_default();
    build_competition(def, &team_ids, season, season_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> WorldValidationContext<'static> {
        WorldValidationContext {
            team_ids: ["team-a", "team-b", "team-c"].into_iter().collect(),
            country_codes: ["TR", "ENG"].into_iter().collect(),
            region_ids: ["europe", "asia"].into_iter().collect(),
        }
    }

    fn explicit(id: &str, teams: &[&str]) -> CompetitionDefinition {
        CompetitionDefinition {
            id: id.to_string(),
            name: format!("{id} name"),
            r#type: CompetitionType::League,
            scope: CompetitionScope::Domestic,
            region_id: None,
            country_id: Some("TR".to_string()),
            required_region_ids: vec![],
            priority: 0,
            format: FormatDef {
                kind: CompetitionFormat::LeagueTable,
                legs: None,
                group_size: None,
                qualifiers_per_group: None,
                best_third_qualifiers: None,
            },
            participants: ParticipantSpec {
                explicit: Some(teams.iter().map(|t| t.to_string()).collect()),
                selector: None,
            },
            berths: Vec::new(),
            season_start_month: None,
            season_start_day: None,
            name_key: None,
            logo: None,
        }
    }

    fn codes(errors: &[DefinitionError]) -> Vec<&str> {
        errors.iter().map(|e| e.code.as_str()).collect()
    }

    #[test]
    fn a_valid_explicit_league_passes() {
        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![explicit("tr-1", &["team-a", "team-b"])],
        };
        assert!(validate_definitions(&file, &ctx()).is_empty());
    }

    fn berth(target: &str, rule: BerthRule) -> Berth {
        Berth {
            target: target.to_string(),
            rule,
            fallback_to: None,
        }
    }

    #[test]
    fn valid_berths_pass() {
        let mut league = explicit("tr-1", &["team-a", "team-b"]);
        league.berths = vec![
            berth("ucl", BerthRule::PositionRange { from: 1, to: 2 }),
            berth("ucl", BerthRule::PlayoffWinner { from: 1, to: 2 }),
        ];
        let mut cup = explicit("tr-cup", &["team-a", "team-b"]);
        cup.format.kind = CompetitionFormat::Knockout;
        cup.berths = vec![berth("ucl", BerthRule::CupWinner)];
        let ucl = explicit("ucl", &["team-a", "team-b"]);

        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![league, cup, ucl],
        };
        assert!(validate_definitions(&file, &ctx()).is_empty());
    }

    #[test]
    fn berth_unknown_target_and_invalid_range_are_reported() {
        let mut league = explicit("tr-1", &["team-a", "team-b"]);
        league.berths = vec![
            berth("ghost", BerthRule::PositionRange { from: 0, to: 2 }),
            Berth {
                target: "tr-1".to_string(),
                rule: BerthRule::PositionRange { from: 1, to: 2 },
                fallback_to: Some("also-ghost".to_string()),
            },
        ];
        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![league],
        };
        let errors = validate_definitions(&file, &ctx());
        let reported = codes(&errors);
        assert!(reported.contains(&"be.error.competitionDef.berthUnknownTarget"));
        assert!(reported.contains(&"be.error.competitionDef.berthInvalidRange"));
    }

    #[test]
    fn berth_rule_must_match_host_format() {
        // cupWinner on a league table, and a position range on a knockout cup.
        let mut league = explicit("tr-1", &["team-a", "team-b"]);
        league.berths = vec![berth("tr-1", BerthRule::CupWinner)];
        let mut cup = explicit("tr-cup", &["team-a", "team-b"]);
        cup.format.kind = CompetitionFormat::Knockout;
        cup.berths = vec![berth("tr-cup", BerthRule::PositionRange { from: 1, to: 2 })];

        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![league, cup],
        };
        let errors = validate_definitions(&file, &ctx());
        let reported = codes(&errors);
        assert!(reported.contains(&"be.error.competitionDef.berthRequiresCup"));
        assert!(reported.contains(&"be.error.competitionDef.berthRequiresLeague"));
    }

    #[test]
    fn berth_playoff_needs_at_least_two_contestants() {
        let mut league = explicit("tr-1", &["team-a", "team-b"]);
        league.berths = vec![berth("tr-1", BerthRule::PlayoffWinner { from: 5, to: 5 })];
        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![league],
        };
        let errors = validate_definitions(&file, &ctx());
        let reported = codes(&errors);
        assert!(reported.contains(&"be.error.competitionDef.berthPlayoffTooSmall"));
    }

    #[test]
    fn unknown_team_country_and_region_are_reported() {
        let mut def = explicit("tr-1", &["team-a", "ghost"]);
        def.country_id = Some("XX".to_string());
        def.region_id = Some("atlantis".to_string());
        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![def],
        };
        let errors = validate_definitions(&file, &ctx());
        assert!(codes(&errors).contains(&"be.error.competitionDef.unknownTeam"));
        assert!(codes(&errors).contains(&"be.error.competitionDef.unknownCountry"));
        assert!(codes(&errors).contains(&"be.error.competitionDef.unknownRegion"));
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![
                explicit("tr-1", &["team-a", "team-b"]),
                explicit("tr-1", &["team-a", "team-c"]),
            ],
        };
        let errors = validate_definitions(&file, &ctx());
        assert!(codes(&errors).contains(&"be.error.competitionDef.duplicateId"));
    }

    #[test]
    fn participants_must_be_exactly_one_source() {
        let mut none = explicit("tr-1", &["team-a", "team-b"]);
        none.participants = ParticipantSpec::default();
        let mut both = explicit("tr-2", &["team-a", "team-b"]);
        both.participants.selector = Some(SelectorSpec {
            kind: SelectorKind::AllInCountry,
            country: Some("TR".to_string()),
            region: None,
            count: None,
            exclude_competitions: vec![],
            source_competition: None,
        });
        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![none, both],
        };
        let errors = validate_definitions(&file, &ctx());
        let codes = codes(&errors);
        assert!(codes.contains(&"be.error.competitionDef.noParticipants"));
        assert!(codes.contains(&"be.error.competitionDef.bothParticipantSources"));
    }

    #[test]
    fn top_by_reputation_needs_a_known_country_and_count() {
        let mut def = explicit("tr-1", &[]);
        def.participants = ParticipantSpec {
            explicit: None,
            selector: Some(SelectorSpec {
                kind: SelectorKind::TopByReputation,
                country: Some("XX".to_string()),
                region: None,
                count: Some(1),
                exclude_competitions: vec![],
                source_competition: None,
            }),
        };
        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![def],
        };
        let errors = validate_definitions(&file, &ctx());
        let codes = codes(&errors);
        assert!(codes.contains(&"be.error.competitionDef.unknownCountry"));
        assert!(codes.contains(&"be.error.competitionDef.selectorCountTooSmall"));
    }

    #[test]
    fn champions_of_must_reference_a_known_competition_and_not_cycle() {
        let mut a = explicit("a", &[]);
        a.participants = ParticipantSpec {
            explicit: None,
            selector: Some(SelectorSpec {
                kind: SelectorKind::ChampionsOf,
                country: None,
                region: None,
                count: Some(4),
                exclude_competitions: vec![],
                source_competition: Some("b".to_string()),
            }),
        };
        let mut b = explicit("b", &[]);
        b.participants = ParticipantSpec {
            explicit: None,
            selector: Some(SelectorSpec {
                kind: SelectorKind::ChampionsOf,
                country: None,
                region: None,
                count: Some(4),
                exclude_competitions: vec![],
                source_competition: Some("a".to_string()),
            }),
        };
        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![a, b],
        };
        let errors = validate_definitions(&file, &ctx());
        assert!(codes(&errors).contains(&"be.error.competitionDef.selectorCycle"));

        // An unknown source is reported too.
        let mut lonely = explicit("c", &[]);
        lonely.participants = ParticipantSpec {
            explicit: None,
            selector: Some(SelectorSpec {
                kind: SelectorKind::ChampionsOf,
                country: None,
                region: None,
                count: Some(2),
                exclude_competitions: vec![],
                source_competition: Some("ghost".to_string()),
            }),
        };
        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![lonely],
        };
        let errors = validate_definitions(&file, &ctx());
        assert!(codes(&errors).contains(&"be.error.competitionDef.unknownSourceCompetition"));
    }

    // --- resolution ---

    fn world() -> super::super::WorldData {
        let make = |id: &str, nation: &str, reputation: u32| {
            let mut team = Team::new(
                id.to_string(),
                id.to_string(),
                id.to_string(),
                "Country".to_string(),
                "City".to_string(),
                "Stadium".to_string(),
                10_000,
            );
            team.football_nation = nation.to_string();
            team.reputation = reputation;
            team
        };
        super::super::WorldData {
            teams: vec![
                make("tr-a", "TR", 900),
                make("tr-b", "TR", 800),
                make("tr-c", "TR", 700),
                make("tr-d", "TR", 600),
                make("jp-a", "JP", 850),
                make("jp-b", "JP", 750),
            ],
            regions: vec![
                super::super::WorldRegionDefinition {
                    id: "europe".to_string(),
                    name: "Europe".to_string(),
                    country_codes: vec!["TR".to_string()],
                },
                super::super::WorldRegionDefinition {
                    id: "asia".to_string(),
                    name: "Asia".to_string(),
                    country_codes: vec!["JP".to_string()],
                },
            ],
            ..Default::default()
        }
    }

    fn start() -> DateTime<Utc> {
        use chrono::TimeZone;
        Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap()
    }

    fn selector_def(
        id: &str,
        ty: CompetitionType,
        format: CompetitionFormat,
        selector: SelectorSpec,
    ) -> CompetitionDefinition {
        CompetitionDefinition {
            id: id.to_string(),
            name: id.to_string(),
            r#type: ty,
            scope: CompetitionScope::Domestic,
            region_id: None,
            country_id: None,
            required_region_ids: vec![],
            priority: 0,
            format: FormatDef {
                kind: format,
                legs: None,
                group_size: None,
                qualifiers_per_group: None,
                best_third_qualifiers: None,
            },
            participants: ParticipantSpec {
                explicit: None,
                selector: Some(selector),
            },
            berths: Vec::new(),
            season_start_month: None,
            season_start_day: None,
            name_key: None,
            logo: None,
        }
    }

    #[test]
    fn resolves_top_by_reputation_with_exclusions_into_a_pyramid() {
        // First division: top 2 Turkish clubs. Second division: next clubs,
        // excluding the first division's.
        let first = selector_def(
            "tr-1",
            CompetitionType::League,
            CompetitionFormat::LeagueTable,
            SelectorSpec {
                kind: SelectorKind::TopByReputation,
                country: Some("TR".to_string()),
                region: None,
                count: Some(2),
                exclude_competitions: vec![],
                source_competition: None,
            },
        );
        let second = selector_def(
            "tr-2",
            CompetitionType::League,
            CompetitionFormat::LeagueTable,
            SelectorSpec {
                kind: SelectorKind::TopByReputation,
                country: Some("TR".to_string()),
                region: None,
                count: Some(2),
                exclude_competitions: vec!["tr-1".to_string()],
                source_competition: None,
            },
        );
        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![first, second],
        };

        let competitions = resolve_definitions(&file, &world(), 2026, start());

        let div1 = competitions.iter().find(|c| c.id == "tr-1").unwrap();
        let div2 = competitions.iter().find(|c| c.id == "tr-2").unwrap();
        assert_eq!(div1.participant_ids, vec!["tr-a", "tr-b"]);
        assert_eq!(div2.participant_ids, vec!["tr-c", "tr-d"]);
        // The competition's stable id flows onto its fixtures.
        assert!(div1.fixtures.iter().all(|f| f.competition_id == "tr-1"));
        assert_eq!(div1.fixtures.len(), 2); // 2 clubs, home & away
    }

    #[test]
    fn resolves_champions_of_into_a_group_knockout_with_a_stable_id() {
        let league = selector_def(
            "tr-1",
            CompetitionType::League,
            CompetitionFormat::LeagueTable,
            SelectorSpec {
                kind: SelectorKind::TopByReputation,
                country: Some("TR".to_string()),
                region: None,
                count: Some(4),
                exclude_competitions: vec![],
                source_competition: None,
            },
        );
        let mut continental = selector_def(
            "afc",
            CompetitionType::ContinentalClub,
            CompetitionFormat::GroupAndKnockout,
            SelectorSpec {
                kind: SelectorKind::ChampionsOf,
                country: None,
                region: None,
                count: Some(4),
                exclude_competitions: vec![],
                source_competition: Some("tr-1".to_string()),
            },
        );
        continental.scope = CompetitionScope::Continental;
        // Author the continental cup BEFORE its source to exercise dependency
        // ordering.
        let file = CompetitionDefinitionFile {
            format_version: 1,
            competitions: vec![continental, league],
        };

        let competitions = resolve_definitions(&file, &world(), 2026, start());

        let afc = competitions.iter().find(|c| c.id == "afc").unwrap();
        assert_eq!(afc.rules.format, CompetitionFormat::GroupAndKnockout);
        // Top 4 Turkish clubs by reputation qualified.
        assert_eq!(afc.participant_ids, vec!["tr-a", "tr-b", "tr-c", "tr-d"]);
        // The generated id was rewritten to the stable definition id everywhere.
        assert!(afc.fixtures.iter().all(|f| f.competition_id == "afc"));
        assert!(afc.groups.iter().all(|g| g.id.starts_with("afc-group-")));
    }

    #[test]
    fn group_config_on_a_league_is_rejected_and_future_versions_blocked() {
        let mut def = explicit("tr-1", &["team-a", "team-b"]);
        def.format.group_size = Some(4);
        let file = CompetitionDefinitionFile {
            format_version: 99,
            competitions: vec![def],
        };
        let errors = validate_definitions(&file, &ctx());
        let codes = codes(&errors);
        assert!(codes.contains(&"be.error.competitionDef.groupConfigOnNonGroupFormat"));
        assert!(codes.contains(&"be.error.competitionDef.unsupportedVersion"));
    }
}
