use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::data::{NATIONALITY_POOLS, TEAM_TEMPLATES};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamDef {
    pub name: String,
    #[serde(default)]
    pub short_name: String,
    pub city: String,
    /// ISO 3166-1 alpha-2 country code.
    pub country: String,
    pub colors: TeamColorsDef,
    #[serde(default = "default_play_style")]
    pub play_style: String,
    #[serde(default)]
    pub stadium_name: String,
    #[serde(default)]
    pub reputation_range: Option<[u32; 2]>,
    #[serde(default)]
    pub finance_range: Option<[i64; 2]>,
}

fn default_play_style() -> String {
    "Balanced".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamColorsDef {
    pub primary: String,
    pub secondary: String,
}

/// Try to load a names definition from a file, returning None on any error.
pub fn load_names_definition(path: &std::path::Path) -> Option<NamesDefinition> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Try to load a teams definition from a file, returning None on any error.
pub fn load_teams_definition(path: &std::path::Path) -> Option<TeamsDefinition> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
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

/// Build the hardcoded teams definition as fallback.
pub(super) fn default_teams_definition() -> TeamsDefinition {
    TeamsDefinition {
        version: 1,
        description: "Built-in default".to_string(),
        teams: TEAM_TEMPLATES
            .iter()
            .map(|t| TeamDef {
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
    pub kind: WorldDataKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_year: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_date: Option<String>,
}

impl Default for WorldDataMetadata {
    fn default() -> Self {
        Self {
            kind: WorldDataKind::RosterBaseline,
            base_year: None,
            snapshot_date: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorldData {
    pub name: String,
    pub description: String,
    pub teams: Vec<domain::team::Team>,
    pub players: Vec<domain::player::Player>,
    pub staff: Vec<domain::staff::Staff>,
    pub managers: Vec<domain::manager::Manager>,
    pub league: Option<domain::league::League>,
    pub news: Vec<domain::news::NewsArticle>,
    pub stats: domain::stats::StatsState,
    pub world_history: domain::world_history::WorldHistoryArchive,
    pub metadata: WorldDataMetadata,
}

impl Default for WorldData {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            teams: Vec::new(),
            players: Vec::new(),
            staff: Vec::new(),
            managers: Vec::new(),
            league: None,
            news: Vec::new(),
            stats: domain::stats::StatsState::default(),
            world_history: domain::world_history::WorldHistoryArchive::default(),
            metadata: WorldDataMetadata::default(),
        }
    }
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
