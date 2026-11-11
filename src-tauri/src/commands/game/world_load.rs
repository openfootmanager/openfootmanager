//! Getting a world into memory, and deciding what date it opens on.
//!
//! Three sources feed the same `WorldData`: a generated random world, a
//! `.json` file, and a stack of installed `.ofm` packages merged last-wins.
//! `game_clock_for_world` is where a world's own opinion about its date — a
//! historical snapshot's `base_year` — is reconciled against the start year
//! the player picked.
//!
//! `definition_sources` lives here too: every path above needs to know where
//! the generator should look for definition files, and nothing outside this
//! module asks.

use chrono::Utc;
use log::warn;

use ofm_core::clock::GameClock;

use super::{
    current_date_for_phase, first_package_error_message, start_date_for_year, StartupOptions,
};

pub(super) fn load_world_data_from_path(
    world_source: &str,
) -> Result<ofm_core::generator::WorldData, String> {
    let path = world_source.strip_prefix("file:").unwrap_or(world_source);
    // The loader's own key is the answer: a file that is there but not a world says so, instead
    // of "failed to read".
    ofm_core::generator::load_world_from_path(std::path::Path::new(path))
}

/// A world with the reputations its competitions were validated against: a package
/// authors them, any other source ranks by the clubs' own.
pub(super) type LoadedWorld = (ofm_core::generator::WorldData, AuthoredReputations);

/// Load a world from a modular package directory (recursively scanned, schema
/// typed). Rejects an invalid package so a broken mod never loads half-applied,
/// with the same complete-world check a stack of installed packages gets, and
/// refuses a world with no clubs as that path does.
pub(super) fn load_world_data_from_package(
    dir: &str,
    sources: &ofm_core::generator::DefinitionSources,
) -> Result<LoadedWorld, String> {
    let (package, errors) =
        ofm_core::generator::load_world_package_as_world(std::path::Path::new(dir));
    if !errors.is_empty() {
        return Err(first_package_error_message(&errors));
    }
    let world = ofm_core::generator::build_world_from_package_seeded(
        rand::random(),
        &package,
        None,
        sources,
    )?;
    if world.teams.is_empty() {
        return Err("be.error.package.noDatabasePackage".to_string());
    }
    Ok((world, ofm_core::generator::authored_reputations(&package)))
}

/// Where the generator looks for definition files, in priority order.
///
/// The player's own `data/` directory under the app data dir wins, so a file
/// dropped there overrides the shipped one; the bundled `data/` beside the
/// installed game comes next, and gives the player a real file to read and
/// copy. Both are `Option`s from the OS, and both may be absent — in a dev
/// build there is no resource dir at all — which is exactly why the generator
/// keeps a copy of the same files compiled in.
pub(super) fn definition_sources(
    app_handle: &tauri::AppHandle,
) -> ofm_core::generator::DefinitionSources {
    use tauri::Manager;
    let resolver = app_handle.path();
    let dirs = [resolver.app_data_dir().ok(), resolver.resource_dir().ok()]
        .into_iter()
        .flatten()
        .map(|dir| dir.join("data"))
        .collect::<Vec<_>>();
    ofm_core::generator::DefinitionSources::searching(dirs)
}

pub(super) fn load_world_data(
    world_source: Option<&str>,
    sources: &ofm_core::generator::DefinitionSources,
) -> Result<LoadedWorld, String> {
    match world_source {
        None | Some("random") => Ok((
            ofm_core::generator::generate_world_data(sources),
            AuthoredReputations::new(),
        )),
        Some(source) => {
            let raw = source.strip_prefix("file:").unwrap_or(source);
            if std::path::Path::new(raw).is_dir() {
                load_world_data_from_package(raw, sources)
            } else {
                Ok((
                    load_world_data_from_path(source)?,
                    AuthoredReputations::new(),
                ))
            }
        }
    }
}

/// The authored reputation of each club by id, which competition selectors rank by.
pub(super) type AuthoredReputations = std::collections::HashMap<String, u32>;

