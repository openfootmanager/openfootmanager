//! The command layer's entry to building a `Game` from loaded world data: it translates the
//! startup options into what `ofm_core::world::build` takes, and nothing more.

use domain::manager::Manager;
use domain::stats::StatsState;
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::world::build::PastHistory;

use super::StartupOptions;

/// Only a world that is not built from a package comes through here: the MCP bootstrap, and tests.
#[cfg(any(test, feature = "mcp"))]
pub(super) fn build_game_from_world_data(
    clock: GameClock,
    manager: Manager,
    startup_options: &StartupOptions,
    world: ofm_core::generator::WorldData,
) -> (Game, StatsState) {
    build_game_from_world_data_ranked(
        clock,
        manager,
        startup_options,
        world,
        ofm_core::generator::ReputationBasis::Stored,
    )
}

/// [`build_game_from_world_data`], resolving the world's competition definitions with clubs
/// ranked by `reputation_basis`.
pub(super) fn build_game_from_world_data_ranked(
    clock: GameClock,
    manager: Manager,
    startup_options: &StartupOptions,
    world: ofm_core::generator::WorldData,
    reputation_basis: ofm_core::generator::ReputationBasis<'_>,
) -> (Game, StatsState) {
    let history = PastHistory {
        start_year: startup_options.start_year,
        depth_years: startup_options.history_depth_years,
    };
    ofm_core::world::build::build_game_from_world_data(
        clock,
        manager,
        history,
        world,
        reputation_basis,
    )
}
