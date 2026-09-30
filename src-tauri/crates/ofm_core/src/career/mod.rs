//! Starting a career: putting the player's manager in charge of a club.
//!
//! Whatever door the player comes through — the app, or an agent playing over
//! MCP — choosing a club is the same sequence of game rules, so it lives here,
//! once, and the callers are thin adapters over it.

mod bootstrap;

use domain::stats::StatsState;

use crate::contracts::{club_season_anchors, stamp_opening_contract_starts};
use crate::game::Game;
use crate::world::{
    ensure_multi_competition_foundations, rebuild_competitions_for_management_date,
    team_season_anchor,
};

use bootstrap::{
    bootstrap_existing_world_takeover, bootstrap_midseason_takeover, bootstrap_season_start,
    has_existing_world_context,
};

/// Where in its season a career opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartPhase {
    SeasonStart,
    MidSeason,
}

impl StartPhase {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "seasonStart" => Some(Self::SeasonStart),
            "midSeason" => Some(Self::MidSeason),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SeasonStart => "seasonStart",
            Self::MidSeason => "midSeason",
        }
    }
}

pub fn start_phase_for_game(game: &Game) -> StartPhase {
    if game.clock.current_date > game.clock.start_date {
        StartPhase::MidSeason
    } else {
        StartPhase::SeasonStart
    }
}

/// When the player picks `SeasonStart` for a southern-hemisphere (or other
/// non-August-start) club, align the game clock to that club's actual season-start
/// date and rebuild competitions from that anchor, so the player arrives at the
/// beginning of their season, not in July.
fn align_clock_to_club_season(game: &mut Game, team_id: &str) {
    if start_phase_for_game(game) == StartPhase::SeasonStart
        && let Some(actual_start) = team_season_anchor(game, team_id)
        && actual_start < game.clock.current_date
    {
        game.clock.current_date = actual_start;
        game.clock.start_date = actual_start;
        rebuild_competitions_for_management_date(game, actual_start);
        game.national_teams.clear();
        ensure_multi_competition_foundations(game);
    }
}

/// Settle the date a career opens on, then give every contract that has no start the
/// day it began, measured against that date.
///
/// `align_clock_to` is the club whose season the clock is pulled back to, for the
/// paths that do that. The order is the point and lives here so it is written once:
/// every club's season anchor is read *first*, because Brazil's is worked out from
/// the clock's year and reading it after the clock has moved lands a year early;
/// the clock is then moved; and starts are stamped last, against the date the career
/// really opens on, so none can land after it.
pub fn date_opening_contracts(game: &mut Game, align_clock_to: Option<&str>) {
    let club_anchors = club_season_anchors(game);
    if let Some(team_id) = align_clock_to {
        align_clock_to_club_season(game, team_id);
    }
    stamp_opening_contract_starts(game, &club_anchors);
}

pub fn bootstrap_team_selection(
    game: &mut Game,
    team_id: &str,
    start_phase: StartPhase,
    stats_state: StatsState,
) -> Result<StatsState, String> {
    let stats_state = if has_existing_world_context(game, &stats_state) {
        bootstrap_existing_world_takeover(game, team_id, stats_state)?
    } else {
        match start_phase {
            StartPhase::SeasonStart => bootstrap_season_start(game, team_id)?,
            StartPhase::MidSeason => bootstrap_midseason_takeover(game, team_id)?,
        }
    };

    // World generation has already equipped every club for AI management.
    // At career selection the chosen club becomes the player's blank slate;
    // rivals keep the identity the generator gave them.
    if let Some(team) = game.teams.iter_mut().find(|team| team.id == team_id) {
        team.tactics_phase = domain::team::TacticsPhaseSettings::default();
        team.player_roles.clear();
    }

    crate::transfers::seed_opening_ai_loan_market(game);
    Ok(stats_state)
}
