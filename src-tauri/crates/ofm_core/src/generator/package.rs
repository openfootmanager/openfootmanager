//! Modular world packages: a folder of JSON/YAML files, each declaring a
//! top-level `schema` discriminator. The loader walks the folder **recursively**
//! and classifies every file by its `schema` — never by which directory it sits
//! in — so authors can organise files however they like. Entities link to one
//! another by stable string ids, resolved after every file is read.
//!
//! This module covers loading, classification, and structural validation
//! (recognised schema, well-formed entities, unique non-empty ids). Cross-file
//! reference checks and building a runnable world come in later slices.

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use domain::league::CompetitionScope;
use domain::player::{PlayerAttributes, Position};
use domain::staff::{CoachingSpecialization, StaffAttributes, StaffRole};

use super::{CompetitionDefinition, NamePool, NamesDefinition, TeamDef};

// ---------------------------------------------------------------------------
// Authoring structs for the entity types a package can contain
// ---------------------------------------------------------------------------

/// A confederation / region. Its `id` is the region id used throughout the game.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfederationDef {
    #[serde(default)]
    pub id: String,
    pub name: String,
}

/// A country, tied to a confederation. `id` is the ISO/football code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CountryDef {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub confederation: String,
}

/// A player authored by hand. Ability may be given as a single `overall` (the
/// engine generates a realistic attribute spread) or as an explicit
/// `attributes` block. `club` references a [`TeamDef`] id, `nationality` a
/// country id.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDef {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default)]
    pub club: String,
    #[serde(default)]
    pub nationality: String,
    #[serde(default)]
    pub position: Position,
    #[serde(default)]
    pub date_of_birth: Option<String>,
    #[serde(default)]
    pub age: Option<u32>,
    #[serde(default)]
    pub overall: Option<u8>,
    #[serde(default)]
    pub attributes: Option<PlayerAttributes>,
    /// Optional path to a profile photo, relative to the package root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photo: Option<String>,
    /// Preferred foot ("Left", "Right", "Both"). Defaults to "Right" if omitted.
    #[serde(default)]
    pub footedness: Option<String>,
    /// If true, the player belongs to the club's youth / academy squad rather than the first team.
    #[serde(default, skip_serializing_if = "is_false")]
    pub youth: bool,
}

fn is_false(v: &bool) -> bool {
    !v
}

fn default_staff_role() -> StaffRole {
    StaffRole::Coach
}

/// A coaching staff member defined in a world package.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffDef {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    /// Club team id this staff member belongs to. Empty = unattached / free agent.
    #[serde(default)]
    pub club: String,
    #[serde(default)]
    pub nationality: String,
    #[serde(default = "default_staff_role")]
    pub role: StaffRole,
    #[serde(default)]
    pub attributes: Option<StaffAttributes>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specialization: Option<CoachingSpecialization>,
    #[serde(default)]
    pub date_of_birth: Option<String>,
    #[serde(default)]
    pub age: Option<u32>,
}

/// Package-level metadata (at most one per package).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldMetaDef {
    /// Stable slug used as the install key (e.g. `"premier-league-2026"`).
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default_active_regions: Vec<String>,
    #[serde(default)]
    pub default_active_competitions: Vec<String>,
    #[serde(default)]
    pub base_year: Option<i32>,
    /// Semantic version string (e.g. `"1.0.0"`).
    #[serde(default)]
    pub version: String,
    /// Package author / creator.
    #[serde(default)]
    pub author: String,
    /// Monotonic format version for future compatibility.
    #[serde(default)]
    pub format_version: u32,
    /// SPDX license expression (e.g. `"CC-BY-4.0"`).
    #[serde(default)]
    pub license: String,
    /// Minimum game version required (semver, e.g. `"0.3.0"`). Empty = no requirement.
    #[serde(default)]
    pub game_min_version: String,
    /// Package type: `"database"` | `"patch"` | `"assets"`. Defaults to `"database"`.
    #[serde(default = "default_package_type")]
    pub package_type: String,
    /// Relative path to the package logo image within the package (e.g. `"assets/images/logo.png"`).
    #[serde(default)]
    pub logo: Option<String>,
    /// Optional overrides for the league auto-generated when a `database` package
    /// declares teams but no competitions. Absent = use the built-in defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_league: Option<FallbackLeagueConfig>,
}

/// Author-supplied shape for the auto-generated fallback league. Every field is
/// optional and falls back to the built-in default when unset.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FallbackLeagueConfig {
    /// Display name. When set, it is used verbatim instead of the localized
    /// default name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Rounds each pair plays: `1` (single) or `2` (double round-robin, default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legs: Option<u8>,
    /// Competition scope. Defaults to `Domestic`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<CompetitionScope>,
}

fn default_package_type() -> String {
    "database".to_string()
}

/// A package summarised for display and install management.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub license: String,
    pub game_min_version: String,
    pub package_type: String,
    pub team_count: usize,
    pub player_count: usize,
    pub competition_count: usize,
    /// Absolute path to the installed `.ofm` file.
    pub installed_path: String,
    /// Logo encoded as a data URL (`data:<mime>;base64,...`), if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_data_url: Option<String>,
}

/// Everything a package declares, aggregated across all its files.
#[derive(Debug, Default)]
pub struct WorldPackage {
    pub meta: Option<WorldMetaDef>,
    pub confederations: Vec<ConfederationDef>,
    pub countries: Vec<CountryDef>,
    pub teams: Vec<TeamDef>,
    pub players: Vec<PlayerDef>,
    pub staff: Vec<StaffDef>,
    pub competitions: Vec<CompetitionDefinition>,
    pub names: Option<NamesDefinition>,
    /// Per-locale translation bundles supplied by the package, keyed by locale
    /// code (e.g. `"de"`, `"fr"`). Loaded from `translations.{locale}.json`
    /// files found anywhere in the package directory tree.
    pub extra_translations: std::collections::HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

const READ_FAILED: &str = "be.error.package.readFailed";
const MISSING_SCHEMA: &str = "be.error.package.missingSchema";
const UNKNOWN_SCHEMA: &str = "be.error.package.unknownSchema";
const INVALID_ENTITY: &str = "be.error.package.invalidEntity";
const MISSING_ID: &str = "be.error.package.missingId";
const DUPLICATE_ID: &str = "be.error.package.duplicateId";
const UNKNOWN_CONFEDERATION: &str = "be.error.package.unknownConfederation";
const UNKNOWN_COUNTRY: &str = "be.error.package.unknownCountry";
const UNKNOWN_TEAM: &str = "be.error.package.unknownTeam";
const UNKNOWN_COMPETITION: &str = "be.error.package.unknownCompetition";
const UNKNOWN_REGION: &str = "be.error.package.unknownRegion";
const REVERSED_RANGE: &str = "be.error.package.reversedRange";
const OUT_OF_RANGE: &str = "be.error.package.outOfRange";

/// Maximum team reputation. Reputation is a `u32`, so it cannot go below 0.
const MAX_REPUTATION: u32 = 1000;

/// A structured problem found while loading a package. `code` is an i18n key,
/// `file` locates the offending file (empty for aggregate-level problems), and
/// `params` fills the message placeholders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageError {
    pub code: String,
    pub file: String,
    pub params: Vec<(String, String)>,
}

impl PackageError {
    fn new(code: &str, file: &str) -> Self {
        Self {
            code: code.to_string(),
            file: file.to_string(),
            params: Vec::new(),
        }
    }

    fn with(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.push((key.to_string(), value.into()));
        self
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Recursively collect every JSON/YAML file under `dir`.
fn collect_data_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_data_files(&path, out);
        } else if matches!(
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("json") | Some("yaml") | Some("yml")
        ) {
            out.push(path);
        }
    }
}

fn parse_entity<T: serde::de::DeserializeOwned>(
    value: Value,
    file: &str,
    schema: &str,
    errors: &mut Vec<PackageError>,
) -> Option<T> {
    match serde_yaml::from_value::<T>(value) {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            errors.push(PackageError::new(INVALID_ENTITY, file).with("schema", schema));
            None
        }
    }
}

fn classify_entity(
    schema: &str,
    value: Value,
    file: &str,
    package: &mut WorldPackage,
    errors: &mut Vec<PackageError>,
) {
    match schema {
        "confederation" => {
            if let Some(def) = parse_entity::<ConfederationDef>(value, file, schema, errors) {
                package.confederations.push(def);
            }
        }
        "country" => {
            if let Some(def) = parse_entity::<CountryDef>(value, file, schema, errors) {
                package.countries.push(def);
            }
        }
        "team" => {
            if let Some(def) = parse_entity::<TeamDef>(value, file, schema, errors) {
                package.teams.push(def);
            }
        }
        "player" => {
            if let Some(def) = parse_entity::<PlayerDef>(value, file, schema, errors) {
                package.players.push(def);
            }
        }
        "staff" => {
            if let Some(def) = parse_entity::<StaffDef>(value, file, schema, errors) {
                package.staff.push(def);
            }
        }
        "competition" => {
            if let Some(def) = parse_entity::<CompetitionDefinition>(value, file, schema, errors) {
                package.competitions.push(def);
            }
        }
        "names" => {
            if let Some(def) = parse_entity::<NamesDefinition>(value, file, schema, errors) {
                package.names = Some(def);
            }
        }
        "world" => {
            if let Some(def) = parse_entity::<WorldMetaDef>(value, file, schema, errors) {
                package.meta = Some(def);
            }
        }
        other => {
            errors.push(PackageError::new(UNKNOWN_SCHEMA, file).with("schema", other));
        }
    }
}

