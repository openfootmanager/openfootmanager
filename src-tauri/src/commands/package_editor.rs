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



// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn scaffold_project_dir(pkg_dir: &Path, meta: &WorldMetaDef) -> Result<(), String> {
    // The skeleton lives in `ofm_core` so `ofm-cli new` and this produce the
    // same package. When they were separate the CLI's manifest silently missed
    // fields the editor wrote (`fallbackLeague`, `logo`), and its `names.json`
    // used an `items` array that loads without error and is then dropped — the
    // file looked fine and did nothing.
    ofm_core::generator::scaffold_package(pkg_dir, meta)
}

fn dir_is_nonempty(path: &Path) -> bool {
    std::fs::read_dir(path).map(|mut d| d.next().is_some()).unwrap_or(false)
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
    let base_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
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

    // Anything that is not a directory cannot be walked for entity files, and
    // walking it fails *silently* — the loader simply finds nothing. Naming the
    // problem here is the difference between "that is not a package folder" and
    // an editor that opens with every field blank.
    if !path.is_dir() {
        return Err("be.error.package.notAProjectDirectory".to_string());
    }

    let (pkg, errors) = load_world_package(path);

    // A package that resolved to nothing must not open as a blank project.
    // Every field would read empty, Save would write that emptiness back over
    // the source, and Build would produce an empty archive — so an unreadable
    // package silently becomes a destroyed one. Refuse, and say why: the first
    // load error is the actual reason (an unsupported format version, an
    // unknown schema), which is far more useful than "nothing here".
    if !errors.is_empty() && ofm_core::generator::is_unreadable(&pkg) {
        // Carries the error's params too, in the `key?param=value` form
        // `resolveBackendError` understands, so the message can name the
        // offending version or schema rather than leaving blanks.
        return Err(crate::commands::game::first_package_error_message(&errors));
    }

    // Nothing resolved *and* nothing went wrong: the directory is simply not a
    // package. The guard above cannot catch this — it needs an error to report,
    // and finding no files to read is not an error. A newly scaffolded project
    // is legitimately empty but always has its manifest, which is what tells the
    // two apart.
    if pkg.meta.is_none() && ofm_core::generator::is_unreadable(&pkg) {
        return Err("be.error.package.notAProjectDirectory".to_string());
    }

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

/// Persist in-memory edits, overwriting every package entity file.
///
/// Each file is written atomically — `write_json_atomic` writes a `.tmp` sibling
/// and renames over the target — so no single file can be left half-written.
/// The *set* of files is **not** transactional: this walks the entity types in
/// sequence with `?`, so a failure partway through leaves the earlier files
/// updated and the later ones stale. Recovering from that means saving again
/// from the still-intact in-memory project.
// One parameter per entity type, matching the `.ofm` package layout — the editor
// sends every collection in one call, so the argument count tracks the entity
// count by design.
#[allow(clippy::too_many_arguments)]
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

    write_json_atomic(&pkg_dir.join("package.json"), &ofm_core::generator::manifest_json(&meta)?)?;

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

    write_json_atomic(&pkg_dir.join("names").join("names.json"), &ofm_core::generator::names_json(&names)?)?;

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

    fn temp_project(tag: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        // No uuid dependency in this crate; the caller-supplied tag keeps
        // concurrent tests from sharing a directory.
        let dir = std::env::temp_dir().join(format!("ofm-editor-open-{tag}"));
        std::fs::remove_dir_all(&dir).ok();
        for (name, body) in files {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        }
        dir
    }

    /// A `.ofm` archive built from `files`, plus the editing paths for it.
    fn archive_for(
        tag: &str,
        files: &[(&str, &str)],
    ) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let source = temp_project(&format!("{tag}-src"), files);
        let root = std::env::temp_dir().join(format!("ofm-editor-open-{tag}-edit"));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        let archive = root.join("pkg.ofm");
        export_directory_to_ofm(&source, &archive).unwrap();
        std::fs::remove_dir_all(&source).ok();
        (archive, root.join("pkg"), root.join("pkg.source"))
    }

    #[test]
    fn reopening_an_installed_package_keeps_the_edits_made_to_it() {
        // The editing directory is a real project once it is open — it goes into
        // recent projects and is saved into like any other. Re-extracting over
        // it threw away every edit since, and only for the author who came back
        // through the installed list instead of through recents: the same
        // package, opened two ways, with different contents.
        let (archive, edit_dir, marker) = archive_for(
            "keeps-edits",
            &[(
                "package.json",
                r#"{"schema":"world","id":"keep","name":"Keep"}"#,
            )],
        );

        extract_ofm_for_editing_into(&archive, &edit_dir, &marker).unwrap();
        let edited = edit_dir.join("countries/added-by-the-author.json");
        std::fs::create_dir_all(edited.parent().unwrap()).unwrap();
        std::fs::write(&edited, r#"{"schema":"country","id":"ES","name":"Spain"}"#).unwrap();

        extract_ofm_for_editing_into(&archive, &edit_dir, &marker).unwrap();

        assert!(
            edited.exists(),
            "reopening the same archive must not discard what was authored in it"
        );
        std::fs::remove_dir_all(edit_dir.parent().unwrap()).ok();
    }

    #[test]
    fn a_different_archive_under_the_same_name_starts_clean() {
        // The reuse is keyed on the archive, not on the name. A package that was
        // updated or reinstalled no longer describes what was extracted, and
        // merging the two would present a mix of two packages as one.
        let (first, edit_dir, marker) = archive_for(
            "replaced",
            &[(
                "package.json",
                r#"{"schema":"world","id":"v1","name":"One"}"#,
            )],
        );
        extract_ofm_for_editing_into(&first, &edit_dir, &marker).unwrap();
        let stale = edit_dir.join("countries/from-the-old-package.json");
        std::fs::create_dir_all(stale.parent().unwrap()).unwrap();
        std::fs::write(&stale, r#"{"schema":"country","id":"ES","name":"Spain"}"#).unwrap();

        // Same path, different contents — as a reinstall would leave it.
        let replacement = temp_project(
            "replaced-v2",
            &[(
                "package.json",
                r#"{"schema":"world","id":"v2","name":"Two"}"#,
            )],
        );
        std::fs::remove_file(&first).unwrap();
        export_directory_to_ofm(&replacement, &first).unwrap();

        extract_ofm_for_editing_into(&first, &edit_dir, &marker).unwrap();

        assert!(
            !stale.exists(),
            "content from the previous archive must not survive into the new one"
        );
        let manifest = std::fs::read_to_string(edit_dir.join("package.json")).unwrap();
        assert!(manifest.contains("v2"), "the new archive's manifest: {manifest}");

        // ...but it is not destroyed either. Whatever the author wrote against
        // the previous version is still on disk beside the new extract, because
        // nothing has asked them whether they were finished with it.
        let kept = superseded_path(&edit_dir).join("countries/from-the-old-package.json");
        assert!(
            kept.exists(),
            "work done against the previous archive must be moved aside, not deleted"
        );

        std::fs::remove_dir_all(edit_dir.parent().unwrap()).ok();
        std::fs::remove_dir_all(&replacement).ok();
    }

    #[test]
    fn something_blocking_the_supersede_slot_does_not_cost_the_author_their_edits() {
        // The slot is expected to hold a previous supersede — a directory — but
        // nothing guarantees it. A regular file there made `remove_dir_all` fail
        // without removing it and the rename fail after that, and the fallback
        // then deleted the editing directory: the one outcome this whole path
        // exists to prevent, reached by the error handling meant to protect it.
        let (first, edit_dir, marker) = archive_for(
            "blocked-supersede",
            &[(
                "package.json",
                r#"{"schema":"world","id":"v1","name":"One"}"#,
            )],
        );
        extract_ofm_for_editing_into(&first, &edit_dir, &marker).unwrap();
        let authored = edit_dir.join("countries/mine.json");
        std::fs::create_dir_all(authored.parent().unwrap()).unwrap();
        std::fs::write(&authored, r#"{"schema":"country","id":"ES","name":"Spain"}"#).unwrap();

        // Not a directory: something else got to the name first.
        std::fs::write(superseded_path(&edit_dir), b"not a directory").unwrap();

        let replacement = temp_project(
            "blocked-supersede-v2",
            &[(
                "package.json",
                r#"{"schema":"world","id":"v2","name":"Two"}"#,
            )],
        );
        std::fs::remove_file(&first).unwrap();
        export_directory_to_ofm(&replacement, &first).unwrap();

        extract_ofm_for_editing_into(&first, &edit_dir, &marker)
            .expect("a blocked slot is clearable, so the open still succeeds");

        assert!(
            superseded_path(&edit_dir).join("countries/mine.json").exists(),
            "the author's work must be moved aside, not destroyed by the blocked slot"
        );
        let manifest = std::fs::read_to_string(edit_dir.join("package.json")).unwrap();
        assert!(manifest.contains("v2"), "the new archive's manifest: {manifest}");

        std::fs::remove_dir_all(edit_dir.parent().unwrap()).ok();
        std::fs::remove_dir_all(&replacement).ok();
    }

    #[test]
    fn a_freshly_scaffolded_empty_project_still_opens() {
        // `ofm-cli new` and the editor's own "new project" both produce a
        // manifest plus empty `items` lists. That is legitimately empty, not
        // unreadable — refusing it would break creating a package from scratch.
        let dir = temp_project(
            "fresh-scaffold",
            &[
                (
                    "package.json",
                    r#"{"schema":"world","id":"fresh","name":"Fresh"}"#,
                ),
                ("teams/teams.json", r#"{"schema":"team","items":[]}"#),
                ("players/players.json", r#"{"schema":"player","items":[]}"#),
            ],
        );

        let Ok(data) = read_package_project(dir.to_string_lossy().to_string()) else {
            panic!("a newly scaffolded project must open");
        };

        assert_eq!(data.meta.id, "fresh");
        assert!(data.teams.is_empty());
        assert!(data.issues.is_empty(), "an empty scaffold has no issues");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_package_with_entities_in_subdirectories_opens_with_all_of_them() {
        // The other half of the reported bug: refusing what is not a package is
        // only useful if the real one still opens. Shaped like the package that
        // was reported — a manifest at the root, entities spread across
        // subdirectories, some files holding several entities.
        let dir = temp_project(
            "real-shape",
            &[
                (
                    "package.json",
                    r#"{"schema":"world","id":"brazil-1962","name":"Brazil 1962"}"#,
                ),
                (
                    "teams/teams.json",
                    r##"{"schema":"team","items":[
                        {"id":"santos","name":"Santos","city":"Santos","country":"BR",
                         "colors":{"primary":"#ffffff","secondary":"#000000"}},
                        {"id":"palmeiras","name":"Palmeiras","city":"Sao Paulo","country":"BR",
                         "colors":{"primary":"#00aa00","secondary":"#ffffff"}}
                    ]}"##,
                ),
                (
                    "players/santos.json",
                    r#"{"schema":"player","items":[
                        {"id":"pele","name":"Pele","firstName":"Edson","lastName":"Nascimento",
                         "club":"santos","nationality":"BR","position":"Striker","overall":99}
                    ]}"#,
                ),
            ],
        );

        let data = read_package_project(dir.to_string_lossy().to_string())
            .expect("a package with real content must open");

        assert_eq!(data.meta.id, "brazil-1962");
        assert_eq!(data.teams.len(), 2);
        assert_eq!(data.players.len(), 1);
        assert!(data.issues.is_empty(), "unexpected issues: {}", data.issues.len());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pointing_the_editor_at_an_ofm_archive_is_refused() {
        // Reported from real use: "Open in Editor" on an installed package
        // showed an empty editor, for a package that loads fine in the game.
        // An installed package is a `.ofm` *file*, and this command takes a
        // directory — walking a file finds no entity files, which produced an
        // empty package and, because nothing failed, *no errors*. So the
        // "must not open as a blank project" guard never fired.
        let dir = temp_project("ofm-archive", &[]);
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("brazil-1962.ofm");
        std::fs::write(&archive, b"PK\x03\x04 not really a zip").unwrap();

        let result = read_package_project(archive.to_string_lossy().to_string());

        assert!(
            result.is_err(),
            "an archive is not a package directory and must not open as a blank project"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn opening_a_directory_that_is_not_there_is_refused() {
        // A recent-projects entry outlives the directory it points at. Reading
        // one that has been moved or deleted resolves to nothing at all, which
        // is the same blank-project failure by another route.
        let dir = std::env::temp_dir().join("ofm-editor-open-vanished");
        std::fs::remove_dir_all(&dir).ok();

        let result = read_package_project(dir.to_string_lossy().to_string());

        assert!(result.is_err(), "a missing directory must not open as a blank project");
    }

    #[test]
    fn a_folder_with_no_package_in_it_is_refused() {
        // Picking the wrong folder is easy to do and, before this, indistinguishable
        // from opening a real package that happened to be empty.
        let dir = temp_project("not-a-package", &[("notes.txt", "just some files")]);

        let result = read_package_project(dir.to_string_lossy().to_string());

        assert!(result.is_err(), "a folder with no manifest and no entities is not a package");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn opening_a_package_from_a_newer_format_reports_the_version() {
        // The user-visible symptom was an editor with nothing in it. Opening
        // must fail with the actual reason instead, because a blank project
        // that Save writes back is how an unreadable package becomes a
        // destroyed one.
        let dir = temp_project(
            "future-format",
            &[(
                "package.json",
                r#"{"schema":"world","id":"future","name":"Future","formatVersion":99}"#,
            )],
        );

        let result = read_package_project(dir.to_string_lossy().to_string());

        assert_eq!(
            result.err().as_deref(),
            Some("be.error.package.unsupportedFormatVersion?version=99&supported=1"),
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn opening_a_package_of_entirely_unknown_schemas_is_refused() {
        let dir = temp_project(
            "unknown-schema",
            &[(
                "stuff.json",
                r#"{"schema":"quantum-club","id":"x","name":"X"}"#,
            )],
        );

        let result = read_package_project(dir.to_string_lossy().to_string());

        assert_eq!(
            result.err().as_deref(),
            Some("be.error.package.unknownSchema?schema=quantum-club"),
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_partly_broken_package_still_opens_for_repair() {
        // Repairing a broken package is exactly what the editor is for, so a
        // package that yields *some* content must keep opening, issues and all.
        let dir = temp_project(
            "partly-broken",
            &[
                (
                    "package.json",
                    r#"{"schema":"world","id":"partly","name":"Partly"}"#,
                ),
                (
                    "teams.json",
                    r##"{"schema":"team","items":[{"id":"zed-fc","name":"Zed FC","city":"Zedtown",
                    "country":"ZZ","colors":{"primary":"#000","secondary":"#fff"}}]}"##,
                ),
                ("broken.json", r#"{"schema":"nonsense","id":"n"}"#),
            ],
        );

        let Ok(data) = read_package_project(dir.to_string_lossy().to_string()) else {
            panic!("a package with usable content must still open");
        };

        assert_eq!(data.teams.len(), 1, "the readable club should survive");
        assert!(
            !data.issues.is_empty(),
            "the unreadable file should still be reported as an issue",
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    fn test_meta() -> WorldMetaDef {
        WorldMetaDef {
            id: "round-trip-test".to_string(),
            name: "Round Trip".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn creating_a_project_here_matches_what_the_cli_scaffolds() {
        // The editor's New Package and `ofm-cli new` must produce the same
        // package. They agree today because both delegate to
        // `scaffold_package` — this asserts that `create_package_project` still
        // *does* delegate, rather than growing a second skeleton of its own,
        // which is exactly how the manifest drifted before (`fallbackLeague` and
        // `logo` reached the editor's output and never reached the CLI's).
        //
        // Comparing the command against the shared function is the only crossing
        // available from this crate: `cmd_new` lives in the CLI binary. Calling
        // `scaffold_package` twice and diffing would compare a function with
        // itself and could never fail.
        let meta = ofm_core::generator::new_package_meta("Parity", "Author", "1.0.0", "database");

        let via_command = std::env::temp_dir().join("ofm-editor-parity-command");
        std::fs::remove_dir_all(&via_command).ok();
        create_package_project(via_command.to_string_lossy().to_string(), meta.clone())
            .expect("the editor creates a project");

        let via_core = std::env::temp_dir().join("ofm-editor-parity-core");
        std::fs::remove_dir_all(&via_core).ok();
        ofm_core::generator::scaffold_package(&via_core, &meta).expect("the shared scaffolder runs");

        let listing = |root: &std::path::Path| {
            let mut found: Vec<(String, String)> = Vec::new();
            let mut stack = vec![root.to_path_buf()];
            while let Some(dir) = stack.pop() {
                for entry in std::fs::read_dir(&dir).expect("readable").flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else {
                        let rel = path
                            .strip_prefix(root)
                            .expect("under root")
                            .to_string_lossy()
                            .replace('\\', "/");
                        found.push((rel, std::fs::read_to_string(&path).expect("readable")));
                    }
                }
            }
            found.sort();
            found
        };

        assert_eq!(listing(&via_command), listing(&via_core));
        std::fs::remove_dir_all(&via_command).ok();
        std::fs::remove_dir_all(&via_core).ok();
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
            founded_year: None,
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
        let attrs = p.attributes.as_ref().expect("attributes must survive round-trip");
        assert_eq!(attrs.shooting, 88);
        assert_eq!(attrs.pace, 75);

        let names_rt = loaded.names.expect("names must survive round-trip");
        let eng = names_rt.pools.get("ENG").expect("ENG pool must survive round-trip");
        assert_eq!(eng.first_names, ["James", "John"]);
        assert_eq!(eng.last_names, ["Smith", "Jones"]);

        // Staff round-trip
        assert_eq!(loaded.staff.len(), 1);
        let s = &loaded.staff[0];
        assert_eq!(s.id, "fergie");
        assert_eq!(s.first_name, "Alex");
        assert_eq!(s.role, StaffRole::AssistantManager);
        assert_eq!(s.club, "man-utd");
        let s_attrs = s.attributes.as_ref().expect("staff attributes must survive round-trip");
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
        let traversal = format!("{}/assets/images/../../../{}", base.display(),
            secret.file_name().unwrap().to_str().unwrap());
        let denied2 = read_file_as_data_url(traversal, base.to_str().unwrap().to_string());
        assert_eq!(denied2, Err("be.error.invalidPath".to_string()));

        std::fs::remove_dir_all(&base).ok();
        std::fs::remove_file(&secret).ok();
    }
}

/// Extract a `.ofm` archive to an editing directory, or reuse the one already
/// extracted from that same archive.
///
/// The reuse is what keeps an author's work. The editing directory is a real
/// project once it is open — it is added to recent projects and saved into like
/// any other — so re-extracting over it silently discards every edit made since
/// it was opened. That only bites the author who returns to the package through
/// the installed list rather than through recents, which is the harder loss to
/// understand: the same package, opened two ways, with different contents.
///
/// The archive's hash decides. Identical archive, keep what is there; a
/// different one under the same name (the package was updated or reinstalled)
/// no longer describes the extract, so start clean rather than present a mix of
/// two packages as one.
///
/// The marker lives beside the directory rather than inside it, so it cannot be
/// picked up as package content or end up inside a rebuilt archive.
/// Where an editing directory is kept when a different archive replaces it.
/// A sibling, so it is beside the extract an author would go looking from.
fn superseded_path(edit_dir: &Path) -> std::path::PathBuf {
    let name = edit_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("world");
    edit_dir.with_file_name(format!("{name}.superseded"))
}

fn extract_ofm_for_editing_into(
    ofm: &Path,
    edit_dir: &Path,
    marker: &Path,
) -> Result<(), String> {
    let fingerprint = ofm_core::generator::hash_package_file(ofm).unwrap_or_default();
    let extracted_from = std::fs::read_to_string(marker).unwrap_or_default();

    if edit_dir.is_dir() && !fingerprint.is_empty() && extracted_from == fingerprint {
        return Ok(());
    }

    // A different archive under this name — the package was updated or
    // reinstalled — no longer describes what is here, so the new one is
    // extracted fresh rather than merged into a mix of two packages.
    //
    // What was there is moved aside rather than deleted. It may hold work the
    // author did against the previous version, and nothing has asked them
    // whether they still want it; deleting it is a decision this function is
    // not in a position to make. One copy is kept — the previous supersede is
    // replaced — so this cannot grow without bound.
    if edit_dir.exists() {
        let superseded = superseded_path(edit_dir);
        // Clear the slot whichever shape it is in. A previous supersede leaves a
        // directory, but nothing guarantees that is what is there — and a
        // regular file would make `remove_dir_all` fail without removing it,
        // and the rename fail after that.
        if superseded.is_dir() {
            std::fs::remove_dir_all(&superseded).map_err(|e| e.to_string())?;
        } else if superseded.exists() {
            std::fs::remove_file(&superseded).map_err(|e| e.to_string())?;
        }
        // If the move cannot be made, stop here and leave the directory alone.
        // There is no fallback that deletes it: that would discard exactly the
        // work the move exists to keep, and a failed open the author can retry
        // is a far better outcome than a successful one that costs them an
        // afternoon's editing.
        std::fs::rename(edit_dir, &superseded).map_err(|e| e.to_string())?;
    }

    // extract_ofm_to_dir removes the destination on any entry error, so a
    // rejected archive never leaves a partial tree in world-editor-temp.
    extract_ofm_to_dir(ofm, edit_dir)?;

    // Best-effort: an unwritable marker costs a re-extract next time, which is
    // the old behaviour, and must not fail an otherwise good open.
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(marker, &fingerprint).ok();
    Ok(())
}

/// Extract a `.ofm` archive to its editing directory, keeping any edits already
/// made to an extract of that same archive. Returns the path to the directory.
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
    let temp_root = base_dir.join("world-editor-temp");
    let edit_dir = temp_root.join(stem);
    let marker = temp_root.join(format!("{stem}.source"));

    extract_ofm_for_editing_into(ofm, &edit_dir, &marker)?;

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
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;

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
