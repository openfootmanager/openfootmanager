mod team_builder;
pub use team_builder::auto_select_set_pieces;
use team_builder::build_team_with_bench;
pub(crate) use team_builder::domain_to_engine_role;
pub(crate) use team_builder::domain_to_engine_tactics;

use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::game::Game;

use domain::league::StandingEntry;
use domain::manager::Manager;
use domain::team::MatchRoles;
use engine::ai::{self, AiPersonality, AiProfile};
use engine::{LiveMatchState, MatchCommand, MatchConfig, MatchSnapshot, MinuteResult, Side};

const LIVE_MATCH_NO_LEAGUE_ERROR: &str = "be.error.liveMatch.noLeague";
const LIVE_MATCH_FIXTURE_NOT_FOUND_ERROR: &str = "be.error.liveMatch.fixtureNotFound";

fn resolve_match_role_assignment(
    assigned_id: &Option<String>,
    starter_ids: &HashSet<String>,
    fallback_id: Option<String>,
) -> Option<String> {
    if let Some(player_id) = assigned_id
        && starter_ids.contains(player_id)
    {
        return Some(player_id.clone());
    }

    fallback_id
}

fn apply_saved_match_roles(
    match_state: &mut LiveMatchState,
    side: Side,
    match_roles: &MatchRoles,
    starter_ids: &[String],
    auto_selection: (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ),
) {
    let starter_id_set = starter_ids.iter().cloned().collect::<HashSet<_>>();
    let (auto_captain, auto_penalty, auto_free_kick, auto_corner) = auto_selection;

    if let Some(player_id) =
        resolve_match_role_assignment(&match_roles.captain, &starter_id_set, auto_captain)
    {
        let _ = match_state.apply_command(MatchCommand::SetCaptain { side, player_id });
    }

    if let Some(player_id) =
        resolve_match_role_assignment(&match_roles.penalty_taker, &starter_id_set, auto_penalty)
    {
        let _ = match_state.apply_command(MatchCommand::SetPenaltyTaker { side, player_id });
    }

    if let Some(player_id) = resolve_match_role_assignment(
        &match_roles.free_kick_taker,
        &starter_id_set,
        auto_free_kick,
    ) {
        let _ = match_state.apply_command(MatchCommand::SetFreeKickTaker { side, player_id });
    }

    if let Some(player_id) =
        resolve_match_role_assignment(&match_roles.corner_taker, &starter_id_set, auto_corner)
    {
        let _ = match_state.apply_command(MatchCommand::SetCornerTaker { side, player_id });
    }
}

// ---------------------------------------------------------------------------
// MatchMode — how the user wants to experience this match
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchMode {
    /// User controls their team live (full interactivity)
    Live,
    /// User watches as spectator (no interaction, can control speed)
    Spectator,
    /// Instantly simulate — no UI, just get the result
    Instant,
}

// ---------------------------------------------------------------------------
// LiveMatchSession — wraps LiveMatchState + metadata for Tauri layer
// ---------------------------------------------------------------------------

pub struct LiveMatchSession {
    pub match_state: LiveMatchState,
    pub rng: StdRng,
    pub mode: MatchMode,
    /// Index into the fixtures of the competition identified by
    /// `competition_id` — NOT necessarily into `game.league`, which
    /// `sync_legacy_league` resets to the user's domestic league.
    pub fixture_index: usize,
    /// Id of the competition (league or cup) this fixture belongs to; the
    /// finish path uses it to apply the report to the right competition.
    pub competition_id: String,
    pub round_matchday: u32,
    pub round_previous_standings: Vec<StandingEntry>,
    pub home_team_id: String,
    pub away_team_id: String,
    pub user_side: Option<Side>,
    pub ai_home: AiProfile,
    pub ai_away: AiProfile,
}

impl LiveMatchSession {
    /// Step one minute and apply AI decisions for computer-controlled sides.
    pub fn step(&mut self) -> MinuteResult {
        let result = self.match_state.step_minute(&mut self.rng);

        // Apply AI decisions for non-user sides (only during playing phases)
        if !result.is_finished {
            self.apply_ai_decisions();
        }

        result
    }

