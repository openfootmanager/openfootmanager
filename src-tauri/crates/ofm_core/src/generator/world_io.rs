use super::definitions::{
    WorldData, WorldDatabaseInfo, WorldManifestV2, WorldRegionDefinition, WorldShardRefs,
};
use std::path::{Path, PathBuf};

const WORLD_PARSE_FAILED_ERROR: &str = "be.error.worldParseFailed";
const WORLD_SERIALIZE_FAILED_ERROR: &str = "be.error.worldSerializeFailed";
const RANDOM_WORLD_NAME_KEY: &str = "be.msg.world.randomName";
const RANDOM_WORLD_DESCRIPTION_KEY: &str = "be.msg.world.randomDescription";

fn infer_region_id(country_code: &str) -> &'static str {
    match country_code {
        "BR" | "AR" | "UY" | "CL" | "CO" | "PE" | "EC" | "VE" | "PY" | "BO" => "south-america",
        "US" | "CA" | "MX" => "north-america",
        "CR" | "PA" | "HN" | "GT" | "SV" | "NI" => "central-america",
        "AU" | "NZ" => "oceania",
        "JP" | "KR" | "CN" | "SA" | "QA" => "asia",
        _ => "europe",
    }
}

fn region_name(region_id: &str) -> &'static str {
    match region_id {
        "north-america" => "North America",
        "central-america" => "Central America",
        "south-america" => "South America",
        "asia" => "Asia",
        "oceania" => "Oceania",
        _ => "Europe",
    }
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
                name: region_name(&region_id).to_string(),
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
/// If `data_dir` is provided, tries to load definition files from that directory.
pub fn generate_world_data(data_dir: Option<&std::path::Path>) -> WorldData {
    world_data_from_parts(super::generate_world(data_dir))
}

/// Deterministic variant of [`generate_world_data`]: same `seed` → identical world.
pub fn generate_world_data_seeded(seed: u64, data_dir: Option<&std::path::Path>) -> WorldData {
    world_data_from_parts(super::generate_world_seeded(seed, data_dir))
}

/// Deterministic generation with an explicit config — e.g. a small world for
/// fast, reproducible scenario tests.
pub fn generate_world_data_seeded_with(
    seed: u64,
    config: &super::WorldGenConfig,
    data_dir: Option<&std::path::Path>,
) -> WorldData {
    use rand::SeedableRng;
    world_data_from_parts(super::generate_world_with_rng(
        rand::rngs::StdRng::seed_from_u64(seed),
        config,
        data_dir,
    ))
}

fn world_data_from_parts(
    (mut teams, mut players, mut staff): (
        Vec<domain::team::Team>,
        Vec<domain::player::Player>,
        Vec<domain::staff::Staff>,
    ),
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
    })
}

const COMPETITION_DEFINITIONS_INVALID_ERROR: &str = "be.error.competitionDef.invalidEmbedded";

/// Reject a world whose embedded competition definitions don't validate, so a
/// broken definition file never loads half-applied.
fn validate_embedded_definitions(world: &WorldData) -> Result<(), String> {
    if let Some(file) = &world.competition_definitions {
        let ctx = super::competition_def::WorldValidationContext::from_world(world);
        if !super::competition_def::validate_definitions(file, &ctx).is_empty() {
            return Err(COMPETITION_DEFINITIONS_INVALID_ERROR.to_string());
        }
    }
    Ok(())
}

/// Parse a JSON string into a `WorldData`.
pub fn load_world_from_json(json: &str) -> Result<WorldData, String> {
    let world: WorldData =
        serde_json::from_str(json).map_err(|_| WORLD_PARSE_FAILED_ERROR.to_string())?;
    let world = normalize_world(world);
    validate_embedded_definitions(&world)?;
    Ok(world)
}

/// Build a runnable, finalised `WorldData` from a validated world package —
/// normalised and with its embedded definitions checked, exactly like a loaded
/// world file. Call only after [`super::load_world_package`] reports no errors.
pub fn build_world_from_package(
    package: &super::package::WorldPackage,
) -> Result<WorldData, String> {
    let world = normalize_world(super::build_world_data_from_package(package));
    validate_embedded_definitions(&world)?;
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
    }))
}

pub fn load_world_from_path(path: &Path) -> Result<WorldData, String> {
    let json = std::fs::read_to_string(path).map_err(|_| WORLD_PARSE_FAILED_ERROR.to_string())?;
    if let Ok(manifest) = serde_json::from_str::<WorldManifestV2>(&json)
        && manifest.format_version >= 2
        && !manifest.shards.teams.is_empty()
    {
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

    #[test]
    fn export_world_to_json_writes_canonical_football_identity_fields() {
        let mut world = generate_world_data(None);
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
        let mut world = generate_world_data(None);
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
            .map(|i| zip.by_index(i).unwrap().name().to_string())
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
}
