//! Starting a career: putting the player's manager in charge of a club.
//!
//! Whatever door the player comes through — the app, or an agent playing over
//! MCP — choosing a club is the same sequence of game rules, so it lives here,
//! once, and the callers are thin adapters over it.

mod bootstrap;
#[cfg(test)]
mod tests;

use domain::stats::StatsState;

use crate::contracts::{club_season_anchors, stamp_opening_contract_starts};
use crate::game::Game;
use crate::player_identity::upgrade_game_player_identities;
use crate::world::{
    ensure_multi_competition_foundations, rebuild_competitions_for_management_date,
    resolve_simulation_scope, team_season_anchor,
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

/// What the player asked to have simulated in full, on top of the chosen club's own
/// region and competitions. `None` asks for nothing more.
#[derive(Debug, Clone, Default)]
pub struct CareerScope {
    pub regions: Option<Vec<String>>,
    pub competitions: Option<Vec<String>>,
}

/// Put the player's manager in charge of `team_id` and open the career.
///
/// This is the one way a club is chosen. The app and an agent playing over MCP
/// both call it, so the world they arrive at cannot differ by the door they came
/// through. In order:
///
/// 1. a club that does not exist is refused, before anything has moved, so an
///    error leaves the game as it was;
/// 2. the pyramid is built if the world has none;
/// 3. the date the career opens on is settled, and every contract dated against it
///    ([`date_opening_contracts`] — it reads each club's season anchor *before* the
///    clock moves, which is the reason this ordering is written once);
/// 4. the simulation scope is resolved for the club and what was asked for;
/// 5. the manager takes the club ([`bootstrap_team_selection`]);
/// 6. player positions are made granular, so they are right now rather than after
///    the first save and reload.
///
/// `game` is changed in place; on an error after step 1 it may be partly changed,
/// so callers pass a copy and keep the original until this returns `Ok`.
pub fn begin_career(
    game: &mut Game,
    team_id: &str,
    scope: CareerScope,
    stats_state: StatsState,
) -> Result<StatsState, String> {
    if !game.teams.iter().any(|team| team.id == team_id) {
        return Err("be.error.teamNotFound".to_string());
    }

    ensure_multi_competition_foundations(game);
    date_opening_contracts(game, Some(team_id));
    let opening = game.clock.current_date;

    let (regions, competitions) =
        resolve_simulation_scope(game, team_id, scope.regions, scope.competitions)?;
    game.active_region_ids = regions;
    game.active_competition_ids = competitions;

    let stats_state =
        bootstrap_team_selection(game, team_id, start_phase_for_game(game), stats_state)?;

    // Contract starts were stamped against `opening`. The clock may move on from
    // it, never back: a start stamped against a later date could land after it.
    debug_assert!(
        game.clock.current_date >= opening,
        "the career start moved the clock back after contracts were dated"
    );

    upgrade_game_player_identities(game);
    Ok(stats_state)
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
