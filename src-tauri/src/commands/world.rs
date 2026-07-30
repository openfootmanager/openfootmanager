use std::sync::Arc;
use chrono::Datelike;
use log::{info, warn};
use tauri::Manager as TauriManager;
use tauri::State;

use ofm_core::state::StateManager;

const EXPORTED_WORLD_NAME_KEY: &str = "be.msg.world.exportedName";
const EXPORTED_WORLD_DESCRIPTION_KEY: &str = "be.msg.world.exportedDescription";
const RANDOM_WORLD_NAME_KEY: &str = "be.msg.world.randomName";
const RANDOM_WORLD_DESCRIPTION_KEY: &str = "be.msg.world.randomDescription";
const TEAM_COUNT_PARAM: &str = "teamCount";

fn backend_text_with_param(key: &str, param_name: &str, param_value: impl ToString) -> String {
    let mut text = String::from(key);
    text.push('?');
    text.push_str(param_name);
    text.push('=');
    text.push_str(&param_value.to_string());
    text
}

pub fn export_world_database_internal(
    state: &StateManager,
    export_path: &std::path::Path,
) -> Result<String, String> {
    let game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;
    let stats = state
        .get_stats_state(|stats| stats.clone())
        .unwrap_or_default();
    let mut managers = game.managers.clone();
    if let Some(existing) = managers
        .iter_mut()
        .find(|manager| manager.id == game.manager_id)
    {
        *existing = game.manager.clone();
    } else {
        managers.push(game.manager.clone());
    }

    let world = ofm_core::generator::WorldData {
        name: EXPORTED_WORLD_NAME_KEY.to_string(),
        description: backend_text_with_param(
            EXPORTED_WORLD_DESCRIPTION_KEY,
            TEAM_COUNT_PARAM,
            game.teams.len(),
        ),
        teams: game.teams.clone(),
        players: game.players.clone(),
        staff: game.staff.clone(),
        managers,
        competitions: game.competitions.clone(),
        competition_definitions: None,
        national_teams: game.national_teams.clone(),
        regions: Vec::new(),
        default_active_regions: game.active_region_ids.clone(),
        default_active_competitions: game.active_competition_ids.clone(),
        league: game.league.clone(),
        news: game.news.clone(),
        stats,
        world_history: game.world_history.clone(),
        metadata: ofm_core::generator::WorldDataMetadata {
            format_version: 2,
            world_id: format!(
                "world-export-{}-{}",
                game.manager.id,
                game.clock.current_date.timestamp()
            ),
            kind: ofm_core::generator::WorldDataKind::HistoricalSnapshot,
            base_year: Some(game.clock.start_date.year()),
            snapshot_date: Some(game.clock.current_date.to_rfc3339()),
        },
        extra_translations: game.extra_translations.clone(),
        build_notices: Vec::new(),
    };

    ofm_core::generator::export_world_package(&world, export_path)
        .map_err(|_| "be.error.worldWriteFileFailed".to_string())
}

fn write_database_json_to_dir(db_dir: &std::path::Path, json: &str) -> Result<String, String> {
    std::fs::create_dir_all(db_dir).map_err(|_| "be.error.worldWriteDatabaseFailed".to_string())?;

    let world = ofm_core::generator::load_world_from_json(json)?;
    let normalized_json = ofm_core::generator::export_world_to_json(&world)?;

    let filename = format!(
        "imported_{}.json",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    let path = db_dir.join(filename);
    std::fs::write(&path, normalized_json)
        .map_err(|_| "be.error.worldWriteDatabaseFailed".to_string())?;
    Ok(path.to_string_lossy().to_string())
}

/// List available world databases (built-in random + any user JSON files).
#[tauri::command]
pub fn list_world_databases(
    app_handle: tauri::AppHandle,
) -> Result<Vec<ofm_core::generator::WorldDatabaseInfo>, String> {
    info!("[cmd] list_world_databases");
    use ofm_core::generator::WorldDatabaseInfo;

    // Always include the built-in random option. Counts mirror the standard
    // generation config used by generate_world_data(None): 440 clubs with
    // 22-man squads (= 9,680 players). Keep in sync if that config changes.
    let mut databases = vec![WorldDatabaseInfo {
        id: "random".to_string(),
        name: RANDOM_WORLD_NAME_KEY.to_string(),
        description: backend_text_with_param(RANDOM_WORLD_DESCRIPTION_KEY, TEAM_COUNT_PARAM, 440),
        team_count: 440,
        player_count: 9_680,
        history_mode: "generated".to_string(),
        base_year: None,
        snapshot_date: None,
        source: "builtin".to_string(),
        path: String::new(),
    }];

    // Scan bundled databases directory (next to the executable / in resources)
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let bundled_dir = resource_dir.join("databases");
        let mut bundled = ofm_core::generator::scan_world_databases(&bundled_dir);
        for db in &mut bundled {
            db.source = "builtin".to_string();
        }
        databases.extend(bundled);
    }

    // Scan user databases directory in app data
    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        let user_dir = app_data_dir.join("databases");
        let user_dbs = ofm_core::generator::scan_world_databases(&user_dir);
        databases.extend(user_dbs);
    }

    Ok(databases)
}

