use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::data::NATIONALITY_POOLS;

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
    #[serde(default)]
    pub structure: Option<TeamsStructureDefinition>,
    #[serde(default)]
    pub teams: Vec<TeamDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsStructureDefinition {
    pub nations: Vec<NationStructureDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NationStructureDefinition {
    pub name: String,
    pub code: String,
    pub leagues: Vec<LeagueStructureDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeagueStructureDefinition {
    pub name: String,
    pub club_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamDef {
    pub name: String,
    #[serde(default)]
    pub short_name: String,
    pub city: String,
    /// ISO 3166-1 alpha-2 country code.
    pub country: String,
    #[serde(default)]
    pub league: String,
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
    const NATIONS: [(&str, &str, [&str; 2]); 4] = [
        ("England", "ENG", ["Premier Division", "Championship"]),
        ("Spain", "ESP", ["Primera Division", "Segunda Division"]),
        ("Germany", "GER", ["Bundesliga", "2. Bundesliga"]),
        ("France", "FRA", ["Ligue 1", "Ligue 2"]),
    ];
    let mut teams = Vec::new();
    for (country_name, code, leagues) in NATIONS {
        for league_name in leagues {
            for index in 1..=10 {
                let team_name = format!("{code} {league_name} Club {index:02}");
                let city = format!("{code} City {index:02}");
                teams.push(TeamDef {
                    name: team_name.clone(),
                    short_name: format!("{code}{index:02}"),
                    city: city.clone(),
                    country: country_name.to_string(),
                    league: league_name.to_string(),
                    colors: TeamColorsDef {
                        primary: "#1d4ed8".to_string(),
                        secondary: "#ffffff".to_string(),
                    },
                    play_style: "Balanced".to_string(),
                    stadium_name: format!("{city} Arena"),
                    reputation_range: Some([300, 900]),
                    finance_range: Some([500_000, 10_000_000]),
                });
            }
        }
    }
    TeamsDefinition {
        version: 1,
        description: "Built-in default".to_string(),
        structure: None,
        teams,
    }
}

/// Serialisable world database — can be saved to / loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldData {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub countries: Vec<WorldCountry>,
    #[serde(default)]
    pub clubs: Vec<domain::club::Club>,
    pub teams: Vec<domain::team::Team>,
    pub players: Vec<domain::player::Player>,
    pub staff: Vec<domain::staff::Staff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldCountry {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub league_names: Vec<String>,
}

/// Lightweight metadata shown in the UI when listing available databases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldDatabaseInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub country_count: usize,
    #[serde(default)]
    pub club_count: usize,
    pub team_count: usize,
    pub player_count: usize,
    /// "builtin" | "user"
    pub source: String,
    /// Filesystem path (empty for built-in random)
    pub path: String,
}
