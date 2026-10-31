// Squad selection and the domain → engine conversion live in `turn::squad`,
// which is where the crate's boundary rule puts them: `engine` has no knowledge
// of `domain`, and `ofm_core/turn/` is the single bridge between the two. The
// live path and the instant path both build their sides from there, so there is
// one answer to "who is playing" rather than one per code path.
use crate::turn::matchday_lookups::MatchdayLookups;
pub use crate::turn::squad::auto_select_set_pieces;
use crate::turn::squad::{build_team_with_bench_from, set_piece_takers_among};

use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::game::Game;

use domain::league::{FixtureStatus, StandingEntry};
use domain::manager::Manager;
use domain::player::Player;
use domain::team::MatchRoles;
use engine::ai::{self, AiPersonality, AiProfile};
use engine::{
    LiveMatchState, MatchCommand, MatchConfig, MatchPhase, MatchReport, MatchSnapshot,
    MinuteResult, Side, TeamData,
};

const LIVE_MATCH_NO_LEAGUE_ERROR: &str = "be.error.liveMatch.noLeague";

/// Upper bound on minutes a single match can run: 120 of extra time plus generous stoppage.
/// Used only to keep a caller-supplied step count from over-reserving.
const MAX_MATCH_MINUTES: usize = 140;

/// Phases a fast-forward has to stop at, because the manager is owed a decision:
/// a team talk at either interval, or the shootout order before penalties.
fn phase_needs_manager(phase: MatchPhase) -> bool {
    matches!(
        phase,
        MatchPhase::HalfTime | MatchPhase::ExtraTimeHalfTime | MatchPhase::PenaltyShootout
    )
}
/// Shared with [`crate::matchday`], which refuses a competition index that names no competition
/// rather than quietly playing the fixture out of whatever the legacy mirror holds. One key, so
/// the two refusals cannot drift into saying different things about the same failure.
pub(crate) const LIVE_MATCH_FIXTURE_NOT_FOUND_ERROR: &str = "be.error.liveMatch.fixtureNotFound";
const LIVE_MATCH_FIXTURE_NOT_SCHEDULED_ERROR: &str = "be.error.liveMatch.fixtureNotScheduled";
/// A side with nobody available cannot play. Refused here rather than handed to
/// the engine, which has no way to resolve a pass, a shot or a goalkeeper.
const LIVE_MATCH_EMPTY_SQUAD_ERROR: &str = "be.error.liveMatch.emptySquad";

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

/// A shootout order for a side the user does not manage, or from a user who manages neither.
const SHOOTOUT_ORDER_NOT_YOUR_SIDE: &str = "be.error.liveMatch.shootoutOrderNotYourSide";

