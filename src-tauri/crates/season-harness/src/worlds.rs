//! The world a run plays in: generated from a seed, or built from a package the way the app
//! builds one, and given a pyramid by the same builder every new career uses, with the manager in
//! charge of a club.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use domain::manager::Manager;
use domain::stats::StatsState;
use ofm_core::career::{CareerScope, begin_career};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, ReputationBasis, WorldGenConfig, WorldPackage, authored_reputations,
    build_world_from_package_seeded, export_directory_to_ofm, generate_world_data_seeded_with,
    load_world_package_from_ofm, merge_world_packages, repair_opening_youth_academies,
};
use ofm_core::world::build::{PastHistory, build_game_from_imported_world};
use ofm_core::world::start_date_for_year;

/// The package the harness builds a world from: forty English clubs in two divisions of twenty,
/// and a cup. Divisions keep a real size: an eight-club league ends in October, before the game's
/// calendar has met its international windows. Text on disk, packed into a `.ofm` the way an installed package is.
pub const TWO_TIER_PACKAGE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/two-tier-package"
);

/// A package in which every player of every club is authored, rating and ceiling included: a world
/// the generator's rating bell draws nothing for, so what it builds must not move when the bell does.
pub const AUTHORED_ONLY_PACKAGE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/authored-only-package"
);

/// The shipped world's nations that between them cover every shape that has
/// broken the ladder: England's two tiers, Portugal's single tier, and
/// Argentina's split season, all berthing into one continental cup.
pub const GATE_NATIONS: &[&str] = &["ENG", "PT", "AR"];

/// Which competitions the game plays in full. The game's default follows the player's own
/// competitions and settles the rest of the world by scoreline (the dormant tier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    /// What a new career gets: the player's competitions and the regions they need.
    #[default]
    Followed,
    /// No scope at all, which the game reads as "everything is active": every club fixture is
    /// played by the match engine, and every club is simulated in full each day.
    Everything,
}

/// A world to play: the shipped one, minus some nations.
///
/// Divisions keep their real size. Shrinking them is tempting — a season costs
/// more the more clubs there are — but an 8-club league finishes in October, so
/// the rollover fires before that year's international windows have happened and
/// the calendar the world runs on is not the game's. Fewer *nations* keeps every
/// shape and the real calendar: England (40) + Portugal (20) + Argentina (20) is
/// 80 clubs and plays two seasons in about eight seconds.
#[derive(Debug, Clone)]
pub struct WorldSpec {
    pub seed: u64,
    /// The nations kept from the shipped world; empty keeps all of them.
    pub nations: &'static [&'static str],
    /// The manager takes this nation's strongest club.
    pub user_nation: &'static str,
    pub start_year: i32,
    pub scope: Scope,
    /// A package directory to build the world from, as the app does for an installed package.
    /// `None` generates the world from `nations` instead.
    pub package: Option<PathBuf>,
}

impl WorldSpec {
    /// England, Portugal and Argentina from 2033, which puts a World Cup summer
    /// on the first rollover.
    pub fn gate(seed: u64) -> Self {
        Self {
            seed,
            nations: GATE_NATIONS,
            user_nation: "ENG",
            start_year: 2033,
            scope: Scope::Followed,
            package: None,
        }
    }

    /// Every nation the game ships with (440 clubs), as a player gets it.
    pub fn shipped(seed: u64) -> Self {
        Self {
            nations: &[],
            ..Self::gate(seed)
        }
    }

    /// The world the app builds from the [`TWO_TIER_PACKAGE`] fixture, from 2033.
    pub fn two_tier_package(seed: u64) -> Self {
        Self {
            nations: &[],
            package: Some(PathBuf::from(TWO_TIER_PACKAGE)),
            ..Self::gate(seed)
        }
    }

    /// The world the app builds from the [`AUTHORED_ONLY_PACKAGE`] fixture, from 2033.
    pub fn authored_only_package(seed: u64) -> Self {
        Self {
            nations: &[],
            package: Some(PathBuf::from(AUTHORED_ONLY_PACKAGE)),
            ..Self::gate(seed)
        }
    }

    pub fn with_scope(self, scope: Scope) -> Self {
        Self { scope, ..self }
    }

