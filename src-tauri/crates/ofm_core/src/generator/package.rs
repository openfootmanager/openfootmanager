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

use domain::player::{PlayerAttributes, Position};

use super::{CompetitionDefinition, NamesDefinition, TeamDef};

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
}

/// Everything a package declares, aggregated across all its files.
#[derive(Debug, Default)]
pub struct WorldPackage {
    pub meta: Option<WorldMetaDef>,
    pub confederations: Vec<ConfederationDef>,
    pub countries: Vec<CountryDef>,
    pub teams: Vec<TeamDef>,
    pub players: Vec<PlayerDef>,
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
                    .with("player", &player.id)
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

    errors.extend(validate_competition_references(package));
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

/// Merge multiple packages into one, with last-wins semantics for duplicate ids,
/// then run full id + reference validation on the combined result. This is the
/// primitive that makes cross-package references work: a Champions League
/// package can reference teams defined in a Premier League package as long as
/// both are included in the stack.
pub fn merge_world_packages(packages: Vec<WorldPackage>) -> (WorldPackage, Vec<PackageError>) {
    use std::collections::BTreeMap;

    let mut merged = WorldPackage::default();
    let mut confeds: BTreeMap<String, ConfederationDef> = BTreeMap::new();
    let mut countries: BTreeMap<String, CountryDef> = BTreeMap::new();
    let mut teams: BTreeMap<String, TeamDef> = BTreeMap::new();
    let mut players: BTreeMap<String, PlayerDef> = BTreeMap::new();
    let mut competitions: BTreeMap<String, CompetitionDefinition> = BTreeMap::new();

    for package in packages {
        if package.meta.is_some() {
            merged.meta = package.meta;
        }
        for c in package.confederations {
            confeds.insert(c.id.clone(), c);
        }
        for c in package.countries {
            countries.insert(c.id.clone(), c);
        }
        for t in package.teams {
            teams.insert(t.id.clone(), t);
        }
        for p in package.players {
            players.insert(p.id.clone(), p);
        }
        for c in package.competitions {
            competitions.insert(c.id.clone(), c);
        }
        if package.names.is_some() {
            merged.names = package.names;
        }
        for (locale, bundle) in package.extra_translations {
            merged.extra_translations.insert(locale, bundle);
        }
    }

    merged.confederations = confeds.into_values().collect();
    merged.countries = countries.into_values().collect();
    merged.teams = teams.into_values().collect();
    merged.players = players.into_values().collect();
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

/// Extract a `.ofm` zip archive to a temp directory, load the package from it,
/// clean up, and return. Zip-slip paths are silently skipped.
pub fn load_world_package_from_ofm(path: &Path) -> (WorldPackage, Vec<PackageError>) {
    use std::io::Read;

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (WorldPackage::default(), vec![PackageError::new(READ_FAILED, "")]),
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return (WorldPackage::default(), vec![PackageError::new(READ_FAILED, "")]),
    };
    let temp_dir =
        std::env::temp_dir().join(format!("ofm-extract-{}", uuid::Uuid::new_v4()));
    if std::fs::create_dir_all(&temp_dir).is_err() {
        return (WorldPackage::default(), vec![PackageError::new(READ_FAILED, "")]);
    }

    if archive.len() > MAX_FILE_COUNT {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return (
            WorldPackage::default(),
            vec![PackageError::new(TOO_MANY_FILES_ERROR, "")],
        );
    }

    let mut extract_errors = Vec::new();
    let mut total_uncompressed: u64 = 0;
    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        if entry.is_symlink() {
            let name = entry.name().to_string();
            extract_errors.push(PackageError::new(SYMLINK_ERROR, &name));
            continue;
        }
        let entry_name = entry.name().to_string();
        let Some(dest) = safe_entry_path(&temp_dir, &entry_name) else {
            extract_errors.push(PackageError::new(ZIPSLIP_ERROR, &entry_name));
            continue;
        };
        if let Some(parent) = dest.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                extract_errors.push(PackageError::new(READ_FAILED, &entry_name));
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
                        let _ = std::fs::remove_dir_all(&temp_dir);
                        return (
                            WorldPackage::default(),
                            vec![PackageError::new(ARCHIVE_TOO_LARGE_ERROR, "")],
                        );
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
            extract_errors.push(PackageError::new(READ_FAILED, &entry_name));
            continue;
        }
        if std::fs::write(&dest, &buf).is_err() {
            extract_errors.push(PackageError::new(READ_FAILED, &entry_name));
        }
    }

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
    use std::io::Read;

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
        let mut text = String::new();
        if entry.read_to_string(&mut text).is_err() {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