pub struct LiveMatchSession {
    pub match_state: LiveMatchState,
    pub rng: ChaCha12Rng,
    pub mode: MatchMode,
    /// Index into the fixtures of the competition identified by
    /// `competition_id` — NOT necessarily into `game.league`, which
    /// `sync_legacy_league` resets to the user's domestic league.
    pub fixture_index: usize,
    /// Stable identity of the selected fixture, checked again before finish.
    pub fixture_id: String,
    /// Id of the competition (league or cup) this fixture belongs to; the
    /// finish path uses it to apply the report to the right competition.
    pub competition_id: String,
    /// The matchday and table of the competition *being played* — a cup, if the user is in a cup
    /// tie. Kept for callers that describe the tie itself.
    pub round_matchday: u32,
    pub round_previous_standings: Vec<StandingEntry>,
    /// The matchday and table of the user's own league, as they stood before any of today's
    /// fixtures were played, for the round digest.
    ///
    /// Two separate things, because they are two separate competitions on a cup day. The digest
    /// describes the user's league round, so it needs that league's baseline — and it has to be
    /// captured here, at session creation, because the GUI simulates the rest of the round
    /// immediately afterwards. Reading the table at finish time instead reports every delta as
    /// zero, since by then the round has been played.
    ///
    /// `None` when the user has no competition, in which case the finish path falls back to
    /// today's context.
    pub league_round_context: Option<crate::matchday::LeagueRoundContext>,
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
    ///
    /// Stops early at full time **and when the match reaches a phase that needs the manager**.
    /// Callers decide what to do from the last result, so batching straight through half time
    /// would leave the last result reading "second half" and the half-time team talk would never
    /// be offered. Stopping on the transition means a caller sees the same sequence of decision
    /// points it would have seen stepping a minute at a time.
    ///
    /// Use [`Self::run_to_completion`] to simulate a whole match without stopping.
    pub fn step_many(&mut self, count: u16) -> Vec<MinuteResult> {
        // Callers include MCP tools, where the count comes from a model and can be any u16.
        // Reserving for the request rather than for a plausible match would let `match_step(65535)`
        // allocate ~65k slots to return about 45.
        let mut results = Vec::with_capacity((count as usize).min(MAX_MATCH_MINUTES));
        let starting_phase = self.match_state.phase();
        for _ in 0..count {
            let result = self.step();
            let finished = result.is_finished;
            let needs_manager = result.phase != starting_phase && phase_needs_manager(result.phase);
            results.push(result);
            if finished || needs_manager {
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

    /// The match as the user may see it. Of the shootout orders, only the user's own side's is
    /// filled in: the opponent's stays hidden until each kick is taken, and a match between two
    /// other clubs shows neither.
    pub fn snapshot(&self) -> MatchSnapshot {
        let mut snapshot = self.match_state.snapshot();
        if let (Some(shootout), Some(side)) = (snapshot.penalty_shootout.as_mut(), self.user_side) {
            let own_order = self.match_state.shootout_order(side);
            match side {
                Side::Home => shootout.home_order = own_order,
                Side::Away => shootout.away_order = own_order,
            }
        }
        snapshot
    }

    pub fn apply_command(&mut self, cmd: MatchCommand) -> Result<(), String> {
        if let MatchCommand::SetShootoutOrder { side, .. } = &cmd
            && self.user_side != Some(*side)
        {
            return Err(SHOOTOUT_ORDER_NOT_YOUR_SIDE.into());
        }
        self.match_state.apply_command(cmd)
    }

    /// Sets the user's own side's shootout order, for a caller that does not name the side.
    pub fn set_own_shootout_order(&mut self, player_ids: Vec<String>) -> Result<(), String> {
        let side = self
            .user_side
            .ok_or_else(|| SHOOTOUT_ORDER_NOT_YOUR_SIDE.to_string())?;
        self.apply_command(MatchCommand::SetShootoutOrder { side, player_ids })
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

/// Make both sides of a fixture in `game.league` fit to kick off: a club short
/// of the squad floor signs free agents first (see
/// [`crate::squad_floor::ready_for_kick_off`]). Every path that plays a club
/// fixture goes through this before building the teams — the unwatched path
/// directly, the player's own matches through [`kick_off_live_match`] — so no
/// match starts with a side that cannot be fielded.
pub(crate) fn prepare_kick_off(
    game: &mut Game,
    fixture_index: usize,
    lookups: Option<&mut MatchdayLookups>,
) {
    let Some((home_team_id, away_team_id)) = game.league.as_ref().and_then(|league| {
        league
            .fixtures
            .get(fixture_index)
            .map(|fixture| (fixture.home_team_id.clone(), fixture.away_team_id.clone()))
    }) else {
        return;
    };
    // Counted together before either club is topped up: a top-up draws on the free agents and on
    // that club's own academy, so it cannot change the other club's seniors.
    let (home_seniors, away_seniors) = match lookups.as_deref() {
        Some(lookups) => (
            crate::squad_floor::senior_counts_among(
                game,
                &home_team_id,
                lookups.club(game, &home_team_id),
            ),
            crate::squad_floor::senior_counts_among(
                game,
                &away_team_id,
                lookups.club(game, &away_team_id),
            ),
        ),
        None => crate::squad_floor::senior_counts_of_two(game, &home_team_id, &away_team_id),
    };
    let brought_in = crate::squad_floor::ready_for_kick_off(game, &home_team_id, home_seniors);
    let brought_in =
        crate::squad_floor::ready_for_kick_off(game, &away_team_id, away_seniors) || brought_in;
    if brought_in && let Some(lookups) = lookups {
        lookups.players_moved(game);
    }
    crate::turn::squad::reconcile_user_starting_xi(game, &home_team_id);
    crate::turn::squad::reconcile_user_starting_xi(game, &away_team_id);
}

/// Kick off a fixture in `game.league` as a live session: both squads made fit
/// to play first, then the session built.
///
/// The entry point for starting a real match. [`create_live_match`] only reads
/// the game, so it cannot sign anyone; calling it directly skips the kick-off
/// top-up, which is only right for a caller that has already done it.
pub fn kick_off_live_match(
    game: &mut Game,
    fixture_index: usize,
    mode: MatchMode,
    allows_extra_time: bool,
) -> Result<LiveMatchSession, String> {
    kick_off_live_match_with(game, fixture_index, mode, allows_extra_time, None)
}

fn kick_off_live_match_with(
    game: &mut Game,
    fixture_index: usize,
    mode: MatchMode,
    allows_extra_time: bool,
    mut lookups: Option<&mut MatchdayLookups>,
) -> Result<LiveMatchSession, String> {
    prepare_kick_off(game, fixture_index, lookups.as_deref_mut());
    create_live_match_with(
        game,
        fixture_index,
        mode,
        allows_extra_time,
        lookups.as_deref(),
    )
}

fn starter_ids(team: &TeamData) -> Vec<String> {
    team.players
        .iter()
        .map(|player| player.id.clone())
        .collect()
}

/// The domain players of `team`'s starting eleven, in the team's order, found among the club's.
fn starters_among<'a>(team: &TeamData, club: &[&'a Player]) -> Vec<&'a Player> {
    team.players
        .iter()
        .filter_map(|starter| club.iter().find(|player| player.id == starter.id).copied())
        .collect()
}

/// Create a live match session for a specific fixture.
pub fn create_live_match(
    game: &Game,
    fixture_index: usize,
    mode: MatchMode,
    allows_extra_time: bool,
) -> Result<LiveMatchSession, String> {
    create_live_match_with(game, fixture_index, mode, allows_extra_time, None)
}

/// [`create_live_match`] for a matchday that has already found the clubs with a match soon
/// (`lookups`: where each club's players stand, and who plays again soon).
fn create_live_match_with(
    game: &Game,
    fixture_index: usize,
    mode: MatchMode,
    allows_extra_time: bool,
    lookups: Option<&MatchdayLookups>,
) -> Result<LiveMatchSession, String> {
    let league = game.league.as_ref().ok_or(LIVE_MATCH_NO_LEAGUE_ERROR)?;
    let fixture = league
        .fixtures
        .get(fixture_index)
        .ok_or(LIVE_MATCH_FIXTURE_NOT_FOUND_ERROR)?;
    if fixture.status != FixtureStatus::Scheduled {
        return Err(LIVE_MATCH_FIXTURE_NOT_SCHEDULED_ERROR.to_string());
    }

    let home_team_id = fixture.home_team_id.clone();
    let away_team_id = fixture.away_team_id.clone();

    // Each club's players are found once and serve its team, its bench and its set-piece takers.
    let club_of = |team_id: &str| -> Vec<&Player> {
        match lookups {
            Some(lookups) => lookups
                .club(game, team_id)
                .iter()
                .map(|&position| &game.players[position])
                .collect(),
            None => crate::match_day_eligibility::club_players(&game.players, team_id),
        }
    };
    let (home_club, away_club) = (club_of(&home_team_id), club_of(&away_team_id));
    let (home_xi, home_bench) =
        build_team_with_bench_from(game, &home_team_id, &home_club, lookups);
    let (away_xi, away_bench) =
        build_team_with_bench_from(game, &away_team_id, &away_club, lookups);
    let home_starter_ids = starter_ids(&home_xi);
    let away_starter_ids = starter_ids(&away_xi);
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
    let home_auto_selection = set_piece_takers_among(&starters_among(&home_xi, &home_club));
    let away_auto_selection = set_piece_takers_among(&starters_among(&away_xi, &away_club));

    if home_xi.players.is_empty() || away_xi.players.is_empty() {
        return Err(LIVE_MATCH_EMPTY_SQUAD_ERROR.to_string());
    }

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
        // The clubs tell this match from any other played today, as for a batch match.
        rng: game.rng_today(&format!("match/{home_team_id}/{away_team_id}")),
        mode,
        fixture_index,
        fixture_id: fixture.id.clone(),
        competition_id: league.id.clone(),
        round_matchday: fixture.matchday,
        round_previous_standings: league.standings.clone(),
        league_round_context: crate::matchday::user_league_round_context(game),
        home_team_id,
        away_team_id,
        user_side,
        ai_home,
        ai_away,
    })
}