fn classify_file(
    value: Value,
    file: &str,
    package: &mut WorldPackage,
    errors: &mut Vec<PackageError>,
) {
    let Some(map) = value.as_mapping() else {
        errors.push(PackageError::new(MISSING_SCHEMA, file));
        return;
    };
    let schema = map.get("schema").and_then(Value::as_str).map(str::to_string);
    let Some(schema) = schema else {
        errors.push(PackageError::new(MISSING_SCHEMA, file));
        return;
    };

    // A file holds one entity (its fields at the top level) or a bulk `items`
    // list of entities of the same schema.
    let entities: Vec<Value> = match map.get("items") {
        Some(Value::Sequence(items)) => items.clone(),
        _ => vec![value.clone()],
    };
    for entity in entities {
        classify_entity(&schema, entity, file, package, errors);
    }
}

/// Load a world package from a directory: walk it recursively, classify each
/// file by its `schema`, and validate ids. Returns the aggregated package and
/// every problem found. Collections are sorted by id so the result is
/// independent of file-discovery order (and therefore of folder layout).
/// Extract the locale code from a translation file name of the form
/// `translations.{locale}.json`. The locale must be non-empty and must not
/// contain dots (BCP 47 subtags use hyphens, e.g. `pt-BR`). Returns `None`
/// for any name that doesn't match this exact pattern.
fn translation_locale_from_filename(name: &str) -> Option<&str> {
    let lower = name.to_ascii_lowercase();
    let stem = lower.strip_suffix(".json")?;
    let locale_lower = stem.strip_prefix("translations.")?;
    if locale_lower.is_empty() || locale_lower.contains('.') {
        return None;
    }
    // Return the original-cased locale slice.
    let start = "translations.".len();
    let end = name.len() - ".json".len();
    Some(&name[start..end])
}

/// Load and classify all files in `dir`, running only id-uniqueness checks.
/// Cross-reference validation is deliberately deferred so callers can merge
/// multiple packages before running references (which may span packages).
pub fn load_world_package_files(dir: &Path) -> (WorldPackage, Vec<PackageError>) {
    let mut files = Vec::new();
    collect_data_files(dir, &mut files);
    files.sort();

    let mut package = WorldPackage::default();
    let mut errors = Vec::new();

    for path in &files {
        let file = path
            .strip_prefix(dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        // Translation files are loaded separately and not treated as entity definitions.
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if let Some(locale) = translation_locale_from_filename(file_name) {
            let canonical = locale.to_ascii_lowercase();
            if package.extra_translations.contains_key(&canonical) {
                errors.push(PackageError::new(READ_FAILED, &file));
            } else {
                match std::fs::read_to_string(path) {
                    Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(serde_json::Value::Object(map)) => {
                            package
                                .extra_translations
                                .insert(canonical, serde_json::Value::Object(map));
                        }
                        Ok(_) | Err(_) => errors.push(PackageError::new(READ_FAILED, &file)),
                    },
                    Err(_) => errors.push(PackageError::new(READ_FAILED, &file)),
                }
            }
            continue;
        }

        match std::fs::read_to_string(path) {
            Ok(text) => match super::parse_definition_str::<Value>(&text) {
                Ok(value) => classify_file(value, &file, &mut package, &mut errors),
                Err(_) => errors.push(PackageError::new(READ_FAILED, &file)),
            },
            Err(_) => errors.push(PackageError::new(READ_FAILED, &file)),
        }
    }

    package.confederations.sort_by(|a, b| a.id.cmp(&b.id));
    package.countries.sort_by(|a, b| a.id.cmp(&b.id));
    package.teams.sort_by(|a, b| a.id.cmp(&b.id));
    package.players.sort_by(|a, b| a.id.cmp(&b.id));
    package.staff.sort_by(|a, b| a.id.cmp(&b.id));
    package.competitions.sort_by(|a, b| a.id.cmp(&b.id));

    errors.extend(validate_ids(&package));
    (package, errors)
}

/// Load a world package from a directory: walk it recursively, classify each
/// file by its `schema`, and validate ids. Returns the aggregated package and
/// every problem found. Collections are sorted by id so the result is
/// independent of file-discovery order (and therefore of folder layout).
pub fn load_world_package(dir: &Path) -> (WorldPackage, Vec<PackageError>) {
    let (package, mut errors) = load_world_package_files(dir);
    errors.extend(validate_references(&package));
    (package, errors)
}

/// Validate that every entity has a non-empty id and that ids are unique within
/// each entity type.
pub fn validate_ids(package: &WorldPackage) -> Vec<PackageError> {
    let mut errors = Vec::new();
    check_ids(
        package.confederations.iter().map(|c| c.id.as_str()),
        "confederation",
        &mut errors,
    );
    check_ids(
        package.countries.iter().map(|c| c.id.as_str()),
        "country",
        &mut errors,
    );
    check_ids(
        package.teams.iter().map(|t| t.id.as_str()),
        "team",
        &mut errors,
    );
    check_ids(
        package.players.iter().map(|p| p.id.as_str()),
        "player",
        &mut errors,
    );
    check_ids(
        package.staff.iter().map(|s| s.id.as_str()),
        "staff",
        &mut errors,
    );
    check_ids(
        package.competitions.iter().map(|c| c.id.as_str()),
        "competition",
        &mut errors,
    );
    errors
}

fn check_ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    kind: &str,
    errors: &mut Vec<PackageError>,
) {
    let mut seen: HashSet<&str> = HashSet::new();
    for id in ids {
        if id.is_empty() {
            errors.push(PackageError::new(MISSING_ID, "").with("kind", kind));
        } else if !seen.insert(id) {
            errors.push(
                PackageError::new(DUPLICATE_ID, "")
                    .with("kind", kind)
                    .with("id", id),
            );
        }
    }
}

/// Validate cross-file references: a country's confederation, a team's country,
/// a player's club and nationality, and every competition reference. References
/// resolve against entities defined in the package **plus** the built-in
/// confederation/country catalog, so a package may reference (e.g.) `europe` or
/// `ES` without redefining them. Empty (unspecified) references are left for the
/// world-build step to default.
pub fn validate_references(package: &WorldPackage) -> Vec<PackageError> {
    let mut errors = Vec::new();

    let team_ids: HashSet<&str> = package.teams.iter().map(|t| t.id.as_str()).collect();
    let country_ids: HashSet<&str> = package.countries.iter().map(|c| c.id.as_str()).collect();
    let confederation_ids: HashSet<&str> =
        package.confederations.iter().map(|c| c.id.as_str()).collect();

    let known_confederation =
        |id: &str| confederation_ids.contains(id) || crate::nations::is_builtin_region(id);
    let known_country =
        |code: &str| country_ids.contains(code) || crate::nations::nation_by_code(code).is_some();

    for country in &package.countries {
        if !country.confederation.is_empty() && !known_confederation(&country.confederation) {
            errors.push(
                PackageError::new(UNKNOWN_CONFEDERATION, "")
                    .with("country", &country.id)
                    .with("confederation", &country.confederation),
            );
        }
    }

    for team in &package.teams {
        if !team.country.is_empty() && !known_country(&team.country) {
            errors.push(
                PackageError::new(UNKNOWN_COUNTRY, "")
                    .with("entity", &team.id)
                    .with("country", &team.country),
            );
        }
    }

    for player in &package.players {
        if !player.club.is_empty() && !team_ids.contains(player.club.as_str()) {
            errors.push(
                PackageError::new(UNKNOWN_TEAM, "")
                    .with("entity", &player.id)
                    .with("team", &player.club),
            );
        }
        if !player.nationality.is_empty() && !known_country(&player.nationality) {
            errors.push(
                PackageError::new(UNKNOWN_COUNTRY, "")
                    .with("entity", &player.id)
                    .with("country", &player.nationality),
            );
        }
    }

    for staff in &package.staff {
        if !staff.club.is_empty() && !team_ids.contains(staff.club.as_str()) {
            errors.push(
                PackageError::new(UNKNOWN_TEAM, "")
                    .with("entity", &staff.id)
                    .with("team", &staff.club),
            );
        }
        if !staff.nationality.is_empty() && !known_country(&staff.nationality) {
            errors.push(
                PackageError::new(UNKNOWN_COUNTRY, "")
                    .with("entity", &staff.id)
                    .with("country", &staff.nationality),
            );
        }
    }

    errors.extend(validate_competition_references(package));

    // Check that every id in defaultActiveCompetitions exists as a competition
    // in the same package. Skipped for `patch` packages, which are expected to
    // reference competitions defined in the base database they supplement; those
    // cross-package references are validated after merge_world_packages combines
    // the full stack.
    let is_patch = package
        .meta
        .as_ref()
        .map(|m| m.package_type == "patch")
        .unwrap_or(false);
    if !is_patch {
        if let Some(meta) = &package.meta {
            let comp_ids: HashSet<&str> = package.competitions.iter().map(|c| c.id.as_str()).collect();
            for id in &meta.default_active_competitions {
                if !id.is_empty() && !comp_ids.contains(id.as_str()) {
                    errors.push(
                        PackageError::new(UNKNOWN_COMPETITION, "")
                            .with("id", id)
                            .with("field", "defaultActiveCompetitions"),
                    );
                }
            }
            // Each defaultActiveRegions id must be a known region: a confederation
            // defined in this package or a built-in region (e.g. `europe`).
            for id in &meta.default_active_regions {
                if !id.is_empty() && !known_confederation(id) {
                    errors.push(
                        PackageError::new(UNKNOWN_REGION, "")
                            .with("id", id)
                            .with("field", "defaultActiveRegions"),
                    );
                }
            }
        }
    }

    // Check team reputation / finance ranges for reversed (min > max) and
    // out-of-bounds endpoints (reputation 0..=1000, finance >= 0).
    for team in &package.teams {
        if let Some([min, max]) = team.reputation_range {
            if min > max {
                errors.push(
                    PackageError::new(REVERSED_RANGE, "")
                        .with("team", &team.id)
                        .with("field", "reputationRange"),
                );
            }
            if min > MAX_REPUTATION || max > MAX_REPUTATION {
                errors.push(
                    PackageError::new(OUT_OF_RANGE, "")
                        .with("team", &team.id)
                        .with("field", "reputationRange"),
                );
            }
        }
        if let Some([min, max]) = team.finance_range {
            if min > max {
                errors.push(
                    PackageError::new(REVERSED_RANGE, "")
                        .with("team", &team.id)
                        .with("field", "financeRange"),
                );
            }
            if min < 0 || max < 0 {
                errors.push(
                    PackageError::new(OUT_OF_RANGE, "")
                        .with("team", &team.id)
                        .with("field", "financeRange"),
                );
            }
        }
    }

    errors
}