    /// Step multiple minutes at once (for fast-forward / instant sim).
    pub fn step_many(&mut self, count: u16) -> Vec<MinuteResult> {
        let mut results = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let result = self.step();
            let finished = result.is_finished;
            results.push(result);
            if finished {
                break;
            }
        }
        results
    }

    /// Run the entire match to completion instantly.
    pub fn run_to_completion(&mut self) -> Vec<MinuteResult> {
        let mut results = Vec::with_capacity(100);
        loop {
            let result = self.step();
            let finished = result.is_finished;
            results.push(result);
            if finished {
                break;
            }
        }
        results
    }

    pub fn snapshot(&self) -> MatchSnapshot {
        self.match_state.snapshot()
    }

    pub fn apply_command(&mut self, cmd: MatchCommand) -> Result<(), String> {
        self.match_state.apply_command(cmd)
    }

    pub fn is_finished(&self) -> bool {
        self.match_state.is_finished()
    }

    fn apply_ai_decisions(&mut self) {
        // AI for home team (if not user-controlled)
        if self.user_side != Some(Side::Home) {
            let cmds = ai::ai_decide(&self.match_state, Side::Home, &self.ai_home, &mut self.rng);
            for cmd in cmds {
                let _ = self.match_state.apply_command(cmd);
            }
        }

        // AI for away team (if not user-controlled)
        if self.user_side != Some(Side::Away) {
            let cmds = ai::ai_decide(&self.match_state, Side::Away, &self.ai_away, &mut self.rng);
            for cmd in cmds {
                let _ = self.match_state.apply_command(cmd);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: build a LiveMatchSession from the Game state
// ---------------------------------------------------------------------------

/// Create a live match session for a specific fixture.
pub fn create_live_match(
    game: &Game,
    fixture_index: usize,
    mode: MatchMode,
    allows_extra_time: bool,
) -> Result<LiveMatchSession, String> {
    let league = game.league.as_ref().ok_or(LIVE_MATCH_NO_LEAGUE_ERROR)?;
    let fixture = league
        .fixtures
        .get(fixture_index)
        .ok_or(LIVE_MATCH_FIXTURE_NOT_FOUND_ERROR)?;

    let home_team_id = fixture.home_team_id.clone();
    let away_team_id = fixture.away_team_id.clone();

    // Build engine TeamData (starting XI = first 11 players by position)
    let (home_xi, home_bench) = build_team_with_bench(game, &home_team_id);
    let (away_xi, away_bench) = build_team_with_bench(game, &away_team_id);
    let home_starter_ids = home_xi
        .players
        .iter()
        .map(|player| player.id.clone())
        .collect::<Vec<_>>();
    let away_starter_ids = away_xi
        .players
        .iter()
        .map(|player| player.id.clone())
        .collect::<Vec<_>>();
    let home_match_roles = game
        .teams
        .iter()
        .find(|team| team.id == home_team_id)
        .map(|team| team.match_roles.clone())
        .unwrap_or_default();
    let away_match_roles = game
        .teams
        .iter()
        .find(|team| team.id == away_team_id)
        .map(|team| team.match_roles.clone())
        .unwrap_or_default();
    let home_auto_selection = auto_select_set_pieces(game, &home_starter_ids);
    let away_auto_selection = auto_select_set_pieces(game, &away_starter_ids);

    let config = MatchConfig::default();

    let mut match_state = LiveMatchState::new(
        home_xi,
        away_xi,
        config,
        home_bench,
        away_bench,
        allows_extra_time,
    );
    apply_saved_match_roles(
        &mut match_state,
        Side::Home,
        &home_match_roles,
        &home_starter_ids,
        home_auto_selection,
    );
    apply_saved_match_roles(
        &mut match_state,
        Side::Away,
        &away_match_roles,
        &away_starter_ids,
        away_auto_selection,
    );

    // Determine user side
    let user_side = game.manager.team_id.as_ref().and_then(|tid| {
        if *tid == home_team_id {
            Some(Side::Home)
        } else if *tid == away_team_id {
            Some(Side::Away)
        } else {
            None
        }
    });

    // Build AI profiles from team reputation
    let home_rep = game
        .teams
        .iter()
        .find(|t| t.id == home_team_id)
        .map(|t| t.reputation)
        .unwrap_or(500);
    let away_rep = game
        .teams
        .iter()
        .find(|t| t.id == away_team_id)
        .map(|t| t.reputation)
        .unwrap_or(500);

    let ai_home = AiProfile {
        reputation: home_rep,
        experience: (home_rep / 10).min(100) as u8,
        personality: derive_personality(home_rep, manager_for_team(game, &home_team_id)),
    };
    let ai_away = AiProfile {
        reputation: away_rep,
        experience: (away_rep / 10).min(100) as u8,
        personality: derive_personality(away_rep, manager_for_team(game, &away_team_id)),
    };

    Ok(LiveMatchSession {
        match_state,
        rng: StdRng::from_rng(&mut rand::rng()),
        mode,
        fixture_index,
        competition_id: league.id.clone(),
        round_matchday: fixture.matchday,
        round_previous_standings: league.standings.clone(),
        home_team_id,
        away_team_id,
        user_side,
        ai_home,
        ai_away,
    })
}

fn manager_for_team<'a>(game: &'a Game, team_id: &str) -> Option<&'a Manager> {
    let manager_id = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .and_then(|team| team.manager_id.as_deref())?;

    game.managers
        .iter()
        .find(|manager| manager.id == manager_id)
        .or_else(|| (game.manager.id == manager_id).then_some(&game.manager))
}

/// Derive an AI personality from reputation and career statistics.
/// - Visionary: high reputation (700+) with substantial matches managed (50+)
/// - Reactive: moderate reputation with a winning record (win rate ≥ 55 %)
/// - Pragmatist: default
fn derive_personality(rep: u32, manager: Option<&Manager>) -> AiPersonality {
    if let Some(manager) = manager {
        let stats = &manager.career_stats;
        let total = stats.matches_managed;
        if rep >= 700 && total >= 50 {
            return AiPersonality::Visionary;
        }
        if total >= 20 {
            let win_rate = stats.wins as f64 / total as f64;
            if win_rate >= 0.55 {
                return AiPersonality::Reactive;
            }
        }
    }

    if rep >= 800 {
        return AiPersonality::Visionary;
    }

    AiPersonality::Pragmatist
}
