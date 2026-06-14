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
}

/// Package-level metadata (at most one per package).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldMetaDef {
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
pub fn load_world_package(dir: &Path) -> (WorldPackage, Vec<PackageError>) {
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
}
