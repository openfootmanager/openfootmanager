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

    /// Load `stem` from the first tier that yields a *usable* definition,
    /// falling back to `embedded`.
    ///
    /// Every tier is tried in turn. A file that is missing, unparseable, or
    /// parseable-but-unusable is skipped and the next one considered — a
    /// half-edited override in the user's directory must not stop the game
    /// starting, and it must not hide the bundled copy either. Trying only the
    /// first file *found* would have done exactly that.
    ///
    /// "Unusable" is a real category, not pedantry: `serde` happily accepts a
    /// nations file with no cities or an empty colour palette, and generation
    /// then indexes those empty collections — a modulo by zero and an
    /// out-of-bounds index. Rejecting them here keeps the failure at the
    /// boundary, where there is a path to name in the log.
    fn load<T: DeserializeOwned + UsableDefinition>(&self, stem: &str, embedded: &str) -> T {
        for dir in &self.dirs {
            let Some(path) = find_definition_file(dir, stem) else {
                continue;
            };
            match super::file_format::load_definition_file::<T>(&path) {
                None => log::warn!("[generator] {path:?} could not be parsed; trying the next source"),
                Some(value) => match value.unusable_reason() {
                    None => {
                        log::info!("[generator] loaded {stem} from {path:?}");
                        return value;
                    }
                    Some(reason) => log::warn!(
                        "[generator] {path:?} cannot drive generation ({reason}); \
                         trying the next source"
                    ),
                },
            }
        }
        // Unwrap is safe and deliberate: `embedded` is compiled in, so a failure
        // here is a build-time mistake, not a runtime condition. The
        // `the_embedded_*` tests are what keep that true.
        serde_json::from_str(embedded)
            .unwrap_or_else(|e| panic!("built-in {stem} definition must parse: {e}"))
    }
}

/// A definition that can be rejected after it deserializes.
///
/// Parsing proves the shape; this proves the content can actually drive
/// generation. Kept separate from `Deserialize` so the reason can be reported
/// with the file that failed rather than swallowed as a parse error.
trait UsableDefinition {
    /// Why this definition cannot be used, or `None` when it can.
    fn unusable_reason(&self) -> Option<&'static str>;
}

/// Whether a name-pool key is a country code, and so can ever be looked up.
///
/// A key is not a label. `pick_name_from_def` resolves a player's nationality
/// against these keys, and a nationality is always a code — so a pool keyed
/// `"ENGLAND"` is not merely untidy, it is unreachable: no player will ever
/// carry that string, and the names filed under it can only ever be drawn by
/// accident, as somebody else's borrowed fallback.
fn is_pool_key_usable_as_nationality(key: &str) -> bool {
    (2..=3).contains(&key.len()) && key.chars().all(|c| c.is_ascii_uppercase())
}

impl UsableDefinition for NamesDefinition {
    fn unusable_reason(&self) -> Option<&'static str> {
        // One usable pool is enough: a nationality with none of its own borrows
        // a regional neighbour's. Zero usable pools leaves nothing to borrow.
        //
        // "Usable" has to include the key, not just the two name lists. A file
        // keyed by country names parses and reads perfectly well to its author,
        // while every pool in it is unreachable by lookup — so it is no more
        // usable than one with no names in it at all.
        if self.pools.iter().any(|(code, pool)| {
            is_pool_key_usable_as_nationality(code)
                && !pool.first_names.is_empty()
                && !pool.last_names.is_empty()
        }) {
            None
        } else {
            Some("it declares no pool with both first and last names under a country code")
        }
    }
}

/// Ceilings for the two factors of the club count. Deliberately far above any
/// plausible world — the largest shipped nation runs 2 divisions of 20 — so they
/// only ever catch a typo or a hostile file, never an ambitious one.
const MAX_CLUBS_PER_DIVISION: usize = 1_000;
const MAX_TIERS: usize = 20;
/// Ceiling on the whole world, which is the number that actually gets
/// allocated. The shipped definition builds 440 clubs and roughly 9,700
/// players, so this allows a world some twenty times larger while keeping the
/// squad count (~220,000 players) inside what a desktop game can hold.
const MAX_TOTAL_CLUBS: usize = 10_000;
/// Top of the documented `strength` range on [`super::clubs::NationGen`].
const MAX_NATION_STRENGTH: u8 = 5;

