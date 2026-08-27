use rand::{Rng, RngExt};

use crate::live_match::{AiObservation, LiveMatchState, MatchCommand, MatchPhase};
use crate::types::{PlayStyle, PlayerData, PlayerRole, Position, Side};

// ---------------------------------------------------------------------------
// AiPersonality — determines decision-making style
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum AiPersonality {
    /// Safe play: subs early for fatigue, incremental style changes.
    Pragmatist,
    /// Bold: can trigger formation changes after minute 60 when losing.
    Visionary,
    /// Reactive: 1.5× base chance after any score-differential change.
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

    // AI only acts during playing phases
    match obs.phase {
        MatchPhase::FirstHalf
        | MatchPhase::SecondHalf
        | MatchPhase::ExtraTimeFirstHalf
        | MatchPhase::ExtraTimeSecondHalf => {}
        _ => return commands,
    }

    // --- Substitution decisions ---
    if obs.subs_made < obs.max_subs
        && let Some(sub_cmd) = consider_substitution(&obs, profile, rng)
    {
        commands.push(sub_cmd);
    }

    // --- Tactical adjustments ---
    if let Some(tactic_cmd) = consider_tactic_change(&obs, profile, rng) {
        commands.push(tactic_cmd);
    }

    commands
}

// ---------------------------------------------------------------------------
// Substitution logic
// ---------------------------------------------------------------------------

fn consider_substitution<R: Rng>(
    obs: &AiObservation<'_>,
    profile: &AiProfile,
    rng: &mut R,
) -> Option<MatchCommand> {
    let (side, minute, subs_made) = (obs.side, obs.minute, obs.subs_made);
    let team = obs.team;
    let bench = obs.bench;

    if bench.is_empty() {
        return None;
    }

    let goal_diff = obs.goal_diff;

    // Higher experience → earlier and smarter substitutions
    let experience_factor = profile.experience as f64 / 100.0;

    // --- Fatigue-based substitutions (after minute 55+) ---
    let fatigue_threshold = if minute >= 75 {
        55.0 - experience_factor * 10.0 // experienced managers sub earlier
    } else if minute >= 60 {
        45.0 - experience_factor * 8.0
    } else {
        35.0 // only for very tired players before 60'
    };

    // Find the most fatigued outfield player
    let mut worst_player: Option<(&PlayerData, f64)> = None;
    for p in &team.players {
        if p.position == Position::Goalkeeper {
            continue; // Don't sub the goalkeeper for fatigue
        }
        if obs.sent_off.contains(&p.id) {
            continue;
        }
        let condition = obs.condition_of(p);
        if condition < fatigue_threshold {
            match &worst_player {
                None => worst_player = Some((p, condition)),
                Some((_, worst_cond)) => {
                    if condition < *worst_cond {
                        worst_player = Some((p, condition));
                    }
                }
            }
        }
    }

    if let Some((tired_player, _)) = worst_player {
        // Find best replacement from bench with same position
        if let Some(replacement) =
            find_best_bench_replacement(bench, tired_player.position, obs.sent_off, None)
        {
            return Some(MatchCommand::Substitute {
                side,
                player_off_id: tired_player.id.clone(),
                player_on_id: replacement.id.clone(),
            });
        }
    }

    // --- Tactical substitutions (losing and past 65') ---
    if goal_diff < 0 && minute >= 65 && subs_made < 3 {
        // Bring on an attacker if losing.
        // When playing HighPress, prefer a PressingForward for role synergy.
        let chance = 0.03 * experience_factor * (1.0 + (minute as f64 - 65.0) / 25.0);
        if rng.random_range(0.0..1.0f64) < chance {
            // Find a defender or midfielder to take off
            let candidates: Vec<&PlayerData> = team
                .players
                .iter()
                .filter(|p| {
                    (p.position == Position::Defender || p.position == Position::Midfielder)
                        && !obs.sent_off.contains(&p.id)
                })
                .collect();

            let preferred_role = if team.play_style == PlayStyle::HighPress {
                Some(PlayerRole::PressingForward)
            } else {
                None
            };

            if let Some(player_off) = candidates.last()
                && let Some(attacker_on) = find_best_bench_replacement(
                    bench,
                    Position::Forward,
                    obs.sent_off,
                    preferred_role,
                )
            {
                return Some(MatchCommand::Substitute {
                    side,
                    player_off_id: player_off.id.clone(),
                    player_on_id: attacker_on.id.clone(),
                });
            }
        }
    }

    // --- Defensive substitutions (winning and past 80') ---
    if goal_diff > 0 && minute >= 80 && subs_made < 3 {
        let chance = 0.04 * experience_factor;
        if rng.random_range(0.0..1.0f64) < chance {
            // Bring on a defender
            let forwards: Vec<&PlayerData> = team
                .players
                .iter()
                .filter(|p| p.position == Position::Forward && !obs.sent_off.contains(&p.id))
                .collect();

            if let Some(player_off) = forwards.first()
                && let Some(defender_on) =
                    find_best_bench_replacement(bench, Position::Defender, obs.sent_off, None)
            {
                return Some(MatchCommand::Substitute {
                    side,
                    player_off_id: player_off.id.clone(),
                    player_on_id: defender_on.id.clone(),
                });
            }
        }
    }

    None
}

