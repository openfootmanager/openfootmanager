use super::definitions::{
    WorldData, WorldDatabaseInfo, WorldManifestV2, WorldRegionDefinition, WorldShardRefs,
};
use std::path::{Path, PathBuf};

const WORLD_PARSE_FAILED_ERROR: &str = "be.error.worldParseFailed";
const WORLD_READ_FILE_FAILED_ERROR: &str = "be.error.worldReadFileFailed";
const WORLD_SERIALIZE_FAILED_ERROR: &str = "be.error.worldSerializeFailed";
const RANDOM_WORLD_NAME_KEY: &str = "be.msg.world.randomName";
const RANDOM_WORLD_DESCRIPTION_KEY: &str = "be.msg.world.randomDescription";

/// The region a country belongs to.
///
/// This used to be a hand-maintained match over 21 codes that answered
/// `"europe"` for everything else — and it sits on the *live* path, because
/// `normalize_world` infers regions whenever a world declares none, which a
/// procedurally generated world always does. While generation only ever
/// produced clubs in the same handful of European-plus-BR/AR countries the
/// match happened to list, the gap was invisible. Now that the nations a world
/// contains are read from `data/default_nations.json`, the first person to add
/// Japan or Nigeria would have had them filed under Europe.
///
/// `nations::region_for_code` already knows the correct region for all 211
/// catalogued nations. Its own `"europe"` default is the right behaviour here:
/// region inference has to answer *something*, and an uncatalogued code has no
/// better answer available.
fn infer_region_id(country_code: &str) -> &'static str {
    crate::nations::region_for_code(country_code)
}

fn backend_text_with_param(key: &str, param_name: &str, param_value: usize) -> String {
    let param_value = param_value.to_string();
    let mut message = String::with_capacity(key.len() + param_name.len() + param_value.len() + 2);
    message.push_str(key);
    message.push('?');
    message.push_str(param_name);
    message.push('=');
    message.push_str(&param_value);
    message
}

fn infer_world_regions(teams: &[domain::team::Team]) -> Vec<WorldRegionDefinition> {
    use std::collections::BTreeMap;

    let mut countries_by_region: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for team in teams {
        let country_code = if !team.football_nation.is_empty() {
            team.football_nation.clone()
        } else {
            team.country.clone()
        };
        let region_id = infer_region_id(&country_code).to_string();
        countries_by_region
            .entry(region_id)
            .or_default()
            .push(country_code);
    }

    countries_by_region
        .into_iter()
        .map(|(region_id, mut country_codes)| {
            country_codes.sort();
            country_codes.dedup();
            WorldRegionDefinition {
                // Shared with the package path rather than a second, shorter
                // list of its own — the copy here had no `africa` arm at all.
                name: super::builtin_region_name(&region_id),
                id: region_id,
                country_codes,
            }
        })
        .collect()
}

fn normalize_world(mut world: WorldData) -> WorldData {
    crate::football_identity::upgrade_world_football_identities(
        &mut world.teams,
        &mut world.players,
        &mut world.staff,
    );
    crate::football_identity::upgrade_world_manager_identities(&world.teams, &mut world.managers);
    // A world file written before the display name existed carries none.
    world
        .staff
        .iter_mut()
        .for_each(domain::staff::Staff::refresh_display_name);
    world
        .managers
        .iter_mut()
        .for_each(domain::manager::Manager::refresh_display_name);
    if world.competitions.is_empty()
        && let Some(league) = world.league.clone()
    {
        world.competitions.push(league);
    }
    if world.metadata.world_id.is_empty() {
        world.metadata.world_id = uuid::Uuid::new_v4().to_string();
    }
    if world.regions.is_empty() {
        world.regions = infer_world_regions(&world.teams);
    }
    if world.default_active_regions.is_empty() {
        world.default_active_regions = world
            .regions
            .iter()
            .map(|region| region.id.clone())
            .collect();
    }
    if world.default_active_competitions.is_empty() {
        world.default_active_competitions = world
            .competitions
            .iter()
            .map(|competition| competition.id.clone())
            .collect();
    }
    world.league = world.competitions.first().cloned().or(world.league);
    world
}

fn manifest_shard_path(base: &Path, shard_ref: &str) -> PathBuf {
    base.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(shard_ref)
}

/// Generate a random world and wrap it in a `WorldData`.
/// `sources` decides where definition files are read from.
pub fn generate_world_data(sources: &super::DefinitionSources) -> WorldData {
    // The seed is drawn first and kept, rather than handing the generator an
    // anonymous random stream: a game built from this world needs a seed to replay
    // from, and there is nowhere to recover one after the fact.
    generate_world_data_seeded(rand::random(), sources)
}

/// Deterministic variant of [`generate_world_data`]: same `seed` → identical world.
pub fn generate_world_data_seeded(seed: u64, sources: &super::DefinitionSources) -> WorldData {
    world_data_from_parts(super::generate_world_seeded(seed, sources), Some(seed))
}

/// Deterministic generation with an explicit config — e.g. a small world for
/// fast, reproducible scenario tests.
pub fn generate_world_data_seeded_with(
    seed: u64,
    config: &super::WorldGenConfig,
    sources: &super::DefinitionSources,
) -> WorldData {
    use rand::SeedableRng;
    world_data_from_parts(
        super::generate_world_with_rng(rand::rngs::StdRng::seed_from_u64(seed), config, sources),
        Some(seed),
    )
}

fn world_data_from_parts(
    (mut teams, mut players, mut staff): (
        Vec<domain::team::Team>,
        Vec<domain::player::Player>,
        Vec<domain::staff::Staff>,
    ),
    generation_seed: Option<u64>,
) -> WorldData {
    crate::football_identity::upgrade_world_football_identities(
        &mut teams,
        &mut players,
        &mut staff,
    );

    normalize_world(WorldData {
        name: RANDOM_WORLD_NAME_KEY.to_string(),
        description: backend_text_with_param(
            RANDOM_WORLD_DESCRIPTION_KEY,
            "teamCount",
            teams.len(),
        ),
        teams,
        players,
        staff,
        managers: vec![],
        retired_managers: vec![],
        retired_players: vec![],
        competitions: vec![],
        competition_definitions: None,
        national_teams: vec![],
        regions: vec![],
        default_active_regions: vec![],
        default_active_competitions: vec![],
        league: None,
        news: vec![],
        stats: domain::stats::StatsState::default(),
        world_history: domain::world_history::WorldHistoryArchive::default(),
        metadata: super::definitions::WorldDataMetadata::default(),
        extra_translations: std::collections::HashMap::new(),
        build_notices: Vec::new(),
        generation_seed,
    })
}

const COMPETITION_DEFINITIONS_INVALID_ERROR: &str = "be.error.competitionDef.invalidEmbedded";

/// Reject a world whose embedded competition definitions don't validate, so a
/// broken definition file never loads half-applied.
fn validate_embedded_definitions(
    world: &WorldData,
    basis: super::ReputationBasis<'_>,
) -> Result<(), String> {
    if let Some(file) = &world.competition_definitions
        && !super::competition_def::validate_definitions_for_world_ranked(file, world, basis)
            .is_empty()
    {
        return Err(COMPETITION_DEFINITIONS_INVALID_ERROR.to_string());
    }
    Ok(())
}

/// The `WorldData` fields that hold a roster or its history, under the names a world file uses.
/// A file must carry at least one: `name`, `description`, `metadata` and `regions` are shared
/// with settings files, package manifests and `package.json`, so they prove nothing.
const WORLD_CONTENT_KEYS: &[&str] = &[
    "teams",
    "players",
    "staff",
    "managers",
    "retired_managers",
    "retired_players",
    "competitions",
    "competitionDefinitions",
    "national_teams",
    "league",
    "news",
    "stats",
    "world_history",
];