/// Load world data from a stack of installed `.ofm` packages (by id).
/// Packages are merged in order with last-wins semantics for duplicate ids.
/// Also returns the package lockfile entries for saving alongside the game.
pub(super) fn load_world_data_from_package_ids(
    packages_dir: &std::path::Path,
    package_ids: &[String],
    opening_year: Option<u32>,
    asset_root: Option<&std::path::Path>,
    sources: &ofm_core::generator::DefinitionSources,
) -> Result<
    (
        ofm_core::generator::WorldData,
        Vec<ofm_core::generator::PackageLock>,
        AuthoredReputations,
    ),
    String,
> {
    let mut loaded = Vec::with_capacity(package_ids.len());
    let mut lockfile = Vec::with_capacity(package_ids.len());
    for id in package_ids {
        // Ids come from the frontend selection; reject traversal tokens before
        // joining into a filesystem path under packages_dir.
        crate::commands::world::validate_package_id(id)?;
        let path = packages_dir.join(format!("{id}.ofm"));
        let (mut pkg, errors) = ofm_core::generator::load_world_package_from_ofm(&path);
        if !errors.is_empty() {
            return Err(first_package_error_message(&errors));
        }
        // Land this package's artwork somewhere the webview can read it, and
        // stamp the package id onto every asset path. Both must happen before
        // the merge below, which collapses the manifests and would otherwise
        // lose which package a club's badge came from. Extraction failure is
        // not fatal — the world still plays, clubs just keep generated crests.
        if let Some(root) = asset_root {
            let package_assets = root.join(id);
            match ofm_core::generator::extract_package_assets(&path, &package_assets) {
                Ok(skipped) if !skipped.is_empty() => {
                    warn!("[assets] {id}: {} asset entries skipped", skipped.len());
                }
                Err(err) => warn!("[assets] {id}: extraction failed: {err}"),
                _ => {}
            }
            ofm_core::generator::qualify_package_asset_paths(&mut pkg, id);
        }
        let version = pkg
            .meta
            .as_ref()
            .map(|m| m.version.clone())
            .unwrap_or_default();
        let hash = ofm_core::generator::hash_package_file(&path).unwrap_or_default();
        lockfile.push(ofm_core::generator::PackageLock {
            id: id.clone(),
            version,
            hash,
        });
        loaded.push(pkg);
    }
    let (merged, errors) = ofm_core::generator::merge_world_packages(loaded);
    if !errors.is_empty() {
        return Err(first_package_error_message(&errors));
    }
    let world = ofm_core::generator::build_world_from_package_seeded(
        rand::random(),
        &merged,
        opening_year,
        sources,
    )?;
    if world.teams.is_empty() {
        return Err("be.error.package.noDatabasePackage".to_string());
    }
    Ok((
        world,
        lockfile,
        ofm_core::generator::authored_reputations(&merged),
    ))
}

pub(super) fn world_start_year(
    startup_options: &StartupOptions,
    metadata: &ofm_core::generator::WorldDataMetadata,
) -> i32 {
    match metadata.kind {
        ofm_core::generator::WorldDataKind::HistoricalSnapshot => {
            metadata.base_year.unwrap_or(startup_options.start_year)
        }
        ofm_core::generator::WorldDataKind::RosterBaseline => startup_options.start_year,
    }
}