fn find_best_bench_replacement<'a>(
    bench: &'a [PlayerData],
    preferred_position: Position,
    sent_off: &std::collections::HashSet<String>,
    preferred_role: Option<PlayerRole>,
) -> Option<&'a PlayerData> {
    // If a role preference is set, try to find a position+role match first
    if let Some(role) = preferred_role {
        let mut role_candidates: Vec<&PlayerData> = bench
            .iter()
            .filter(|p| {
                p.position == preferred_position && p.role == role && !sent_off.contains(&p.id)
            })
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
        .filter(|p| p.position == preferred_position && !sent_off.contains(&p.id))
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
    let mut all: Vec<&PlayerData> = bench.iter().filter(|p| !sent_off.contains(&p.id)).collect();
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

fn consider_tactic_change<R: Rng>(
    obs: &AiObservation<'_>,
    profile: &AiProfile,
    rng: &mut R,
) -> Option<MatchCommand> {
    let (side, minute) = (obs.side, obs.minute);
    let team = obs.team;
    let goal_diff = obs.goal_diff;
    let experience_factor = profile.experience as f64 / 100.0;

    // Only consider changes after a meaningful period
    if minute < 55 {
        return None;
    }

    // Base chance per minute; Reactive personality gets a 1.5× boost
    let personality_mult = if profile.personality == AiPersonality::Reactive {
        1.5
    } else {
        1.0
    };
    let base_chance = 0.02 * experience_factor * personality_mult;

    // ---------------------------------------------------------------------------
    // Zone-reactive: if the ball has been stuck in our defensive half for most of
    // the last 10 minutes, shift to a more defensive style to soak pressure.
    // ---------------------------------------------------------------------------
    let pressure_ticks = obs.pressure_ticks;
    // 6+ of the last 10 minutes under defensive pressure → consider going defensive.
    // Guard: only when not already losing (a losing team should be attacking, not absorbing).
    if pressure_ticks >= 6
        && goal_diff >= 0
        && team.play_style != PlayStyle::Defensive
        && rng.random_range(0.0..1.0f64) < base_chance * 2.5
    {
        return Some(MatchCommand::ChangePlayStyle {
            side,
            play_style: PlayStyle::Defensive,
        });
    }

    // Losing by 2+ goals after 70': switch to attacking
    if goal_diff <= -2
        && minute >= 70
        && team.play_style != PlayStyle::Attacking
        && rng.random_range(0.0..1.0f64) < base_chance * 3.0
    {
        return Some(MatchCommand::ChangePlayStyle {
            side,
            play_style: PlayStyle::Attacking,
        });
    }

    // Losing by 1 goal after 75': consider more attacking
    if goal_diff == -1
        && minute >= 75
        && team.play_style != PlayStyle::Attacking
        && team.play_style != PlayStyle::HighPress
        && rng.random_range(0.0..1.0f64) < base_chance * 2.0
    {
        return Some(MatchCommand::ChangePlayStyle {
            side,
            play_style: PlayStyle::Attacking,
        });
    }

    // Winning by 1+ goals after 80': switch to defensive
    if goal_diff >= 1
        && minute >= 80
        && team.play_style != PlayStyle::Defensive
        && rng.random_range(0.0..1.0f64) < base_chance * 2.0
    {
        return Some(MatchCommand::ChangePlayStyle {
            side,
            play_style: PlayStyle::Defensive,
        });
    }

    // Winning by 2+ goals after 85': very defensive / time wasting
    if goal_diff >= 2
        && minute >= 85
        && team.play_style != PlayStyle::Defensive
        && rng.random_range(0.0..1.0f64) < base_chance * 4.0
    {
        return Some(MatchCommand::ChangePlayStyle {
            side,
            play_style: PlayStyle::Defensive,
        });
    }

    // Visionary: losing after 60' → try a formation change
    if profile.personality == AiPersonality::Visionary
        && goal_diff < 0
        && minute >= 60
        && rng.random_range(0.0..1.0f64) < base_chance * 1.5
    {
        let current = &team.formation;
        let new_formation = if current == "4-4-2" {
            "4-3-3"
        } else if current == "4-3-3" || current == "4-5-1" {
            "4-2-3-1"
        } else {
            return None;
        };
        return Some(MatchCommand::ChangeFormation {
            side,
            formation: new_formation.to_string(),
        });
    }

    None
}