/// Whether `value` is an object carrying at least one roster or history collection.
///
/// `WorldData` is `serde(default)` throughout, so without this check any JSON object (a settings
/// file, a manifest whose shards failed to parse) loads as an empty world. Shape rather than
/// emptiness is tested, so a blank world still loads.
fn has_world_shape(value: &serde_json::Value) -> bool {
    value.as_object().is_some_and(|object| {
        WORLD_CONTENT_KEYS
            .iter()
            .any(|key| object.contains_key(*key))
    })
}

/// Parse a JSON string into a `WorldData`.
pub fn load_world_from_json(json: &str) -> Result<WorldData, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|_| WORLD_PARSE_FAILED_ERROR.to_string())?;
    if !has_world_shape(&value) {
        return Err(WORLD_PARSE_FAILED_ERROR.to_string());
    }
    let world: WorldData =
        serde_json::from_value(value).map_err(|_| WORLD_PARSE_FAILED_ERROR.to_string())?;
    let world = normalize_world(world);
    validate_embedded_definitions(&world, super::ReputationBasis::Stored)?;
    Ok(world)
}

/// Build a runnable, finalised `WorldData` from a validated world package —
/// normalised and with its embedded definitions checked, exactly like a loaded
/// world file. Call only after [`super::load_world_package`] reports no errors.
pub fn build_world_from_package(
    package: &super::package::WorldPackage,
    opening_year: Option<u32>,
    sources: &super::DefinitionSources,
) -> Result<WorldData, String> {
    let world = super::build_world_data_from_package(package, opening_year, sources);
    finish_package_world(package, world)
}

/// [`build_world_from_package`] from `seed`: the same package, year and seed give the same world.
pub fn build_world_from_package_seeded(
    seed: u64,
    package: &super::package::WorldPackage,
    opening_year: Option<u32>,
    sources: &super::DefinitionSources,
) -> Result<WorldData, String> {
    use rand::{RngExt, SeedableRng};
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut world =
        super::build_world_data_from_package_with_rng(package, opening_year, sources, &mut rng);
    // The world's identity is part of the world: left empty it is a fresh UUID per build.
    world.metadata.world_id = uuid::Builder::from_random_bytes(rng.random())
        .into_uuid()
        .to_string();
    let mut world = finish_package_world(package, world)?;
    world.generation_seed = Some(seed);
    Ok(world)
}

fn finish_package_world(
    package: &super::package::WorldPackage,
    world: WorldData,
) -> Result<WorldData, String> {
    let world = normalize_world(world);
    // Ranked by the package's authored reputations, as `resolve_definitions_ranked` will be when a
    // career starts, so a package that builds here is not refused there on a reputation roll.
    let authored = super::authored_reputations(package);
    validate_embedded_definitions(&world, super::ReputationBasis::Authored(&authored))?;
    Ok(world)
}

/// Serialise a `WorldData` to a pretty-printed JSON string.
pub fn export_world_to_json(world: &WorldData) -> Result<String, String> {
    let normalized = normalize_world(world.clone());
    serde_json::to_string_pretty(&normalized).map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())
}

fn load_world_from_manifest_path(
    path: &Path,
    manifest: WorldManifestV2,
) -> Result<WorldData, String> {
    fn read_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
        let json =
            std::fs::read_to_string(path).map_err(|_| WORLD_PARSE_FAILED_ERROR.to_string())?;
        serde_json::from_str(&json).map_err(|_| WORLD_PARSE_FAILED_ERROR.to_string())
    }

    let teams = read_json_file(&manifest_shard_path(path, &manifest.shards.teams))?;
    let players = read_json_file(&manifest_shard_path(path, &manifest.shards.players))?;
    let staff = read_json_file(&manifest_shard_path(path, &manifest.shards.staff))?;
    let managers = read_json_file(&manifest_shard_path(path, &manifest.shards.managers))?;
    // Absent from a manifest written before the archives existed: nothing to read.
    fn read_optional<T: serde::de::DeserializeOwned>(
        manifest_path: &Path,
        shard: &str,
    ) -> Result<Vec<T>, String> {
        if shard.is_empty() {
            Ok(Vec::new())
        } else {
            read_json_file(&manifest_shard_path(manifest_path, shard))
        }
    }
    let retired_managers = read_optional(path, &manifest.shards.retired_managers)?;
    let retired_players = read_optional(path, &manifest.shards.retired_players)?;
    let competitions = read_json_file(&manifest_shard_path(path, &manifest.shards.competitions))?;
    let national_teams =
        read_json_file(&manifest_shard_path(path, &manifest.shards.national_teams))?;
    let news = read_json_file(&manifest_shard_path(path, &manifest.shards.news))?;
    let stats = read_json_file(&manifest_shard_path(path, &manifest.shards.stats))?;
    let world_history = read_json_file(&manifest_shard_path(path, &manifest.shards.world_history))?;

    Ok(normalize_world(WorldData {
        name: manifest.name,
        description: manifest.description,
        teams,
        players,
        staff,
        managers,
        retired_managers,
        retired_players,
        competitions,
        competition_definitions: None,
        national_teams,
        regions: manifest.regions,
        default_active_regions: manifest.default_active_regions,
        default_active_competitions: manifest.default_active_competitions,
        league: None,
        news,
        stats,
        world_history,
        metadata: manifest
            .compatibility
            .unwrap_or_else(|| super::definitions::WorldDataMetadata {
                format_version: manifest.format_version,
                world_id: manifest.world_id,
                ..Default::default()
            }),
        extra_translations: std::collections::HashMap::new(),
        build_notices: Vec::new(),
        generation_seed: None,
    }))
}

pub fn load_world_from_path(path: &Path) -> Result<WorldData, String> {
    let json =
        std::fs::read_to_string(path).map_err(|_| WORLD_READ_FILE_FAILED_ERROR.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&json).map_err(|_| WORLD_PARSE_FAILED_ERROR.to_string())?;
    let manifest_shaped = value.as_object().is_some_and(|object| {
        object.contains_key("shards") || object.contains_key("formatVersion")
    });
    if manifest_shaped {
        let manifest: WorldManifestV2 =
            serde_json::from_value(value).map_err(|_| WORLD_PARSE_FAILED_ERROR.to_string())?;
        // No loader reads an older manifest; its `name` must not pass it off as a world file.
        if manifest.format_version < 2 {
            return Err(WORLD_PARSE_FAILED_ERROR.to_string());
        }
        return load_world_from_manifest_path(path, manifest);
    }
    load_world_from_json(&json)
}