/// Run the existing competition validator over a package's competitions, with a
/// world context built from the package's teams/countries/regions plus the
/// built-in catalog. Definition errors are surfaced as package errors.
fn validate_competition_references(package: &WorldPackage) -> Vec<PackageError> {
    if package.competitions.is_empty() {
        return Vec::new();
    }

    let team_ids: HashSet<&str> = package.teams.iter().map(|t| t.id.as_str()).collect();

    let mut country_codes: HashSet<&str> =
        package.countries.iter().map(|c| c.id.as_str()).collect();
    let mut region_ids: HashSet<&str> =
        package.confederations.iter().map(|c| c.id.as_str()).collect();
    for nation in crate::nations::NATION_CATALOG {
        country_codes.insert(nation.code);
        region_ids.insert(nation.region_id);
    }

    let ctx = super::WorldValidationContext {
        team_ids,
        country_codes,
        region_ids,
    };
    let file = super::CompetitionDefinitionFile {
        format_version: super::SUPPORTED_DEFINITION_FORMAT_VERSION,
        competitions: package.competitions.clone(),
    };

    super::validate_definitions(&file, &ctx)
        .into_iter()
        .map(|error| {
            let mut params = error.params;
            if !error.competition_id.is_empty() {
                params.push(("competition".to_string(), error.competition_id));
            }
            PackageError {
                code: error.code,
                file: String::new(),
                params,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Package stack conflict detection
// ---------------------------------------------------------------------------

/// Severity of a conflict detected between two or more stacked packages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictSeverity {
    /// The conflict may produce unexpected results but does not block game start.
    Warning,
    /// The conflict blocks game start and must be resolved.
    Error,
}

/// Describes a single conflict between packages in a stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackConflict {
    pub severity: ConflictSeverity,
    /// Human-readable i18n key for the conflict kind.
    pub code: String,
    /// Entity type ("team", "competition", …).
    pub entity_kind: String,
    /// The conflicting entity id.
    pub entity_id: String,
    /// Package ids involved in the conflict (winner listed last).
    pub packages: Vec<String>,
}

impl StackConflict {
    fn db_clash(entity_kind: &str, entity_id: &str, pkg_a: &str, pkg_b: &str) -> Self {
        Self {
            severity: ConflictSeverity::Warning,
            code: "be.error.conflict.duplicateId".to_string(),
            entity_kind: entity_kind.to_string(),
            entity_id: entity_id.to_string(),
            packages: vec![pkg_a.to_string(), pkg_b.to_string()],
        }
    }
}

/// Inspect a slice of packages for cross-package id conflicts **before** merging.
///
/// Rules:
/// - Two `database` packages declaring the **same id with different content** →
///   `Warning` (last-wins, but the user should know).
/// - Two `database` packages declaring the **same id with identical content** →
///   no conflict (safe dedup).
/// - A `patch` package overriding any id from a `database` package →
///   no conflict (intentional override).
/// - Two `patch` packages clashing → `Warning`.
/// - Packages with the same package-level `id` → `Error`.
pub fn validate_package_stack(packages: &[&WorldPackage]) -> Vec<StackConflict> {
    let mut conflicts = Vec::new();

    // Check duplicate package ids.
    let mut pkg_ids_seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (i, pkg) in packages.iter().enumerate() {
        if let Some(meta) = &pkg.meta {
            if !meta.id.is_empty() {
                if let Some(&prev) = pkg_ids_seen.get(meta.id.as_str()) {
                    conflicts.push(StackConflict {
                        severity: ConflictSeverity::Error,
                        code: "be.error.conflict.duplicatePackageId".to_string(),
                        entity_kind: "package".to_string(),
                        entity_id: meta.id.clone(),
                        packages: vec![format!("#{}", prev + 1), format!("#{}", i + 1)],
                    });
                } else {
                    pkg_ids_seen.insert(meta.id.as_str(), i);
                }
            }
        }
    }

    fn pkg_type(pkg: &WorldPackage) -> &str {
        pkg.meta.as_ref().map(|m| m.package_type.as_str()).unwrap_or("database")
    }

    // Build per-entity-type index: id → (package_index, serialized_content)
    // for detecting content divergence between packages of the same tier.
    macro_rules! check_entity_conflicts {
        ($field:ident, $id_fn:expr, $kind:expr) => {{
            let mut seen: std::collections::HashMap<String, (usize, String)> = std::collections::HashMap::new();
            for (i, pkg) in packages.iter().enumerate() {
                let is_patch = pkg_type(pkg) == "patch";
                for entity in &pkg.$field {
                    let id = $id_fn(entity);
                    if id.is_empty() { continue; }
                    let content = serde_json::to_string(entity).unwrap_or_default();
                    if let Some((prev_i, prev_content)) = seen.get(id.as_str()) {
                        let prev_is_patch = pkg_type(packages[*prev_i]) == "patch";
                        // patch overriding database → intentional, no conflict
                        if is_patch && !prev_is_patch {
                            // update to this version as the new winner
                            seen.insert(id.clone(), (i, content));
                            continue;
                        }
                        // database after patch → patch still wins, no conflict
                        if !is_patch && prev_is_patch {
                            continue;
                        }
                        // identical content → safe dedup, no conflict
                        if content == *prev_content {
                            continue;
                        }
                        // db-db or patch-patch clash with divergent content → warning
                        let prev_pkg_id = packages[*prev_i].meta.as_ref()
                            .map(|m| m.id.as_str()).unwrap_or("(unknown)");
                        let this_pkg_id = pkg.meta.as_ref()
                            .map(|m| m.id.as_str()).unwrap_or("(unknown)");
                        conflicts.push(StackConflict::db_clash($kind, id.as_str(), prev_pkg_id, this_pkg_id));
                        // update seen so subsequent packages compare against this one
                        seen.insert(id.clone(), (i, content));
                    } else {
                        seen.insert(id.clone(), (i, content));
                    }
                }
            }
        }};
    }

    check_entity_conflicts!(teams, |t: &TeamDef| t.id.clone(), "team");
    check_entity_conflicts!(competitions, |c: &CompetitionDefinition| c.id.clone(), "competition");
    check_entity_conflicts!(confederations, |c: &ConfederationDef| c.id.clone(), "confederation");
    check_entity_conflicts!(countries, |c: &CountryDef| c.id.clone(), "country");

    conflicts
}

/// Merge multiple packages into one.
///
/// **Precedence**: `database` packages are processed first (in stack order),
/// then `patch` packages (in stack order), so patches always win over databases.
/// Within the same tier, later entries win (last-in-stack wins). This makes
/// `patch` packages unambiguously override `database` ones without surfacing a
/// conflict warning.
///
/// **Meta merging**: `defaultActiveCompetitions` and `defaultActiveRegions` are
/// unioned across all metas. `baseYear` takes the maximum value. `name` and
/// other scalar fields come from the last non-empty value across all metas.
///
/// After merging, full id + reference validation runs on the combined result.
/// Cross-package references resolve correctly because all entities are present
/// before validation runs.
pub fn merge_world_packages(packages: Vec<WorldPackage>) -> (WorldPackage, Vec<PackageError>) {
    use std::collections::BTreeMap;

    // Split into tiers: database (or unknown) first, patch second.
    let (databases, patches): (Vec<WorldPackage>, Vec<WorldPackage>) = packages
        .into_iter()
        .partition(|p| p.meta.as_ref().map(|m| m.package_type.as_str()).unwrap_or("database") != "patch");
    // A merged stack that contains any non-patch package is a complete world and
    // must be validated as one (the per-package "patch" skip would otherwise
    // suppress dangling-reference checks for the whole stack).
    let has_database = !databases.is_empty();

    let mut merged = WorldPackage::default();
    let mut confeds: BTreeMap<String, ConfederationDef> = BTreeMap::new();
    let mut countries: BTreeMap<String, CountryDef> = BTreeMap::new();
    let mut teams: BTreeMap<String, TeamDef> = BTreeMap::new();
    let mut players: BTreeMap<String, PlayerDef> = BTreeMap::new();
    let mut staff_map: BTreeMap<String, StaffDef> = BTreeMap::new();
    let mut competitions: BTreeMap<String, CompetitionDefinition> = BTreeMap::new();

    // Collected meta fields for union/max merging.
    let mut all_default_active_competitions: Vec<String> = Vec::new();
    let mut all_default_active_competitions_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut all_default_active_regions: Vec<String> = Vec::new();
    let mut all_default_active_regions_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut merged_meta_base: Option<WorldMetaDef> = None;
    // Name pools are unioned per-key across packages (like every other entity
    // collection) rather than wholesale-replaced, so stacking packages that each
    // supply distinct pools keeps them all.
    let mut merged_pools: std::collections::HashMap<String, NamePool> = std::collections::HashMap::new();
    let mut names_version = 0u32;
    let mut names_description = String::new();
    let mut saw_names = false;

    for package in databases.into_iter().chain(patches.into_iter()) {
        if let Some(meta) = package.meta {
            // Union the list fields; scalar fields take the last non-empty value.
            for id in &meta.default_active_competitions {
                if !id.is_empty() && all_default_active_competitions_seen.insert(id.clone()) {
                    all_default_active_competitions.push(id.clone());
                }
            }
            for id in &meta.default_active_regions {
                if !id.is_empty() && all_default_active_regions_seen.insert(id.clone()) {
                    all_default_active_regions.push(id.clone());
                }
            }
            if let Some(ref mut base) = merged_meta_base {
                if !meta.name.is_empty() { base.name = meta.name; }
                if !meta.description.is_empty() { base.description = meta.description; }
                if !meta.author.is_empty() { base.author = meta.author; }
                if !meta.version.is_empty() { base.version = meta.version; }
                if !meta.id.is_empty() { base.id = meta.id; }
                if meta.base_year > base.base_year { base.base_year = meta.base_year; }
                if meta.logo.is_some() { base.logo = meta.logo; }
                if !meta.license.is_empty() { base.license = meta.license; }
                if !meta.game_min_version.is_empty() { base.game_min_version = meta.game_min_version; }
                if meta.format_version > base.format_version { base.format_version = meta.format_version; }
                if !meta.package_type.is_empty() { base.package_type = meta.package_type; }
            } else {
                merged_meta_base = Some(meta);
            }
        }
        for c in package.confederations { confeds.insert(c.id.clone(), c); }
        for c in package.countries { countries.insert(c.id.clone(), c); }
        for t in package.teams { teams.insert(t.id.clone(), t); }
        for p in package.players { players.insert(p.id.clone(), p); }
        for s in package.staff { staff_map.insert(s.id.clone(), s); }
        for c in package.competitions { competitions.insert(c.id.clone(), c); }
        if let Some(names) = package.names {
            saw_names = true;
            if names.version > names_version { names_version = names.version; }
            if !names.description.is_empty() { names_description = names.description; }
            for (key, pool) in names.pools { merged_pools.insert(key, pool); }
        }
        for (locale, bundle) in package.extra_translations {
            merged.extra_translations.insert(locale, bundle);
        }
    }

    if let Some(mut meta) = merged_meta_base {
        meta.default_active_competitions = all_default_active_competitions.into_iter().collect();
        meta.default_active_regions = all_default_active_regions.into_iter().collect();
        if has_database {
            meta.package_type = default_package_type();
        }
        merged.meta = Some(meta);
    }

    if saw_names {
        merged.names = Some(NamesDefinition {
            version: names_version,
            description: names_description,
            pools: merged_pools,
        });
    }

    merged.confederations = confeds.into_values().collect();
    merged.countries = countries.into_values().collect();
    merged.teams = teams.into_values().collect();
    merged.players = players.into_values().collect();
    merged.staff = staff_map.into_values().collect();
    merged.competitions = competitions.into_values().collect();

    let mut errors = validate_ids(&merged);
    errors.extend(validate_references(&merged));
    (merged, errors)
}

// ---------------------------------------------------------------------------
// Package lockfile
// ---------------------------------------------------------------------------

/// Records which `.ofm` package was used to build a save, for reproducibility.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PackageLock {
    pub id: String,
    pub version: String,
    /// SHA-256 hex digest of the installed `.ofm` file bytes.
    pub hash: String,
}

/// Compute the SHA-256 hex digest of a file's bytes. Returns `None` on I/O error.
pub fn hash_package_file(path: &std::path::Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).ok()?;
    Some(hex::encode(Sha256::digest(&bytes)))
}