/// Export the current world data to a JSON file so it can be shared/reused.
#[tauri::command]
pub fn export_world_database(
    state: State<'_, Arc<StateManager>>,
    export_path: String,
) -> Result<String, String> {
    info!("[cmd] export_world_database: path={}", export_path);
    export_world_database_internal(&state, std::path::Path::new(&export_path))
}

/// Write imported world database JSON to the user's databases directory.
/// Returns the full path so the frontend can pass it to start_new_game.
#[tauri::command]
pub fn write_temp_database(app_handle: tauri::AppHandle, json: String) -> Result<String, String> {
    info!("[cmd] write_temp_database: json_len={}", json.len());
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let db_dir = app_data_dir.join("databases");
    write_database_json_to_dir(&db_dir, &json)
}

/// Where a package's extracted artwork lives.
///
/// Kept beside the installed archives rather than inside `packages/` so a
/// directory listing of installed `.ofm` files stays a list of files. Allowed by
/// the asset protocol scope in `tauri.conf.json`, which is what lets the webview
/// load these files at all.
pub fn package_assets_dir(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("package-assets"))
}

fn packages_dir(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("packages"))
}

fn package_info_from_path(
    path: &std::path::Path,
) -> Option<ofm_core::generator::PackageInfo> {
    let meta = ofm_core::generator::read_package_manifest_from_ofm(path)?;
    let id = if meta.id.is_empty() {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    } else {
        meta.id.clone()
    };

    let (package, errors) = ofm_core::generator::load_world_package_from_ofm(path);
    if !errors.is_empty() {
        return None;
    }
    let logo_data_url = meta.logo.as_deref()
        .and_then(|logo| ofm_core::generator::read_logo_from_ofm(path, logo));
    Some(ofm_core::generator::PackageInfo {
        id,
        name: meta.name,
        version: meta.version,
        author: meta.author,
        description: meta.description,
        license: meta.license,
        game_min_version: meta.game_min_version,
        package_type: meta.package_type,
        team_count: package.teams.len(),
        player_count: package.players.len(),
        competition_count: package.competitions.len(),
        installed_path: path.to_string_lossy().to_string(),
        logo_data_url,
    })
}

