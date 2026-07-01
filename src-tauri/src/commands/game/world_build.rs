//! World loading and game construction: reading world data from a path,
//! package directory, or package-id set, deriving the game clock, and
//! assembling the initial `Game` (incl. generated past history). Faithful
//! extraction; behaviour locked by the build/clock tests in `game::tests`.

use chrono::Utc;
use domain::league::League;
use domain::manager::Manager;
use domain::stats::StatsState;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;

use super::foundation::ensure_multi_competition_foundations;
use super::startup::{current_date_for_phase, start_date_for_year, StartupOptions};
use super::util::preseason_league_year;

pub(crate) fn load_world_data_from_path(
    world_source: &str,
) -> Result<ofm_core::generator::WorldData, String> {
    let path = world_source.strip_prefix("file:").unwrap_or(world_source);
    ofm_core::generator::load_world_from_path(std::path::Path::new(path))
        .map_err(|_| "be.error.worldReadFileFailed".to_string())
}

/// Load a world from a modular package directory (recursively scanned, schema
/// typed). Rejects an invalid package so a broken mod never loads half-applied.
fn load_world_data_from_package(dir: &str) -> Result<ofm_core::generator::WorldData, String> {
    let (package, errors) = ofm_core::generator::load_world_package(std::path::Path::new(dir));
    if !errors.is_empty() {
        return Err("be.error.package.invalid".to_string());
    }
    ofm_core::generator::build_world_from_package(&package)
}

pub(crate) fn apply_generated_past_history(game: &mut Game, startup_options: &StartupOptions) {
    ofm_core::history_generation::generate_past_world_history(
        game,
        startup_options.start_year,
        startup_options.history_depth_years,
    );
}

pub(crate) fn load_world_data(
    world_source: Option<&str>,
) -> Result<ofm_core::generator::WorldData, String> {
    match world_source {
        None | Some("random") => Ok(ofm_core::generator::generate_world_data(None)),
        Some(source) => {
            let raw = source.strip_prefix("file:").unwrap_or(source);
            if std::path::Path::new(raw).is_dir() {
                load_world_data_from_package(raw)
            } else {
                load_world_data_from_path(source)
            }
        }
    }
}

/// Load world data from a stack of installed `.ofm` packages (by id).
/// Packages are merged in order with last-wins semantics for duplicate ids.
/// Also returns the package lockfile entries for saving alongside the game.
pub(crate) fn load_world_data_from_package_ids(
    packages_dir: &std::path::Path,
    package_ids: &[String],
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
        let (pkg, errors) = ofm_core::generator::load_world_package_from_ofm(&path);
        if !errors.is_empty() {
            return Err("be.error.package.invalid".to_string());
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
        return Err("be.error.package.invalid".to_string());
    }
    let world = ofm_core::generator::build_world_from_package(&merged)?;
    if world.teams.is_empty() {
        return Err("be.error.package.noDatabasePackage".to_string());
    }
    Ok((world, lockfile))
}

fn world_start_year(
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

pub(crate) fn game_clock_for_world(
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

pub(crate) fn build_game_from_world_data(
    clock: GameClock,
    manager: Manager,
    startup_options: &StartupOptions,
    world: ofm_core::generator::WorldData,
) -> (Game, StatsState) {
    // Resolve any authored competition definitions while we still hold the
    // world (validation already passed at load). These replace the auto-built
    // foundation competitions.
    let game_start = clock.start_date;
    let defined_competitions: Vec<League> = world
        .competition_definitions
        .as_ref()
        .map(|file| {
            let mut comps = ofm_core::generator::resolve_definitions(
                file,
                &world,
                preseason_league_year(&clock),
                game_start,
            );
            for comp in &mut comps {
                let (_, is_mid_season) = ofm_core::generator::start_date_at_game_open(
                    game_start,
                    comp.season_start_month,
                    comp.season_start_day,
                );
                if is_mid_season {
                    ofm_core::catchup::simulate_past_fixtures(comp, &world.players, game_start);
                }
            }
            comps
        })
        .unwrap_or_default();

    let ofm_core::generator::WorldData {
        teams,
        players,
        staff,
        managers,
        competitions,
        national_teams,
        default_active_regions,
        default_active_competitions,
        league,
        news,
        stats,
        world_history,
        metadata,
        extra_translations,
        ..
    } = world;

    let mut game = Game::new(clock, manager, teams, players, staff, vec![]);
    if game
        .staff
        .iter()
        .any(|staff_member| staff_member.team_id.is_none())
    {
        game.available_staff_market_last_activity_date =
            Some(game.clock.current_date.format("%Y-%m-%d").to_string());
    }
    ofm_core::generator::repair_opening_youth_academies(&mut game);

    // Authored definitions take precedence over both the snapshot's stored
    // competitions and the auto-built foundations.
    let competitions = if defined_competitions.is_empty() {
        competitions
    } else {
        defined_competitions
    };

    match metadata.kind {
        ofm_core::generator::WorldDataKind::HistoricalSnapshot => {
            game.managers.extend(
                managers
                    .into_iter()
                    .filter(|existing_manager| existing_manager.id != game.manager.id),
            );
            game.competitions = competitions;
            game.national_teams = national_teams;
            game.active_region_ids = default_active_regions;
            game.active_competition_ids = default_active_competitions;
            game.league = league;
            game.promote_legacy_league();
            game.news = news;
            game.world_history = world_history;
            game.extra_translations = extra_translations;
            ensure_multi_competition_foundations(&mut game);
            ofm_core::season_context::refresh_game_context(&mut game);
            (game, stats)
        }
        ofm_core::generator::WorldDataKind::RosterBaseline => {
            // Authored definitions, if any, become the world's competitions;
            // otherwise ensure_multi_competition_foundations auto-builds them.
            game.competitions = competitions;
            game.extra_translations = extra_translations;
            // Build the league/division foundations *before* generating history so
            // each club's past seasons are attributed to its real ~20-team
            // division. Otherwise history runs with no competitions and treats the
            // whole world as one mega-league (≈880-match seasons).
            ensure_multi_competition_foundations(&mut game);
            apply_generated_past_history(&mut game, startup_options);
            (game, StatsState::default())
        }
    }
}
