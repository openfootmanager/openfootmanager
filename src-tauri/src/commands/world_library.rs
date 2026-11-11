use std::path::Path;

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct WorldLibraryRow {
    pub path: String,
    pub name: String,
    pub version: Option<String>,
    pub clubs: usize,
    pub players: usize,
    pub countries: usize,
    pub error: Option<String>,
}

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct WorldsLibrary {
    pub projects: Vec<WorldLibraryRow>,
    pub installed: Vec<WorldLibraryRow>,
}

fn project_row(path: &str) -> WorldLibraryRow {
    let mut row = WorldLibraryRow {
        path: path.to_string(),
        name: String::new(),
        version: None,
        clubs: 0,
        players: 0,
        countries: 0,
        error: None,
    };
    let dir = Path::new(path);
    if !dir.is_dir() {
        row.error = Some("be.error.package.notAProjectDirectory".to_string());
        return row;
    }
    if std::fs::read_dir(dir).is_err() {
        row.error = Some("be.error.package.readFailed".to_string());
        return row;
    }
    let (package, errors) = ofm_core::generator::package::load_world_package_files(dir);
    if !errors.is_empty() {
        row.error = Some(super::game::first_package_error_message(&errors));
        return row;
    }
    let Some(meta) = package.meta else {
        row.error = Some("be.error.package.noManifest".to_string());
        return row;
    };
    row.name = meta.name;
    row.clubs = package.teams.len();
    row.players = package.players.len();
    row.countries = package.countries.len();
    row
}

pub fn worlds_library_internal(paths: &[String], packages_dir: &Path) -> WorldsLibrary {
    let installed = super::world::list_installed_packages_in(packages_dir, false)
        .into_iter()
        .map(|package| WorldLibraryRow {
            path: package.installed_path,
            name: package.name,
            version: Some(package.version),
            clubs: package.team_count,
            players: package.player_count,
            countries: package.country_count,
            error: None,
        })
        .collect();
    WorldsLibrary {
        projects: paths.iter().map(|path| project_row(path)).collect(),
        installed,
    }
}