/// Install a `.ofm` package file into the user's packages directory.
#[tauri::command]
pub fn install_package(
    app_handle: tauri::AppHandle,
    path: String,
) -> Result<ofm_core::generator::PackageInfo, String> {
    info!("[cmd] install_package: path={}", path);
    let src = std::path::Path::new(&path);

    // Reject archives that exceed the on-disk size limit before doing any I/O.
    let src_size = std::fs::metadata(src)
        .map(|m| m.len())
        .unwrap_or(0);
    if src_size > ofm_core::generator::MAX_ARCHIVE_BYTES {
        return Err("be.error.package.archiveTooLarge".to_string());
    }

    let meta = ofm_core::generator::read_package_manifest_from_ofm(src)
        .ok_or_else(|| "be.error.package.noManifest".to_string())?;

    // Enforce game_min_version: reject packages that require a newer game version.
    // A non-parseable version string is treated as incompatible rather than skipped.
    if !meta.game_min_version.is_empty() {
        match semver::Version::parse(&meta.game_min_version) {
            Ok(required) => {
                let current = app_handle.package_info().version.clone();
                if current < required {
                    return Err("be.error.package.versionTooOld".to_string());
                }
            }
            Err(_) => return Err("be.error.package.versionTooOld".to_string()),
        }
    }

    let id = if meta.id.is_empty() {
        src.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    } else {
        meta.id.clone()
    };
    // The id (from the package manifest) becomes a filename under packages_dir,
    // so reject traversal tokens before it is ever joined into a path.
    validate_package_id(&id)?;

    // Validate before copying: surface extraction errors (symlinks, zip-slip, size) and
    // cross-reference errors (unknown country, unknown team) before the file is installed.
    let (package, validation_errors) = ofm_core::generator::load_world_package_from_ofm(src);
    if let Some(err) = validation_errors.first() {
        return Err(err.code.clone());
    }
    let ref_errors = ofm_core::generator::validate_references(&package);
    if let Some(err) = ref_errors.first() {
        return Err(err.code.clone());
    }

    let packages_dir = packages_dir(&app_handle)?;
    std::fs::create_dir_all(&packages_dir)
        .map_err(|_| "be.error.package.installFailed".to_string())?;

    let dest = packages_dir.join(format!("{id}.ofm"));
    std::fs::copy(src, &dest).map_err(|_| "be.error.package.installFailed".to_string())?;

    // Land the artwork now, so "extracted assets exist" tracks "package is
    // installed" and nothing else. World creation extracts too, but a save
    // stores already-qualified paths and reloads without ever going through
    // that path — so a package uninstalled and reinstalled after the save was
    // made would otherwise leave every club in it on a generated crest forever.
    // Best effort, exactly as at world load: the world still plays without art.
    if let Ok(assets_root) = package_assets_dir(&app_handle) {
        extract_assets_for_package(&dest, &assets_root, &id);
    }

    let logo_data_url = meta.logo.as_deref()
        .and_then(|logo| ofm_core::generator::read_logo_from_ofm(&dest, logo));
    Ok(ofm_core::generator::PackageInfo {
        id,
        name: meta.name,
        version: meta.version,
        author: meta.author,
        description: meta.description,
        license: meta.license,
        game_min_version: meta.game_min_version,
        package_type: meta.package_type,
        team_count: package.teams.len(),
        player_count: package.players.len(),
        competition_count: package.competitions.len(),
        installed_path: dest.to_string_lossy().to_string(),
        logo_data_url,
    })
}