impl UsableDefinition for NationsDefinition {
    fn unusable_reason(&self) -> Option<&'static str> {
        if self.nations.is_empty() {
            return Some("it declares no nations");
        }
        if self.clubs_per_division == 0 {
            return Some("clubsPerDivision is zero");
        }
        // The club count is `clubs_per_division * tiers`, so guarding one factor
        // guards nothing: zero divisions generates zero clubs just as surely,
        // and a career that opens with no teams reports no error on the way in.
        if self.nations.iter().any(|nation| nation.tiers == 0) {
            return Some("a nation has no divisions to fill");
        }
        // Both factors bounded, because the product is an allocation. This runs
        // inside `start_new_game`, an async command: a multi-gigabyte
        // `Vec::with_capacity` there does not fail cleanly, it leaves the player
        // watching a spinner that never resolves (#441). The ceilings are far
        // above any sane world and exist only to keep a typo from becoming one.
        if self.clubs_per_division > MAX_CLUBS_PER_DIVISION {
            return Some("clubsPerDivision is implausibly large");
        }
        if self.nations.iter().any(|nation| nation.tiers > MAX_TIERS) {
            return Some("a nation has implausibly many divisions");
        }
        // Bounding each factor does not bound the product. The nation list has
        // no length limit of its own, so forty nations each sitting exactly on
        // the per-nation ceilings still sums to 800,000 clubs — every one of
        // them individually legal. The total is what gets allocated, so the
        // total is what has to be checked.
        //
        // This also puts `unique_short_code`'s three-letter space out of reach:
        // it panics once a single nation needs more than 26^3 codes, and no
        // nation can exceed the aggregate.
        let total_clubs = self
            .nations
            .iter()
            .map(|nation| self.clubs_per_division.saturating_mul(nation.tiers))
            .fold(0usize, |total, clubs| total.saturating_add(clubs));
        if total_clubs > MAX_TOTAL_CLUBS {
            return Some("the nations add up to more clubs than a world can hold");
        }
        // `strength` seeds the reputation band and is documented 1-5. Out of
        // range it walks past the clamp that bounds reputation, because the
        // `.max(rep_lo + 1)` that keeps the range non-empty undoes the
        // `.min(950)` that caps it -- so a single bad digit mints a world of
        // clubs with reputations in the tens of thousands.
        if self
            .nations
            .iter()
            .any(|nation| nation.strength == 0 || nation.strength > MAX_NATION_STRENGTH)
        {
            return Some("a nation's strength is outside the 1-5 range");
        }
        if self.color_palette.is_empty() {
            return Some("colorPalette is empty");
        }
        if self.nations.iter().any(|nation| nation.cities.is_empty()) {
            // Not merely cosmetic: club naming takes `index % cities.len()`.
            return Some("a nation has no cities to name its clubs after");
        }
        if self.generic_cities.is_empty() {
            // These name the filler clubs that pad a thin package whose country
            // has no curated pool, so an empty list is the same modulo-by-zero
            // one step removed.
            return Some("genericCities is empty");
        }
        None
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch directory to stand in for one tier of the search path.
    fn tier(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ofm-defs-{name}-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn one_nation_json(code: &str) -> String {
        format!(
            r##"{{"clubsPerDivision":4,
                "colorPalette":[{{"primary":"#ff0000","secondary":"#ffffff"}}],
                "genericCities":["Testville"],
                "nations":[{{"code":"{code}","style":"Generic","tiers":1,"strength":3,
                             "cities":["Alpha","Beta","Gamma","Delta"]}}]}}"##
        )
    }

    /// The compiled-in copies are the last line of defence — every other tier
    /// can be missing. If one stops parsing the game cannot generate a world at
    /// all, so this is the test that keeps `load`'s unwrap honest.
    #[test]
    fn the_embedded_definitions_parse_and_carry_the_shipped_data() {
        let sources = DefinitionSources::embedded_only();

        let names = names_definition(&sources);
        assert_eq!(
            names.pools.len(),
            17,
            "the shipped name pools moved out of Rust unchanged"
        );

        let nations = nations_definition(&sources);
        assert_eq!(nations.nations.len(), 16, "the shipped generation nations");
        assert_eq!(
            nations.nations.iter().map(|n| n.cities.len()).sum::<usize>(),
            280,
            "every curated city survived the move out of Rust"
        );
        assert_eq!(nations.clubs_per_division, 20);
        assert_eq!(nations.color_palette.len(), 12);
        assert_eq!(nations.generic_cities.len(), 15);
    }

    #[test]
    fn an_override_wins_over_the_embedded_definition() {
        let dir = tier("override");
        std::fs::write(dir.join("default_nations.json"), one_nation_json("TR")).unwrap();

        let def = nations_definition(&DefinitionSources::searching([dir.clone()]));

        assert_eq!(def.nations.len(), 1);
        assert_eq!(def.nations[0].code, "TR");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// The tiers are a *search path*, not a single lookup. Trying only the
    /// first file found meant a broken file in the user's directory hid the
    /// bundled copy entirely, which is not what the load order promises.
    #[test]
    fn a_broken_higher_tier_falls_through_to_the_next_one() {
        let user = tier("user-broken");
        let bundled = tier("bundled-good");
        std::fs::write(user.join("default_nations.json"), "{ not valid json").unwrap();
        std::fs::write(bundled.join("default_nations.json"), one_nation_json("JP")).unwrap();

        let def = nations_definition(&DefinitionSources::searching([
            user.clone(),
            bundled.clone(),
        ]));

        assert_eq!(
            def.nations[0].code, "JP",
            "the bundled tier should be used, not the embedded one"
        );

        std::fs::remove_dir_all(&user).ok();
        std::fs::remove_dir_all(&bundled).ok();
    }

    #[test]
    fn an_unparseable_override_falls_back_to_the_shipped_definition() {
        let dir = tier("unparseable");
        std::fs::write(dir.join("default_nations.json"), "{ not valid json").unwrap();

        let def = nations_definition(&DefinitionSources::searching([dir.clone()]));

        assert_eq!(def.nations.len(), 16, "the shipped nations");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Parsing proves the shape, not that generation can run on it. `serde`
    /// accepts every one of these, and each then panics or divides by zero
    /// deep inside club generation.
    #[test]
    fn a_parseable_but_unusable_nations_override_is_rejected() {
        let cases = [
            (r##"{"nations":[]}"##, "no nations"),
            (
                r##"{"colorPalette":[{"primary":"#000","secondary":"#fff"}],
                    "nations":[{"code":"TR","style":"Generic","tiers":1,"strength":3,"cities":[]}]}"##,
                "a nation with no cities",
            ),
            (
                r##"{"nations":[{"code":"TR","style":"Generic","tiers":1,"strength":3,
                                "cities":["A"]}]}"##,
                "an empty colour palette",
            ),
            (
                r##"{"clubsPerDivision":0,
                    "colorPalette":[{"primary":"#000","secondary":"#fff"}],
                    "genericCities":["G"],
                    "nations":[{"code":"TR","style":"Generic","tiers":1,"strength":3,
                                "cities":["A"]}]}"##,
                "zero clubs per division",
            ),
            (
                r##"{"colorPalette":[{"primary":"#000","secondary":"#fff"}],
                    "genericCities":[],
                    "nations":[{"code":"TR","style":"Generic","tiers":1,"strength":3,
                                "cities":["A"]}]}"##,
                "no generic cities to pad a thin package with",
            ),
            (
                r##"{"clubsPerDivision":20,
                    "colorPalette":[{"primary":"#000","secondary":"#fff"}],
                    "genericCities":["G"],
                    "nations":[{"code":"TR","style":"Generic","tiers":0,"strength":3,
                                "cities":["A"]}]}"##,
                // `clubs_per_division * tiers` is the club count, and only one
                // factor was guarded. Zero tiers generates zero clubs, and the
                // random path has no emptiness check, so this is a career that
                // starts with no teams and no error.
                "a nation with no divisions",
            ),
            (
                r##"{"clubsPerDivision":100000000,
                    "colorPalette":[{"primary":"#000","secondary":"#fff"}],
                    "genericCities":["G"],
                    "nations":[{"code":"TR","style":"Generic","tiers":1,"strength":3,
                                "cities":["A"]}]}"##,
                "an absurd number of clubs per division",
            ),
            (
                r##"{"clubsPerDivision":20,
                    "colorPalette":[{"primary":"#000","secondary":"#fff"}],
                    "genericCities":["G"],
                    "nations":[{"code":"TR","style":"Generic","tiers":1,"strength":255,
                                "cities":["A"]}]}"##,
                // `strength` is documented 1-5 and seeds the reputation band.
                // Out of range it walks straight past the 950 clamp.
                "a strength outside the documented range",
            ),
        ];

        for (json, what) in cases {
            let dir = tier("unusable");
            std::fs::write(dir.join("default_nations.json"), json).unwrap();

            let def = nations_definition(&DefinitionSources::searching([dir.clone()]));

            assert_eq!(
                def.nations.len(),
                16,
                "{what} should be rejected in favour of the shipped nations"
            );
            std::fs::remove_dir_all(&dir).ok();
        }
    }

    /// Bounding each factor does not bound their product: the nation list has no
    /// length limit of its own, so a file whose every nation sits inside the
    /// per-nation ceilings still adds up to a world nothing can build.
    #[test]
    fn a_nations_override_that_sums_to_an_impossible_world_is_rejected() {
        // Each nation is individually legal — 1000 clubs a division, 20 tiers,
        // both exactly at the ceiling — and there are 40 of them.
        let nations: Vec<String> = (0..40)
            .map(|index| {
                format!(
                    r##"{{"code":"N{index}","style":"Generic","tiers":20,"strength":3,"cities":["A"]}}"##
                )
            })
            .collect();
        let json = format!(
            r##"{{"clubsPerDivision":1000,
                 "colorPalette":[{{"primary":"#000","secondary":"#fff"}}],
                 "genericCities":["G"],
                 "nations":[{}]}}"##,
            nations.join(",")
        );

        let dir = tier("aggregate");
        std::fs::write(dir.join("default_nations.json"), &json).unwrap();

        let def = nations_definition(&DefinitionSources::searching([dir.clone()]));

        assert_eq!(
            def.nations.len(),
            16,
            "800,000 clubs across 40 legal-looking nations should be rejected"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_names_override_with_no_usable_pool_is_rejected() {
        let dir = tier("empty-pools");
        std::fs::write(
            dir.join("default_names.json"),
            r##"{"pools":{"TR":{"first_names":[],"last_names":[]}}}"##,
        )
        .unwrap();

        let def = names_definition(&DefinitionSources::searching([dir.clone()]));

        assert_eq!(def.pools.len(), 17, "the shipped pools");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A pool key is not just a label: it becomes a player's nationality, and
    /// from there a flag lookup, a country filter and a saved field. A file
    /// whose keys are country *names* parses and reads sensibly to its author,
    /// then mints players whose nationality is "ENGLAND".
    #[test]
    fn a_names_override_keyed_by_country_names_is_rejected() {
        let dir = tier("named-pools");
        std::fs::write(
            dir.join("default_names.json"),
            r##"{"pools":{"ENGLAND":{"first_names":["A"],"last_names":["B"]}}}"##,
        )
        .unwrap();

        let def = names_definition(&DefinitionSources::searching([dir.clone()]));

        assert_eq!(
            def.pools.len(),
            17,
            "a file with no well-formed code key falls back to the shipped pools"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A stray key does not condemn the file: one well-formed pool is enough to
    /// keep the override, and the rest of its names stay reachable by borrowing.
    #[test]
    fn a_stray_pool_key_does_not_reject_an_otherwise_usable_file() {
        let dir = tier("mixed-pools");
        std::fs::write(
            dir.join("default_names.json"),
            r##"{"pools":{"BR":{"first_names":["A"],"last_names":["B"]},
                         "ENGLAND":{"first_names":["C"],"last_names":["D"]},
                         "":{"first_names":["E"],"last_names":["F"]}}}"##,
        )
        .unwrap();

        let def = names_definition(&DefinitionSources::searching([dir.clone()]));
        assert!(def.pools.contains_key("BR"), "the usable override is kept");
        assert_ne!(def.pools.len(), 17, "and it is the override, not the shipped set");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A pool for one nationality is enough — everyone else borrows a regional
    /// neighbour's — so this override must be *accepted*, not treated as the
    /// empty case.
    #[test]
    fn a_names_override_with_a_single_usable_pool_is_accepted() {
        let dir = tier("one-pool");
        std::fs::write(
            dir.join("default_names.json"),
            r##"{"pools":{"TR":{"first_names":["Emre"],"last_names":["Yilmaz"]}}}"##,
        )
        .unwrap();

        let def = names_definition(&DefinitionSources::searching([dir.clone()]));

        assert_eq!(def.pools.len(), 1);
        assert!(def.pools.contains_key("TR"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