#[tauri::command]
pub fn get_worlds_library(
    app_handle: tauri::AppHandle,
    paths: Vec<String>,
) -> Result<WorldsLibrary, String> {
    let packages =
        super::world::packages_dir(&app_handle).map_err(|_| "be.error.invalidPath".to_string())?;
    Ok(worlds_library_internal(&paths, &packages))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given authored entity files, when the library reads them, then it reports their counts.
    #[test]
    fn library_counts_entities_from_project_files() {
        let dir = std::env::temp_dir().join(format!("ofm-library-counts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("package.json"), r#"{"schema":"world","id":"test","name":"Test world","version":"1.0.0","license":"CC0-1.0"}"#).unwrap();
        std::fs::write(
            dir.join("countries.json"),
            r#"{"schema":"country","items":[{"id":"QQ","name":"Qland"}]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("teams.json"), r##"{"schema":"team","items":[{"id":"a","name":"A","city":"C","country":"QQ","colors":{"primary":"#000000","secondary":"#ffffff"}}]}"##).unwrap();
        std::fs::write(dir.join("players.json"), r#"{"schema":"player","items":[{"id":"p1","name":"P One","club":"a","nationality":"QQ","position":"Striker","overall":70},{"id":"p2","name":"P Two","club":"a","nationality":"QQ","position":"Striker","overall":70}]}"#).unwrap();
        let answer = worlds_library_internal(
            &[dir.to_string_lossy().into_owned()],
            Path::new("/nonexistent"),
        );
        assert_eq!(
            (
                answer.projects[0].name.as_str(),
                answer.projects[0].clubs,
                answer.projects[0].players,
                answer.projects[0].countries
            ),
            ("Test world", 1, 2, 1)
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Given one missing folder and one project, when the library reads both, then only the missing row fails.
    #[test]
    fn library_keeps_readable_projects_when_another_is_missing() {
        let dir = std::env::temp_dir().join(format!("ofm-library-batch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("package.json"), r#"{"schema":"world","id":"test","name":"Test world","version":"1.0.0","license":"CC0-1.0"}"#).unwrap();
        let answer = worlds_library_internal(
            &[
                dir.join("missing").to_string_lossy().into_owned(),
                dir.to_string_lossy().into_owned(),
            ],
            Path::new("/nonexistent"),
        );
        assert!(answer.projects[0].error.is_some());
        assert_eq!(answer.projects[1].name, "Test world");
        assert!(answer.projects[1].error.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Given a malformed entity file, when the library reads it, then it reports an error instead of partial counts.
    #[test]
    fn library_reports_a_malformed_entity() {
        let dir = std::env::temp_dir().join(format!("ofm-library-invalid-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("package.json"), r#"{"schema":"world","id":"test","name":"Test world","version":"1.0.0","license":"CC0-1.0"}"#).unwrap();
        std::fs::write(
            dir.join("teams.json"),
            r#"{"schema":"team","items":[{"id":"broken","name":5}]}"#,
        )
        .unwrap();
        let answer = worlds_library_internal(
            &[dir.to_string_lossy().into_owned()],
            Path::new("/nonexistent"),
        );
        assert!(answer.projects[0].error.is_some());
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Given an unreadable entity directory, when the library reads it, then it reports a path error.
    #[cfg(unix)]
    #[test]
    fn library_reports_an_unreadable_subdirectory() {
        use std::os::unix::fs::PermissionsExt;
        let dir =
            std::env::temp_dir().join(format!("ofm-library-unreadable-{}", std::process::id()));
        let nested = dir.join("players");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(dir.join("package.json"), r#"{"schema":"world","id":"test","name":"Test world","version":"1.0.0","license":"CC0-1.0"}"#).unwrap();
        std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0)).unwrap();
        let answer = worlds_library_internal(
            &[dir.to_string_lossy().into_owned()],
            Path::new("/nonexistent"),
        );
        std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        assert!(answer.projects[0].error.is_some());
    }

    /// Given an installed archive, when the library reads it, then its manifest and counts are reported.
    #[test]
    fn library_counts_entities_from_installed_archives() {
        let root = std::env::temp_dir().join(format!("ofm-library-archive-{}", std::process::id()));
        let project = root.join("project");
        let installed = root.join("packages");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::write(project.join("package.json"), r#"{"schema":"world","id":"test","name":"Archive world","version":"2.3.4","license":"CC0-1.0"}"#).unwrap();
        std::fs::write(
            project.join("countries.json"),
            r#"{"schema":"country","items":[{"id":"QQ","name":"Qland"}]}"#,
        )
        .unwrap();
        std::fs::write(project.join("teams.json"), r##"{"schema":"team","items":[{"id":"a","name":"A","city":"C","country":"QQ","colors":{"primary":"#000000","secondary":"#ffffff"}}]}"##).unwrap();
        std::fs::write(project.join("players.json"), r#"{"schema":"player","items":[{"id":"p1","name":"P One","club":"a","nationality":"QQ","position":"Striker","overall":70},{"id":"p2","name":"P Two","club":"a","nationality":"QQ","position":"Striker","overall":70}]}"#).unwrap();
        let archive = installed.join("test.ofm");
        ofm_core::generator::export_directory_to_ofm(&project, &archive).unwrap();
        let answer = worlds_library_internal(&[], &installed);
        assert_eq!(answer.installed.len(), 1);
        let row = &answer.installed[0];
        assert_eq!(
            (&*row.name, row.version.as_deref(), row.countries),
            ("Archive world", Some("2.3.4"), 1)
        );
        assert_eq!((row.clubs, row.players), (1, 2));
        assert_eq!(row.path, archive.to_string_lossy());
        std::fs::remove_dir_all(root).unwrap();
    }
}
