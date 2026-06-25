use ofm_core::generator::{
    export_directory_to_ofm, extract_ofm_to_dir, load_world_package, CompetitionDefinition,
    ConfederationDef, CountryDef, NamesDefinition, PlayerDef, TeamDef, WorldMetaDef,
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

/// Create a new package project directory with an empty scaffold.
#[tauri::command]
pub fn create_package_project(dir: String, meta: WorldMetaDef) -> Result<(), String> {
    let pkg_dir = Path::new(&dir);

    let subdirs = [
        "teams",
        "players",
        "confederations",
        "countries",
        "competitions",
        "names",
    ];
    for sub in &subdirs {
        std::fs::create_dir_all(pkg_dir.join(sub)).map_err(|e| e.to_string())?;
    }

    let manifest = meta_to_manifest(&meta)?;
    write_json_atomic(&pkg_dir.join("package.json"), &manifest)?;

    let stubs: &[(&str, &str, serde_json::Value)] = &[
        ("teams", "teams.json", json!({"schema": "team", "items": []})),
        ("players", "players.json", json!({"schema": "player", "items": []})),
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

    write_json_atomic(&pkg_dir.join("names").join("names.json"), &names_to_file(&names)?)?;

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
    use ofm_core::generator::{
        ConfederationDef, CountryDef, FormatDef, NamePool, NamesDefinition, ParticipantSpec,
        PlayerDef, SelectorKind, SelectorSpec, TeamColorsDef, TeamDef, WorldMetaDef,
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
        }];

        save_package_project(
            dir.to_str().unwrap().to_string(),
            test_meta(),
            confederations,
            countries,
            teams,
            players,
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
        let attrs = p.attributes.as_ref().expect("attributes must survive round-trip");
        assert_eq!(attrs.shooting, 88);
        assert_eq!(attrs.pace, 75);

        let names_rt = loaded.names.expect("names must survive round-trip");
        let eng = names_rt.pools.get("ENG").expect("ENG pool must survive round-trip");
        assert_eq!(eng.first_names, ["James", "John"]);
        assert_eq!(eng.last_names, ["Smith", "Jones"]);

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
}

/// Extract a `.ofm` archive to a temporary editing directory.
/// Returns the path to the extracted directory.
#[tauri::command]
pub fn extract_ofm_for_editing(
    app_handle: tauri::AppHandle,
    ofm_path: String,
) -> Result<String, String> {
    let ofm = Path::new(&ofm_path);
    let stem = ofm
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("world");

    let base_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let edit_dir = base_dir.join("world-editor-temp").join(stem);

    extract_ofm_to_dir(ofm, &edit_dir)?;
    edit_dir
        .to_str()
        .map(|s: &str| s.to_string())
        .ok_or_else(|| "Invalid path".to_string())
}

/// Validate then export a package directory to a .ofm archive.
#[tauri::command]
pub fn build_ofm(dir: String, output: String) -> Result<(), String> {
    let dir_path = Path::new(&dir);
    let out_path = Path::new(&output);

    // Prevent the output archive from being written inside the source directory,
    // which would cause it to zip itself into the archive.
    let out_parent = out_path.parent().unwrap_or(out_path);
    if let (Ok(abs_dir), Ok(abs_out_parent)) =
        (dir_path.canonicalize(), out_parent.canonicalize())
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
        return Err(format!("be.error.package.validationFailed?errors={}", summary));
    }

    export_directory_to_ofm(dir_path, out_path)
}
