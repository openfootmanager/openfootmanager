//! Fixtures shared by the tests that build worlds.
//!
//! Defined once here rather than per file: the world builders and the career
//! start are tested from several modules, and a club constructor copied into
//! each is exactly the drift this crate is trying to stop.

use domain::manager::Manager;
use domain::player::{Player, PlayerAttributes, Position};

use crate::clock::GameClock;
use crate::game::Game;

use super::start_date_for_year;

/// A club that belongs to `nation`, strong enough to be ranked against others.
pub(crate) fn nation_team(id: &str, nation: &str, reputation: u32) -> domain::team::Team {
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

pub(crate) fn manager_for(team_id: &str) -> Manager {
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

/// A manager who has not taken a club yet.
pub(crate) fn unemployed_manager() -> Manager {
    Manager::new(
        "user-manager".to_string(),
        "Pat".to_string(),
        "Player".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    )
}

/// A midfielder of `team_id` on a contract that runs to `contract_end`, with the
/// generic position every generated player starts with.
pub(crate) fn player_at(id: &str, team_id: &str, contract_end: &str) -> Player {
    let level = 60;
    let mut player = Player::new(
        id.to_string(),
        id.to_string(),
        id.to_string(),
        "1998-01-01".to_string(),
        "England".to_string(),
        Position::Midfielder,
        PlayerAttributes {
            pace: level,
            stamina: level,
            strength: level,
            agility: level,
            passing: level,
            shooting: level,
            tackling: level,
            dribbling: level,
            defending: level,
            positioning: level,
            vision: level,
            decisions: level,
            composure: level,
            aggression: level,
            teamwork: level,
            leadership: level,
            handling: level,
            reflexes: level,
            aerial: level,
        },
    );
    player.team_id = Some(team_id.to_string());
    player.contract_end = Some(contract_end.to_string());
    player
}

/// Two clubs and a manager, which is all the simulation-scope tests need: they
/// set `football_nation` and `competitions` themselves.
pub(crate) fn scope_test_game() -> Game {
    let clock = GameClock::new(start_date_for_year(2032).unwrap());
    let teams = vec![
        nation_team("team1", "England", 500),
        nation_team("team2", "England", 400),
    ];
    Game::new(clock, manager_for("team1"), teams, vec![], vec![], vec![])
}
