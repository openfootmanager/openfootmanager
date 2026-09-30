//! The manager on the touchline.
//!
//! Consulted once a minute for every side the player is not managing. What
//! follows is a model of when a manager looks up from the game and what he does
//! about what he sees — not a search, not a policy learned from anything.
//!
//! # Checkpoints, not a lottery
//!
//! This used to be a per-minute dice roll: about a 1% chance a minute of
//! noticing anything at all, scaled by experience. Two goals down with fifteen
//! minutes left, that is roughly a one-in-four chance of the manager ever
//! reacting — and when he did react, the minute he reacted in was noise. It is
//! also why the answer to "why did that side never make a substitution?" was
//! always "it rolled badly", which is not an answer a supporter accepts.
//!
//! A manager here looks up at a few well-known moments — the interval, the hour,
//! seventy, eighty — and immediately when something happens that will not wait.
//! What he decides at those moments is then a function of the match rather than
//! of the roll. Randomness is left in exactly one place: which of two players
//! who are equally spent comes off, where a less experienced manager takes the
//! wrong one off more often. It never decides *whether* he acts.
//!
//! # Why this cannot oscillate
//!
//! Nothing here ever issues a command to undo an earlier one. A target is only
//! produced by a position that calls for it, and a position that has stopped
//! calling for it produces no target rather than the opposite one — so a side
//! that dropped deep under pressure stays deep when the pressure lifts instead
//! of flapping between shapes every time it takes stock. That is the hysteresis,
//! and it is a property of the shape of the code rather than a timer.

use rand::{Rng, RngExt};

use crate::live_match::{AiObservation, LiveMatchState, MatchCommand, MatchPhase};
use crate::types::{
    BreakSpeed, CounterPressDuration, DefensiveLine, DefensiveShape, PlayStyle, PlayerData,
    PlayerRole, Position, PressingIntensity, Side, TacticalDial, TacticsConfig, TacticsPitchWidth,
};

// ---------------------------------------------------------------------------
// AiPersonality — determines decision-making style
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AiPersonality {
    /// Safe play: works to the scheduled checkpoints and changes the personnel.
    Pragmatist,
    /// Bold: reaches for a different shape before a different label, and looks
    /// up the moment a goal goes against him.
    Visionary,
    /// Reactive: takes stock every time the score changes, either way round.
    Reactive,
}

// ---------------------------------------------------------------------------
// AI Manager profile — drives decision-making style
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AiProfile {
    /// Team reputation 0–1000. Higher = more sophisticated decisions.
    pub reputation: u32,
    /// Manager experience level 0–100. Affects timing and quality of subs.
    pub experience: u8,
    /// Personality archetype derived from manager stats.
    pub personality: AiPersonality,
}

impl Default for AiProfile {
    fn default() -> Self {
        Self {
            reputation: 500,
            experience: 50,
            personality: AiPersonality::Pragmatist,
        }
    }
}

// ---------------------------------------------------------------------------
// When a manager looks up
// ---------------------------------------------------------------------------

/// The hour, and the two windows either side of it, are when substitutions get
/// made in a football match. An experienced manager works to all three; one who
/// has seen less waits longer before admitting the game is getting away.
///
/// Every manager keeps the eighty-minute look. Below that there is a position
/// nobody disagrees about, and a manager who could not see it would be a bug
/// rather than a character.
fn checkpoints(experience: u8) -> &'static [u8] {
    match experience {
        0..=39 => &[80],
        40..=69 => &[70, 80],
        _ => &[60, 70, 80],
    }
}

/// The last look before penalties. The interval in extra time is covered by the
/// phase itself, the same way half-time is.
const LAST_CHANCE: u8 = 113;

/// Is this a moment the manager is paying attention?
fn takes_stock(obs: &AiObservation<'_>, profile: &AiProfile) -> bool {
    // The interval. Every manager gets one, whatever his experience: it is the
    // one moment in a football match when nobody is playing and everybody is
    // listening. The AI used to sit both of them out entirely, because
    // `HalfTime` is a phase of its own and the phase gate only admitted play.
    if matches!(
        obs.phase,
        MatchPhase::HalfTime | MatchPhase::ExtraTimeHalfTime
    ) {
        return true;
    }

    // A sending-off rewrites the match for both sides and waits for nothing.
    if obs.dismissal_this_minute {
        return true;
    }

    // A goal is news to some managers and only confirmation to others.
    if obs.goal_this_minute && reacts_to_the_score(profile, obs) {
        return true;
    }

    checkpoints(profile.experience).contains(&obs.minute) || obs.minute == LAST_CHANCE
}

