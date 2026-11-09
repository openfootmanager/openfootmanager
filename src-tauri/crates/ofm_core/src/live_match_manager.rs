// Squad selection and the domain → engine conversion live in `turn::squad`,
// which is where the crate's boundary rule puts them: `engine` has no knowledge
// of `domain`, and `ofm_core/turn/` is the single bridge between the two. The
// live path and the instant path both build their sides from there, so there is
// one answer to "who is playing" rather than one per code path.
use crate::custom_tactics::INVALID_FORMATION_ERROR;
use crate::player_rating::{formation_slots, parse_formation};
use crate::turn::matchday_lookups::MatchdayLookups;
pub use crate::turn::squad::auto_select_set_pieces;
use crate::turn::squad::{
    admitted_roles_for_formation, build_team_with_bench_from, domain_to_engine_role,
    domain_to_engine_tactics, engine_line, line_familiarity, set_piece_takers_among,
    slot_admits_engine_role,
};

use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::game::Game;

use domain::league::{FixtureStatus, StandingEntry};
use domain::manager::Manager;
use domain::player::{Player, Position as DomainPosition};
use domain::team::{MatchRoles, PlayerRole as DomainPlayerRole};
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
const NOT_YOUR_SIDE: &str = "be.error.liveMatch.notYourSide";

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
    /// The career this match belongs to (`Game::career_token` when it was started), so a screen's
    /// step, command or finish meant for another career is refused instead of landing here.
    pub career_token: u64,
    /// Both clubs' players as the game knows them, by id, so a swap before kick-off can work
    /// out what the newcomer's slot asks of him (`refit_pre_match_swap`).
    squad_players: HashMap<String, Player>,
    /// The roles the user's team has saved, by player, as the session last saw them: seeded when
    /// the match is built and kept current by `follow_saved_role_before_kick_off` (a role saved
    /// for one player) and `follow_saved_roles_before_kick_off` (a save that rewrites them all:
    /// a formation, an eleven, a tactic), so it is exact until the match starts. A
    /// pre-match swap reads it to give the newcomer the role his new slot admits; the game's
    /// roles are not at hand there, and taking the game lock under the live-match lock is the
    /// nesting this codebase avoids.
    saved_roles: HashMap<String, DomainPlayerRole>,
    /// The role intended for each player, including a session choice temporarily suppressed by a slot.
    intended_roles: HashMap<String, engine::PlayerRole>,
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

    /// The rest of a match nobody is left to manage, played out at once: the AI takes the
    /// user's dugout too, as it does for a delegated match. For a caller that finishes a live
    /// match early (MCP `match_finish`): the user is gone, and a side the AI skips would play
    /// the rest of it with no substitutions and no replacement keeper (#1231).
    pub fn play_out_unmanaged(&mut self) -> Vec<MinuteResult> {
        self.user_side = None;
        self.run_to_completion()
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

    /// A manager's order for his own team. Every command names the side it is for; one for the
    /// other side, or for a match the user is not playing in (`user_side` None), is refused and
    /// changes nothing. The screens only ever build commands for the user's side, but an agent
    /// sends its own, and one naming the opponent took off their best players, made their keeper
    /// the penalty taker and rewrote their tactics (#1192). The AI picks the other side's orders
    /// (`apply_ai_decisions`), which do not come through here.
    pub fn apply_command(&mut self, cmd: MatchCommand) -> Result<(), String> {
        if self.user_side != Some(cmd.side()) {
            let refusal = match cmd {
                MatchCommand::SetShootoutOrder { .. } => SHOOTOUT_ORDER_NOT_YOUR_SIDE,
                _ => NOT_YOUR_SIDE,
            };
            return Err(refusal.into());
        }
        if let MatchCommand::PreMatchSwap {
            side,
            player_off_id,
            player_on_id,
        } = &cmd
        {
            let (side, off, on) = (*side, player_off_id.clone(), player_on_id.clone());
            let slot = self.slot_of(side, &off);
            self.match_state.apply_command(cmd)?;
            self.refit_pre_match_swap(side, &off, &on, slot.as_ref());
            return Ok(());
        }
        if let MatchCommand::ChangeFormation { side, formation } = &cmd {
            let side = *side;
            // The engine reads any string leniently ("4-5-1-x" plays 4, 5 and 1) while the
            // slots are laid out by `parse_formation`; only an agent can send such a shape.
            if parse_formation(formation).is_none() {
                return Err(INVALID_FORMATION_ERROR.to_string());
            }
            self.match_state.apply_command(cmd)?;
            self.sync_slot_roles(side);
            self.normalize_formation_roles(side);
            self.refit_formation_change(side);
            return Ok(());
        }
        let role_change = match &cmd {
            MatchCommand::ChangePlayerRole {
                player_id, role, ..
            } => Some((player_id.clone(), *role)),
            _ => None,
        };
        self.match_state.apply_command(cmd)?;
        if let Some((player_id, role)) = role_change {
            self.intended_roles.insert(player_id, role);
        }
        Ok(())
    }

    /// The formation slot `player_id` fills in `side`'s eleven: entry i plays slot i.
    fn slot_of(&self, side: Side, player_id: &str) -> Option<DomainPosition> {
        let team = self.match_state.team(side);
        let index = team.players.iter().position(|p| p.id == player_id)?;
        formation_slots(&team.formation).get(index).cloned()
    }

    /// After a swap before kick-off, the newcomer plays at the familiarity of the slot he took,
    /// and the player he replaced, now on the bench, at his best in each line, as any bench
    /// player does. The engine cannot work either out: it knows lines, not slots. A change of
    /// formation moves starters between slots too: `refit_formation_change`.
    fn refit_pre_match_swap(
        &mut self,
        side: Side,
        off: &str,
        on: &str,
        slot: Option<&DomainPosition>,
    ) {
        for (player_id, slot) in [(on, slot), (off, None)] {
            let Some(player) = self.squad_players.get(player_id) else {
                continue;
            };
            let familiarity = line_familiarity(player, slot);
            if !self
                .match_state
                .set_line_familiarity(side, player_id, familiarity)
            {
                log::warn!("[live_match] pre-match swap: {player_id} is not in the match");
            }
        }
        if let Some(player) = self.squad_players.get(off) {
            self.match_state
                .set_bench_position(side, off, engine_line(&player.natural_position));
        }
        self.follow_saved_role_on_the_bench(side, off);
        if let Some(slot) = slot {
            self.refit_swapped_in_role(side, on, slot);
        }
    }

    /// The newcomer plays the role his new slot admits: the team's saved role for him if it does,
    /// else Standard, not the role his bench copy carried from a slot or line he no longer fills.
    fn refit_swapped_in_role(&mut self, side: Side, on: &str, slot: &DomainPosition) {
        let Some(newcomer) = self
            .match_state
            .team(side)
            .players
            .iter()
            .find(|player| player.id == on)
        else {
            return;
        };
        let role = role_at(self.saved_roles.get(on), slot);
        if role == newcomer.role {
            return;
        }
        if let Err(error) = self
            .match_state
            .apply_command(MatchCommand::ChangePlayerRole {
                side,
                player_id: on.to_string(),
                role,
            })
        {
            log::warn!("[live_match] pre-match swap: {on} did not take his role: {error}");
        }
    }

    /// After a change of shape before kick-off, every starter plays at the familiarity of the
    /// slot he now stands in: the engine regroups the eleven so that entry i plays slot i of the
    /// new formation, and cannot tell which slot within a line that is. A match in play keeps
    /// the familiarity each starter kicked off with (`PlayerData::line_familiarity` is his
    /// kick-off slot's), as a substitute keeps the line he came on in.
    fn refit_formation_change(&mut self, side: Side) {
        if self.match_state.phase() != MatchPhase::PreKickOff {
            return;
        }
        let team = self.match_state.team(side);
        let starters: Vec<(String, DomainPosition)> = team
            .players
            .iter()
            .zip(formation_slots(&team.formation))
            .map(|(player, slot)| (player.id.clone(), slot))
            .collect();
        for (player_id, slot) in starters {
            let Some(player) = self.squad_players.get(&player_id) else {
                continue;
            };
            let familiarity = line_familiarity(player, Some(&slot));
            if !self
                .match_state
                .set_line_familiarity(side, &player_id, familiarity)
            {
                log::warn!("[live_match] change of shape: {player_id} is not in the match");
            }
        }
    }

    /// Apply the intended role only while the player's current slot admits it.
    fn normalize_formation_roles(&mut self, side: Side) {
        let team = self.match_state.team(side);
        let roles: Vec<_> = team
            .players
            .iter()
            .zip(formation_slots(&team.formation))
            .map(|(player, slot)| {
                let slot = if player.position == engine::Position::Goalkeeper {
                    DomainPosition::Goalkeeper
                } else {
                    slot
                };
                let intended = self
                    .intended_roles
                    .get(&player.id)
                    .copied()
                    .unwrap_or(player.role);
                let effective = if slot_admits_engine_role(&slot, intended) {
                    intended
                } else {
                    engine::PlayerRole::Standard
                };
                (player.id.clone(), player.role, effective)
            })
            .collect();
        for (player_id, current, effective) in roles {
            if current != effective
                && let Err(error) = self
                    .match_state
                    .apply_command(MatchCommand::ChangePlayerRole {
                        side,
                        player_id: player_id.clone(),
                        role: effective,
                    })
            {
                log::warn!("[live_match] formation role for {player_id} was refused: {error}");
            }
        }
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

    /// The session is built on match day, before the pre-match screen opens, so dials saved
    /// there are handed to the match too. Once it has kicked off, the live commands change them
    /// instead. Only the dials: a role changed in the session is left alone.
    pub fn follow_saved_dials_before_kick_off(&mut self, game: &Game) -> Result<(), String> {
        let Some((side, team)) = self.user_team_before_kick_off(game) else {
            return Ok(());
        };
        for dial in domain_to_engine_tactics(&team.tactics_phase).dials() {
            self.match_state
                .apply_command(MatchCommand::ChangeTacticalDial { side, dial })?;
        }
        Ok(())
    }

    /// Every role the user's team has saved, handed to the match as a whole: for a save that
    /// rewrites them (a formation, a starting eleven or a tactic reconciles them and can drop a
    /// role). A player whose role is gone returns to `Standard`, and the session stops
    /// remembering it, so a later swap does not bring it back. The same rule as for one player.
    pub fn follow_saved_roles_before_kick_off(&mut self, game: &Game) -> Result<(), String> {
        let Some((_, team)) = self.user_team_before_kick_off(game) else {
            return Ok(());
        };
        let players: HashSet<String> = self
            .saved_roles
            .keys()
            .chain(team.player_roles.keys())
            .cloned()
            .collect();
        for player_id in players {
            self.follow_saved_role_before_kick_off(game, &player_id)?;
        }
        Ok(())
    }

    /// The role saved for one player before kick-off, handed to the match if he starts. It
    /// counts only where the slot he fills in the match admits it, the rule the match was built
    /// by (`turn::squad`): a swap or a change of shape before kick-off moves players in the
    /// session alone, so the saved team's slots no longer describe it. A role either refuses
    /// leaves him on `Standard`. The engine refuses nothing that passed those checks; if it ever
    /// did, the caller logs the error and does not report the save as failed.
    pub fn follow_saved_role_before_kick_off(
        &mut self,
        game: &Game,
        player_id: &str,
    ) -> Result<(), String> {
        let Some((side, team)) = self.user_team_before_kick_off(game) else {
            return Ok(());
        };
        match team.player_roles.get(player_id) {
            Some(role) => {
                self.saved_roles.insert(player_id.to_string(), role.clone());
                self.intended_roles
                    .insert(player_id.to_string(), domain_to_engine_role(role));
            }
            None => {
                self.saved_roles.remove(player_id);
                self.intended_roles
                    .insert(player_id.to_string(), engine::PlayerRole::Standard);
            }
        }
        let Some(slot) = self.slot_of(side, player_id) else {
            self.follow_saved_role_on_the_bench(side, player_id);
            return Ok(());
        };
        let Some(starter) = self
            .match_state
            .team(side)
            .players
            .iter()
            .find(|player| player.id == player_id)
        else {
            return Ok(());
        };
        let role = role_at(self.saved_roles.get(player_id), &slot);
        if role == starter.role {
            return Ok(());
        }
        self.match_state
            .apply_command(MatchCommand::ChangePlayerRole {
                side,
                player_id: player_id.to_string(),
                role,
            })
    }

    /// A benched player's saved role is the one he comes on with, if his natural position admits
    /// it and his line can take it: the bench copy was built before the role was saved.
    fn follow_saved_role_on_the_bench(&mut self, side: Side, player_id: &str) {
        let Some(player) = self.squad_players.get(player_id) else {
            return;
        };
        let role = role_at(self.saved_roles.get(player_id), &player.natural_position);
        self.match_state.set_bench_role(side, player_id, role);
    }

    /// The slot the user's `player_id` fills in the match still waiting at kick-off, with the
    /// career the match belongs to: the position a role saved for him is judged at, which a
    /// pre-match swap can have changed from the saved team's. A bench player is judged at his
    /// natural position. `None` if absent, in play, or not the user's match. The caller reads this
    /// before it takes the game lock, never inside it.
    pub fn user_slot_before_kick_off(&self, player_id: &str) -> Option<(u64, DomainPosition)> {
        if self.match_state.phase() != MatchPhase::PreKickOff {
            return None;
        }
        let side = self.user_side?;
        let position = self.slot_of(side, player_id).or_else(|| {
            self.match_state
                .bench(side)
                .iter()
                .find(|p| p.id == player_id)?;
            Some(self.squad_players.get(player_id)?.natural_position.clone())
        })?;
        Some((self.career_token, position))
    }

    /// The user's side and saved team, while this career's match still waits at kick-off.
    fn user_team_before_kick_off<'g>(
        &self,
        game: &'g Game,
    ) -> Option<(Side, &'g domain::team::Team)> {
        if self.match_state.phase() != MatchPhase::PreKickOff
            || self.career_token != game.career_token
        {
            return None;
        }
        let side = self.user_side?;
        let team_id = match side {
            Side::Home => &self.home_team_id,
            Side::Away => &self.away_team_id,
        };
        game.teams
            .iter()
            .find(|team| &team.id == team_id)
            .map(|team| (side, team))
    }

    fn apply_ai_decisions(&mut self) {
        // AI for home team (if not user-controlled)
        if self.user_side != Some(Side::Home) {
            let cmds = ai::ai_decide(&self.match_state, Side::Home, &self.ai_home, &mut self.rng);
            for cmd in cmds {
                self.apply_ai_command(Side::Home, cmd);
            }
        }

        // AI for away team (if not user-controlled)
        if self.user_side != Some(Side::Away) {
            let cmds = ai::ai_decide(&self.match_state, Side::Away, &self.ai_away, &mut self.rng);
            for cmd in cmds {
                self.apply_ai_command(Side::Away, cmd);
            }
        }
    }

    fn apply_ai_command(&mut self, side: Side, cmd: MatchCommand) {
        let formation_changed = matches!(cmd, MatchCommand::ChangeFormation { .. });
        match self.match_state.apply_command(cmd) {
            Ok(()) if formation_changed => {
                self.sync_slot_roles(side);
                self.normalize_formation_roles(side);
            }
            Ok(()) => {}
            Err(error) => log::warn!("[live_match] AI command failed: {error}"),
        }
    }

    fn sync_slot_roles(&mut self, side: Side) {
        let roles = admitted_roles_for_formation(&self.match_state.team(side).formation);
        self.match_state.set_slot_roles(side, roles);
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

/// The roles the user's team has saved, for the session to remember (`LiveMatchSession::saved_roles`).
fn saved_roles_of(
    game: &Game,
    user_side: Option<Side>,
    home_team_id: &str,
    away_team_id: &str,
) -> HashMap<String, DomainPlayerRole> {
    let Some(side) = user_side else {
        return HashMap::new();
    };
    let team_id = match side {
        Side::Home => home_team_id,
        Side::Away => away_team_id,
    };
    game.teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.player_roles.clone())
        .unwrap_or_default()
}

/// A club's players, found through the matchday lookups when the caller has them.
fn club_players_of<'g>(
    game: &'g Game,
    lookups: Option<&MatchdayLookups>,
    team_id: &str,
) -> Vec<&'g Player> {
    match lookups {
        Some(lookups) => lookups
            .club(game, team_id)
            .iter()
            .map(|&position| &game.players[position])
            .collect(),
        None => crate::match_day_eligibility::club_players(&game.players, team_id),
    }
}