/// List all installed `.ofm` packages in the user's packages directory.
#[tauri::command]
pub fn list_installed_packages(
    app_handle: tauri::AppHandle,
) -> Result<Vec<ofm_core::generator::PackageInfo>, String> {
    info!("[cmd] list_installed_packages");
    let packages_dir = packages_dir(&app_handle)?;
    if !packages_dir.exists() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(&packages_dir) else {
        return Ok(result);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("ofm") {
            if let Some(info) = package_info_from_path(&path) {
                result.push(info);
            }
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// Reject any package id that contains path separators or traversal tokens.
pub(crate) fn validate_package_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
        || id.contains('\0')
        // "assets" is reserved: a qualified path is `<package_id>/assets/...`,
        // so this id yields `assets/assets/…`, which `isPackageQualifiedAsset`
        // on the frontend classifies as an app-bundled asset and resolves
        // against the wrong root. Refusing the id keeps that classifier simple.
        || id == RESERVED_PACKAGE_ID
    {
        return Err("be.error.package.invalid".to_string());
    }
    Ok(())
}

/// The one package id that cannot be installed — it collides with the prefix
/// that tells a bundled asset path from a package-qualified one.
const RESERVED_PACKAGE_ID: &str = "assets";

/// Remove an installed package by id.
#[tauri::command]
pub fn uninstall_package(
    app_handle: tauri::AppHandle,
    id: String,
) -> Result<(), String> {
    info!("[cmd] uninstall_package: id={}", id);
    validate_package_id(&id)?;
    let dest = packages_dir(&app_handle)?.join(format!("{id}.ofm"));
    if dest.exists() {
        std::fs::remove_file(&dest)
            .map_err(|_| "be.error.package.installFailed".to_string())?;
    }
    // The archive is not the only thing on disk: its artwork was extracted
    // alongside it at world load. Leaving that behind would accumulate an
    // orphaned directory per package the user ever installed.
    if let Ok(assets_root) = package_assets_dir(&app_handle) {
        remove_package_assets(&assets_root, &id);
    }
    Ok(())
}

/// Unpack a package's artwork into `<assets_root>/<id>/`. Best effort: a
/// package with no artwork produces no directory, and a failure here must not
/// block the install — the world plays, its clubs just keep generated crests.
fn extract_assets_for_package(ofm_path: &std::path::Path, assets_root: &std::path::Path, id: &str) {
    let package_assets = assets_root.join(id);
    match ofm_core::generator::extract_package_assets(ofm_path, &package_assets) {
        Ok(skipped) if !skipped.is_empty() => {
            warn!("[assets] {id}: {} asset entries skipped", skipped.len());
        }
        Err(err) => warn!("[assets] {id}: extraction failed: {err}"),
        _ => {}
    }
}

/// Delete a package's extracted artwork. Best effort: a package with no assets
/// has no directory, and a failure here must not block the uninstall itself.
fn remove_package_assets(assets_root: &std::path::Path, id: &str) {
    let dir = assets_root.join(id);
    if dir.exists() {
        if let Err(err) = std::fs::remove_dir_all(&dir) {
            warn!("[assets] could not remove {}: {err}", dir.display());
        }
    }
}

/// Serialisable conflict info returned to the frontend.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConflictInfo {
    pub severity: String,
    pub code: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub packages: Vec<String>,
}

impl From<ofm_core::generator::StackConflict> for ConflictInfo {
    fn from(c: ofm_core::generator::StackConflict) -> Self {
        Self {
            severity: match c.severity {
                ofm_core::generator::ConflictSeverity::Warning => "warning".to_string(),
                ofm_core::generator::ConflictSeverity::Error => "error".to_string(),
            },
            code: c.code,
            entity_kind: c.entity_kind,
            entity_id: c.entity_id,
            packages: c.packages,
        }
    }
}

/// Validate a stack of installed packages by id and return any conflicts.
/// Called live from WorldSelect as the user adds/removes packages.
#[tauri::command]
pub fn check_package_stack(
    app_handle: tauri::AppHandle,
    package_ids: Vec<String>,
) -> Result<Vec<ConflictInfo>, String> {
    let packages_dir = packages_dir(&app_handle)?;
    let mut loaded = Vec::with_capacity(package_ids.len());
    for id in &package_ids {
        validate_package_id(id)?;
        let path = packages_dir.join(format!("{id}.ofm"));
        let (pkg, errors) = ofm_core::generator::load_world_package_from_ofm(&path);
        if !errors.is_empty() {
            return Err("be.error.package.invalid".to_string());
        }
        loaded.push(pkg);
    }
    let refs: Vec<&ofm_core::generator::WorldPackage> = loaded.iter().collect();
    let conflicts = ofm_core::generator::validate_package_stack(&refs)
        .into_iter()
        .map(ConflictInfo::from)
        .collect();
    Ok(conflicts)
}

/// A selectable football nation, mirrored from the backend's combined nation
/// catalog (`all_nations()` — World Cup pool + wider FIFA membership).
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct NationInfo {
    pub code: String,
    pub name: String,
    pub region: String,
}

/// Every nation the game recognises (World Cup pool + wider FIFA membership).
/// This is the single source of truth for which nationalities are selectable in
/// the world editor / manager creation, so the frontend never offers a nation
/// the importer would reject (see #270). Codes match the frontend's ISO +
/// football-identity handling.
#[tauri::command]
pub fn get_nations() -> Vec<NationInfo> {
    ofm_core::nations::all_nations()
        .map(|nation| NationInfo {
            code: nation.code.to_string(),
            name: nation.name.to_string(),
            region: nation.region_id.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        export_world_database_internal, validate_package_id, write_database_json_to_dir,
        EXPORTED_WORLD_NAME_KEY,
    };
    use chrono::{TimeZone, Utc};
    use domain::league::League;
    use domain::manager::Manager;
    use domain::news::{NewsArticle, NewsCategory};
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::stats::StatsState;
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::generator::{load_world_from_path, WorldData, WorldDataKind, WorldManifestV2};
    use ofm_core::state::StateManager;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempCommandDir {
        path: PathBuf,
    }

    impl TempCommandDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ofm-world-command-tests-{}", unique));
            fs::create_dir_all(&path).expect("temporary command dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempCommandDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn sample_attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 65,
            stamina: 65,
            strength: 65,
            agility: 65,
            passing: 65,
            shooting: 65,
            tackling: 65,
            dribbling: 65,
            defending: 65,
            positioning: 65,
            vision: 65,
            decisions: 65,
            composure: 65,
            aggression: 50,
            teamwork: 65,
            leadership: 50,
            handling: 20,
            reflexes: 20,
            aerial: 60,
        }
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr-1".to_string(),
            "Ada".to_string(),
            "Lovelace".to_string(),
            "1980-01-01".to_string(),
            "British".to_string(),
        );
        manager.hire("team-1".to_string());

        let mut team = Team::new(
            "team-1".to_string(),
            "London FC".to_string(),
            "LFC".to_string(),
            "GB".to_string(),
            "London".to_string(),
            "London Arena".to_string(),
            50_000,
        );
        team.football_nation.clear();

        let mut player = Player::new(
            "player-1".to_string(),
            "J. Doe".to_string(),
            "John Doe".to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            sample_attrs(),
        );
        player.team_id = Some("team-1".to_string());
        player.football_nation.clear();
        player.birth_country = None;

        Game::new(clock, manager, vec![team], vec![player], vec![], vec![])
    }

    #[test]
    fn export_world_database_internal_writes_canonicalized_world_json() {
        let temp_dir = TempCommandDir::new();
        let export_path = temp_dir.path().join("world-export.json");
        let state = StateManager::new();
        let mut game = make_game();
        game.teams[0].football_nation.clear();
        game.players[0].football_nation.clear();
        game.players[0].birth_country = None;
        game.league = Some(League::new(
            "league-1".to_string(),
            "Open League".to_string(),
            2026,
            &[game.teams[0].id.clone()],
        ));
        game.news.push(NewsArticle::new(
            "news-1".to_string(),
            "Season underway".to_string(),
            "The campaign has begun.".to_string(),
            "World Feed".to_string(),
            "2026-07-01".to_string(),
            NewsCategory::SeasonPreview,
        ));
        game.world_history
            .upsert_rivalry("team-1", "team-2", 70, Some(2025));
        state.set_game(game);
        state.set_stats_state(StatsState::default());

        let written_path = export_world_database_internal(&state, &export_path).unwrap();
        let json = fs::read_to_string(&written_path).unwrap();
        let manifest: WorldManifestV2 = serde_json::from_str(&json).unwrap();
        let world = load_world_from_path(Path::new(&written_path)).unwrap();
        let raw_json: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(manifest.name, EXPORTED_WORLD_NAME_KEY);
        assert_eq!(
            world.description,
            "be.msg.world.exportedDescription?teamCount=1"
        );
        assert_eq!(world.teams[0].football_nation, "ENG");
        assert_eq!(world.players[0].football_nation, "ENG");
        assert_eq!(world.managers.len(), 1);
        assert_eq!(world.managers[0].football_nation, "ENG");
        assert_eq!(
            world.league.as_ref().map(|league| league.season),
            Some(2026)
        );
        assert_eq!(world.news.len(), 1);
        assert_eq!(world.world_history.rivalries.len(), 1);
        assert_eq!(world.metadata.kind, WorldDataKind::HistoricalSnapshot);
        assert_eq!(manifest.format_version, 2);
        assert!(!manifest.world_id.is_empty());
        assert!(raw_json.get("shards").is_some());
        assert!(raw_json.get("compatibility").is_some());
    }

    #[test]
    fn export_world_database_internal_requires_active_game() {
        let temp_dir = TempCommandDir::new();
        let export_path = temp_dir.path().join("world-export.json");
        let state = StateManager::new();

        let result = export_world_database_internal(&state, &export_path);

        assert_eq!(result.unwrap_err(), "be.error.noActiveGameSession");
    }

    #[test]
    fn export_world_database_internal_returns_write_file_key_on_write_failure() {
        let temp_dir = TempCommandDir::new();
        let state = StateManager::new();
        state.set_game(make_game());

        let result = export_world_database_internal(&state, temp_dir.path());

        assert_eq!(result.unwrap_err(), "be.error.worldWriteFileFailed");
    }

    #[test]
    fn write_database_json_to_dir_normalizes_imported_world_json() {
        let temp_dir = TempCommandDir::new();
        let json = r##"
        {
          "name": "Legacy Import",
          "description": "Old GB import",
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

        let written_path = write_database_json_to_dir(temp_dir.path(), json).unwrap();
        let stored_json = fs::read_to_string(&written_path).unwrap();
        let world: WorldData = serde_json::from_str(&stored_json).unwrap();

        assert_eq!(world.teams[0].football_nation, "ENG");
        assert_eq!(world.players[0].football_nation, "ENG");
    }

    #[test]
    fn write_database_json_to_dir_rejects_invalid_json() {
        let temp_dir = TempCommandDir::new();
        let result = write_database_json_to_dir(temp_dir.path(), "not valid json");

        assert_eq!(result.unwrap_err(), "be.error.worldParseFailed");
        let written_files = fs::read_dir(temp_dir.path()).unwrap().count();
        assert_eq!(written_files, 0);
    }

    #[test]
    fn write_database_json_to_dir_returns_write_database_key_when_dir_cannot_be_created() {
        let temp_dir = TempCommandDir::new();
        let blocked_path = temp_dir.path().join("blocked-path");
        fs::write(&blocked_path, "occupied").expect("blocked path file should be created");

        let result = write_database_json_to_dir(&blocked_path, "{}");

        assert_eq!(result.unwrap_err(), "be.error.worldWriteDatabaseFailed");
    }

    #[test]
    fn removing_package_assets_deletes_only_that_package() {
        // Uninstalling has to take the extracted artwork with it, or every
        // package a user ever installed leaves a directory behind.
        let temp_dir = TempCommandDir::new();
        let root = temp_dir.path().join("package-assets");
        for id in ["going", "staying"] {
            let dir = root.join(id).join("assets/images");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("badge.png"), b"PNG").unwrap();
        }

        super::remove_package_assets(&root, "going");

        assert!(
            !root.join("going").exists(),
            "the package's assets should be gone"
        );
        assert!(
            root.join("staying/assets/images/badge.png").exists(),
            "another package's assets must be untouched",
        );
    }

    #[test]
    fn installing_a_package_lands_its_artwork_for_an_existing_save() {
        // A save stores already-qualified asset paths and reloads without going
        // through world creation, so extraction cannot only happen there: a
        // package reinstalled after the save was made would never get its
        // artwork back. Installing is what puts it on disk.
        let temp_dir = TempCommandDir::new();
        let assets_root = temp_dir.path().join("package-assets");
        let archive = temp_dir.path().join("badge-pkg.ofm");
        write_ofm_with_badge(&archive);

        super::extract_assets_for_package(&archive, &assets_root, "badge-pkg");

        assert_eq!(
            fs::read(assets_root.join("badge-pkg/assets/images/santos.png")).unwrap(),
            b"PNGBYTES",
            "the badge should be readable at the path a save's qualified path resolves to",
        );
    }

    /// A minimal `.ofm` carrying one image under the asset tree. Built through
    /// the exporter rather than the zip crate directly, so the app crate does
    /// not take a dependency just to author a fixture.
    fn write_ofm_with_badge(path: &Path) {
        let source = path.with_extension("src");
        let images = source.join("assets/images");
        fs::create_dir_all(&images).expect("fixture tree should be created");
        fs::write(images.join("santos.png"), b"PNGBYTES").expect("badge should be written");
        ofm_core::generator::export_directory_to_ofm(&source, path).expect("archive should build");
    }

    #[test]
    fn removing_package_assets_tolerates_a_package_with_none() {
        let temp_dir = TempCommandDir::new();
        let root = temp_dir.path().join("package-assets");
        std::fs::create_dir_all(&root).unwrap();

        super::remove_package_assets(&root, "never-had-assets");
    }

    #[test]
    fn validate_package_id_rejects_traversal_tokens() {
        // Legitimate ids pass.
        assert!(validate_package_id("eng-premier-league").is_ok());
        assert!(validate_package_id("brasileirao_2026").is_ok());
        // Anything that could escape packages_dir as a path component is rejected.
        for bad in [
            "",
            "..",
            "../evil",
            "../../etc/passwd",
            "a/b",
            "a\\b",
            "with\0null",
            // Qualified asset paths are `<package_id>/assets/...`, so a package
            // called "assets" produces `assets/assets/images/x.png`, which the
            // frontend classifier reads as an app-bundled path and serves from
            // the wrong root — every club in it silently loses its badge.
            "assets",
        ] {
            assert!(
                validate_package_id(bad).is_err(),
                "expected {bad:?} to be rejected"
            );
        }
    }
}