fn reacts_to_the_score(profile: &AiProfile, obs: &AiObservation<'_>) -> bool {
    match profile.personality {
        AiPersonality::Reactive => true,
        // Bold, not twitchy: he looks up when one has gone against him.
        AiPersonality::Visionary => obs.goal_diff < 0,
        // Waits for the interval or the hour, and is usually right to.
        AiPersonality::Pragmatist => false,
    }
}

// ---------------------------------------------------------------------------
// AI decision engine — called once per minute for AI-controlled sides
// ---------------------------------------------------------------------------

/// Evaluate the current match state and return any commands the AI wants to
/// execute. Should be called after each `step_minute` for AI-controlled sides.
pub fn ai_decide<R: Rng>(
    match_state: &LiveMatchState,
    side: Side,
    profile: &AiProfile,
    rng: &mut R,
) -> Vec<MatchCommand> {
    let mut commands = Vec::new();

    // One borrowed view for the whole decision. This used to be three
    // `MatchSnapshot`s — see `live_match::observation` for what that cost.
    let obs = match_state.observe(side);

    // The manager is present while the match is being played and while it is
    // paused for an interval; not before the kick-off, and not once it is over.
    match obs.phase {
        MatchPhase::FirstHalf
        | MatchPhase::SecondHalf
        | MatchPhase::ExtraTimeFirstHalf
        | MatchPhase::ExtraTimeSecondHalf
        | MatchPhase::HalfTime
        | MatchPhase::ExtraTimeHalfTime => {}
        _ => return commands,
    }

    let taking_stock = takes_stock(&obs, profile);

    if obs.subs_made < obs.max_subs
        && let Some(sub_cmd) = consider_substitution(&obs, profile, taking_stock, rng)
    {
        commands.push(sub_cmd);
    }

    if taking_stock && let Some(tactic_cmd) = consider_tactic_change(&obs, profile) {
        commands.push(tactic_cmd);
    }

    commands
}

// ---------------------------------------------------------------------------
// Substitution logic
// ---------------------------------------------------------------------------

/// One goal down is a position; two goals down is a problem. The first is worth
/// changing something for once the hour has gone, the second at any point a
/// manager looks up, the interval included.
const CHASE_ONE_GOAL_FROM: u8 = 60;
/// A lead is worth protecting once there is little enough left of the match for
/// protecting it to be the whole job.
const SEE_OUT_A_LEAD_FROM: u8 = 80;
/// However badly the game is going, somebody has to hold the line together.
const A_BACK_LINE_WORTH_KEEPING: usize = 3;
/// However well it is going, somebody has to chase the clearances.
const A_FORWARD_TO_CHASE_CLEARANCES: usize = 1;
/// Two players this close in condition are, to the eye, the same player. Inside
/// the gap a manager can be wrong about which of them to take off.
const TOO_CLOSE_TO_CALL: f64 = 10.0;

fn consider_substitution<R: Rng>(
    obs: &AiObservation<'_>,
    profile: &AiProfile,
    taking_stock: bool,
    rng: &mut R,
) -> Option<MatchCommand> {
    if obs.bench.is_empty() {
        return None;
    }

    // Nobody in goal is not a decision, it is an emergency, and it does not
    // wait for a checkpoint or for anything else on this list.
    if let Some(cmd) = put_someone_in_goal(obs, profile, rng) {
        return Some(cmd);
    }

    // Exhaustion is not a judgement call, and this branch has never been behind
    // a dice roll. Holding it back to a checkpoint would make the AI react to a
    // spent player *later* than it does today, and the condition economy was
    // measured with it firing the minute a starter crosses the line.
    if let Some(cmd) = replace_the_exhausted(obs, profile) {
        return Some(cmd);
    }

    if !taking_stock {
        return None;
    }

    if chasing(obs) {
        return chase_the_game(obs, profile, rng);
    }
    if obs.goal_diff > 0 && obs.minute >= SEE_OUT_A_LEAD_FROM {
        return protect_the_lead(obs, profile, rng);
    }

    None
}

/// Is this side behind in a way that calls for something to be done?
fn chasing(obs: &AiObservation<'_>) -> bool {
    obs.goal_diff <= -2 || (obs.goal_diff == -1 && obs.minute >= CHASE_ONE_GOAL_FROM)
}

