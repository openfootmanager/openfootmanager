use ofm_core::generator::{
    export_directory_to_ofm, extract_ofm_to_dir, load_world_package, CompetitionDefinition,
    ConfederationDef, CountryDef, NamesDefinition, PlayerDef, StaffDef, TeamDef, WorldMetaDef,
};
use serde_json::json;
use std::path::Path;
use tauri::Manager as _;

// ---------------------------------------------------------------------------
// Return types
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageIssue {
    pub code: String,
    pub file: String,
    pub params: std::collections::HashMap<String, String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageProjectData {
    pub meta: WorldMetaDef,
    pub confederations: Vec<ConfederationDef>,
    pub countries: Vec<CountryDef>,
    pub teams: Vec<TeamDef>,
    pub players: Vec<PlayerDef>,
    pub staff: Vec<StaffDef>,
    pub names: Option<NamesDefinition>,
    pub competitions: Vec<CompetitionDefinition>,
    pub issues: Vec<PackageIssue>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_json_atomic(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let content = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &content).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn meta_to_manifest(meta: &WorldMetaDef) -> Result<serde_json::Value, String> {
    let mut v = serde_json::to_value(meta).map_err(|e| e.to_string())?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert("schema".to_string(), json!("world"));
    }
    Ok(v)
}

fn names_to_file(names: &NamesDefinition) -> Result<serde_json::Value, String> {
    let mut v = serde_json::to_value(names).map_err(|e| e.to_string())?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert("schema".to_string(), json!("names"));
    }
    Ok(v)
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn scaffold_project_dir(pkg_dir: &Path, meta: &WorldMetaDef) -> Result<(), String> {
    let subdirs = [
        "teams",
        "players",
        "staff",
        "confederations",
        "countries",
        "competitions",
        "names",
    ];
    for sub in &subdirs {
        std::fs::create_dir_all(pkg_dir.join(sub)).map_err(|e| e.to_string())?;
    }

    let manifest = meta_to_manifest(meta)?;
    write_json_atomic(&pkg_dir.join("package.json"), &manifest)?;

    let stubs: &[(&str, &str, serde_json::Value)] = &[
        (
            "teams",
            "teams.json",
            json!({"schema": "team", "items": []}),
        ),
        (
            "players",
            "players.json",
            json!({"schema": "player", "items": []}),
        ),
        (
            "staff",
            "staff.json",
            json!({"schema": "staff", "items": []}),
        ),
        (
            "confederations",
            "confederations.json",
            json!({"schema": "confederation", "items": []}),
        ),
        (
            "countries",
            "countries.json",
            json!({"schema": "country", "items": []}),
        ),
        (
            "competitions",
            "competitions.json",
            json!({"schema": "competition", "items": []}),
        ),
        (
            "names",
            "names.json",
            json!({"schema": "names", "version": 1, "description": "", "pools": {}}),
        ),
    ];

    for (sub, file, content) in stubs {
        write_json_atomic(&pkg_dir.join(sub).join(file), content)?;
    }

    Ok(())
}

fn dir_is_nonempty(path: &Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut d| d.next().is_some())
        .unwrap_or(false)
}

/// Create a new package project directory with an empty scaffold.
#[tauri::command]
pub fn create_package_project(dir: String, meta: WorldMetaDef) -> Result<(), String> {
    let pkg_dir = Path::new(&dir);
    if pkg_dir.exists() && dir_is_nonempty(pkg_dir) {
        return Err("be.error.package.projectAlreadyExists".to_string());
    }
    scaffold_project_dir(pkg_dir, &meta)
}

/// Create a new world project under the app-managed `world-editor/<slug>/` directory.
/// Returns the absolute path to the created project directory.
#[tauri::command]
pub fn create_world_project(
    app_handle: tauri::AppHandle,
    slug: String,
    meta: WorldMetaDef,
) -> Result<String, String> {
    sanitize_entity_id(&slug)?;
    let base_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let project_dir = base_dir.join("world-editor").join(&slug);
    if project_dir.exists() && dir_is_nonempty(&project_dir) {
        return Err("be.error.package.projectAlreadyExists".to_string());
    }
    scaffold_project_dir(&project_dir, &meta)?;
    project_dir
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "be.error.invalidPath".to_string())
}