pub fn export_world_package(world: &WorldData, manifest_path: &Path) -> Result<String, String> {
    fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
        let json = serde_json::to_string_pretty(value)
            .map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())?;
        std::fs::write(path, json).map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())
    }

    let normalized = normalize_world(world.clone());
    let stem = manifest_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("world");
    let shard_dir = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}.shards"));
    std::fs::create_dir_all(&shard_dir).map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())?;

    write_json(&shard_dir.join("teams.json"), &normalized.teams)?;
    write_json(&shard_dir.join("players.json"), &normalized.players)?;
    write_json(&shard_dir.join("staff.json"), &normalized.staff)?;
    write_json(&shard_dir.join("managers.json"), &normalized.managers)?;
    write_json(
        &shard_dir.join("retired_managers.json"),
        &normalized.retired_managers,
    )?;
    write_json(
        &shard_dir.join("retired_players.json"),
        &normalized.retired_players,
    )?;
    write_json(
        &shard_dir.join("competitions.json"),
        &normalized.competitions,
    )?;
    write_json(
        &shard_dir.join("national_teams.json"),
        &normalized.national_teams,
    )?;
    write_json(&shard_dir.join("news.json"), &normalized.news)?;
    write_json(&shard_dir.join("stats.json"), &normalized.stats)?;
    write_json(
        &shard_dir.join("world_history.json"),
        &normalized.world_history,
    )?;

    let manifest = WorldManifestV2 {
        format_version: 2,
        world_id: normalized.metadata.world_id.clone(),
        name: normalized.name.clone(),
        description: normalized.description.clone(),
        regions: normalized.regions.clone(),
        default_active_regions: normalized.default_active_regions.clone(),
        default_active_competitions: normalized.default_active_competitions.clone(),
        shards: WorldShardRefs {
            teams: format!("{stem}.shards/teams.json"),
            players: format!("{stem}.shards/players.json"),
            staff: format!("{stem}.shards/staff.json"),
            managers: format!("{stem}.shards/managers.json"),
            retired_managers: format!("{stem}.shards/retired_managers.json"),
            retired_players: format!("{stem}.shards/retired_players.json"),
            competitions: format!("{stem}.shards/competitions.json"),
            national_teams: format!("{stem}.shards/national_teams.json"),
            news: format!("{stem}.shards/news.json"),
            stats: format!("{stem}.shards/stats.json"),
            world_history: format!("{stem}.shards/world_history.json"),
        },
        compatibility: Some(super::definitions::WorldDataMetadata {
            format_version: 2,
            ..normalized.metadata.clone()
        }),
    };
    write_json(manifest_path, &manifest)?;
    Ok(manifest_path.to_string_lossy().to_string())
}

/// Pack a directory tree into a `.ofm` zip archive. All files under `dir` are
/// included; asset files (images, etc.) are carried along with data files.
pub fn export_directory_to_ofm(dir: &Path, output: &Path) -> Result<(), String> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let file =
        std::fs::File::create(output).map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    fn add_dir(
        zip: &mut zip::ZipWriter<std::fs::File>,
        base: &Path,
        current: &Path,
        options: SimpleFileOptions,
    ) -> Result<(), String> {
        // Propagate read failures instead of silently producing a partial
        // archive that is missing whole subtrees.
        let entries =
            std::fs::read_dir(current).map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                add_dir(zip, base, &path, options)?;
            } else {
                let rel = path
                    .strip_prefix(base)
                    .map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                let data =
                    std::fs::read(&path).map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())?;
                zip.start_file(rel, options)
                    .map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())?;
                zip.write_all(&data)
                    .map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())?;
            }
        }
        Ok(())
    }

    add_dir(&mut zip, dir, dir, options)?;
    zip.finish()
        .map_err(|_| WORLD_SERIALIZE_FAILED_ERROR.to_string())?;
    Ok(())
}

/// Extract a `.ofm` ZIP archive into `dest_dir` for editing, applying the same
/// hardened guards (zip-slip, symlink, file-count and uncompressed-size caps)
/// used when installing/loading packages. Any unsafe or unreadable entry aborts
/// the extraction with its `be.error.*` code rather than partially unpacking a
/// malicious archive.
pub fn extract_ofm_to_dir(ofm_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let result = super::package::extract_archive_safely(ofm_path, dest_dir);
    let err = match result {
        Ok(entry_errors) => entry_errors.into_iter().next().map(|e| e.code),
        Err(e) => Some(e),
    };
    if let Some(code) = err {
        // Earlier safe entries may already be on disk; don't leave a partially
        // unpacked tree behind when a later entry is rejected or unreadable.
        std::fs::remove_dir_all(dest_dir).ok();
        return Err(code);
    }
    Ok(())
}