/// The goalkeeper has been sent off. Somebody has to go in.
///
/// The AI had no branch for this at all — the exhaustion search skips
/// goalkeepers by design, and the other two are about outfield shape — so a side
/// that lost its keeper played out the match with `pick_goalkeeper` falling
/// through to whoever happened to be first in the list.
///
/// Who makes way is the same question as anywhere else, asked of a shorter list:
/// a side down to ten and without a keeper is not taking a defender off.
fn put_someone_in_goal<R: Rng>(
    obs: &AiObservation<'_>,
    profile: &AiProfile,
    rng: &mut R,
) -> Option<MatchCommand> {
    if obs
        .team
        .players
        .iter()
        .any(|p| p.position == Position::Goalkeeper && obs.available(p))
    {
        return None;
    }

    // `find_best_bench_replacement` falls back to the best player of any
    // position, which is the right answer everywhere else and useless here.
    let keeper_on = find_best_bench_replacement(obs.bench, Position::Goalkeeper, obs, None)?;
    if keeper_on.position != Position::Goalkeeper {
        return None;
    }

    let makes_way = [Position::Forward, Position::Midfielder, Position::Defender]
        .into_iter()
        .find_map(|position| {
            let candidates: Vec<&PlayerData> = obs
                .team
                .players
                .iter()
                .filter(|p| p.position == position && obs.available(p))
                .collect();
            least_missed(&candidates, obs, profile, rng)
        })?;

    Some(MatchCommand::Substitute {
        side: obs.side,
        player_off_id: makes_way.id.clone(),
        player_on_id: keeper_on.id.clone(),
    })
}

/// Take off whoever has least left to give, and replace him in kind.
fn replace_the_exhausted(obs: &AiObservation<'_>, profile: &AiProfile) -> Option<MatchCommand> {
    // Higher experience → earlier substitutions.
    let experience_factor = profile.experience as f64 / 100.0;

    let fatigue_threshold = if obs.minute >= 75 {
        55.0 - experience_factor * 10.0
    } else if obs.minute >= 60 {
        45.0 - experience_factor * 8.0
    } else {
        35.0 // only for very tired players before 60'
    };

    let mut worst: Option<(&PlayerData, f64)> = None;
    for p in &obs.team.players {
        if p.position == Position::Goalkeeper {
            continue; // Don't sub the goalkeeper for fatigue
        }
        if !obs.available(p) {
            continue;
        }
        let condition = obs.condition_of(p);
        if condition >= fatigue_threshold {
            continue;
        }
        match worst {
            None => worst = Some((p, condition)),
            Some((_, worst_condition)) if condition < worst_condition => {
                worst = Some((p, condition));
            }
            Some(_) => {}
        }
    }

    let (tired_player, _) = worst?;
    let replacement = find_best_bench_replacement(obs.bench, tired_player.position, obs, None)?;
    Some(MatchCommand::Substitute {
        side: obs.side,
        player_off_id: tired_player.id.clone(),
        player_on_id: replacement.id.clone(),
    })
}

/// Behind: a forward for someone further back, without dismantling the defence.
fn chase_the_game<R: Rng>(
    obs: &AiObservation<'_>,
    profile: &AiProfile,
    rng: &mut R,
) -> Option<MatchCommand> {
    let defenders = obs
        .team
        .players
        .iter()
        .filter(|p| p.position == Position::Defender && obs.available(p))
        .count();

    let candidates: Vec<&PlayerData> = obs
        .team
        .players
        .iter()
        .filter(|p| obs.available(p))
        .filter(|p| match p.position {
            Position::Defender => defenders > A_BACK_LINE_WORTH_KEEPING,
            Position::Midfielder => true,
            _ => false,
        })
        .collect();

    let player_off = least_missed(&candidates, obs, profile, rng)?;

    // A side that presses wants a forward who presses.
    let preferred_role =
        (obs.team.play_style == PlayStyle::HighPress).then_some(PlayerRole::PressingForward);
    let attacker_on =
        find_best_bench_replacement(obs.bench, Position::Forward, obs, preferred_role)?;

    Some(MatchCommand::Substitute {
        side: obs.side,
        player_off_id: player_off.id.clone(),
        player_on_id: attacker_on.id.clone(),
    })
}

/// Ahead: a defender for a forward, keeping someone up there to hold the ball.
fn protect_the_lead<R: Rng>(
    obs: &AiObservation<'_>,
    profile: &AiProfile,
    rng: &mut R,
) -> Option<MatchCommand> {
    let forwards: Vec<&PlayerData> = obs
        .team
        .players
        .iter()
        .filter(|p| p.position == Position::Forward && obs.available(p))
        .collect();

    if forwards.len() <= A_FORWARD_TO_CHASE_CLEARANCES {
        return None;
    }

    let player_off = least_missed(&forwards, obs, profile, rng)?;
    let defender_on = find_best_bench_replacement(obs.bench, Position::Defender, obs, None)?;

    Some(MatchCommand::Substitute {
        side: obs.side,
        player_off_id: player_off.id.clone(),
        player_on_id: defender_on.id.clone(),
    })
}

