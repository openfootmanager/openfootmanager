use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Where definition files come from
// ---------------------------------------------------------------------------

/// The definition files the engine ships, compiled in.
///
/// These are the *same files* that are bundled beside the installed game — one
/// copy in the repository, delivered two ways. That is what makes drift
/// impossible: there is no second, hand-maintained Rust table to fall out of
/// step with the JSON, which is exactly what #454 was about.
const EMBEDDED_NAMES: &str = include_str!("../../data/default_names.json");
const EMBEDDED_NATIONS: &str = include_str!("../../data/default_nations.json");

/// Directories searched for definition files, highest priority first.
///
/// Three tiers ship:
///
/// 1. the player's own `data/` directory under the app data dir — writable, so
///    a definition file dropped there overrides the shipped one;
/// 2. the `data/` directory bundled beside the game — read-only, but *visible*,
///    which is the point: it gives a player a real file to read and copy;
/// 3. the embedded copy above, which can never be missing.
///
/// Tier 3 means a broken install, a `--no-bundle` dev build, or a test degrades
/// to correct behaviour rather than a world with no names. Tier 1 is what
/// `docs/DEFINITIONS.md` has always promised and never delivered.
#[derive(Debug, Clone, Default)]
pub struct DefinitionSources {
    dirs: Vec<PathBuf>,
}

impl DefinitionSources {
    /// Only the compiled-in defaults. What tests and the CLI use.
    pub fn embedded_only() -> Self {
        Self::default()
    }

    /// Search `dirs` in order, then fall back to the embedded defaults.
    pub fn searching(dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            dirs: dirs.into_iter().collect(),
        }
    }

    /// The first file named `<stem>.{json,yaml,yml}` in any search directory.
    fn find(&self, stem: &str) -> Option<PathBuf> {
        self.dirs.iter().find_map(|dir| find_definition_file(dir, stem))
    }

    /// Load `stem`, preferring an on-disk override and falling back to `embedded`.
    ///
    /// A file that exists but does not parse is skipped rather than fatal: a
    /// half-edited override must not stop the game starting, and the embedded
    /// copy is always a correct answer.
    fn load<T: DeserializeOwned>(&self, stem: &str, embedded: &str) -> T {
        if let Some(path) = self.find(stem) {
            match super::file_format::load_definition_file::<T>(&path) {
                Some(value) => {
                    log::info!("[generator] loaded {stem} from {path:?}");
                    return value;
                }
                None => {
                    log::warn!("[generator] {path:?} could not be parsed; using the built-in {stem}")
                }
            }
        }
        // Unwrap is safe and deliberate: `embedded` is compiled in, so a failure
        // here is a build-time mistake, not a runtime condition. The
        // `the_embedded_*` tests are what keep that true.
        serde_json::from_str(embedded)
            .unwrap_or_else(|e| panic!("built-in {stem} definition must parse: {e}"))
    }
}

/// Find a data file by stem, accepting JSON or YAML (`.json`/`.yaml`/`.yml`).
fn find_definition_file(dir: &Path, stem: &str) -> Option<PathBuf> {
    ["json", "yaml", "yml"]
        .iter()
        .map(|ext| dir.join(format!("{stem}.{ext}")))
        .find(|path| path.exists())
}

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

/// Per-nation inputs to procedural club generation.
///
/// This is the seed the generator grows a world from — which nations exist,
/// what their cities are called, how deep their league pyramid runs, and how
/// strong they are. It is deliberately *not* a list of clubs: a curated club
/// list is hand-authored content and belongs in a `.ofm` package, where it
/// stacks and merges. Adding a nation here used to mean editing Rust and
/// recompiling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NationsDefinition {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub description: String,
    /// Clubs generated per division. 20 unless a file says otherwise.
    #[serde(default = "default_clubs_per_division")]
    pub clubs_per_division: usize,
    /// Kit colour pairs drawn from at random.
    #[serde(default)]
    pub color_palette: Vec<ColorPairDef>,
    /// City names used for clubs in a country with no curated pool of its own.
    #[serde(default)]
    pub generic_cities: Vec<String>,
    pub nations: Vec<super::clubs::NationGen>,
}

fn default_clubs_per_division() -> usize {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColorPairDef {
    pub primary: String,
    pub secondary: String,
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
    /// Year the team was founded. If not provided, will be randomly generated.
    #[serde(default)]
    pub founded_year: Option<u32>,
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

/// The name pools the generator draws from: an override if the player supplied
/// one, otherwise the shipped file compiled in.
pub(super) fn names_definition(sources: &DefinitionSources) -> NamesDefinition {
    sources.load("default_names", EMBEDDED_NAMES)
}

/// The per-nation club-generation seed, resolved the same way.
pub(super) fn nations_definition(sources: &DefinitionSources) -> NationsDefinition {
    sources.load("default_nations", EMBEDDED_NATIONS)
}

/// The shipped name pools, ignoring any player override.
///
/// Used by the paths that generate a person *after* the world exists — a youth
/// intake recruit, a synthesised national-team player, the free-agent staff
/// market — none of which have a search path in scope, because nothing threads
/// the player's data directory through mid-game turn processing.
///
/// So an override currently shapes the names a world *opens* with but not the
/// ones generated later in a career. Closing that means carrying the resolved
/// definitions on the game rather than re-reading them, which is follow-up work
/// tracked with the rest of the data-driven epic.
pub(super) fn default_names_definition() -> NamesDefinition {
    names_definition(&DefinitionSources::embedded_only())
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