/// The captain and set-piece takers the club has saved.
fn match_roles_of(game: &Game, team_id: &str) -> MatchRoles {
    game.teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.match_roles.clone())
        .unwrap_or_default()
}

/// The side the user manages. Only in a live match: watched as a spectator, or played instantly,
/// nobody would manage it, the screen turns the user's controls off and the AI skips the user's
/// side, so the AI manages both dugouts, as it does for a delegated match (#1211).
fn user_side_of(
    game: &Game,
    mode: MatchMode,
    home_team_id: &str,
    away_team_id: &str,
) -> Option<Side> {
    if mode != MatchMode::Live {
        return None;
    }
    let managed = game.manager.team_id.as_deref()?;
    if managed == home_team_id {
        Some(Side::Home)
    } else if managed == away_team_id {
        Some(Side::Away)
    } else {
        None
    }
}

/// The AI's profile of a club, from its reputation.
fn ai_profile_of(game: &Game, team_id: &str) -> AiProfile {
    let reputation = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.reputation)
        .unwrap_or(500);
    AiProfile {
        reputation,
        experience: (reputation / 10).min(100) as u8,
        personality: derive_personality(reputation, game.manager_of_club(team_id)),
    }
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
    let (home_club, away_club) = (
        club_players_of(game, lookups, &home_team_id),
        club_players_of(game, lookups, &away_team_id),
    );
    let squad_players = home_club
        .iter()
        .chain(&away_club)
        .map(|player| (player.id.clone(), (*player).clone()))
        .collect();
    let (home_xi, home_bench) =
        build_team_with_bench_from(game, &home_team_id, &home_club, lookups);
    let (away_xi, away_bench) =
        build_team_with_bench_from(game, &away_team_id, &away_club, lookups);
    let home_starter_ids = starter_ids(&home_xi);
    let away_starter_ids = starter_ids(&away_xi);
    let home_auto_selection = set_piece_takers_among(&starters_among(&home_xi, &home_club));
    let away_auto_selection = set_piece_takers_among(&starters_among(&away_xi, &away_club));

    if home_xi.players.is_empty() || away_xi.players.is_empty() {
        return Err(LIVE_MATCH_EMPTY_SQUAD_ERROR.to_string());
    }

    let mut match_state = LiveMatchState::new(
        home_xi,
        away_xi,
        MatchConfig::default(),
        home_bench,
        away_bench,
        allows_extra_time,
    );
    for side in [Side::Home, Side::Away] {
        let roles = admitted_roles_for_formation(&match_state.team(side).formation);
        match_state.set_slot_roles(side, roles);
    }
    apply_saved_match_roles(
        &mut match_state,
        Side::Home,
        &match_roles_of(game, &home_team_id),
        &home_starter_ids,
        home_auto_selection,
    );
    apply_saved_match_roles(
        &mut match_state,
        Side::Away,
        &match_roles_of(game, &away_team_id),
        &away_starter_ids,
        away_auto_selection,
    );

    // The user manages their side only in a live match. Watched as a spectator, or played
    // instantly, nobody else would: the screen turns the user's controls off and the AI skips
    // the user's side, so the AI manages both dugouts, as it does for a delegated match (#1211).
    let user_side = user_side_of(game, mode, &home_team_id, &away_team_id);

    let (ai_home, ai_away) = (
        ai_profile_of(game, &home_team_id),
        ai_profile_of(game, &away_team_id),
    );

    let saved_roles = saved_roles_of(game, user_side, &home_team_id, &away_team_id);
    let intended_roles = game
        .teams
        .iter()
        .filter(|team| team.id == home_team_id || team.id == away_team_id)
        .flat_map(|team| team.player_roles.iter())
        .map(|(id, role)| (id.clone(), domain_to_engine_role(role)))
        .collect();

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
        career_token: game.career_token,
        squad_players,
        saved_roles,
        intended_roles,
    })
}