/// Which of these players will be missed least — that is, who is most spent.
///
/// The old code took `candidates.last()` and `forwards.first()`: the last
/// defender or midfielder the squad list happened to hold, and the first
/// forward. Squad order is an artefact of how the side was built, so the same
/// player came off every time and it was never the tired one.
///
/// The roll is the in-match twin of the lineup picker's misjudgement. Two
/// players within a few points of each other look identical from the touchline,
/// and a manager who has seen less football takes the wrong one off more often.
/// It decides which of two close calls he lands on, never whether he acts.
fn least_missed<'a, R: Rng>(
    candidates: &[&'a PlayerData],
    obs: &AiObservation<'_>,
    profile: &AiProfile,
    rng: &mut R,
) -> Option<&'a PlayerData> {
    let mut ranked: Vec<(&'a PlayerData, f64)> = candidates
        .iter()
        .map(|player| (*player, obs.condition_of(player)))
        .collect();
    // Ties broken on id, so the same match played twice names the same player.
    ranked.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });

    let (spent, spent_condition) = *ranked.first()?;
    let Some(&(next, next_condition)) = ranked.get(1) else {
        return Some(spent);
    };

    let misjudgement = (1.0 - profile.experience as f64 / 100.0) / 2.0;
    if next_condition - spent_condition <= TOO_CLOSE_TO_CALL
        && rng.random_range(0.0..1.0f64) < misjudgement
    {
        return Some(next);
    }
    Some(spent)
}

