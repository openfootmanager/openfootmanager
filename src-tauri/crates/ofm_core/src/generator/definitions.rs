use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::data::NATIONALITY_POOLS;
#[cfg(test)]
use super::data::TEAM_TEMPLATES;

// ---------------------------------------------------------------------------
// Definition file types (JSON-serialisable)
// ---------------------------------------------------------------------------

/// Name pools definition file format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamesDefinition {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub description: String,
    /// Keyed by ISO 3166-1 alpha-2 country code.
    pub pools: HashMap<String, NamePool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamePool {
    pub first_names: Vec<String>,
    pub last_names: Vec<String>,
}

/// Team templates definition file format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsDefinition {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub description: String,
    pub teams: Vec<TeamDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TeamDef {
    /// Stable id used to reference this club (e.g. from player or competition
    /// files). Empty for procedurally generated clubs, which get a UUID.
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default, alias = "short_name")]
    pub short_name: String,
    pub city: String,
    /// ISO 3166-1 alpha-2 / football country code.
    pub country: String,
    pub colors: TeamColorsDef,
    #[serde(default = "default_play_style", alias = "play_style")]
    pub play_style: String,
    #[serde(default, alias = "stadium_name")]
    pub stadium_name: String,
    #[serde(default, alias = "reputation_range")]
    pub reputation_range: Option<[u32; 2]>,
    #[serde(default, alias = "finance_range")]
    pub finance_range: Option<[i64; 2]>,
    /// Optional path to a logo/crest image, relative to the package root.
    /// Populated with an absolute path after the package is extracted.
    #[serde(default)]
    pub logo: Option<String>,
    /// Kit jersey pattern (Solid, Stripes, Hoops, HalfAndHalf, Diagonal).
    #[serde(default, alias = "kit_pattern")]
    pub kit_pattern: Option<String>,
}

fn default_play_style() -> String {
    "Balanced".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamColorsDef {
    pub primary: String,
    pub secondary: String,
}

/// Try to load a names definition from a JSON or YAML file, returning None on
/// any error.
pub fn load_names_definition(path: &std::path::Path) -> Option<NamesDefinition> {
    super::file_format::load_definition_file(path)
}

/// Try to load a teams definition from a JSON or YAML file, returning None on
/// any error.
pub fn load_teams_definition(path: &std::path::Path) -> Option<TeamsDefinition> {
    super::file_format::load_definition_file(path)
}

/// Build the hardcoded names definition as fallback.
pub(super) fn default_names_definition() -> NamesDefinition {
    let mut pools = HashMap::new();
    for entry in NATIONALITY_POOLS {
        pools.insert(
            entry.nationality.to_string(),
            NamePool {
                first_names: entry.first_names.iter().map(|s| s.to_string()).collect(),
                last_names: entry.last_names.iter().map(|s| s.to_string()).collect(),
            },
        );
    }
    NamesDefinition {
        version: 1,
        description: "Built-in default".to_string(),
        pools,
    }
}

/// Build the hardcoded teams definition. Retained as a test fixture now that
/// the shipped world is generated procedurally.
#[cfg(test)]
pub(super) fn default_teams_definition() -> TeamsDefinition {
    TeamsDefinition {
        version: 1,
        description: "Built-in default".to_string(),
        teams: TEAM_TEMPLATES
            .iter()
            .map(|t| TeamDef {
                id: String::new(),
                name: t.name.to_string(),
                short_name: t
                    .name
                    .split_whitespace()
                    .filter_map(|w| w.chars().next())
                    .collect::<String>()
                    .to_uppercase()
                    .chars()
                    .take(3)
                    .collect(),
                city: t.city.to_string(),
                country: t.country.to_string(),
                colors: TeamColorsDef {
                    primary: t.colors.0.to_string(),
                    secondary: t.colors.1.to_string(),
                },
                play_style: t.play_style.to_string(),
                stadium_name: format!("{} Arena", t.city),
                reputation_range: Some([300, 900]),
                finance_range: Some([500_000, 10_000_000]),
                logo: None,
                kit_pattern: None,
            })
            .collect(),
    }
}

/// Serialisable world database — can be saved to / loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum WorldDataKind {
    #[default]
    RosterBaseline,
    HistoricalSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldDataMetadata {
    #[serde(default)]
    pub format_version: u32,
    #[serde(default)]
    pub world_id: String,
    #[serde(default)]
    pub kind: WorldDataKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_year: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_date: Option<String>,
}

impl Default for WorldDataMetadata {
    fn default() -> Self {
        Self {
            format_version: 1,
            world_id: Uuid::new_v4().to_string(),
            kind: WorldDataKind::RosterBaseline,
            base_year: None,
            snapshot_date: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorldRegionDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub country_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorldShardRefs {
    pub teams: String,
    pub players: String,
    pub staff: String,
    pub managers: String,
    pub competitions: String,
    pub national_teams: String,
    pub news: String,
    pub stats: String,
    pub world_history: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorldManifestV2 {
    pub format_version: u32,
    pub world_id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub regions: Vec<WorldRegionDefinition>,
    #[serde(default)]
    pub default_active_regions: Vec<String>,
    #[serde(default)]
    pub default_active_competitions: Vec<String>,
    pub shards: WorldShardRefs,
    #[serde(default)]
    pub compatibility: Option<WorldDataMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WorldData {
    pub name: String,
    pub description: String,
    pub teams: Vec<domain::team::Team>,
    pub players: Vec<domain::player::Player>,
    pub staff: Vec<domain::staff::Staff>,
    pub managers: Vec<domain::manager::Manager>,
    pub competitions: Vec<domain::league::CompetitionState>,
    /// Optional authored competition definitions resolved at game creation.
    #[serde(
        default,
        rename = "competitionDefinitions",
        skip_serializing_if = "Option::is_none"
    )]
    pub competition_definitions: Option<super::competition_def::CompetitionDefinitionFile>,
    pub national_teams: Vec<domain::national_team::NationalTeam>,
    pub regions: Vec<WorldRegionDefinition>,
    pub default_active_regions: Vec<String>,
    pub default_active_competitions: Vec<String>,
    pub league: Option<domain::league::League>,
    pub news: Vec<domain::news::NewsArticle>,
    pub stats: domain::stats::StatsState,
    pub world_history: domain::world_history::WorldHistoryArchive,
    pub metadata: WorldDataMetadata,
    /// Per-locale translation bundles supplied by a world package, keyed by
    /// locale code. Carried to the game state so the frontend can merge them
    /// into the active i18n namespace when loading a custom world.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub extra_translations: std::collections::HashMap<String, serde_json::Value>,
    /// Backend i18n notice keys generated during world build (e.g. auto-fallback
    /// league creation). Not persisted to save files; cleared on load.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub build_notices: Vec<String>,
}

/// Lightweight metadata shown in the UI when listing available databases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldDatabaseInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub team_count: usize,
    pub player_count: usize,
    pub history_mode: String,
    pub base_year: Option<i32>,
    pub snapshot_date: Option<String>,
    /// "builtin" | "user"
    pub source: String,
    /// Filesystem path (empty for built-in random)
    pub path: String,
}