/// The role a player plays when he is judged at `position`: the one the team saved for him if
/// that position admits it, else Standard. This is the match construction rule (`turn::squad`);
/// on the bench, the position is his natural one.
fn role_at(saved: Option<&DomainPlayerRole>, position: &DomainPosition) -> engine::PlayerRole {
    saved
        .filter(|role| position.admits_role(role))
        .map(domain_to_engine_role)
        .unwrap_or(engine::PlayerRole::Standard)
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
    let league_round_context = session.league_round_context.clone();
    session.run_to_completion();

    Ok(UnwatchedFixture {
        home_team_id: session.home_team_id.clone(),
        away_team_id: session.away_team_id.clone(),
        report: session.match_state.into_report(),
        league_round_context,
    })
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
    fn formation_role_fixture() -> super::LiveMatchSession {
        use domain::player::{Player, PlayerAttributes, Position};
        use domain::team::{PlayerRole, Team};
        let attrs: PlayerAttributes = serde_json::from_value(serde_json::json!({
            "pace": 65, "stamina": 65, "strength": 65, "agility": 65, "passing": 65,
            "shooting": 65, "tackling": 65, "dribbling": 65, "defending": 65,
            "positioning": 65, "vision": 65, "decisions": 65, "composure": 65,
            "aggression": 65, "teamwork": 65, "leadership": 65, "handling": 65,
            "reflexes": 65, "aerial": 65
        }))
        .unwrap();
        let positions = [
            Position::Goalkeeper,
            Position::Defender,
            Position::Defender,
            Position::Defender,
            Position::Defender,
            Position::Midfielder,
            Position::Midfielder,
            Position::Midfielder,
            Position::Midfielder,
            Position::Forward,
            Position::Forward,
        ];
        let mut teams = Vec::new();
        let mut players = Vec::new();
        for id in ["home", "away"] {
            let mut team = Team::new(
                id.into(),
                id.into(),
                id.into(),
                "GB".into(),
                "City".into(),
                "Ground".into(),
                10000,
            );
            for (index, position) in positions.iter().enumerate() {
                let player_id = format!("{id}-{index}");
                let mut player = Player::new(
                    player_id.clone(),
                    player_id.clone(),
                    player_id.clone(),
                    "1995-01-01".into(),
                    "GB".into(),
                    position.clone(),
                    attrs.clone(),
                );
                player.team_id = Some(id.into());
                if id == "away" && index == 6 {
                    player.attributes.pace = 99;
                    player.attributes.dribbling = 99;
                    player.attributes.shooting = 99;
                }
                team.starting_xi_ids.push(player_id);
                players.push(player);
            }
            teams.push(team);
        }
        teams[1]
            .player_roles
            .insert("away-4".into(), PlayerRole::AttackingFB);
        teams[1]
            .player_roles
            .insert("away-6".into(), PlayerRole::BoxToBox);
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".into(),
            "Test".into(),
            "Manager".into(),
            "1980-01-01".into(),
            "GB".into(),
        );
        manager.hire("away".into());
        let mut game = Game::new(clock, manager, teams, players, vec![], vec![]);
        game.league = Some(League {
            fixtures: vec![Fixture {
                id: "fixture".into(),
                home_team_id: "home".into(),
                away_team_id: "away".into(),
                status: FixtureStatus::Scheduled,
                ..Default::default()
            }],
            ..Default::default()
        });
        let mut session = create_live_match(&game, 0, MatchMode::Live, false).unwrap();
        session.user_side = Some(engine::Side::Home);
        session
    }

    /// Given an AI full-back after kick-off, when the AI changes to a back five, then his
    /// centre-back slot suppresses his full-back role.
    #[test]
    fn an_ai_formation_change_during_play_normalises_a_starters_role() {
        use engine::{MatchCommand, Side};
        let mut session = formation_role_fixture();
        let role = |session: &super::LiveMatchSession| {
            session
                .match_state
                .team(Side::Away)
                .players
                .iter()
                .find(|p| p.id == "away-4")
                .unwrap()
                .role
        };
        assert_eq!(role(&session), engine::PlayerRole::AttackingFB);
        session.step();
        assert_ne!(session.match_state.phase(), engine::MatchPhase::PreKickOff);
        session.apply_ai_command(
            Side::Away,
            MatchCommand::ChangeFormation {
                side: Side::Away,
                formation: "5-3-2".into(),
            },
        );
        assert_eq!(role(&session), engine::PlayerRole::Standard);
        session.apply_ai_command(
            Side::Away,
            MatchCommand::ChangeFormation {
                side: Side::Away,
                formation: "4-4-2".into(),
            },
        );
        assert_eq!(role(&session), engine::PlayerRole::AttackingFB);
    }
    /// Given an AI midfielder with a role invalid in the forward slot, when its real decision
    /// loop changes shape at the hour, then the new slot admits his effective role.
    #[test]
    fn an_ai_decision_loop_normalises_the_role_after_a_formation_change() {
        use engine::ai::AiPersonality;
        use engine::{PlayerRole, Position, Side};
        let mut session = formation_role_fixture();
        session.ai_away.personality = AiPersonality::Visionary;
        session.ai_away.experience = 100;
        for _ in 0..130 {
            if session.match_state.minute() >= 60 {
                break;
            }
            session.match_state.step_minute(&mut session.rng);
        }
        assert_eq!(session.match_state.minute(), 60);
        session.match_state.test_set_score(1, 0);
        let before = session
            .match_state
            .team(Side::Away)
            .players
            .iter()
            .find(|p| p.id == "away-6")
            .unwrap();
        assert_eq!(before.role, PlayerRole::BoxToBox);
        assert_eq!(before.position, Position::Midfielder);
        session.apply_ai_decisions();
        let team = session.match_state.team(Side::Away);
        assert_eq!(team.formation, "4-3-3");
        let index = team.players.iter().position(|p| p.id == "away-6").unwrap();
        assert_eq!(team.players[index].position, Position::Forward);
        let slot = crate::player_rating::formation_slots(&team.formation)[index].clone();
        assert!(!slot.admits_role(&domain::team::PlayerRole::BoxToBox));
        assert_eq!(team.players[index].role, PlayerRole::Standard);
        assert!(slot.admits_role(&domain::team::PlayerRole::Standard));
    }
}