pub(super) fn game_clock_for_world(
    startup_options: &StartupOptions,
    metadata: &ofm_core::generator::WorldDataMetadata,
) -> Result<GameClock, String> {
    let start_year = world_start_year(startup_options, metadata);
    let mut clock = GameClock::new(start_date_for_year(start_year)?);
    clock.current_date = match metadata.kind {
        ofm_core::generator::WorldDataKind::HistoricalSnapshot => metadata
            .snapshot_date
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or(current_date_for_phase(
                start_year,
                startup_options.start_phase,
            )?),
        ofm_core::generator::WorldDataKind::RosterBaseline => {
            current_date_for_phase(startup_options.start_year, startup_options.start_phase)?
        }
    };
    Ok(clock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::game::testkit::{make_historical_snapshot_world, temp_pkg_dir};
    use crate::commands::game::DEFAULT_GENERATED_HISTORY_DEPTH_YEARS;
    use ofm_core::career::StartPhase;

    /// A patch package directory that signs a player for a club no package defines.
    fn ghost_club_patch(tag: &str) -> std::path::PathBuf {
        let dir = temp_pkg_dir(tag);
        std::fs::write(
            dir.join("world.yaml"),
            "schema: world\nid: roster\nname: Roster\nversion: 1.0.0\nlicense: CC0-1.0\npackageType: patch\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("player.yaml"),
            "schema: player\nid: new-signing\nclub: ghost-club\n",
        )
        .unwrap();
        dir
    }

    /// A database package directory of two English clubs, "base-fc" and "other-fc".
    fn base_database_dir(tag: &str) -> std::path::PathBuf {
        let dir = temp_pkg_dir(tag);
        std::fs::write(
            dir.join("world.yaml"),
            "schema: world\nid: base\nname: Base\nversion: 1.0.0\nlicense: CC0-1.0\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("country.yaml"),
            "schema: country\nid: ENG\nname: England\n",
        )
        .unwrap();
        let team = |id: &str, name: &str| {
            format!(
                "schema: team\nid: {id}\nname: {name}\ncity: Town\ncountry: ENG\ncolors:\n  primary: \"#000000\"\n  secondary: \"#ffffff\"\n"
            )
        };
        std::fs::write(dir.join("base-fc.yaml"), team("base-fc", "Base FC")).unwrap();
        std::fs::write(dir.join("other-fc.yaml"), team("other-fc", "Other FC")).unwrap();
        dir
    }

    /// A patch directory signing a player for `club`.
    fn patch_signing_for(tag: &str, club: &str) -> std::path::PathBuf {
        let dir = temp_pkg_dir(tag);
        std::fs::write(
            dir.join("world.yaml"),
            "schema: world\nid: roster\nname: Roster\nversion: 1.0.0\nlicense: CC0-1.0\npackageType: patch\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("player.yaml"),
            format!("schema: player\nid: new-signing\nclub: {club}\n"),
        )
        .unwrap();
        dir
    }

    /// The game's stack loader on `base` then `patch`, installed as `.ofm` packages, beside what
    /// `ofm-cli validate --base` (`check_world_package_on_bases`) says of the same directories.
    fn game_and_cli_on_a_stack(
        tag: &str,
        patch_club: &str,
    ) -> (Result<(), String>, Vec<ofm_core::generator::PackageError>) {
        let base = base_database_dir(&format!("{tag}-base"));
        let patch = patch_signing_for(&format!("{tag}-patch"), patch_club);
        let packages = temp_pkg_dir(&format!("{tag}-installed"));
        ofm_core::generator::export_directory_to_ofm(&base, &packages.join("base.ofm"))
            .expect("the base is packed");
        ofm_core::generator::export_directory_to_ofm(&patch, &packages.join("roster.ofm"))
            .expect("the patch is packed");

        let game = super::load_world_data_from_package_ids(
            &packages,
            &["base".to_string(), "roster".to_string()],
            None,
            None,
            &ofm_core::generator::DefinitionSources::embedded_only(),
        )
        .map(|_| ());
        let (_, cli) =
            ofm_core::generator::check_world_package_on_bases(&patch, std::slice::from_ref(&base));
        for dir in [&base, &patch, &packages] {
            std::fs::remove_dir_all(dir).ok();
        }
        (game, cli)
    }

    /// Given a patch naming a club its base defines, stacked on that base
    /// When the game starts a career on the stack, and the CLI validates the patch `--base` it
    /// Then the game starts and the CLI reports nothing: the two agree (#1364).
    #[test]
    fn a_patch_on_its_base_passes_the_game_and_the_cli_alike() {
        let (game, cli) = game_and_cli_on_a_stack("parity-legit", "base-fc");

        assert_eq!(game, Ok(()));
        assert!(cli.is_empty(), "{cli:?}");
    }

    /// Given a patch naming a club neither it nor its base defines, stacked on that base
    /// When the game starts a career on the stack, and the CLI validates the patch `--base` it
    /// Then both refuse it with the same finding.
    #[test]
    fn a_ghost_club_patch_on_its_base_fails_the_game_and_the_cli_alike() {
        let (game, cli) = game_and_cli_on_a_stack("parity-ghost", "ghost-club");

        let key = game.expect_err("the game refuses the stack");
        assert!(!cli.is_empty(), "the CLI refuses it too");
        assert!(
            key.starts_with(&cli[0].code),
            "game {key} vs CLI {:?}",
            cli[0]
        );
    }

    /// Given a patch package directory whose player joins a club no package defines
    /// When a career is started on that directory
    /// Then the start is refused with the unknown team: the player is not silently left out
    #[test]
    fn a_patch_directory_naming_a_club_no_package_defines_does_not_start() {
        let dir = ghost_club_patch("start-ghost");

        let result = super::load_world_data(
            Some(dir.to_string_lossy().as_ref()),
            &ofm_core::generator::DefinitionSources::embedded_only(),
        );
        std::fs::remove_dir_all(&dir).ok();

        match result {
            Err(key) => assert!(key.starts_with("be.error.package.unknownTeam"), "{key}"),
            Ok(_) => panic!("a patch naming a ghost club must not start a career"),
        }
    }

    /// Given an installed patch package (`.ofm`) whose player joins a club no package defines,
    ///      stacked alone
    /// When a career is started on the stack
    /// Then the merge's unknown team refuses the start
    #[test]
    fn a_stack_of_one_patch_naming_a_club_no_package_defines_does_not_start() {
        let source = ghost_club_patch("stack-ghost-src");
        let packages = temp_pkg_dir("stack-ghost");
        ofm_core::generator::export_directory_to_ofm(&source, &packages.join("roster.ofm"))
            .expect("the patch is packed");

        let result = super::load_world_data_from_package_ids(
            &packages,
            &["roster".to_string()],
            None,
            None,
            &ofm_core::generator::DefinitionSources::embedded_only(),
        );
        std::fs::remove_dir_all(&source).ok();
        std::fs::remove_dir_all(&packages).ok();

        match result {
            Err(key) => assert!(key.starts_with("be.error.package.unknownTeam"), "{key}"),
            Ok(_) => panic!("a stack of one patch naming a ghost club must not start a career"),
        }
    }

    #[test]
    fn loads_a_world_from_a_package_directory() {
        let dir = temp_pkg_dir("load");
        std::fs::write(
            dir.join("confed.yaml"),
            "schema: confederation\nid: galaxy\nname: Galaxy\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("country.yaml"),
            "schema: country\nid: ZZ\nname: Zedland\nconfederation: galaxy\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("teams.yaml"),
            "schema: team\nitems:\n  - { id: zed-fc, name: Zed FC, city: Zedtown, country: ZZ, colors: { primary: \"#000\", secondary: \"#fff\" } }\n  - { id: zed-utd, name: Zed United, city: Zedford, country: ZZ, colors: { primary: \"#111\", secondary: \"#fff\" } }\n",
        )
        .unwrap();

        let (world, authored) = super::load_world_data(
            Some(dir.to_string_lossy().as_ref()),
            &ofm_core::generator::DefinitionSources::embedded_only(),
        )
        .expect("package loads");
        assert_eq!(
            authored.len(),
            2,
            "a package directory brings its reputations"
        );
        assert!(world.teams.iter().any(|t| t.id == "zed-fc"));
        assert!(world.teams.iter().any(|t| t.id == "zed-utd"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_world_data_from_path_returns_read_file_key_when_missing() {
        let result =
            load_world_data_from_path("file:Z:/definitely-missing/openfootmanager-world.json");

        assert_eq!(result.unwrap_err(), "be.error.worldReadFileFailed");
    }

    /// Given a file that exists but is not a world
    /// When it is loaded for a new game
    /// Then the error says it could not be parsed, not that it could not be read.
    #[test]
    fn load_world_data_from_path_says_parse_failed_for_a_file_that_is_not_a_world() {
        let dir = std::env::temp_dir().join(format!(
            "ofm-world-load-parse-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        std::fs::write(&file, r#"{"name":"settings"}"#).unwrap();

        let result = load_world_data_from_path(file.to_str().unwrap());
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(result.unwrap_err(), "be.error.worldParseFailed");
    }

    #[test]
    fn game_clock_for_world_rejects_out_of_range_snapshot_base_year() {
        let startup_options = StartupOptions {
            start_year: 2032,
            start_phase: StartPhase::MidSeason,
            history_depth_years: DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
        };
        let mut world = make_historical_snapshot_world();
        world.metadata.base_year = Some(i32::MAX);

        let result = game_clock_for_world(&startup_options, &world.metadata);

        assert_eq!(
            result.unwrap_err(),
            "be.error.createManager.invalidStartYear"
        );
    }
}
