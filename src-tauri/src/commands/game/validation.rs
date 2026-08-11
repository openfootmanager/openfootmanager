//! Telling a package author what is wrong with their files.
//!
//! Everything here answers a question the editor asks before a world is ever
//! built: does this competition-definition file parse, do its competitions
//! reference teams the world actually has, and what is inside this `.ofm`?
//! Each type is shaped for the UI that displays it, hence the `camelCase`
//! serialisation.

use super::{definition_sources, load_world_data};

/// One validation problem in a competition-definition file, shaped for the UI.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionDefinitionIssue {
    code: String,
    competition_id: String,
    params: std::collections::HashMap<String, String>,
}

pub(super) fn parse_competition_definitions(
    source: &str,
) -> Result<ofm_core::generator::CompetitionDefinitionFile, String> {
    // Accept either JSON or YAML so definitions can be hand-authored in either.
    ofm_core::generator::parse_definition_str(source)
        .map_err(|_| "be.error.competitionDef.parseFailed".to_string())
}

pub(super) fn validate_against_world(
    file: &ofm_core::generator::CompetitionDefinitionFile,
    world: &ofm_core::generator::WorldData,
) -> Vec<CompetitionDefinitionIssue> {
    let ctx = ofm_core::generator::WorldValidationContext::from_world(world);
    ofm_core::generator::validate_definitions(file, &ctx)
        .into_iter()
        .map(|error| CompetitionDefinitionIssue {
            code: error.code,
            competition_id: error.competition_id,
            params: error.params.into_iter().collect(),
        })
        .collect()
}

/// Validate a standalone competition-definition file against a world. Returns
/// the full list of problems (empty = valid) so the new-game UI can show them
/// before the player commits.
#[tauri::command]
pub fn validate_competition_definitions(
    app_handle: tauri::AppHandle,
    world_source: Option<String>,
    definitions_json: String,
) -> Result<Vec<CompetitionDefinitionIssue>, String> {
    let file = parse_competition_definitions(&definitions_json)?;
    let world = load_world_data(world_source.as_deref(), &definition_sources(&app_handle))?;
    Ok(validate_against_world(&file, &world))
}

/// One problem found while loading a world package, shaped for the UI.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageIssue {
    pub code: String,
    pub file: String,
    pub params: std::collections::HashMap<String, String>,
}

/// Validate a modular world-package directory. Returns the full list of problems
/// (empty = valid) so the new-game UI can show them before the player commits.
#[tauri::command]
pub fn validate_world_package(path: String) -> Result<Vec<PackageIssue>, String> {
    let (_package, errors) = ofm_core::generator::load_world_package(std::path::Path::new(&path));
    Ok(errors
        .into_iter()
        .map(|error| PackageIssue {
            code: error.code,
            file: error.file,
            params: error.params.into_iter().collect(),
        })
        .collect())
}

/// A world package summarised for the import card: a display name (falling back
/// to the folder name when the package declares none), club/player counts, and
/// any validation problems (empty = ready to start).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldPackageInspection {
    name: String,
    team_count: usize,
    player_count: usize,
    issues: Vec<PackageIssue>,
}

pub(super) fn package_folder_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "World Package".to_string())
}

/// Validate and summarise a world package for the new-game picker. On any
/// validation problem the issues are returned (with a folder-name fallback) and
/// the world isn't built; otherwise the built world's name and counts come back.
#[tauri::command]
pub fn inspect_world_package(
    app_handle: tauri::AppHandle,
    path: String,
) -> Result<WorldPackageInspection, String> {
    let (package, errors) = ofm_core::generator::load_world_package(std::path::Path::new(&path));
    let issues: Vec<PackageIssue> = errors
        .into_iter()
        .map(|error| PackageIssue {
            code: error.code,
            file: error.file,
            params: error.params.into_iter().collect(),
        })
        .collect();

    let fallback_name = package_folder_name(&path);
    if !issues.is_empty() {
        return Ok(WorldPackageInspection {
            name: fallback_name,
            team_count: 0,
            player_count: 0,
            issues,
        });
    }

    let world = ofm_core::generator::build_world_from_package(
        &package,
        None,
        &definition_sources(&app_handle),
    )?;
    let name = if world.name.trim().is_empty() {
        fallback_name
    } else {
        world.name.clone()
    };
    Ok(WorldPackageInspection {
        name,
        team_count: world.teams.len(),
        player_count: world.players.len(),
        issues: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::game::testkit::temp_pkg_dir;

    #[test]
    fn validate_world_package_reports_problems_and_passes_clean_packages() {
        let team = "schema: team\nid: {id}\nname: {name}\ncity: X\ncountry: ES\ncolors: { primary: \"#000\", secondary: \"#fff\" }\n";

        let valid = temp_pkg_dir("valid");
        std::fs::write(
            valid.join("team.yaml"),
            team.replace("{id}", "zed-fc").replace("{name}", "Zed FC"),
        )
        .unwrap();
        let clean = super::validate_world_package(valid.to_string_lossy().to_string()).unwrap();
        assert!(clean.is_empty(), "a clean package should have no issues");

        let broken = temp_pkg_dir("broken");
        std::fs::write(
            broken.join("a.yaml"),
            team.replace("{id}", "dup").replace("{name}", "A"),
        )
        .unwrap();
        std::fs::write(
            broken.join("b.yaml"),
            team.replace("{id}", "dup").replace("{name}", "B"),
        )
        .unwrap();
        let issues = super::validate_world_package(broken.to_string_lossy().to_string()).unwrap();
        assert!(!issues.is_empty(), "a duplicate id should be reported");

        std::fs::remove_dir_all(&valid).ok();
        std::fs::remove_dir_all(&broken).ok();
    }

    #[test]
    fn parse_competition_definitions_accepts_yaml_and_json() {
        let yaml = "\
formatVersion: 1
competitions:
  - id: tr-1
    name: Super Lig
    type: League
    scope: Domestic
    format:
      kind: LeagueTable
    participants:
      selector:
        kind: allInCountry
        country: TR
";
        let parsed = parse_competition_definitions(yaml).expect("YAML should parse");
        assert_eq!(parsed.competitions.len(), 1);
        assert_eq!(parsed.competitions[0].id, "tr-1");

        let json = r#"{"formatVersion":1,"competitions":[{"id":"tr-1","name":"Super Lig","type":"League","scope":"Domestic","format":{"kind":"LeagueTable"},"participants":{"selector":{"kind":"allInCountry","country":"TR"}}}]}"#;
        let parsed_json = parse_competition_definitions(json).expect("JSON should parse");
        assert_eq!(parsed_json.competitions[0].id, "tr-1");

        assert!(parse_competition_definitions("not: [valid").is_err());
    }

    #[test]
    fn package_folder_name_falls_back_to_the_directory_name() {
        assert_eq!(package_folder_name("/mods/My World"), "My World");
        assert_eq!(package_folder_name("turkish-league"), "turkish-league");
        // No usable component → a sensible default rather than an empty name.
        assert_eq!(package_folder_name(""), "World Package");
    }
}