/// What a fixture nobody watched produced, with the two clubs it belongs to so
/// the caller can apply it without re-reading the fixture.
#[derive(Debug)]
pub struct UnwatchedFixture {
    pub report: MatchReport,
    pub home_team_id: String,
    pub away_team_id: String,
    /// The user's league round as it stood before this fixture was played, for
    /// the round digest — see [`crate::matchday::user_league_round_context`].
    pub league_round_context: Option<crate::matchday::LeagueRoundContext>,
}

/// Play a fixture nobody is watching, start to finish.
///
/// The same session the player's own match runs on, with one difference: no side
/// belongs to the user. `create_live_match` reads the user's club off
/// `game.manager`, which is the right answer while the player is sitting through
/// the match and exactly the wrong one here — it would leave one dugout empty,
/// and the empty one would always be the player's. A match nobody watches has an
/// AI manager on both touchlines.
///
/// The one way a fixture in `game.league` is played without anyone watching:
/// the matchday loop plays every other fixture of the day through it, and
/// [`crate::matchday::play_user_matchday_with_capture`] the player's own when
/// they delegate. It kicks off through the squad floor's gate
/// ([`kick_off_live_match`]) and decides extra time by the one predicate,
/// [`crate::matchday::fixture_allows_extra_time`], so no caller can pass the
/// wrong answer (#601). An `Err` is a side nobody could field.
pub fn play_unwatched_fixture(
    game: &mut Game,
    fixture_index: usize,
) -> Result<UnwatchedFixture, String> {
    play_unwatched_fixture_with(game, fixture_index, None)
}

