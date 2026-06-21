//! Shared helpers for MCP tool implementations.

use chrono::Datelike;
use ofm_core::state::StateManager;

/// Get the active game from StateManager, returning a formatted error if none.
pub(crate) fn require_game(state_manager: &StateManager) -> Result<ofm_core::game::Game, String> {
    state_manager
        .get_game(|g| g.clone())
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())
}

/// Get the user's team from the game.
pub(crate) fn user_team(game: &ofm_core::game::Game) -> Result<&domain::team::Team, String> {
    let team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or("be.error.noTeamAssigned")?;
    game.teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or_else(|| "be.error.teamNotFound".to_string())
}

/// Get the league, returning a formatted error if none.
pub(crate) fn require_league(game: &ofm_core::game::Game) -> Result<&domain::league::League, String> {
    game.league
        .as_ref()
        .ok_or_else(|| "No league found. Season may not have started yet.".to_string())
}

/// Format a player position as a short code.
pub(crate) fn format_position(pos: &domain::player::Position) -> &'static str {
    match pos {
        domain::player::Position::Goalkeeper => "GK",
        domain::player::Position::Defender => "DF",
        domain::player::Position::Midfielder => "MF",
        domain::player::Position::Forward => "FW",
        domain::player::Position::RightBack => "RB",
        domain::player::Position::CenterBack => "CB",
        domain::player::Position::LeftBack => "LB",
        domain::player::Position::RightWingBack => "RWB",
        domain::player::Position::LeftWingBack => "LWB",
        domain::player::Position::DefensiveMidfielder => "DM",
        domain::player::Position::CentralMidfielder => "CM",
        domain::player::Position::AttackingMidfielder => "AM",
        domain::player::Position::RightMidfielder => "RM",
        domain::player::Position::LeftMidfielder => "LM",
        domain::player::Position::RightWinger => "RW",
        domain::player::Position::LeftWinger => "LW",
        domain::player::Position::Striker => "ST",
    }
}

/// Calculate age from date of birth relative to the game clock.
pub(crate) fn age_from_dob(dob: &str, game: &ofm_core::game::Game) -> String {
    let dob_date = match chrono::NaiveDate::parse_from_str(dob, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return "?".to_string(),
    };
    let ref_date = game.clock.current_date.date_naive();
    let mut age = i32::from(ref_date.year()) - i32::from(dob_date.year());
    if (ref_date.month(), ref_date.day()) < (dob_date.month(), dob_date.day()) {
        age -= 1;
    }
    age.to_string()
}