// ---------------------------------------------------------------------------
// .ofm archive support
// ---------------------------------------------------------------------------

/// Maximum size of an `.ofm` file on disk (256 MB).
pub const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
/// Maximum total uncompressed size of all entries (1 GB — zip-bomb guard).
pub const MAX_UNCOMPRESSED_BYTES: u64 = 1024 * 1024 * 1024;
/// Maximum number of files in an archive.
pub const MAX_FILE_COUNT: usize = 10_000;
/// Maximum decompressed size of a single entry read in isolation (manifest,
/// logo) outside the full hardened extraction path (16 MB — zip-bomb guard).
const MAX_SINGLE_ENTRY_BYTES: u64 = 16 * 1024 * 1024;

/// Read a single zip entry into memory, counting decompressed bytes and
/// returning `None` if it exceeds `max_bytes` or the read fails. Defends against
/// decompression bombs when an entry is read outside [`extract_archive_safely`].
fn read_entry_capped<R: std::io::Read>(entry: &mut R, max_bytes: u64) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    let mut total: u64 = 0;
    let mut chunk = [0u8; 65536];
    loop {
        match entry.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                total = total.saturating_add(n as u64);
                if total > max_bytes {
                    return None;
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            Err(_) => return None,
        }
    }
    Some(buf)
}

const ZIPSLIP_ERROR: &str = "be.error.package.zipSlip";
const SYMLINK_ERROR: &str = "be.error.package.symlinkDetected";
const TOO_MANY_FILES_ERROR: &str = "be.error.package.tooManyFiles";
const ARCHIVE_TOO_LARGE_ERROR: &str = "be.error.package.archiveTooLarge";

/// Return the destination path for a zip entry, or `None` if the entry name
/// is unsafe (zip-slip attempt: absolute path, `..` component, etc.).
fn safe_entry_path(base: &Path, entry_name: &str) -> Option<PathBuf> {
    if entry_name.starts_with('/') || entry_name.starts_with('\\') {
        return None;
    }
    let entry_path = Path::new(entry_name);
    for component in entry_path.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    if entry_name.ends_with('/') || entry_name.ends_with('\\') {
        return None;
    }
    Some(base.join(entry_name))
}

/// Hardened extraction of a `.ofm` zip archive into `dest_dir`. Enforces the
/// file-count, uncompressed-size, symlink and zip-slip guards. Per-entry
/// problems (symlink, zip-slip, read/write failure) are collected and returned
/// so the caller can decide whether to surface or skip them; fatal conditions
/// (open/parse failure, too many files, archive too large) return `Err(code)`
/// with a `be.error.*` key.
///
/// This is the single hardened extraction path: every code path that unpacks an
/// untrusted `.ofm` (package install/load *and* the world-editor "open for
/// editing" flow) must route through it so the guards can never diverge.
pub fn extract_archive_safely(
    ofm_path: &Path,
    dest_dir: &Path,
) -> Result<Vec<PackageError>, String> {
    use std::io::Read;

    let file = std::fs::File::open(ofm_path).map_err(|_| READ_FAILED.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|_| READ_FAILED.to_string())?;
    std::fs::create_dir_all(dest_dir).map_err(|_| READ_FAILED.to_string())?;

    if archive.len() > MAX_FILE_COUNT {
        return Err(TOO_MANY_FILES_ERROR.to_string());
    }

    let mut errors = Vec::new();
    let mut total_uncompressed: u64 = 0;
    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        if entry.is_symlink() {
            errors.push(PackageError::new(SYMLINK_ERROR, &entry.name().to_string()));
            continue;
        }
        let entry_name = entry.name().to_string();
        let Some(dest) = safe_entry_path(dest_dir, &entry_name) else {
            errors.push(PackageError::new(ZIPSLIP_ERROR, &entry_name));
            continue;
        };
        if let Some(parent) = dest.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                errors.push(PackageError::new(READ_FAILED, &entry_name));
                continue;
            }
        }
        // Read in 64 KB chunks and count actual decompressed bytes.
        // entry.size() comes from the zip central-directory header, which an
        // attacker can set to 0, so we must count bytes as they are read.
        let mut buf = Vec::new();
        let mut read_ok = true;
        loop {
            let mut chunk = [0u8; 65536];
            match entry.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    total_uncompressed = total_uncompressed.saturating_add(n as u64);
                    if total_uncompressed > MAX_UNCOMPRESSED_BYTES {
                        return Err(ARCHIVE_TOO_LARGE_ERROR.to_string());
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
                Err(_) => {
                    read_ok = false;
                    break;
                }
            }
        }
        if !read_ok {
            errors.push(PackageError::new(READ_FAILED, &entry_name));
            continue;
        }
        if std::fs::write(&dest, &buf).is_err() {
            errors.push(PackageError::new(READ_FAILED, &entry_name));
        }
    }

    Ok(errors)
}

/// Extract a `.ofm` zip archive to a temp directory, load the package from it,
/// clean up, and return. Zip-slip/symlink paths are silently skipped.
pub fn load_world_package_from_ofm(path: &Path) -> (WorldPackage, Vec<PackageError>) {
    let temp_dir = std::env::temp_dir().join(format!("ofm-extract-{}", uuid::Uuid::new_v4()));
    let extract_errors = match extract_archive_safely(path, &temp_dir) {
        Ok(errors) => errors,
        Err(code) => {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return (WorldPackage::default(), vec![PackageError::new(&code, "")]);
        }
    };

    // Load whatever was successfully extracted, even if some entries had errors.
    let (package, load_errors) = load_world_package_files(&temp_dir);
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Prepend extraction-level errors before the parse/validate errors.
    let mut all_errors = extract_errors;
    all_errors.extend(load_errors);
    (package, all_errors)
}