/// Scan a directory for `.json` world database files and return their metadata.
pub fn scan_world_databases(dir: &std::path::Path) -> Vec<WorldDatabaseInfo> {
    let mut results = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return results;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(world) = load_world_from_path(&path) {
            let file_stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let history_mode = match world.metadata.kind {
                crate::generator::WorldDataKind::HistoricalSnapshot => "reference",
                crate::generator::WorldDataKind::RosterBaseline => "hybrid",
            };
            results.push(WorldDatabaseInfo {
                id: format!("file:{}", path.display()),
                name: world.name,
                description: world.description,
                team_count: world.teams.len(),
                player_count: world.players.len(),
                history_mode: history_mode.to_string(),
                base_year: world.metadata.base_year,
                snapshot_date: world.metadata.snapshot_date,
                source: "user".to_string(),
                path: path.to_string_lossy().to_string(),
            });
            // suppress unused variable warning
            let _ = file_stem;
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempWorldDir {
        path: PathBuf,
    }

    impl TempWorldDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ofm-world-io-tests-{}", unique));
            fs::create_dir_all(&path).expect("temporary world dir should be created");
            Self { path }
        }

        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TempWorldDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    /// Given a world file whose managers predate the display name (no `display_name` in their JSON)
    /// When it is loaded
    /// Then each is displayed by their known-as name or their full name, not left blank (#557).
    #[test]
    fn a_world_file_without_manager_display_names_gets_them_on_load() {
        let json = r#"{"managers":[
            {"id":"m1","first_name":"Josep","last_name":"Guardiola i Sala","date_of_birth":"1971-01-18",
             "nationality":"ES","reputation":900,"satisfaction":80,"team_id":null,"known_as":"Pep Guardiola",
             "career_stats":{"matches_managed":0,"wins":0,"draws":0,"losses":0,"trophies":0,"best_finish":null},
             "career_history":[]},
            {"id":"m2","first_name":"Alice","last_name":"Boss","date_of_birth":"1970-05-10",
             "nationality":"GB","reputation":500,"satisfaction":50,"team_id":null,
             "career_stats":{"matches_managed":0,"wins":0,"draws":0,"losses":0,"trophies":0,"best_finish":null},
             "career_history":[]}]}"#;

        let world = load_world_from_json(json).expect("the world loads");

        let name = |id: &str| {
            world
                .managers
                .iter()
                .find(|m| m.id == id)
                .unwrap()
                .display_name
                .clone()
        };
        assert_eq!(name("m1"), "Pep Guardiola");
        assert_eq!(name("m2"), "Alice Boss");
    }

    /// Given a world file whose manager and staff member carry a blank known-as name
    /// When it is loaded
    /// Then they are called by their full name and keep no known-as, the same as when it is set by hand (#557).
    #[test]
    fn a_blank_known_as_in_a_world_file_is_treated_as_none() {
        let json = r#"{"managers":[
            {"id":"m1","first_name":"Alice","last_name":"Boss","date_of_birth":"1970-05-10",
             "nationality":"GB","reputation":500,"satisfaction":50,"team_id":null,"known_as":"   ",
             "career_stats":{"matches_managed":0,"wins":0,"draws":0,"losses":0,"trophies":0,"best_finish":null},
             "career_history":[]}],
            "staff":[
            {"id":"s1","first_name":"Alice","last_name":"Coach","date_of_birth":"1980-05-10",
             "nationality":"GB","role":"Physio","known_as":"  ",
             "attributes":{"coaching":80,"judgingAbility":50,"judgingPotential":50,"physiotherapy":10},
             "team_id":null}]}"#;

        let world = load_world_from_json(json).expect("the world loads");

        assert_eq!(world.managers[0].display_name, "Alice Boss");
        assert_eq!(world.managers[0].known_as, None);
        assert_eq!(world.staff[0].display_name, "Alice Coach");
        assert_eq!(world.staff[0].known_as, None);
    }

    /// Given a world file whose staff predate the display name (no `display_name` in their JSON)
    /// When it is loaded
    /// Then each is displayed by their known-as name or their full name, not left blank (#557).
    #[test]
    fn a_world_file_without_display_names_gets_them_on_load() {
        let json = r#"{"staff":[
            {"id":"s1","first_name":"Josep","last_name":"Guardiola i Sala","date_of_birth":"1971-01-18",
             "nationality":"ES","role":"Coach","known_as":"Pep Guardiola",
             "attributes":{"coaching":80,"judgingAbility":50,"judgingPotential":50,"physiotherapy":10},
             "team_id":null},
            {"id":"s2","first_name":"Alice","last_name":"Coach","date_of_birth":"1980-05-10",
             "nationality":"GB","role":"Physio",
             "attributes":{"coaching":80,"judgingAbility":50,"judgingPotential":50,"physiotherapy":10},
             "team_id":null}]}"#;

        let world = load_world_from_json(json).expect("the world loads");

        let name = |id: &str| {
            world
                .staff
                .iter()
                .find(|s| s.id == id)
                .unwrap()
                .display_name
                .clone()
        };
        assert_eq!(name("s1"), "Pep Guardiola");
        assert_eq!(name("s2"), "Alice Coach");
    }

    #[test]
    fn load_world_from_json_normalizes_legacy_english_world_data() {
        let json = r##"
                {
                    "name": "Legacy World",
                    "description": "Old GB world",
                    "teams": [
                        {
                            "id": "team-1",
                            "name": "London FC",
                            "short_name": "LFC",
                            "country": "GB",
                            "city": "London",
                            "stadium_name": "London Arena",
                            "stadium_capacity": 50000,
                            "finance": 1000000,
                            "manager_id": null,
                            "reputation": 500,
                            "wage_budget": 100000,
                            "transfer_budget": 250000,
                            "season_income": 0,
                            "season_expenses": 0,
                            "formation": "4-4-2",
                            "play_style": "Balanced",
                            "training_focus": "Physical",
                            "training_intensity": "Medium",
                            "training_schedule": "Balanced",
                            "founded_year": 1900,
                            "colors": { "primary": "#ffffff", "secondary": "#000000" },
                            "starting_xi_ids": [],
                            "match_roles": { "captain": null, "vice_captain": null, "penalty_taker": null, "free_kick_taker": null, "corner_taker": null },
                            "form": [],
                            "history": []
                        }
                    ],
                    "players": [
                        {
                            "id": "player-1",
                            "match_name": "J. Doe",
                            "full_name": "John Doe",
                            "date_of_birth": "2000-01-01",
                            "nationality": "GB",
                            "position": "Midfielder",
                            "natural_position": "Midfielder",
                            "alternate_positions": [],
                            "footedness": "Right",
                            "weak_foot": 2,
                            "attributes": {
                                "pace": 70, "stamina": 70, "strength": 70, "agility": 70,
                                "passing": 70, "shooting": 70, "tackling": 70, "dribbling": 70,
                                "defending": 70, "positioning": 70, "vision": 70, "decisions": 70,
                                "composure": 70, "aggression": 70, "teamwork": 70, "leadership": 70,
                                "handling": 20, "reflexes": 20, "aerial": 60
                            },
                            "condition": 100,
                            "morale": 100,
                            "fitness": 75,
                            "injury": null,
                            "team_id": "team-1",
                            "traits": [],
                            "contract_end": null,
                            "wage": 0,
                            "market_value": 0,
                            "stats": { "appearances": 0, "goals": 0, "assists": 0, "clean_sheets": 0, "yellow_cards": 0, "red_cards": 0, "avg_rating": 0.0, "minutes_played": 0 },
                            "career": [],
                            "training_focus": null,
                            "transfer_listed": false,
                            "loan_listed": false,
                            "transfer_offers": [],
                            "morale_core": { "manager_trust": 50, "unresolved_issue": null, "recent_treatment": null, "pending_promise": null, "talk_cooldown_until": null, "renewal_state": null }
                        }
                    ],
                    "staff": []
                }
                "##;

        let world = load_world_from_json(json).unwrap();

        assert_eq!(world.teams[0].football_nation, "ENG");
        assert_eq!(world.players[0].football_nation, "ENG");
        assert_eq!(world.players[0].birth_country, None);
        assert!(world.managers.is_empty());
        assert!(world.league.is_none());
        assert!(world.news.is_empty());
        assert!(world.stats.player_matches.is_empty());
        assert_eq!(
            world.metadata.kind,
            crate::generator::WorldDataKind::RosterBaseline
        );
    }

    /// What a generated world is: names and birth dates. Ids are left out because this was
    /// written when they were minted at random; they are seeded now, and
    /// `a_seed_regenerates_the_same_world_ids_included` is the test that holds them to it.
    fn fingerprint(world: &WorldData) -> (Vec<String>, Vec<String>) {
        (
            world.teams.iter().map(|team| team.name.clone()).collect(),
            world
                .players
                .iter()
                .map(|player| format!("{} {}", player.full_name, player.date_of_birth))
                .collect(),
        )
    }

    /// Given a world generated without being asked for a seed,
    /// When it is asked which seed it came from,
    /// Then it says, and generating from that seed gives the same world.
    #[test]
    fn a_random_world_remembers_the_seed_it_was_made_from() {
        let sources = crate::generator::DefinitionSources::embedded_only();
        let world = generate_world_data(&sources);

        let seed = world
            .generation_seed
            .expect("a generated world keeps its seed");

        let again = generate_world_data_seeded(seed, &sources);
        assert_eq!(fingerprint(&world), fingerprint(&again));
    }

    /// Given a seed,
    /// When a world is generated from it,
    /// Then the world records that seed, and a different seed gives a different world.
    #[test]
    fn a_seeded_world_records_the_seed_it_was_given() {
        let sources = crate::generator::DefinitionSources::embedded_only();

        let one = generate_world_data_seeded(1, &sources);
        let two = generate_world_data_seeded(2, &sources);

        assert_eq!(one.generation_seed, Some(1));
        assert_ne!(
            fingerprint(&one),
            fingerprint(&two),
            "the seed must be what decides the world, or this proves nothing"
        );
    }

    /// Given one seed,
    /// When a world is generated from it twice,
    /// Then it is the same world all the way down — the ids of its clubs, players and staff
    ///      included, which are what the days of a game key their dice by.
    #[test]
    fn a_seed_regenerates_the_same_world_ids_included() {
        let sources = crate::generator::DefinitionSources::embedded_only();
        let config = crate::generator::WorldGenConfig::compact();
        let everything = |world: &WorldData| {
            serde_json::to_value((&world.teams, &world.players, &world.staff))
                .unwrap()
                .to_string()
        };

        let one = generate_world_data_seeded_with(5, &config, &sources);
        let again = generate_world_data_seeded_with(5, &config, &sources);

        for (a, b) in one.teams.iter().zip(&again.teams) {
            assert_eq!(
                serde_json::to_value(a).unwrap(),
                serde_json::to_value(b).unwrap(),
                "a club differs"
            );
        }
        for (a, b) in one.players.iter().zip(&again.players) {
            let (a, b) = (
                serde_json::to_value(a).unwrap(),
                serde_json::to_value(b).unwrap(),
            );
            let differing: Vec<(&String, &serde_json::Value, &serde_json::Value)> = a
                .as_object()
                .unwrap()
                .iter()
                .filter(|(key, value)| &b[key.as_str()] != *value)
                .map(|(key, value)| (key, value, &b[key.as_str()]))
                .collect();
            assert!(differing.is_empty(), "a player differs in {differing:?}");
        }
        for (a, b) in one.staff.iter().zip(&again.staff) {
            assert_eq!(
                serde_json::to_value(a).unwrap(),
                serde_json::to_value(b).unwrap(),
                "a staff member differs"
            );
        }
        assert!(
            everything(&one) == everything(&again),
            "the same seed gave different worlds"
        );
    }

    fn load_file(contents: &str) -> Result<WorldData, String> {
        let dir = TempWorldDir::new();
        let path = dir.path().join("world.json");
        fs::write(&path, contents).unwrap();
        load_world_from_path(&path)
    }

    /// Given a world manifest whose shard references do not parse (a number where a file name
    ///       belongs, as a newer or damaged manifest might have)
    /// When it is loaded
    /// Then it is an error, not an empty world that every field of `WorldData` would default to.
    #[test]
    fn a_manifest_whose_shards_do_not_parse_is_an_error_not_an_empty_world() {
        let result = load_file(
            r#"{"formatVersion":2,"worldId":"w","name":"W","description":"","shards":{"teams":5}}"#,
        );

        assert_eq!(result.unwrap_err(), WORLD_PARSE_FAILED_ERROR);
    }

    /// Given a version-2 world manifest that names no teams shard
    /// When it is loaded
    /// Then it is an error: a manifest with nowhere to read its clubs from is not a world.
    #[test]
    fn a_manifest_without_a_teams_shard_is_an_error_not_an_empty_world() {
        let result = load_file(
            r#"{"formatVersion":2,"worldId":"w","name":"W","description":"",
                "shards":{"teams":"","players":"p.json","staff":"s.json","managers":"m.json",
                "competitions":"c.json","nationalTeams":"n.json","news":"nw.json",
                "stats":"st.json","worldHistory":"h.json"}}"#,
        );

        assert_eq!(result.unwrap_err(), WORLD_PARSE_FAILED_ERROR);
    }

    /// Given JSON that is not a world at all: an object with none of a world's keys, an empty
    ///       object, and a value that is not an object
    /// When each is loaded, from a file and as text
    /// Then each is an error, not an empty world.
    #[test]
    fn json_that_is_not_a_world_is_refused() {
        for text in [r#"{"foo":1}"#, "{}", "[1,2]", "3"] {
            assert_eq!(
                load_file(text).unwrap_err(),
                WORLD_PARSE_FAILED_ERROR,
                "{text}"
            );
            assert_eq!(
                load_world_from_json(text).unwrap_err(),
                WORLD_PARSE_FAILED_ERROR,
                "{text}"
            );
        }
    }

    /// Given JSON that carries only the generic fields a world shares with other files: a settings
    ///       file with a name and a description, a package's own manifest, an npm package.json
    /// When each is loaded, from a file and as text
    /// Then each is an error: a name is not a roster, so none of them is an empty world.
    #[test]
    fn a_file_with_only_a_name_and_a_description_is_not_a_world() {
        for text in [
            r#"{"name":"My settings","description":"not a world"}"#,
            r#"{"schema":"world","id":"pkg","name":"Some Package","version":"1.0.0"}"#,
            r#"{"name":"app","version":"1.0.0","description":"x","metadata":{"a":1},"regions":[]}"#,
        ] {
            assert_eq!(
                load_file(text).unwrap_err(),
                WORLD_PARSE_FAILED_ERROR,
                "{text}"
            );
            assert_eq!(
                load_world_from_json(text).unwrap_err(),
                WORLD_PARSE_FAILED_ERROR,
                "{text}"
            );
        }
    }

    /// Given a manifest of format version 1 whose shard files are all there (a real export with
    ///       its version lowered), which this build has no loader for
    /// When it is loaded
    /// Then it is an error, not read as a version-2 manifest and not an empty world via its `name`.
    #[test]
    fn a_manifest_of_an_older_format_is_an_error_not_an_empty_world() {
        let dir = TempWorldDir::new();
        let manifest = dir.path().join("world.json");
        export_world_package(&WorldData::default(), &manifest).unwrap();
        assert!(
            load_world_from_path(&manifest).is_ok(),
            "the export loads as it is"
        );
        let written = fs::read_to_string(&manifest).unwrap();
        assert!(written.contains("\"formatVersion\": 2"), "{written}");
        fs::write(
            &manifest,
            written.replace("\"formatVersion\": 2", "\"formatVersion\": 1"),
        )
        .unwrap();

        let result = load_world_from_path(&manifest);

        assert_eq!(result.unwrap_err(), WORLD_PARSE_FAILED_ERROR);
    }

    /// Given a blank world: every list present and empty, as one saved from nothing would be
    /// When it is loaded
    /// Then it loads: refusing a file that is not a world must not refuse an empty one.
    #[test]
    fn a_blank_world_still_loads() {
        let blank = serde_json::to_string(&WorldData::default()).unwrap();

        assert!(load_file(&blank).is_ok());
        assert!(load_world_from_json(r#"{"teams":[],"players":[]}"#).is_ok());
    }

    /// Given a worlds folder that also holds an unrelated JSON file
    /// When the folder is scanned
    /// Then the unrelated file is not listed as a world.
    #[test]
    fn an_unrelated_json_file_is_not_listed_as_a_world_database() {
        let dir = TempWorldDir::new();
        fs::write(dir.path().join("notes.json"), r#"{"todo":["buy boots"]}"#).unwrap();

        assert!(scan_world_databases(dir.path()).is_empty());
    }

    /// Given a fully populated world (every collection non-empty, including the ones skipped when
    ///       empty)
    /// When it is serialized
    /// Then every key the shape check accepts is a key a world file really has: a typo in the list
    ///      (`nationalTeams` for `national_teams`) would otherwise refuse real worlds silently.
    #[test]
    fn every_content_key_is_a_key_a_world_file_has() {
        let mut world = world_with_archives();
        world.competition_definitions = Some(Default::default());
        let written = serde_json::to_value(&world).unwrap();

        for key in WORLD_CONTENT_KEYS {
            assert!(written.get(key).is_some(), "{key} is not a WorldData field");
        }
    }

    /// Given a path with no file behind it
    /// When it is loaded
    /// Then the error says the file could not be read, not that it could not be parsed.
    #[test]
    fn a_missing_world_file_is_a_read_error_not_a_parse_error() {
        let dir = TempWorldDir::new();

        let result = load_world_from_path(&dir.path().join("nothing-here.json"));

        assert_eq!(result.unwrap_err(), WORLD_READ_FILE_FAILED_ERROR);
    }

    fn world_with_archives() -> WorldData {
        let mut world = generate_world_data(&crate::generator::DefinitionSources::embedded_only());
        let mut manager = domain::manager::Manager::new(
            "retired-boss".to_string(),
            "Old".to_string(),
            "Boss".to_string(),
            "1950-01-01".to_string(),
            "England".to_string(),
        );
        manager
            .career_history
            .push(domain::manager::ManagerCareerEntry::open(
                world.teams[0].id.clone(),
                world.teams[0].name.clone(),
                "2010-07-01".to_string(),
            ));
        world.retired_managers.push(manager);
        let mut veteran = world.players[0].clone();
        veteran.id = "retired-veteran".to_string();
        veteran.retired = true;
        veteran.team_id = None;
        world
            .retired_players
            .push(domain::retired_player::RetiredPlayer::from_player(
                &veteran,
                "2030-06-30",
                2030,
            ));
        world
    }

    /// Given a world carrying archived managers and retired players (#814, #799)
    /// When it is exported as a sharded package, and as one JSON, and loaded back
    /// Then both archives come back intact, so a re-import does not forget a club's past managers.
    #[test]
    fn the_archives_survive_a_world_export_round_trip() {
        let world = world_with_archives();
        let dir = std::env::temp_dir().join(format!(
            "ofm-export-archives-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = dir.join("world.json");

        export_world_package(&world, &manifest).unwrap();
        let from_shards = load_world_from_path(&manifest).unwrap();
        let from_json = load_world_from_json(&export_world_to_json(&world).unwrap()).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        for loaded in [from_shards, from_json] {
            assert_eq!(loaded.retired_players, world.retired_players);
            assert_eq!(
                serde_json::to_value(&loaded.retired_managers).unwrap(),
                serde_json::to_value(&world.retired_managers).unwrap()
            );
        }
    }

    /// Given a world exported before the archives existed (no archive keys, in either format)
    /// When it is loaded
    /// Then it loads, with both archives empty: old exports keep working.
    #[test]
    fn an_export_from_before_the_archives_loads_with_none() {
        let world = world_with_archives();

        let mut single = serde_json::to_value(&world).unwrap();
        single.as_object_mut().unwrap().remove("retired_managers");
        single.as_object_mut().unwrap().remove("retired_players");
        let from_json =
            load_world_from_json(&single.to_string()).expect("an old single JSON loads");

        let dir = std::env::temp_dir().join(format!(
            "ofm-old-export-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = dir.join("world.json");
        export_world_package(&world, &manifest).unwrap();
        let mut old: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
        let shards = old["shards"].as_object_mut().unwrap();
        shards.remove("retiredManagers");
        shards.remove("retiredPlayers");
        std::fs::write(&manifest, old.to_string()).unwrap();
        let from_shards = load_world_from_path(&manifest);
        std::fs::remove_dir_all(&dir).ok();
        let from_shards = from_shards.expect("an old manifest loads");

        for loaded in [from_json, from_shards] {
            // The world itself came through: a file that failed to parse and fell back to an
            // empty world would also have empty archives.
            assert_eq!(loaded.players.len(), world.players.len());
            assert_eq!(loaded.teams.len(), world.teams.len());
            assert!(loaded.retired_managers.is_empty());
            assert!(loaded.retired_players.is_empty());
        }
    }

    #[test]
    fn export_world_to_json_writes_canonical_football_identity_fields() {
        let mut world = generate_world_data(&crate::generator::DefinitionSources::embedded_only());
        world.teams[0].country = "GB".to_string();
        world.teams[0].football_nation.clear();

        if let Some(player) = world
            .players
            .iter_mut()
            .find(|player| player.team_id.as_deref() == Some(world.teams[0].id.as_str()))
        {
            player.nationality = "GB".to_string();
            player.football_nation.clear();
            player.birth_country = None;
        }

        let json = export_world_to_json(&world).unwrap();
        let reparsed: WorldData = serde_json::from_str(&json).unwrap();

        assert_eq!(reparsed.name, RANDOM_WORLD_NAME_KEY);
        assert!(
            reparsed
                .description
                .starts_with("be.msg.world.randomDescription?teamCount=")
        );
        assert_eq!(reparsed.teams[0].football_nation, "ENG");
        assert_eq!(
            reparsed.metadata.kind,
            crate::generator::WorldDataKind::RosterBaseline
        );
    }

    #[test]
    fn load_world_from_json_preserves_historical_snapshot_fields() {
        let json = r##"
                {
                    "name": "Snapshot World",
                    "description": "Rich snapshot",
                    "teams": [
                        {
                            "id": "team-1",
                            "name": "London FC",
                            "short_name": "LFC",
                            "country": "GB",
                            "city": "London",
                            "stadium_name": "London Arena",
                            "stadium_capacity": 50000,
                            "finance": 1000000,
                            "manager_id": "mgr-1",
                            "reputation": 500,
                            "wage_budget": 100000,
                            "transfer_budget": 250000,
                            "season_income": 0,
                            "season_expenses": 0,
                            "formation": "4-4-2",
                            "play_style": "Balanced",
                            "training_focus": "Physical",
                            "training_intensity": "Medium",
                            "training_schedule": "Balanced",
                            "founded_year": 1900,
                            "colors": { "primary": "#ffffff", "secondary": "#000000" },
                            "starting_xi_ids": [],
                            "match_roles": { "captain": null, "vice_captain": null, "penalty_taker": null, "free_kick_taker": null, "corner_taker": null },
                            "form": [],
                            "history": []
                        }
                    ],
                    "players": [],
                    "staff": [],
                    "managers": [
                        {
                            "id": "mgr-1",
                            "first_name": "Ada",
                            "last_name": "Lovelace",
                            "date_of_birth": "1980-01-01",
                            "nationality": "GB",
                            "football_nation": "",
                            "birth_country": null,
                            "reputation": 600,
                            "satisfaction": 75,
                            "fan_approval": 55,
                            "team_id": "team-1",
                            "warning_stage": 0,
                            "career_stats": {
                                "matches_managed": 10,
                                "wins": 4,
                                "draws": 3,
                                "losses": 3,
                                "trophies": 0,
                                "best_finish": 5
                            },
                            "career_history": []
                        }
                    ],
                    "league": {
                        "id": "league-1",
                        "name": "Open League",
                        "season": 2024,
                        "fixtures": [],
                        "standings": []
                    },
                    "news": [
                        {
                            "id": "news-1",
                            "headline": "Season underway",
                            "body": "The campaign has begun.",
                            "source": "World Feed",
                            "date": "2024-08-15",
                            "category": "SeasonPreview",
                            "team_ids": ["team-1"],
                            "player_ids": [],
                            "match_score": null,
                            "read": false,
                            "i18n_params": {}
                        }
                    ],
                    "stats": {
                        "player_matches": [],
                        "team_matches": []
                    },
                    "world_history": {
                        "rivalries": [
                            {
                                "team_a_id": "team-1",
                                "team_b_id": "team-2",
                                "intensity": 66,
                                "started_season": 2023
                            }
                        ],
                        "season_awards": []
                    },
                    "metadata": {
                        "kind": "historicalSnapshot",
                        "base_year": 2024,
                        "snapshot_date": "2024-08-15T00:00:00Z"
                    }
                }
                "##;

        let world = load_world_from_json(json).unwrap();

        assert_eq!(world.managers.len(), 1);
        assert_eq!(world.managers[0].football_nation, "ENG");
        assert_eq!(
            world.league.as_ref().map(|league| league.season),
            Some(2024)
        );
        assert_eq!(world.news.len(), 1);
        assert_eq!(world.world_history.rivalries.len(), 1);
        assert_eq!(
            world.metadata.kind,
            crate::generator::WorldDataKind::HistoricalSnapshot
        );
        assert_eq!(world.metadata.base_year, Some(2024));
    }

    #[test]
    fn export_world_to_json_preserves_historical_snapshot_fields() {
        let mut world = generate_world_data(&crate::generator::DefinitionSources::embedded_only());
        world.managers.push(domain::manager::Manager::new(
            "mgr-1".to_string(),
            "Ada".to_string(),
            "Lovelace".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        ));
        world.managers[0].team_id = Some(world.teams[0].id.clone());
        world.league = Some(domain::league::League::new(
            "league-1".to_string(),
            "Open League".to_string(),
            2028,
            &[world.teams[0].id.clone()],
        ));
        world.news.push(domain::news::NewsArticle::new(
            "news-1".to_string(),
            "Season underway".to_string(),
            "The campaign has begun.".to_string(),
            "World Feed".to_string(),
            "2028-08-15".to_string(),
            domain::news::NewsCategory::SeasonPreview,
        ));
        world
            .world_history
            .upsert_rivalry("team-1", "team-2", 72, Some(2027));
        world.metadata = crate::generator::WorldDataMetadata {
            kind: crate::generator::WorldDataKind::HistoricalSnapshot,
            base_year: Some(2028),
            snapshot_date: Some("2028-08-15T00:00:00Z".to_string()),
            ..Default::default()
        };

        let json = export_world_to_json(&world).unwrap();
        let reparsed: WorldData = serde_json::from_str(&json).unwrap();

        assert_eq!(reparsed.managers.len(), 1);
        assert_eq!(reparsed.managers[0].football_nation, "ENG");
        assert_eq!(
            reparsed.league.as_ref().map(|league| league.season),
            Some(2028)
        );
        assert_eq!(reparsed.news.len(), 1);
        assert_eq!(reparsed.world_history.rivalries.len(), 1);
        assert_eq!(
            reparsed.metadata.kind,
            crate::generator::WorldDataKind::HistoricalSnapshot
        );
    }

    #[test]
    fn load_world_from_json_returns_backend_key_when_invalid_json() {
        let result = load_world_from_json("not valid json");

        assert_eq!(result.unwrap_err(), WORLD_PARSE_FAILED_ERROR);
    }

    #[test]
    fn scan_world_databases_exposes_history_mode_metadata() {
        let temp_dir = TempWorldDir::new();
        let path = temp_dir.path().join("snapshot.json");
        fs::write(
            &path,
            r#"
            {
                "name": "Historical Snapshot",
                "description": "Season already underway",
                "teams": [],
                "players": [],
                "staff": [],
                "metadata": {
                    "kind": "historicalSnapshot",
                    "base_year": 2031,
                    "snapshot_date": "2031-11-20T00:00:00+00:00"
                }
            }
            "#,
        )
        .expect("world json should be written");

        let databases = scan_world_databases(temp_dir.path());
        let database = databases
            .iter()
            .find(|database| database.id == format!("file:{}", path.display()))
            .expect("snapshot database should be scanned");

        assert_eq!(database.history_mode, "reference");
        assert_eq!(database.base_year, Some(2031));
        assert_eq!(
            database.snapshot_date.as_deref(),
            Some("2031-11-20T00:00:00+00:00")
        );
    }

    #[test]
    fn export_directory_to_ofm_includes_nested_subtree_files() {
        let temp = TempWorldDir::new();
        let src = temp.path().join("src");
        fs::create_dir_all(src.join("assets/logos")).expect("nested dirs should be created");
        fs::write(src.join("world.json"), b"{}").expect("root file should be written");
        fs::write(src.join("assets/logos/team.png"), b"PNG")
            .expect("nested file should be written");

        let out = temp.path().join("out.ofm");
        export_directory_to_ofm(&src, &out).expect("export of a valid tree should succeed");

        let archive = fs::File::open(&out).expect("archive should open");
        let mut zip = zip::ZipArchive::new(archive).expect("archive should be a valid zip");
        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().unwrap().into_owned())
            .collect();
        assert!(names.contains(&"world.json".to_string()));
        // The nested file proves the recursive read_dir walk still descends after
        // switching from swallow-on-error to propagate-on-error.
        assert!(
            names.contains(&"assets/logos/team.png".to_string()),
            "nested entry missing from archive: {names:?}"
        );
    }

    #[test]
    fn export_directory_to_ofm_errors_when_source_is_unreadable() {
        let temp = TempWorldDir::new();
        // Point at a path that does not exist: read_dir fails and the error must
        // now propagate instead of yielding an empty/partial archive.
        let missing = temp.path().join("does-not-exist");
        let out = temp.path().join("out.ofm");
        let result = export_directory_to_ofm(&missing, &out);
        assert!(
            result.is_err(),
            "unreadable source dir should surface an error"
        );
    }
    fn group_world_json(entrants: usize, format: serde_json::Value, selector: bool) -> String {
        let teams: Vec<domain::team::Team> = (0..entrants)
            .map(|i| {
                domain::team::Team::new(
                    format!("club-{i}"),
                    format!("Club {i}"),
                    format!("C{i}"),
                    "ENG".into(),
                    "City".into(),
                    "Ground".into(),
                    1000,
                )
            })
            .collect();
        let participants = if selector {
            serde_json::json!({"selector":{"kind":"topByReputation","country":"ENG","count":100}})
        } else {
            serde_json::json!({"explicit": teams.iter().map(|t| &t.id).collect::<Vec<_>>()})
        };
        serde_json::json!({"teams":teams, "competitionDefinitions":{"competitions":[{
            "id":"authored-cup", "name":"Authored Cup", "type":"Cup", "scope":"Domestic",
            "format":format, "participants":participants
        }]}})
        .to_string()
    }

    #[test]
    fn public_world_loader_honours_group_size_in_resolved_schedules() {
        use chrono::TimeZone;
        for (entrants, format, sizes, fixtures) in [
            (
                8,
                serde_json::json!({"kind":"GroupAndKnockout","groupSize":2}),
                vec![2, 2, 2, 2],
                8,
            ),
            (
                8,
                serde_json::json!({"kind":"GroupAndKnockout"}),
                vec![4, 4],
                24,
            ),
            (
                7,
                serde_json::json!({"kind":"GroupAndKnockout","groupSize":3,"bestThirdQualifiers":1}),
                vec![2, 2, 3],
                10,
            ),
            (
                64,
                serde_json::json!({"kind":"GroupAndKnockout","groupSize":2}),
                vec![2; 32],
                64,
            ),
        ] {
            let world = load_world_from_json(&group_world_json(entrants, format, false)).unwrap();
            let definitions = world.competition_definitions.as_ref().unwrap();
            let start = chrono::Utc.with_ymd_and_hms(2031, 7, 1, 0, 0, 0).unwrap();
            let cup = super::super::resolve_definitions(definitions, &world, 2031, start).remove(0);
            let mut actual: Vec<usize> = cup.groups.iter().map(|g| g.team_ids.len()).collect();
            actual.sort();
            assert_eq!(actual, sizes);
            assert_eq!(cup.fixtures.len(), fixtures);
            let group_ids: std::collections::HashSet<&String> =
                cup.groups.iter().map(|g| &g.id).collect();
            let group_names: std::collections::HashSet<&String> =
                cup.groups.iter().map(|g| &g.name).collect();
            assert_eq!(
                group_ids.len(),
                cup.groups.len(),
                "group ids must be unique"
            );
            assert_eq!(
                group_names.len(),
                cup.groups.len(),
                "group labels must be unique"
            );
            if entrants == 64 {
                assert_eq!(cup.groups[25].name, "Z");
                assert_eq!(cup.groups[26].name, "AA");
                assert_eq!(cup.groups[31].name, "AF");
            }
            let mut ids: Vec<&String> = cup.groups.iter().flat_map(|g| &g.team_ids).collect();
            ids.sort();
            let mut expected: Vec<&String> = world.teams.iter().map(|t| &t.id).collect();
            expected.sort();
            assert_eq!(ids, expected, "every entrant is assigned exactly once");
            assert!(cup.fixtures.iter().all(|f| {
                cup.groups.iter().any(|g| {
                    g.team_ids.contains(&f.home_team_id) && g.team_ids.contains(&f.away_team_id)
                })
            }));
        }
    }

    #[test]
    fn selector_cups_with_fewer_than_two_resolved_clubs_still_load_and_skip() {
        use chrono::TimeZone;
        for entrants in [0, 1] {
            let json = group_world_json(
                entrants,
                serde_json::json!({"kind":"GroupAndKnockout","groupSize":2}),
                true,
            );
            let world = load_world_from_json(&json).expect("empty selectors remain skippable");
            let start = chrono::Utc.with_ymd_and_hms(2031, 7, 1, 0, 0, 0).unwrap();
            assert!(
                super::super::resolve_definitions(
                    world.competition_definitions.as_ref().unwrap(),
                    &world,
                    2031,
                    start,
                )
                .is_empty()
            );
        }
    }

    #[test]
    fn package_group_qualification_uses_the_final_composed_field() {
        use crate::generator::{CompetitionDefinition, DefinitionSources, TeamDef, WorldPackage};
        let clubs = |range: std::ops::Range<usize>| -> Vec<TeamDef> {
            range.map(|i| serde_json::from_value(serde_json::json!({
                "id":format!("club-{i}"), "name":format!("Club {i}"), "city":"City", "country":"ENG",
                "colors":{"primary":"#000000", "secondary":"#ffffff"}
            })).unwrap()).collect()
        };
        let definition: CompetitionDefinition = serde_json::from_value(serde_json::json!({
            "id":"authored-cup", "name":"Authored Cup", "type":"Cup", "scope":"Domestic",
            "format":{"kind":"GroupAndKnockout","groupSize":3,"qualifiersPerGroup":3},
            "participants":{"selector":{"kind":"topByReputation","country":"ENG","count":100}}
        }))
        .unwrap();
        let mut layer = WorldPackage::default();
        layer.teams.extend(clubs(0..7));
        layer.competitions.push(definition);
        // Standalone authoring validation uses this layer's resolved field.
        let errors = crate::generator::validate_package(&layer);
        assert!(
            errors
                .iter()
                .any(|error| error.code == "be.error.competitionDef.invalidGroupQualification"),
            "standalone qualification must be checked before export: {errors:?}"
        );
        let sources = DefinitionSources::embedded_only();
        assert!(
            matches!(build_world_from_package(&layer, Some(2031), &sources),
            Err(key) if key == COMPETITION_DEFINITIONS_INVALID_ERROR)
        );

        let mut extension = WorldPackage::default();
        extension.teams.extend(clubs(7..9));
        let (merged, errors) = crate::generator::merge_world_packages(vec![layer, extension]);
        assert!(
            errors.is_empty(),
            "the composed package is valid: {errors:?}"
        );
        let world = build_world_from_package(&merged, Some(2031), &sources).unwrap();
        let errors = crate::generator::validate_definitions_for_world(
            world.competition_definitions.as_ref().unwrap(),
            &world,
        );
        assert!(
            errors.is_empty(),
            "qualification uses all nine final clubs: {errors:?}"
        );
    }

    /// Three entrants at maximum size two would leave a lone club with no match.
    #[test]
    fn public_world_loader_rejects_singleton_groups() {
        for selector in [false, true] {
            let json = group_world_json(
                3,
                serde_json::json!({
                    "kind":"GroupAndKnockout", "groupSize":2, "qualifiersPerGroup":1
                }),
                selector,
            );
            assert!(matches!(load_world_from_json(&json),
                Err(key) if key == COMPETITION_DEFINITIONS_INVALID_ERROR));
        }
    }

    #[test]
    fn public_world_loader_rejects_incompatible_group_qualification() {
        for selector in [false, true] {
            for (entrants, format) in [
                (
                    8,
                    serde_json::json!({"kind":"GroupAndKnockout","groupSize":0}),
                ),
                (
                    8,
                    serde_json::json!({"kind":"GroupAndKnockout","groupSize":1}),
                ),
                (
                    8,
                    serde_json::json!({"kind":"GroupAndKnockout","groupSize":2,"qualifiersPerGroup":0}),
                ),
                (
                    8,
                    serde_json::json!({"kind":"GroupAndKnockout","groupSize":2,"qualifiersPerGroup":3}),
                ),
                (
                    7,
                    serde_json::json!({"kind":"GroupAndKnockout","groupSize":3,"qualifiersPerGroup":3}),
                ),
                (
                    7,
                    serde_json::json!({"kind":"GroupAndKnockout","groupSize":3,"bestThirdQualifiers":2}),
                ),
                (
                    8,
                    serde_json::json!({"kind":"GroupAndKnockout","groupSize":2,"bestThirdQualifiers":1}),
                ),
            ] {
                let result = load_world_from_json(&group_world_json(entrants, format, selector));
                assert!(
                    matches!(result, Err(ref key) if key == COMPETITION_DEFINITIONS_INVALID_ERROR),
                    "selector={selector}, entrants={entrants}: {:?}",
                    result.as_ref().map(|_| "accepted")
                );
            }
        }
    }

    /// Given a package and a seed,
    /// When a world is built from them twice,
    /// Then it is the same world, ids included, it records the seed, and another seed gives
    ///      another world.
    #[test]
    fn a_seed_builds_the_same_package_world_ids_included() {
        let sources = crate::generator::DefinitionSources::embedded_only();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/modding/examples/classic-sixteen");
        let (package, errors) = crate::generator::load_world_package(&dir);
        assert!(errors.is_empty(), "{errors:?}");
        let everything = |world: &WorldData| {
            serde_json::to_value((&world.teams, &world.players, &world.staff))
                .unwrap()
                .to_string()
        };

        let one = build_world_from_package_seeded(5, &package, Some(2033), &sources).unwrap();
        let again = build_world_from_package_seeded(5, &package, Some(2033), &sources).unwrap();
        let other = build_world_from_package_seeded(6, &package, Some(2033), &sources).unwrap();

        assert_eq!(one.generation_seed, Some(5));
        assert_eq!(everything(&one), everything(&again));
        assert_ne!(everything(&one), everything(&other));
        assert_eq!(one.metadata.world_id, again.metadata.world_id);
        let world_id = uuid::Uuid::parse_str(&one.metadata.world_id).expect("a world id is a UUID");
        assert_eq!(
            world_id.get_version_num(),
            4,
            "a seeded id is still a version 4 UUID"
        );
        assert_ne!(one.metadata.world_id, other.metadata.world_id);
    }

    /// Given a package of one club and no competitions, which the builder pads with generated
    ///       opponents,
    /// When it is built twice from one seed,
    /// Then the padding is the same too — clubs, players and staff — so the dice drawn for it come
    ///      from the seed like every other.
    #[test]
    fn a_seed_builds_the_same_padded_one_club_package() {
        let sources = crate::generator::DefinitionSources::embedded_only();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/modding/examples/classic-sixteen");
        let (mut package, errors) = crate::generator::load_world_package(&dir);
        assert!(errors.is_empty(), "{errors:?}");
        package.teams.truncate(1);
        package.competitions.clear();
        package.players.clear();
        let everything = |world: &WorldData| {
            serde_json::to_value((&world.teams, &world.players, &world.staff))
                .unwrap()
                .to_string()
        };

        let one = build_world_from_package_seeded(5, &package, Some(2033), &sources).unwrap();
        let again = build_world_from_package_seeded(5, &package, Some(2033), &sources).unwrap();
        let other = build_world_from_package_seeded(6, &package, Some(2033), &sources).unwrap();

        assert!(one.teams.len() > 1, "the opponents were generated");
        assert_eq!(everything(&one), everything(&again));
        assert_ne!(everything(&one), everything(&other));
    }
}