/// The best player on the bench for a given job.
///
/// "On the bench" is not the same as "available": a substituted player is pushed
/// back onto the same list, so every search here goes through
/// [`AiObservation::available`] rather than reading the list directly.
fn find_best_bench_replacement<'a>(
    bench: &'a [PlayerData],
    preferred_position: Position,
    obs: &AiObservation<'_>,
    preferred_role: Option<PlayerRole>,
) -> Option<&'a PlayerData> {
    // If a role preference is set, try to find a position+role match first
    if let Some(role) = preferred_role {
        let mut role_candidates: Vec<&PlayerData> = bench
            .iter()
            .filter(|p| p.position == preferred_position && p.role == role && obs.available(p))
            .collect();
        role_candidates.sort_by(|a, b| {
            b.overall()
                .partial_cmp(&a.overall())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Some(best) = role_candidates.first() {
            return Some(*best);
        }
    }

    // First try exact position match, sorted by overall
    let mut candidates: Vec<&PlayerData> = bench
        .iter()
        .filter(|p| p.position == preferred_position && obs.available(p))
        .collect();
    candidates.sort_by(|a, b| {
        b.overall()
            .partial_cmp(&a.overall())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if let Some(best) = candidates.first() {
        return Some(*best);
    }

    // Fallback: any bench player
    let mut all: Vec<&PlayerData> = bench.iter().filter(|p| obs.available(p)).collect();
    all.sort_by(|a, b| {
        b.overall()
            .partial_cmp(&a.overall())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all.first().copied()
}

// ---------------------------------------------------------------------------
// Tactical change logic
// ---------------------------------------------------------------------------

/// Most of the last ten minutes spent in our own half. Six was enough when the
/// reaction was a one-in-a-hundred roll on top of it; without the roll the
/// reading has to carry the decision by itself.
const PINNED_BACK: usize = 7;

fn consider_tactic_change(obs: &AiObservation<'_>, profile: &AiProfile) -> Option<MatchCommand> {
    // A Visionary reaches for a different shape before a different label. The
    // chain of formations ends, so this fires at most twice in a match and then
    // he changes the instructions like everyone else.
    if profile.personality == AiPersonality::Visionary
        && obs.goal_diff < 0
        && obs.minute >= CHASE_ONE_GOAL_FROM
        && let Some(formation) = bolder_formation(&obs.team.formation)
    {
        return Some(MatchCommand::ChangeFormation {
            side: obs.side,
            formation: formation.to_string(),
        });
    }

    if let Some(target) = target_play_style(obs)
        && target != obs.team.play_style
    {
        return Some(MatchCommand::ChangePlayStyle {
            side: obs.side,
            play_style: target,
        });
    }

    // Already wearing the right shirt. There are still nine dials underneath it.
    turn_a_dial(obs)
}

/// The most of the four under-priced dials (see [`under_priced_dials`]) any one
/// side may hold at once.
///
/// This is the one copy of the rule. It lives here because the engine is what
/// prices the dials, and it is public because `ofm_core::ai_tactics` rations
/// the plan a club takes *into* a match by the same rule: it converts its
/// settings to a [`TacticsConfig`] and asks, rather than keeping a second copy.
/// A manager who could turn these dials freely at eighty minutes would undo
/// between the whistles exactly what the blueprints ration between matches.
pub const MAX_UNDER_PRICED_DIALS: usize = 2;

/// How many of the four dials `--phase-sweep` found the engine pricing
/// one-sidedly a side is holding: a low or very low line, a compact shape, a
/// narrow width and a long counter-press. The engine now charges for all four,
/// but has not been shown to charge them in full; the ration can go when a
/// fresh sweep shows they trade evenly.
pub fn under_priced_dials(tactics: &TacticsConfig) -> usize {
    [
        matches!(
            tactics.defensive_line,
            DefensiveLine::VeryLow | DefensiveLine::Low
        ),
        tactics.defensive_shape == DefensiveShape::Compact,
        tactics.width == TacticsPitchWidth::Narrow,
        tactics.counter_press_duration == CounterPressDuration::Long,
    ]
    .iter()
    .filter(|held| **held)
    .count()
}

/// One instruction, shouted from the touchline.
///
/// The moves are in the order a manager would reach for them, and the first one
/// that changes anything and stays inside the ration is the one he gives.
fn turn_a_dial(obs: &AiObservation<'_>) -> Option<MatchCommand> {
    let reach_for: &[TacticalDial] = if chasing(obs) {
        // Squeeze the pitch from the other end: a higher line, quicker breaks,
        // and press them when the ball is lost. None of these is a dial the
        // engine gives away, so the ration never bites on this side of it.
        &[
            TacticalDial::DefensiveLine(DefensiveLine::High),
            TacticalDial::BreakSpeed(BreakSpeed::Fast),
            TacticalDial::PressingIntensity(PressingIntensity::Aggressive),
        ]
    } else if obs.goal_diff > 0 && obs.minute >= SEE_OUT_A_LEAD_FROM {
        // Drop, narrow the gaps, and stop chasing. The first two are rationed
        // dials, so a side already holding two of them gets the third move.
        &[
            TacticalDial::DefensiveLine(DefensiveLine::Low),
            TacticalDial::DefensiveShape(DefensiveShape::Compact),
            TacticalDial::PressingIntensity(PressingIntensity::Passive),
        ]
    } else {
        return None;
    };

    for dial in reach_for {
        let mut trial = obs.team.tactics.clone();
        dial.set_on(&mut trial);
        if trial != obs.team.tactics && under_priced_dials(&trial) <= MAX_UNDER_PRICED_DIALS {
            return Some(MatchCommand::ChangeTacticalDial {
                side: obs.side,
                dial: *dial,
            });
        }
    }

    None
}

/// How this side ought to be playing, given where the match has got to.
///
/// Only ever names a way of playing that the position calls for. There is no
/// branch returning "back to normal", which is what stops a side changing its
/// mind every time it looks up — see the note on hysteresis at the top of the
/// file.
fn target_play_style(obs: &AiObservation<'_>) -> Option<PlayStyle> {
    // Two down is two down. At the interval or at eighty, the answer is the same.
    if obs.goal_diff <= -2 {
        return Some(PlayStyle::Attacking);
    }

    // One down, with the hour gone. A side already pressing high is committed as
    // far up the pitch as this would take it.
    if obs.goal_diff == -1 && obs.minute >= CHASE_ONE_GOAL_FROM {
        return (obs.team.play_style != PlayStyle::HighPress).then_some(PlayStyle::Attacking);
    }

    if obs.goal_diff >= 1 && obs.minute >= SEE_OUT_A_LEAD_FROM {
        return Some(PlayStyle::Defensive);
    }

    // Pinned in for most of the last ten minutes with nothing to chase. A side
    // that is behind should be attacking its way out of this, not absorbing it.
    if obs.goal_diff >= 0 && obs.pressure_ticks >= PINNED_BACK {
        return Some(PlayStyle::Defensive);
    }

    None
}

/// The next shape up from this one, or nothing if there is no obvious next one.
fn bolder_formation(current: &str) -> Option<&'static str> {
    match current {
        "4-4-2" => Some("4-3-3"),
        "4-3-3" | "4-5-1" => Some("4-2-3-1"),
        _ => None,
    }
}