/// Read only the `schema: world` metadata entry from an `.ofm` archive without
/// fully extracting it. Used by the package manager to list installed packages
/// without extraction overhead.
pub fn read_package_manifest_from_ofm(path: &Path) -> Option<WorldMetaDef> {

    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let count = archive.len();

    for i in 0..count {
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let lower = name.to_ascii_lowercase();
        if !lower.ends_with(".json") && !lower.ends_with(".yaml") && !lower.ends_with(".yml") {
            continue;
        }
        let Some(bytes) = read_entry_capped(&mut entry, MAX_SINGLE_ENTRY_BYTES) else {
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        let Ok(value) = super::parse_definition_str::<Value>(&text) else {
            continue;
        };
        let Some(map) = value.as_mapping() else {
            continue;
        };
        if map.get("schema").and_then(Value::as_str) != Some("world") {
            continue;
        }
        if let Ok(meta) = serde_yaml::from_value::<WorldMetaDef>(value) {
            return Some(meta);
        }
    }
    None
}

/// Read a logo file from an `.ofm` archive and return it encoded as a data URL.
/// The `logo_path` is the relative path stored in `WorldMetaDef.logo`.
pub fn read_logo_from_ofm(archive_path: &Path, logo_path: &str) -> Option<String> {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let file = std::fs::File::open(archive_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let logo_lower = logo_path.to_ascii_lowercase();
    let count = archive.len();
    for i in 0..count {
        let Ok(mut entry) = archive.by_index(i) else { continue };
        if entry.is_dir() { continue }
        let entry_lower = entry.name().to_ascii_lowercase();
        // Match the relative path or the file's trailing suffix.
        if entry_lower != logo_lower && !entry_lower.ends_with(&format!("/{logo_lower}")) {
            continue;
        }
        let Some(bytes) = read_entry_capped(&mut entry, MAX_SINGLE_ENTRY_BYTES) else {
            continue;
        };
        let ext = Path::new(logo_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_ascii_lowercase();
        let mime = match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            _ => "image/png",
        };
        return Some(format!("data:{mime};base64,{}", STANDARD.encode(&bytes)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_meta_def_round_trips_fallback_league_config() {
        // A manifest carrying fallbackLeague must survive load -> save so the
        // field isn't silently dropped (there is no editor UI for it yet).
        let json = r#"{
            "id": "p", "name": "P", "packageType": "database",
            "fallbackLeague": { "name": "Custom Cup", "legs": 1, "scope": "Continental" }
        }"#;
        let meta: WorldMetaDef = serde_json::from_str(json).unwrap();
        let cfg = meta.fallback_league.as_ref().expect("config should deserialize");
        assert_eq!(cfg.name.as_deref(), Some("Custom Cup"));
        assert_eq!(cfg.legs, Some(1));
        assert_eq!(cfg.scope, Some(CompetitionScope::Continental));

        let serialized = serde_json::to_string(&meta).unwrap();
        assert!(serialized.contains("fallbackLeague"), "dropped on save: {serialized}");
        assert!(serialized.contains("Custom Cup"));

        // A manifest without the field deserializes to None and omits it on save.
        let bare: WorldMetaDef = serde_json::from_str(r#"{ "id": "p", "name": "P" }"#).unwrap();
        assert!(bare.fallback_league.is_none());
        assert!(!serde_json::to_string(&bare).unwrap().contains("fallbackLeague"));
    }

    fn temp_package() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ofm-pkg-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, rel: &str, contents: &str) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    /// Build a `.ofm` zip whose entries are `(name, bytes)` pairs, writing the
    /// names verbatim (so tests can inject traversal/zip-slip entry names).
    fn build_zip(entries: &[(&str, &[u8])]) -> PathBuf {
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        let path = std::env::temp_dir().join(format!("ofm-ziptest-{}.ofm", uuid::Uuid::new_v4()));
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, bytes) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    #[test]
    fn extract_archive_safely_rejects_zip_slip_entries() {
        let archive = build_zip(&[
            ("teams/teams.json", b"[]"),
            ("../escape.txt", b"pwned"),
            ("/abs.txt", b"pwned"),
        ]);
        let dest = temp_package();
        let errors = extract_archive_safely(&archive, &dest).unwrap();
        // The two unsafe entries are reported and never written.
        assert_eq!(errors.iter().filter(|e| e.code == ZIPSLIP_ERROR).count(), 2);
        assert!(dest.join("teams/teams.json").exists());
        // The traversal target (sibling of dest) must not have been created.
        assert!(!dest.parent().unwrap().join("escape.txt").exists());
        let _ = std::fs::remove_file(&archive);
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn extract_ofm_to_dir_aborts_on_zip_slip() {
        // A safe entry precedes the zip-slip entry, so extraction writes a file
        // before it hits the rejection — the cleanup must still wipe the dir.
        let archive = build_zip(&[("teams/teams.json", b"{}"), ("../escape.txt", b"pwned")]);
        let dest = temp_package();
        let result = super::super::world_io::extract_ofm_to_dir(&archive, &dest);
        assert_eq!(result, Err(ZIPSLIP_ERROR.to_string()));
        assert!(!dest.parent().unwrap().join("escape.txt").exists());
        // No partially-unpacked tree is left behind for the editor to open.
        assert!(!dest.exists(), "partial extraction directory should be removed");
        let _ = std::fs::remove_file(&archive);
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn player_def_deserializes_footedness_from_frontend_key() {
        // The package-editor frontend sends the chosen foot under the camelCase
        // key "footedness"; this pins that contract so a rename can't silently
        // drop the authored foot again.
        let def: PlayerDef =
            serde_json::from_str(r#"{"id":"p1","footedness":"Left"}"#).unwrap();
        assert_eq!(def.footedness.as_deref(), Some("Left"));
        // Round-trips back out under the same key.
        let json = serde_json::to_value(&def).unwrap();
        assert_eq!(json.get("footedness").and_then(|v| v.as_str()), Some("Left"));
    }

    #[test]
    fn read_entry_capped_rejects_oversized_entry() {
        let small: &[u8] = b"hello";
        assert_eq!(read_entry_capped(&mut &small[..], 1024), Some(small.to_vec()));
        // Exceeds the cap → None (no unbounded allocation).
        assert_eq!(read_entry_capped(&mut &small[..], 4), None);
    }

    const REAL_MADRID_YAML: &str = "\
schema: team
id: real-madrid
name: Real Madrid
city: Madrid
country: ES
colors:
  primary: \"#FEBE10\"
  secondary: \"#FFFFFF\"
";

    #[test]
    fn loads_single_entity_and_bulk_items_files() {
        let dir = temp_package();
        write(&dir, "real.yaml", REAL_MADRID_YAML);
        write(
            &dir,
            "more.yaml",
            "schema: team\nitems:\n  - { id: sevilla, name: Sevilla, city: Seville, country: ES, colors: { primary: \"#D80027\", secondary: \"#fff\" } }\n  - { id: betis, name: Real Betis, city: Seville, country: ES, colors: { primary: \"#00954C\", secondary: \"#fff\" } }\n",
        );

        let (package, errors) = load_world_package(&dir);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        let ids: Vec<&str> = package.teams.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["betis", "real-madrid", "sevilla"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn mixes_json_and_yaml() {
        let dir = temp_package();
        write(&dir, "real.yaml", REAL_MADRID_YAML);
        write(
            &dir,
            "country.json",
            r#"{ "schema": "country", "id": "ES", "name": "Spain", "confederation": "europe" }"#,
        );

        let (package, errors) = load_world_package(&dir);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(package.teams.len(), 1);
        assert_eq!(package.countries.len(), 1);
        assert_eq!(package.countries[0].name, "Spain");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn folder_layout_does_not_affect_the_result() {
        let flat = temp_package();
        write(&flat, "real.yaml", REAL_MADRID_YAML);
        write(
            &flat,
            "spain.json",
            r#"{ "schema": "country", "id": "ES", "name": "Spain", "confederation": "europe" }"#,
        );

        let nested = temp_package();
        write(&nested, "teams/europe/spain/real.yaml", REAL_MADRID_YAML);
        write(
            &nested,
            "deep/nested/dirs/spain.json",
            r#"{ "schema": "country", "id": "ES", "name": "Spain", "confederation": "europe" }"#,
        );

        let (flat_pkg, flat_errors) = load_world_package(&flat);
        let (nested_pkg, nested_errors) = load_world_package(&nested);
        assert!(flat_errors.is_empty() && nested_errors.is_empty());
        assert_eq!(flat_pkg.teams, nested_pkg.teams);
        assert_eq!(flat_pkg.countries, nested_pkg.countries);

        std::fs::remove_dir_all(&flat).ok();
        std::fs::remove_dir_all(&nested).ok();
    }

    #[test]
    fn reports_unknown_and_missing_schema() {
        let dir = temp_package();
        write(&dir, "weird.yaml", "schema: dragon\nid: smaug\n");
        write(&dir, "noschema.yaml", "id: nobody\nname: Nobody\n");

        let (_package, errors) = load_world_package(&dir);
        assert!(errors.iter().any(|e| e.code == UNKNOWN_SCHEMA));
        assert!(errors.iter().any(|e| e.code == MISSING_SCHEMA));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reports_duplicate_and_missing_ids() {
        let dir = temp_package();
        write(&dir, "a.yaml", REAL_MADRID_YAML);
        write(&dir, "b.yaml", REAL_MADRID_YAML); // same id again
        write(
            &dir,
            "noid.yaml",
            "schema: country\nname: Nowhere\nconfederation: europe\n",
        );

        let (_package, errors) = load_world_package(&dir);
        assert!(
            errors.iter().any(|e| e.code == DUPLICATE_ID),
            "expected a duplicate-id error: {errors:?}"
        );
        assert!(
            errors.iter().any(|e| e.code == MISSING_ID),
            "expected a missing-id error: {errors:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ignores_non_data_files() {
        let dir = temp_package();
        write(&dir, "real.yaml", REAL_MADRID_YAML);
        write(&dir, "README.md", "# My world package\n");
        write(&dir, "notes.txt", "scratch notes");

        let (package, errors) = load_world_package(&dir);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(package.teams.len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parses_a_competition_file_without_colliding_with_its_inner_type() {
        let dir = temp_package();
        write(
            &dir,
            "league.yaml",
            "schema: competition\nid: es-1\nname: La Liga\ntype: League\nscope: Domestic\nformat:\n  kind: LeagueTable\nparticipants:\n  selector:\n    kind: allInCountry\n    country: ES\n",
        );

        let (package, errors) = load_world_package(&dir);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(package.competitions.len(), 1);
        assert_eq!(package.competitions[0].id, "es-1");
        assert_eq!(package.competitions[0].r#type, domain::league::CompetitionType::League);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_fully_cross_referenced_package_is_valid() {
        let dir = temp_package();
        write(&dir, "confed.yaml", "schema: confederation\nid: galaxy\nname: Galaxy\n");
        write(
            &dir,
            "country.yaml",
            "schema: country\nid: ZZ\nname: Zedland\nconfederation: galaxy\n",
        );
        write(
            &dir,
            "teams.yaml",
            "schema: team\nitems:\n  - { id: zed-fc, name: Zed FC, city: Zedtown, country: ZZ, colors: { primary: \"#000\", secondary: \"#fff\" } }\n  - { id: zed-utd, name: Zed United, city: Zedford, country: ZZ, colors: { primary: \"#111\", secondary: \"#fff\" } }\n",
        );
        write(
            &dir,
            "player.yaml",
            "schema: player\nid: zed-star\nname: Zed Star\nclub: zed-fc\nnationality: ZZ\nposition: Forward\noverall: 80\n",
        );
        write(
            &dir,
            "league.yaml",
            "schema: competition\nid: zz-1\nname: Zed League\ntype: League\nscope: Domestic\nformat:\n  kind: LeagueTable\nparticipants:\n  selector:\n    kind: allInCountry\n    country: ZZ\n",
        );

        let (_package, errors) = load_world_package(&dir);
        assert!(errors.is_empty(), "expected a valid package, got: {errors:?}");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn references_to_the_builtin_catalog_resolve() {
        let dir = temp_package();
        // A country in the built-in `europe` region, and a club in the built-in
        // country `ES` — neither redefined in the package.
        write(
            &dir,
            "country.yaml",
            "schema: country\nid: CUSTOM\nname: Customland\nconfederation: europe\n",
        );
        write(
            &dir,
            "team.yaml",
            "schema: team\nid: madrid\nname: Madrid FC\ncity: Madrid\ncountry: ES\ncolors: { primary: \"#fff\", secondary: \"#000\" }\n",
        );

        let (_package, errors) = load_world_package(&dir);
        assert!(errors.is_empty(), "builtin refs should resolve: {errors:?}");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unknown_references_are_reported() {
        let dir = temp_package();
        write(
            &dir,
            "country.yaml",
            "schema: country\nid: ZZ\nname: Zedland\nconfederation: nowhere\n",
        );
        write(
            &dir,
            "team.yaml",
            "schema: team\nid: t1\nname: Orphan FC\ncity: Nowhere\ncountry: XX\ncolors: { primary: \"#000\", secondary: \"#fff\" }\n",
        );
        write(
            &dir,
            "player.yaml",
            "schema: player\nid: p1\nname: Lost Player\nclub: ghost\nnationality: XX\nposition: Midfielder\noverall: 70\n",
        );

        let (_package, errors) = load_world_package(&dir);
        let codes: Vec<&str> = errors.iter().map(|e| e.code.as_str()).collect();
        assert!(codes.contains(&UNKNOWN_CONFEDERATION), "{errors:?}");
        assert!(codes.contains(&UNKNOWN_TEAM), "{errors:?}");
        assert!(
            errors
                .iter()
                .filter(|e| e.code == UNKNOWN_COUNTRY)
                .count()
                >= 2,
            "both the team's and player's unknown country should be reported: {errors:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn staff_unknown_club_is_labeled_entity_not_player() {
        let dir = temp_package();
        write(
            &dir,
            "team.yaml",
            "schema: team\nid: t1\nname: Real FC\ncity: Town\ncountry: ENG\ncolors: { primary: \"#000\", secondary: \"#fff\" }\n",
        );
        write(
            &dir,
            "staff.yaml",
            "schema: staff\nid: coach-1\nfirstName: A\nlastName: B\nclub: ghost\nnationality: ENG\nrole: Coach\n",
        );

        let (_package, errors) = load_world_package(&dir);
        let err = errors
            .iter()
            .find(|e| e.code == UNKNOWN_TEAM)
            .expect("a staff member referencing a missing club should error");
        // The referencing entity is identified generically, never mislabeled "player".
        assert!(
            err.params.iter().any(|(k, v)| k == "entity" && v == "coach-1"),
            "{:?}",
            err.params
        );
        assert!(
            !err.params.iter().any(|(k, _)| k == "player"),
            "staff error must not use the player param: {:?}",
            err.params
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn builds_a_playable_world_from_a_package() {
        let dir = temp_package();
        write(&dir, "world.yaml", "schema: world\nname: Zed World\ndescription: A tiny world\n");
        write(&dir, "confed.yaml", "schema: confederation\nid: galaxy\nname: Galaxy\n");
        write(
            &dir,
            "country.yaml",
            "schema: country\nid: ZZ\nname: Zedland\nconfederation: galaxy\n",
        );
        write(
            &dir,
            "teams.yaml",
            "schema: team\nitems:\n  - { id: zed-fc, name: Zed FC, city: Zedtown, country: ZZ, colors: { primary: \"#000\", secondary: \"#fff\" } }\n  - { id: zed-utd, name: Zed United, city: Zedford, country: ZZ, colors: { primary: \"#111\", secondary: \"#fff\" } }\n",
        );
        write(
            &dir,
            "league.yaml",
            "schema: competition\nid: zz-1\nname: Zed League\ntype: League\nscope: Domestic\nformat:\n  kind: LeagueTable\nparticipants:\n  selector:\n    kind: allInCountry\n    country: ZZ\n",
        );

        let (package, errors) = load_world_package(&dir);
        assert!(errors.is_empty(), "package should be valid: {errors:?}");

        let world = crate::generator::build_world_data_from_package(&package);
        assert_eq!(world.name, "Zed World");
        let team_ids: Vec<&str> = world.teams.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(team_ids, vec!["zed-fc", "zed-utd"], "stable authored ids are kept");
        assert_eq!(world.players.len(), 44, "22 players per club are generated");

        let galaxy = world
            .regions
            .iter()
            .find(|r| r.id == "galaxy")
            .expect("the package's confederation becomes a region");
        assert!(galaxy.country_codes.contains(&"ZZ".to_string()));

        let defs = world
            .competition_definitions
            .as_ref()
            .expect("package competitions are embedded for resolution");
        assert_eq!(defs.competitions.len(), 1);
        assert_eq!(defs.competitions[0].id, "zz-1");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn authored_players_are_placed_in_their_clubs() {
        let dir = temp_package();
        write(&dir, "confed.yaml", "schema: confederation\nid: galaxy\nname: Galaxy\n");
        write(
            &dir,
            "country.yaml",
            "schema: country\nid: ZZ\nname: Zedland\nconfederation: galaxy\n",
        );
        write(
            &dir,
            "team.yaml",
            "schema: team\nid: zed-fc\nname: Zed FC\ncity: Zedtown\ncountry: ZZ\ncolors: { primary: \"#000\", secondary: \"#fff\" }\n",
        );
        write(
            &dir,
            "star.yaml",
            "schema: player\nid: zed-star\nname: Zed Star\nclub: zed-fc\nnationality: ZZ\nposition: Forward\noverall: 88\n",
        );

        let (package, errors) = load_world_package(&dir);
        assert!(errors.is_empty(), "{errors:?}");

        let world = crate::generator::build_world_data_from_package(&package);

        let star = world
            .players
            .iter()
            .find(|p| p.id == "zed-star")
            .expect("the authored player should be in the squad");
        assert_eq!(star.team_id.as_deref(), Some("zed-fc"));
        assert_eq!(star.full_name, "Zed Star");
        assert_eq!(star.position, domain::player::Position::Forward);
        assert!(
            star.ovr >= 72,
            "an overall of 88 should yield a high OVR, got {}",
            star.ovr
        );

        // The authored forward replaced a generated one, so the squad stays at 22.
        let squad = world
            .players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some("zed-fc"))
            .count();
        assert_eq!(squad, 22);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn competition_reference_errors_surface_as_package_errors() {
        let dir = temp_package();
        write(
            &dir,
            "league.yaml",
            "schema: competition\nid: bad-1\nname: Bad League\ntype: League\nscope: Domestic\nformat:\n  kind: LeagueTable\nparticipants:\n  selector:\n    kind: allInCountry\n    country: XX\n",
        );

        let (_package, errors) = load_world_package(&dir);
        assert!(
            errors
                .iter()
                .any(|e| e.code == "be.error.competitionDef.unknownCountry"),
            "competition validation should surface: {errors:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn translation_locale_from_filename_valid() {
        assert_eq!(translation_locale_from_filename("translations.en.json"), Some("en"));
        assert_eq!(translation_locale_from_filename("translations.pt-BR.json"), Some("pt-BR"));
        assert_eq!(translation_locale_from_filename("translations.zh-CN.json"), Some("zh-CN"));
    }

    #[test]
    fn translation_locale_from_filename_rejects_invalid() {
        // Empty locale between the two dots
        assert_eq!(translation_locale_from_filename("translations..json"), None);
        // Locale itself contains a dot (would create ambiguous multi-part names)
        assert_eq!(translation_locale_from_filename("translations.pt-BR.extra.json"), None);
        // No "translations." prefix
        assert_eq!(translation_locale_from_filename("en.json"), None);
        // Not a JSON file
        assert_eq!(translation_locale_from_filename("translations.en.yaml"), None);
        // Completely wrong name
        assert_eq!(translation_locale_from_filename("competition.json"), None);
    }

    // -----------------------------------------------------------------------
    // Adversarial / stress tests — document current survival behavior.
    // Lines marked BUG: indicate gaps the implementation should later fix.
    // Lines marked OK: mean the system already handles this correctly.
    // -----------------------------------------------------------------------

    // Helper: build a minimal package in a dir and return the WorldPackage + errors.
    fn package_from_files(files: &[(&str, &str)]) -> (WorldPackage, Vec<PackageError>, PathBuf) {
        let dir = temp_package();
        for (name, content) in files {
            write(&dir, name, content);
        }
        let (pkg, errs) = load_world_package(&dir);
        (pkg, errs, dir)
    }

    const TEAM_A: &str = "schema: team\nid: team-a\nname: Team A\ncity: City A\ncountry: ES\ncolors: { primary: \"#111\", secondary: \"#fff\" }\n";
    const TEAM_B: &str = "schema: team\nid: team-b\nname: Team B\ncity: City B\ncountry: ES\ncolors: { primary: \"#222\", secondary: \"#fff\" }\n";
    const TEAM_A_ALT: &str = "schema: team\nid: team-a\nname: Team A (Alternate)\ncity: Other City\ncountry: ES\ncolors: { primary: \"#333\", secondary: \"#000\" }\n";

    // --- Merge: db-db id clash (different content) ---------------------------

    #[test]
    fn merge_db_db_id_clash_surfaces_stack_conflict_warning() {
        // Two "database" packages with the same team id but different content
        // should surface a StackConflict warning. Last-wins still applies for
        // the merge, but the caller can show the user a conflict notice.
        let dir_a = temp_package();
        write(&dir_a, "world.yaml", "schema: world\nid: pkg-a\nname: Pkg A\npackageType: database\n");
        write(&dir_a, "team.yaml", TEAM_A);
        let (pkg_a, errs_a) = load_world_package(&dir_a);
        assert!(errs_a.is_empty());

        let dir_b = temp_package();
        write(&dir_b, "world.yaml", "schema: world\nid: pkg-b\nname: Pkg B\npackageType: database\n");
        write(&dir_b, "team.yaml", TEAM_A_ALT);
        let (pkg_b, errs_b) = load_world_package(&dir_b);
        assert!(errs_b.is_empty());

        let conflicts = validate_package_stack(&[&pkg_a, &pkg_b]);
        assert!(
            conflicts.iter().any(|c| c.entity_id == "team-a" && c.severity == ConflictSeverity::Warning),
            "expected a Warning conflict for team-a db-db clash: {conflicts:?}"
        );

        // Merge still works; last-in-stack wins.
        let (merged, merge_errors) = merge_world_packages(vec![pkg_a, pkg_b]);
        assert!(merge_errors.is_empty());
        assert_eq!(merged.teams.len(), 1);
        assert_eq!(merged.teams[0].name, "Team A (Alternate)");

        std::fs::remove_dir_all(&dir_a).ok();
        std::fs::remove_dir_all(&dir_b).ok();
    }

    #[test]
    fn merge_db_db_id_clash_identical_content_is_fine() {
        // Two packages declaring the exact same team should dedup silently (no error).
        // This is the "international tournament references base-league teams" case.
        let dir_a = temp_package();
        write(&dir_a, "team.yaml", TEAM_A);
        let (pkg_a, _) = load_world_package(&dir_a);

        let dir_b = temp_package();
        write(&dir_b, "team.yaml", TEAM_A); // identical
        let (pkg_b, _) = load_world_package(&dir_b);

        let (merged, merge_errors) = merge_world_packages(vec![pkg_a, pkg_b]);
        // OK: identical content, last-wins dedup, no error
        assert!(merge_errors.is_empty());
        assert_eq!(merged.teams.len(), 1);

        std::fs::remove_dir_all(&dir_a).ok();
        std::fs::remove_dir_all(&dir_b).ok();
    }

    // --- Merge: meta is wholesale replaced -----------------------------------

    #[test]
    fn merge_meta_unions_default_active_competitions_across_packages() {
        // Stacking PL + CL should union both defaultActiveCompetitions lists.
        let dir_pl = temp_package();
        write(
            &dir_pl,
            "world.yaml",
            "schema: world\nid: pl\nname: Premier League\ndefaultActiveCompetitions:\n  - pl-1\n",
        );
        let (pkg_pl, _) = load_world_package(&dir_pl);

        let dir_cl = temp_package();
        write(
            &dir_cl,
            "world.yaml",
            "schema: world\nid: cl\nname: Champions League\ndefaultActiveCompetitions:\n  - cl-1\n",
        );
        let (pkg_cl, _) = load_world_package(&dir_cl);

        let (merged, _) = merge_world_packages(vec![pkg_pl, pkg_cl]);
        let meta = merged.meta.as_ref().expect("merged meta should exist");
        // Both competition ids must be present after the union.
        assert!(
            meta.default_active_competitions.contains(&"pl-1".to_string()),
            "pl-1 must survive the merge: {:?}", meta.default_active_competitions
        );
        assert!(
            meta.default_active_competitions.contains(&"cl-1".to_string()),
            "cl-1 must survive the merge: {:?}", meta.default_active_competitions
        );

        std::fs::remove_dir_all(&dir_pl).ok();
        std::fs::remove_dir_all(&dir_cl).ok();
    }

    #[test]
    fn merge_database_with_patch_still_validates_dangling_competitions() {
        // A database referencing a competition it never defines, stacked under a
        // patch. Previously the merged package_type became "patch" (last-wins),
        // which suppressed the dangling-reference check for the whole stack.
        let dir_db = temp_package();
        write(
            &dir_db,
            "world.yaml",
            "schema: world\nid: db\nname: DB\ndefaultActiveCompetitions:\n  - missing-comp\n",
        );
        let (pkg_db, _) = load_world_package(&dir_db);

        let dir_patch = temp_package();
        write(
            &dir_patch,
            "world.yaml",
            "schema: world\nid: patch\nname: Patch\npackageType: patch\n",
        );
        let (pkg_patch, _) = load_world_package(&dir_patch);

        let (merged, errors) = merge_world_packages(vec![pkg_db, pkg_patch]);
        assert_eq!(merged.meta.as_ref().unwrap().package_type, "database");
        assert!(
            errors.iter().any(|e| e.code == UNKNOWN_COMPETITION),
            "dangling defaultActiveCompetitions must be flagged after merge: {errors:?}"
        );

        std::fs::remove_dir_all(&dir_db).ok();
        std::fs::remove_dir_all(&dir_patch).ok();
    }

    #[test]
    fn merge_unions_name_pools_across_packages() {
        // Each package supplies a distinct pool; both must survive the merge
        // instead of the last package's names wholesale-replacing the first.
        let dir_a = temp_package();
        write(&dir_a, "world.yaml", "schema: world\nid: a\nname: A\n");
        write(
            &dir_a,
            "names.yaml",
            "schema: names\nversion: 1\npools:\n  ENG:\n    first_names:\n      - John\n    last_names:\n      - Smith\n",
        );
        let (pkg_a, _) = load_world_package(&dir_a);

        let dir_b = temp_package();
        write(&dir_b, "world.yaml", "schema: world\nid: b\nname: B\n");
        write(
            &dir_b,
            "names.yaml",
            "schema: names\nversion: 1\npools:\n  BRA:\n    first_names:\n      - Joao\n    last_names:\n      - Silva\n",
        );
        let (pkg_b, _) = load_world_package(&dir_b);

        let (merged, _) = merge_world_packages(vec![pkg_a, pkg_b]);
        let names = merged.names.as_ref().expect("merged names should exist");
        let keys: Vec<_> = names.pools.keys().cloned().collect();
        assert!(names.pools.contains_key("ENG"), "ENG pool kept: {keys:?}");
        assert!(names.pools.contains_key("BRA"), "BRA pool kept: {keys:?}");

        std::fs::remove_dir_all(&dir_a).ok();
        std::fs::remove_dir_all(&dir_b).ok();
    }

    // --- Thin packages: teams but no competitions ----------------------------

    #[test]
    fn teams_without_competitions_auto_generates_fallback_league() {
        // Package with 4 teams and no competition should auto-generate a fallback
        // single-division league containing all 4 teams, and emit a build notice.
        let (pkg, errors, dir) = package_from_files(&[
            ("a.yaml", TEAM_A),
            ("b.yaml", TEAM_B),
            ("c.yaml", "schema: team\nid: team-c\nname: Team C\ncity: City C\ncountry: ES\ncolors: { primary: \"#444\", secondary: \"#fff\" }\n"),
            ("d.yaml", "schema: team\nid: team-d\nname: Team D\ncity: City D\ncountry: ES\ncolors: { primary: \"#555\", secondary: \"#fff\" }\n"),
        ]);
        assert!(errors.is_empty());
        let world = crate::generator::build_world_data_from_package(&pkg);
        assert_eq!(world.teams.len(), 4);
        // Fallback league must be generated.
        let defs = world.competition_definitions.as_ref()
            .expect("fallback league should be auto-generated");
        assert_eq!(defs.competitions.len(), 1);
        assert_eq!(defs.competitions[0].id, "ofm-fallback-league");
        let explicit = defs.competitions[0].participants.explicit.as_ref()
            .expect("fallback uses explicit participant list");
        assert_eq!(explicit.len(), 4, "all 4 teams included");
        // Build notice must be present.
        assert!(
            world.build_notices.iter().any(|n| n == "be.error.notice.fallbackLeagueGenerated"),
            "build notice must be emitted: {:?}", world.build_notices
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn single_team_no_competitions_fills_procedural_opponents() {
        // 1 authored team + no competitions → 7 procedural fillers generated,
        // totalling 8 teams, and a fallback league covering all of them.
        let (pkg, errors, dir) = package_from_files(&[("a.yaml", TEAM_A)]);
        assert!(errors.is_empty());
        let world = crate::generator::build_world_data_from_package(&pkg);
        assert_eq!(world.teams.len(), 8, "should fill to THIN_PACKAGE_MIN_TEAMS");
        assert!(
            world.competition_definitions.is_some(),
            "fallback league should be generated after fill"
        );
        assert!(
            world.build_notices.iter().any(|n| n == "be.error.notice.fallbackTeamsFilled"),
            "must warn player that filler teams were added: {:?}",
            world.build_notices
        );
        assert!(
            world.build_notices.iter().any(|n| n == "be.error.notice.fallbackLeagueGenerated"),
            "must also warn about fallback league: {:?}",
            world.build_notices
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    // --- Competition format vs participant count ------------------------------

    #[test]
    fn knockout_competition_with_one_explicit_team_already_errors() {
        // OK: competition validation already catches a Knockout with only 1
        // explicit participant. This is better than expected — no fix needed here.
        let (_, errors, dir) = package_from_files(&[
            ("a.yaml", TEAM_A),
            (
                "cup.yaml",
                "schema: competition\nid: broken-cup\nname: Broken Cup\ntype: Cup\nscope: Domestic\nformat:\n  kind: Knockout\nparticipants:\n  explicit:\n    - team-a\n",
            ),
        ]);
        assert!(
            !errors.is_empty(),
            "OK: already catches < 2 explicit participants in a Knockout"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn league_with_competition_referencing_nonexistent_teams_errors() {
        // OK: explicit participants pointing at team ids not in the package
        // are caught by competition reference validation.
        let (_, errors, dir) = package_from_files(&[(
            "cup.yaml",
            "schema: competition\nid: ghost-league\nname: Ghost League\ntype: League\nscope: Domestic\nformat:\n  kind: LeagueTable\nparticipants:\n  explicit:\n    - ghost-team-1\n    - ghost-team-2\n",
        )]);
        assert!(
            !errors.is_empty(),
            "OK: dangling explicit participants should error"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    // --- Dangling defaultActiveCompetitions ----------------------------------

    #[test]
    fn dangling_default_active_competition_is_an_error() {
        // A package whose defaultActiveCompetitions points to a competition id
        // not defined in the package must now produce an error.
        let (_, errors, dir) = package_from_files(&[
            ("a.yaml", TEAM_A),
            (
                "world.yaml",
                "schema: world\nid: test\nname: Test\ndefaultActiveCompetitions:\n  - nonexistent-competition\n",
            ),
        ]);
        assert!(
            errors.iter().any(|e| e.code == UNKNOWN_COMPETITION),
            "dangling defaultActiveCompetitions ref must error: {errors:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dangling_default_active_region_is_an_error() {
        let (_, errors, dir) = package_from_files(&[
            ("a.yaml", TEAM_A),
            (
                "world.yaml",
                "schema: world\nid: test\nname: Test\ndefaultActiveRegions:\n  - nowhere-land\n",
            ),
        ]);
        assert!(
            errors.iter().any(|e| e.code == UNKNOWN_REGION),
            "dangling defaultActiveRegions ref must error: {errors:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn builtin_default_active_region_is_accepted() {
        // A built-in region id (e.g. `europe`) needs no package-defined confederation.
        let (_, errors, dir) = package_from_files(&[
            ("a.yaml", TEAM_A),
            (
                "world.yaml",
                "schema: world\nid: test\nname: Test\ndefaultActiveRegions:\n  - europe\n",
            ),
        ]);
        assert!(
            !errors.iter().any(|e| e.code == UNKNOWN_REGION),
            "built-in region must be accepted: {errors:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    // --- Content values out of plausible range -------------------------------

    #[test]
    fn reversed_reputation_range_is_an_error() {
        // reputationRange where min > max (e.g. [900, 100]) must now produce an error.
        let (_, errors, dir) = package_from_files(&[(
            "a.yaml",
            "schema: team\nid: team-a\nname: Team A\ncity: City A\ncountry: ES\ncolors: { primary: \"#111\", secondary: \"#fff\" }\nreputationRange: [900, 100]\n",
        )]);
        assert!(
            errors.iter().any(|e| e.code == REVERSED_RANGE),
            "reversed reputationRange must produce a REVERSED_RANGE error: {errors:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reputation_above_1000_is_out_of_range() {
        let (_, errors, dir) = package_from_files(&[(
            "a.yaml",
            "schema: team\nid: team-a\nname: Team A\ncity: City A\ncountry: ES\ncolors: { primary: \"#111\", secondary: \"#fff\" }\nreputationRange: [500, 2000]\n",
        )]);
        assert!(
            errors.iter().any(|e| e.code == OUT_OF_RANGE),
            "reputation above 1000 must produce an OUT_OF_RANGE error: {errors:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn negative_finance_is_out_of_range() {
        let (_, errors, dir) = package_from_files(&[(
            "a.yaml",
            "schema: team\nid: team-a\nname: Team A\ncity: City A\ncountry: ES\ncolors: { primary: \"#111\", secondary: \"#fff\" }\nfinanceRange: [-100, 500000]\n",
        )]);
        assert!(
            errors.iter().any(|e| e.code == OUT_OF_RANGE),
            "negative finance must produce an OUT_OF_RANGE error: {errors:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn in_bounds_ranges_are_accepted() {
        let (_, errors, dir) = package_from_files(&[(
            "a.yaml",
            "schema: team\nid: team-a\nname: Team A\ncity: City A\ncountry: ES\ncolors: { primary: \"#111\", secondary: \"#fff\" }\nreputationRange: [300, 900]\nfinanceRange: [500000, 10000000]\n",
        )]);
        assert!(
            !errors.iter().any(|e| e.code == OUT_OF_RANGE || e.code == REVERSED_RANGE),
            "valid ranges must not error: {errors:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    // --- Hostile content in string fields ------------------------------------

    #[test]
    fn xss_and_sql_in_names_are_stored_as_plain_strings() {
        // OK: the package loader is JSON/YAML → struct; no query or HTML
        // construction happens at load time. Hostile strings are stored literally.
        let (pkg, errors, dir) = package_from_files(&[(
            "a.yaml",
            "schema: team\nid: xss-team\nname: \"<script>alert(1)</script>\"\ncity: \"'; DROP TABLE teams; --\"\ncountry: ES\ncolors: { primary: \"#111\", secondary: \"#fff\" }\n",
        )]);
        assert!(errors.is_empty(), "hostile strings are not a parse error");
        assert_eq!(pkg.teams[0].name, "<script>alert(1)</script>");
        // The UI layer is responsible for escaping — React does this automatically.
        std::fs::remove_dir_all(&dir).ok();
    }

    // --- Zero-teams hard error (already correct) -----------------------------

    #[test]
    fn zero_teams_hard_error_at_game_start() {
        // OK: stacking packages that produce 0 teams after merge returns an error.
        let dir = temp_package();
        write(&dir, "world.yaml", "schema: world\nid: empty\nname: Empty World\n");
        let (pkg, _) = load_world_package(&dir);
        let world = crate::generator::build_world_data_from_package(&pkg);
        // The world builds but has no teams; game.rs rejects this as noDatabasePackage.
        assert!(world.teams.is_empty(), "OK: correctly produces 0 teams");
        std::fs::remove_dir_all(&dir).ok();
    }

    // --- Cross-package reference (intended use case) -------------------------

    #[test]
    fn cross_package_reference_resolves_after_merge() {
        // OK: a CL package referencing teams from a PL package works after merge.
        let dir_pl = temp_package();
        write(&dir_pl, "team.yaml", TEAM_A);
        write(&dir_pl, "team2.yaml", TEAM_B);
        let (pkg_pl, _) = load_world_package(&dir_pl);

        let dir_cl = temp_package();
        write(
            &dir_cl,
            "cup.yaml",
            "schema: competition\nid: intl-cup\nname: International Cup\ntype: ContinentalClub\nscope: Continental\nformat:\n  kind: GroupAndKnockout\nparticipants:\n  explicit:\n    - team-a\n    - team-b\n    - team-a\n    - team-b\n",
        );
        let (pkg_cl, _) = load_world_package(&dir_cl);

        let (_merged, merge_errors) = merge_world_packages(vec![pkg_pl, pkg_cl]);
        // OK: after merge, team-a and team-b resolve correctly for the CL competition.
        // Duplicate explicit participants are the only issue here.
        let ref_errors: Vec<_> = merge_errors
            .iter()
            .filter(|e| e.code == UNKNOWN_TEAM)
            .collect();
        assert!(
            ref_errors.is_empty(),
            "OK: cross-package team refs resolve after merge: {ref_errors:?}"
        );

        std::fs::remove_dir_all(&dir_pl).ok();
        std::fs::remove_dir_all(&dir_cl).ok();
    }

    // --- Patch package override (future: should be silent last-wins) ---------

    #[test]
    fn patch_package_overrides_database_team_without_conflict_warning() {
        // A "patch" package overriding a "database" team should be silent:
        // no StackConflict, but the patch's version wins in the merge.
        let dir_db = temp_package();
        write(&dir_db, "world.yaml", "schema: world\nid: base-db\nname: Base DB\npackageType: database\n");
        write(&dir_db, "team.yaml", TEAM_A);
        let (pkg_db, _) = load_world_package(&dir_db);

        let dir_patch = temp_package();
        write(&dir_patch, "world.yaml", "schema: world\nid: team-a-patch\nname: Team A Stats Patch\npackageType: patch\n");
        write(&dir_patch, "team.yaml", TEAM_A_ALT);
        let (pkg_patch, _) = load_world_package(&dir_patch);

        // No conflict: patch-over-database is intentional.
        let conflicts = validate_package_stack(&[&pkg_db, &pkg_patch]);
        assert!(
            !conflicts.iter().any(|c| c.entity_id == "team-a"),
            "patch override must not generate a conflict: {conflicts:?}"
        );

        // Patch wins in the merge.
        let (merged, _) = merge_world_packages(vec![pkg_db, pkg_patch]);
        assert_eq!(merged.teams[0].name, "Team A (Alternate)", "patch version must win");

        std::fs::remove_dir_all(&dir_db).ok();
        std::fs::remove_dir_all(&dir_patch).ok();
    }
}
