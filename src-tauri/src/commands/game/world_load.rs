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
    ofm_core::generator::load_world_from_path(std::path::Path::new(path))
        .map_err(|_| "be.error.worldReadFileFailed".to_string())
}

/// Load a world from a modular package directory (recursively scanned, schema
/// typed). Rejects an invalid package so a broken mod never loads half-applied.
pub(super) fn load_world_data_from_package(
    dir: &str,
    sources: &ofm_core::generator::DefinitionSources,
) -> Result<ofm_core::generator::WorldData, String> {
    let (package, errors) = ofm_core::generator::load_world_package(std::path::Path::new(dir));
    if !errors.is_empty() {
        return Err(first_package_error_message(&errors));
    }
    ofm_core::generator::build_world_from_package(&package, None, sources)
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
) -> Result<ofm_core::generator::WorldData, String> {
    match world_source {
        None | Some("random") => Ok(ofm_core::generator::generate_world_data(sources)),
        Some(source) => {
            let raw = source.strip_prefix("file:").unwrap_or(source);
            if std::path::Path::new(raw).is_dir() {
                load_world_data_from_package(raw, sources)
            } else {
                load_world_data_from_path(source)
            }
        }
    }
}

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
    let world = ofm_core::generator::build_world_from_package(&merged, opening_year, sources)?;
    if world.teams.is_empty() {
        return Err("be.error.package.noDatabasePackage".to_string());
    }
    Ok((world, lockfile))
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
    use crate::commands::game::{StartPhase, DEFAULT_GENERATED_HISTORY_DEPTH_YEARS};

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

        let world = super::load_world_data(
            Some(dir.to_string_lossy().as_ref()),
            &ofm_core::generator::DefinitionSources::embedded_only(),
        )
        .expect("package loads");
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