    pub fn build(&self) -> Result<Game, String> {
        match &self.package {
            Some(directory) => self.build_from_package(directory),
            None => self.build_generated(),
        }
    }

    /// What a new career does with an installed package: pack it, load it, merge it with the
    /// others (here none), build the world from the seed with the authored reputations, and build
    /// the game from the world.
    fn build_from_package(&self, directory: &Path) -> Result<Game, String> {
        let sources = DefinitionSources::embedded_only();
        let merged = load_installed_package(directory)?;
        let authored = authored_reputations(&merged);
        let opening_year = u32::try_from(self.start_year).map_err(|error| error.to_string())?;
        let world =
            build_world_from_package_seeded(self.seed, &merged, Some(opening_year), &sources)?;
        let clock = GameClock::new(start_date_for_year(self.start_year)?);
        let history = PastHistory {
            start_year: self.start_year,
            depth_years: 0,
        };
        // A package is an imported world: prepared for a career as the app prepares it.
        let (mut game, stats) = build_game_from_imported_world(
            clock,
            harness_manager(),
            history,
            world,
            ReputationBasis::Authored(&authored),
        );
        self.begin_career_in(&mut game, stats)?;
        Ok(game)
    }

    fn build_generated(&self) -> Result<Game, String> {
        let sources = DefinitionSources::embedded_only();
        let mut config = WorldGenConfig::standard_from(&sources);
        if !self.nations.is_empty() {
            config
                .nations
                .retain(|nation| self.nations.contains(&nation.code.as_str()));
        }
        // Dated for the year the clock opens in. Left to default it follows the
        // wall clock, which makes a run differ by the day it is made and opens
        // a 2033 game with every contract a few years expired.
        config.opening_year = u32::try_from(self.start_year).ok();
        let world = generate_world_data_seeded_with(self.seed, &config, &sources);
        // What the app does when it builds a game from a generated world: the world's seed is
        // the game's, so the day's dice are the seed's too.
        let game_seed = world.generation_seed.unwrap_or(self.seed);

        let start = start_date_for_year(self.start_year)?;
        let mut game = Game::new(
            GameClock::new(start),
            harness_manager(),
            world.teams,
            world.players,
            world.staff,
            vec![],
        );
        game.seed = game_seed;
        game.available_staff_market_last_activity_date = Some(start.format("%Y-%m-%d").to_string());
        repair_opening_youth_academies(&mut game);

        self.begin_career_in(&mut game, StatsState::default())?;
        Ok(game)
    }

    fn begin_career_in(&self, game: &mut Game, stats: StatsState) -> Result<(), String> {
        let club = self.strongest_club(game)?;
        begin_career(game, &club, CareerScope::default(), stats)?;
        if self.scope == Scope::Everything {
            game.active_region_ids.clear();
            game.active_competition_ids.clear();
        }
        Ok(())
    }

    fn strongest_club(&self, game: &Game) -> Result<String, String> {
        ofm_core::world::strongest_club(&game.teams, self.user_nation)
            .map(str::to_string)
            .ok_or_else(|| format!("the world has no {} club", self.user_nation))
    }
}

fn harness_manager() -> Manager {
    Manager::new(
        "season-harness-manager".to_string(),
        "Season".to_string(),
        "Harness".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    )
}

/// Packs `directory` into a `.ofm` in a scratch directory and loads it from there, as the app
/// loads an installed package, then merges it as the app merges a stack of one.
fn load_installed_package(directory: &Path) -> Result<WorldPackage, String> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let scratch = std::env::temp_dir().join(format!(
        "season-harness-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&scratch).map_err(|error| error.to_string())?;
    let archive = scratch.join("package.ofm");
    let loaded = export_directory_to_ofm(directory, &archive)
        .map(|()| load_world_package_from_ofm(&archive));
    let cleaned = std::fs::remove_dir_all(&scratch).map_err(|error| error.to_string());
    let (package, errors) = loaded?;
    cleaned?;
    if !errors.is_empty() {
        return Err(format!("the package does not load: {errors:?}"));
    }
    let (merged, errors) = merge_world_packages(vec![package]);
    if !errors.is_empty() {
        return Err(format!("the package does not merge: {errors:?}"));
    }
    Ok(merged)
}
