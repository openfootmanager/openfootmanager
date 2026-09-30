//! Fixtures shared by the tests in this module.
//!
//! Defined once here rather than per file: the world builders are tested from
//! both `mod.rs` and `foundations.rs`, and a club constructor copied into each
//! is exactly the drift this crate is trying to stop.

use domain::manager::Manager;

use crate::clock::GameClock;
use crate::game::Game;

use super::start_date_for_year;

/// A club that belongs to `nation`, strong enough to be ranked against others.
pub(super) fn nation_team(id: &str, nation: &str, reputation: u32) -> domain::team::Team {
    let mut team = domain::team::Team::new(
        id.to_string(),
        id.to_string(),
        id.to_string(),
        nation.to_string(),
        "City".to_string(),
        "Stadium".to_string(),
        10_000,
    );
    team.football_nation = nation.to_string();
    team.reputation = reputation;
    team
}

pub(super) fn manager_for(team_id: &str) -> Manager {
    let mut manager = Manager::new(
        "mgr".to_string(),
        "A".to_string(),
        "B".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(team_id.to_string());
    manager
}

/// Two clubs and a manager, which is all the simulation-scope tests need: they
/// set `football_nation` and `competitions` themselves.
pub(super) fn scope_test_game() -> Game {
    let clock = GameClock::new(start_date_for_year(2032).unwrap());
    let teams = vec![
        nation_team("team1", "England", 500),
        nation_team("team2", "England", 400),
    ];
    Game::new(clock, manager_for("team1"), teams, vec![], vec![], vec![])
}