/// Load an existing package directory for editing.
#[tauri::command]
pub fn read_package_project(dir: String) -> Result<PackageProjectData, String> {
    let path = Path::new(&dir);
    let (pkg, errors) = load_world_package(path);

    let issues = errors
        .into_iter()
        .map(|e| PackageIssue {
            code: e.code,
            file: e.file,
            params: e.params.into_iter().collect(),
        })
        .collect();

    Ok(PackageProjectData {
        meta: pkg.meta.unwrap_or_default(),
        confederations: pkg.confederations,
        countries: pkg.countries,
        teams: pkg.teams,
        players: pkg.players,
        staff: pkg.staff,
        names: pkg.names,
        competitions: pkg.competitions,
        issues,
    })
}

/// Persist in-memory edits: atomically overwrites all package entity files.
#[tauri::command]
pub fn save_package_project(
    dir: String,
    meta: WorldMetaDef,
    confederations: Vec<ConfederationDef>,
    countries: Vec<CountryDef>,
    teams: Vec<TeamDef>,
    players: Vec<PlayerDef>,
    staff: Vec<StaffDef>,
    names: NamesDefinition,
    competitions: Vec<CompetitionDefinition>,
) -> Result<(), String> {
    let pkg_dir = Path::new(&dir);

    write_json_atomic(&pkg_dir.join("package.json"), &meta_to_manifest(&meta)?)?;

    let confs = serde_json::to_value(&confederations).map_err(|e| e.to_string())?;
    write_json_atomic(
        &pkg_dir.join("confederations").join("confederations.json"),
        &json!({"schema": "confederation", "items": confs}),
    )?;

    let ctrs = serde_json::to_value(&countries).map_err(|e| e.to_string())?;
    write_json_atomic(
        &pkg_dir.join("countries").join("countries.json"),
        &json!({"schema": "country", "items": ctrs}),
    )?;

    let tms = serde_json::to_value(&teams).map_err(|e| e.to_string())?;
    write_json_atomic(
        &pkg_dir.join("teams").join("teams.json"),
        &json!({"schema": "team", "items": tms}),
    )?;

    let pls = serde_json::to_value(&players).map_err(|e| e.to_string())?;
    write_json_atomic(
        &pkg_dir.join("players").join("players.json"),
        &json!({"schema": "player", "items": pls}),
    )?;

    std::fs::create_dir_all(pkg_dir.join("staff")).map_err(|e| e.to_string())?;
    let stf = serde_json::to_value(&staff).map_err(|e| e.to_string())?;
    write_json_atomic(
        &pkg_dir.join("staff").join("staff.json"),
        &json!({"schema": "staff", "items": stf}),
    )?;

    write_json_atomic(
        &pkg_dir.join("names").join("names.json"),
        &names_to_file(&names)?,
    )?;

    let comps = serde_json::to_value(&competitions).map_err(|e| e.to_string())?;
    write_json_atomic(
        &pkg_dir.join("competitions").join("competitions.json"),
        &json!({"schema": "competition", "items": comps}),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::league::{CompetitionFormat, CompetitionScope, CompetitionType};
    use domain::player::{PlayerAttributes, Position};
    use domain::staff::{StaffAttributes, StaffRole};
    use ofm_core::generator::{
        ConfederationDef, CountryDef, FormatDef, NamePool, NamesDefinition, ParticipantSpec,
        PlayerDef, SelectorKind, SelectorSpec, StaffDef, TeamColorsDef, TeamDef, WorldMetaDef,
    };
    use std::collections::HashMap;

    fn test_meta() -> WorldMetaDef {
        WorldMetaDef {
            id: "round-trip-test".to_string(),
            name: "Round Trip".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn package_editor_round_trip_all_entities() {
        let dir = std::env::temp_dir().join(format!(
            "ofm-rt-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        create_package_project(dir.to_str().unwrap().to_string(), test_meta()).unwrap();

        let confederations = vec![ConfederationDef {
            id: "europe".to_string(),
            name: "Europe".to_string(),
        }];
        let countries = vec![CountryDef {
            id: "ENG".to_string(),
            name: "England".to_string(),
            confederation: "europe".to_string(),
        }];
        let teams = vec![TeamDef {
            id: "man-utd".to_string(),
            name: "Manchester United".to_string(),
            short_name: "MUN".to_string(),
            city: "Manchester".to_string(),
            country: "ENG".to_string(),
            colors: TeamColorsDef {
                primary: "#da291c".to_string(),
                secondary: "#ffe500".to_string(),
            },
            play_style: "Balanced".to_string(),
            stadium_name: "Old Trafford".to_string(),
            reputation_range: Some([700, 900]),
            finance_range: None,
            logo: None,
            kit_pattern: None,
        }];

        // Player WITH explicit attributes — exercises camelCase serde mapping and Position round-trip
        let players = vec![PlayerDef {
            id: "rooney".to_string(),
            name: "Wayne Rooney".to_string(),
            first_name: "Wayne".to_string(),
            last_name: "Rooney".to_string(),
            club: "man-utd".to_string(),
            nationality: "ENG".to_string(),
            position: Position::Striker,
            date_of_birth: Some("1985-10-24".to_string()),
            age: None,
            overall: None,
            attributes: Some(PlayerAttributes {
                pace: 75,
                stamina: 80,
                strength: 72,
                agility: 68,
                passing: 78,
                shooting: 88,
                tackling: 55,
                dribbling: 80,
                defending: 45,
                positioning: 85,
                vision: 78,
                decisions: 82,
                composure: 80,
                aggression: 72,
                teamwork: 75,
                leadership: 70,
                handling: 15,
                reflexes: 15,
                aerial: 65,
            }),
            photo: None,
            footedness: None,
            youth: false,
        }];

        let mut pools = HashMap::new();
        pools.insert(
            "ENG".to_string(),
            NamePool {
                first_names: vec!["James".to_string(), "John".to_string()],
                last_names: vec!["Smith".to_string(), "Jones".to_string()],
            },
        );
        let names = NamesDefinition {
            version: 1,
            description: "Test names".to_string(),
            pools,
        };

        // Competition WITH selector — exercises type PascalCase and SelectorKind camelCase serde
        let competitions = vec![CompetitionDefinition {
            id: "premier-league".to_string(),
            name: "Premier League".to_string(),
            r#type: CompetitionType::League,
            scope: CompetitionScope::Domestic,
            region_id: None,
            country_id: Some("ENG".to_string()),
            required_region_ids: vec![],
            priority: 1,
            format: FormatDef {
                kind: CompetitionFormat::LeagueTable,
                legs: None,
                group_size: None,
                qualifiers_per_group: None,
                best_third_qualifiers: None,
            },
            participants: ParticipantSpec {
                explicit: None,
                selector: Some(SelectorSpec {
                    kind: SelectorKind::AllInCountry,
                    country: Some("ENG".to_string()),
                    region: None,
                    count: None,
                    exclude_competitions: vec![],
                    source_competition: None,
                }),
            },
            berths: vec![],
            season_start_month: None,
            season_start_day: None,
            name_key: None,
            logo: None,
        }];

        let staff = vec![StaffDef {
            id: "fergie".to_string(),
            first_name: "Alex".to_string(),
            last_name: "Ferguson".to_string(),
            club: "man-utd".to_string(),
            nationality: "ENG".to_string(),
            role: StaffRole::AssistantManager,
            attributes: Some(StaffAttributes {
                coaching: 90,
                judging_ability: 85,
                judging_potential: 80,
                physiotherapy: 30,
            }),
            specialization: None,
            date_of_birth: Some("1941-12-31".to_string()),
            age: None,
        }];

        save_package_project(
            dir.to_str().unwrap().to_string(),
            test_meta(),
            confederations,
            countries,
            teams,
            players,
            staff,
            names,
            competitions,
        )
        .unwrap();

        let loaded = read_package_project(dir.to_str().unwrap().to_string()).unwrap();

        assert_eq!(loaded.confederations.len(), 1);
        assert_eq!(loaded.confederations[0].id, "europe");
        assert_eq!(loaded.confederations[0].name, "Europe");

        assert_eq!(loaded.countries.len(), 1);
        assert_eq!(loaded.countries[0].id, "ENG");
        assert_eq!(loaded.countries[0].confederation, "europe");

        assert_eq!(loaded.teams.len(), 1);
        assert_eq!(loaded.teams[0].id, "man-utd");
        assert_eq!(loaded.teams[0].colors.primary, "#da291c");

        // Exercises camelCase mapping: firstName/lastName/dateOfBirth/position
        assert_eq!(loaded.players.len(), 1);
        let p = &loaded.players[0];
        assert_eq!(p.id, "rooney");
        assert_eq!(p.first_name, "Wayne");
        assert_eq!(p.last_name, "Rooney");
        assert_eq!(p.position, Position::Striker);
        assert_eq!(p.date_of_birth.as_deref(), Some("1985-10-24"));
        let attrs = p
            .attributes
            .as_ref()
            .expect("attributes must survive round-trip");
        assert_eq!(attrs.shooting, 88);
        assert_eq!(attrs.pace, 75);

        let names_rt = loaded.names.expect("names must survive round-trip");
        let eng = names_rt
            .pools
            .get("ENG")
            .expect("ENG pool must survive round-trip");
        assert_eq!(eng.first_names, ["James", "John"]);
        assert_eq!(eng.last_names, ["Smith", "Jones"]);

        // Staff round-trip
        assert_eq!(loaded.staff.len(), 1);
        let s = &loaded.staff[0];
        assert_eq!(s.id, "fergie");
        assert_eq!(s.first_name, "Alex");
        assert_eq!(s.role, StaffRole::AssistantManager);
        assert_eq!(s.club, "man-utd");
        let s_attrs = s
            .attributes
            .as_ref()
            .expect("staff attributes must survive round-trip");
        assert_eq!(s_attrs.coaching, 90);

        // Exercises competition type PascalCase and selector kind camelCase
        assert_eq!(loaded.competitions.len(), 1);
        let c = &loaded.competitions[0];
        assert_eq!(c.id, "premier-league");
        assert_eq!(c.r#type, CompetitionType::League);
        assert_eq!(c.scope, CompetitionScope::Domestic);
        let sel = c
            .participants
            .selector
            .as_ref()
            .expect("selector must survive round-trip");
        assert_eq!(sel.kind, SelectorKind::AllInCountry);
        assert_eq!(sel.country.as_deref(), Some("ENG"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_file_as_data_url_confines_reads_to_base_dir() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("ofm-asset-{unique}"));
        std::fs::create_dir_all(base.join("assets/images")).unwrap();
        let logo = base.join("assets/images/team.png");
        std::fs::write(&logo, b"\x89PNG\r\n").unwrap();

        // A logo inside the project dir resolves to a data URL.
        let ok = read_file_as_data_url(
            logo.to_str().unwrap().to_string(),
            base.to_str().unwrap().to_string(),
        );
        assert!(ok.unwrap().starts_with("data:image/png;base64,"));

        // A real, readable file that sits *outside* the base dir is rejected by
        // the containment check (it is not a missing-file error).
        let secret = std::env::temp_dir().join(format!("ofm-secret-{unique}.png"));
        std::fs::write(&secret, b"\x89PNG\r\n").unwrap();
        let denied = read_file_as_data_url(
            secret.to_str().unwrap().to_string(),
            base.to_str().unwrap().to_string(),
        );
        assert_eq!(denied, Err("be.error.invalidPath".to_string()));

        // A relative traversal that escapes the base is likewise rejected.
        let traversal = format!(
            "{}/assets/images/../../../{}",
            base.display(),
            secret.file_name().unwrap().to_str().unwrap()
        );
        let denied2 = read_file_as_data_url(traversal, base.to_str().unwrap().to_string());
        assert_eq!(denied2, Err("be.error.invalidPath".to_string()));

        std::fs::remove_dir_all(&base).ok();
        std::fs::remove_file(&secret).ok();
    }
}

/// Extract a `.ofm` archive to a temporary editing directory.
/// Returns the path to the extracted directory.
#[tauri::command]
pub fn extract_ofm_for_editing(
    app_handle: tauri::AppHandle,
    ofm_path: String,
) -> Result<String, String> {
    let ofm = Path::new(&ofm_path);
    let stem = ofm.file_stem().and_then(|s| s.to_str()).unwrap_or("world");

    let base_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let edit_dir = base_dir.join("world-editor-temp").join(stem);

    // Always start from a clean directory so stale files from a previous edit of
    // a same-named archive can't leak into the freshly opened project.
    if edit_dir.exists() {
        std::fs::remove_dir_all(&edit_dir).map_err(|e| e.to_string())?;
    }

    // extract_ofm_to_dir removes the destination on any entry error, so a
    // rejected archive never leaves a partial tree in world-editor-temp.
    extract_ofm_to_dir(ofm, &edit_dir)?;

    edit_dir
        .to_str()
        .map(|s: &str| s.to_string())
        .ok_or_else(|| "be.error.invalidPath".to_string())
}

/// Validate then export a package directory to a .ofm archive.
#[tauri::command]
pub fn build_ofm(dir: String, output: String) -> Result<(), String> {
    let dir_path = Path::new(&dir);
    let out_path = Path::new(&output);

    // Prevent the output archive from being written inside the source directory,
    // which would cause it to zip itself into the archive.
    let out_parent = out_path.parent().unwrap_or(out_path);
    if let (Ok(abs_dir), Ok(abs_out_parent)) = (dir_path.canonicalize(), out_parent.canonicalize())
    {
        if abs_out_parent == abs_dir || abs_out_parent.starts_with(&abs_dir) {
            return Err("be.error.package.outputInsideSource".to_string());
        }
    }

    let (_pkg, errors) = load_world_package(dir_path);
    if !errors.is_empty() {
        let summary = errors
            .iter()
            .map(|e| format!("{}: {}", e.file, e.code))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "be.error.package.validationFailed?errors={}",
            summary
        ));
    }

    export_directory_to_ofm(dir_path, out_path)
}

fn sanitize_entity_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err("be.error.invalidPath".to_string());
    }
    Ok(())
}

/// Copy a local image file into `<dir>/assets/images/<entity_id>.<ext>` and
/// return the relative path from the package root (e.g. `assets/images/man-utd.png`).
/// The destination is namespaced by `entity_id` so two entities picking files
/// with the same source name cannot overwrite each other.
#[tauri::command]
pub fn copy_package_asset(
    dir: String,
    entity_id: String,
    src_path: String,
) -> Result<String, String> {
    sanitize_entity_id(&entity_id)?;

    let pkg_dir = Path::new(&dir);
    let src = Path::new(&src_path);

    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();

    let assets_dir = pkg_dir.join("assets").join("images");
    std::fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;

    let dest_name = format!("{}.{}", entity_id, ext);
    let dest = assets_dir.join(&dest_name);
    std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;

    Ok(format!("assets/images/{}", dest_name))
}

/// Read a local file and return it as a data URL (`data:<mime>;base64,<data>`).
/// Used by the package editor to display local images without the asset protocol.
/// `base_dir` is required and the resolved path must be within it, preventing
/// a malicious package from using a crafted logo path to read arbitrary files.
#[tauri::command]
pub fn read_file_as_data_url(path: String, base_dir: String) -> Result<String, String> {
    use base64::Engine as _;

    let file_path = Path::new(&path);
    let abs_base = Path::new(&base_dir)
        .canonicalize()
        .map_err(|_| "be.error.invalidPath".to_string())?;
    let abs_path = file_path
        .canonicalize()
        .map_err(|_| "be.error.invalidPath".to_string())?;

    if !abs_path.starts_with(&abs_base) {
        return Err("be.error.invalidPath".to_string());
    }

    let bytes = std::fs::read(&abs_path).map_err(|e| e.to_string())?;

    let ext = abs_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, encoded))
}