/// [`play_unwatched_fixture`] for a matchday that has already found the clubs with a match soon.
pub(crate) fn play_unwatched_fixture_with(
    game: &mut Game,
    fixture_index: usize,
    lookups: Option<&mut MatchdayLookups>,
) -> Result<UnwatchedFixture, String> {
    let allows_extra_time = crate::matchday::fixture_allows_extra_time(game, fixture_index);
    let mut session = kick_off_live_match_with(
        game,
        fixture_index,
        MatchMode::Instant,
        allows_extra_time,
        lookups,
    )?;
    session.user_side = None;
    let league_round_context = session.league_round_context.clone();
    session.run_to_completion();

    Ok(UnwatchedFixture {
        home_team_id: session.home_team_id.clone(),
        away_team_id: session.away_team_id.clone(),
        report: session.match_state.into_report(),
        league_round_context,
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

#[cfg(test)]
mod tests {
    use super::{MatchMode, create_live_match};
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::league::{Fixture, FixtureStatus, League};
    use domain::manager::Manager;

    #[test]
    fn create_live_match_refuses_completed_fixture_directly() {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "manager".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        game.league = Some(League {
            id: "league".to_string(),
            fixtures: vec![Fixture {
                id: "already-played".to_string(),
                home_team_id: "home".to_string(),
                away_team_id: "away".to_string(),
                status: FixtureStatus::Completed,
                ..Fixture::default()
            }],
            ..League::default()
        });

        let error = create_live_match(&game, 0, MatchMode::Instant, false)
            .err()
            .expect("the core entry point must reject a completed fixture");
        assert_eq!(error, "be.error.liveMatch.fixtureNotScheduled");
    }
}
