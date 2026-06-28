mod fouls;
mod resolution;

use rand::{Rng, RngExt};

use crate::event::{EventType, MatchEvent};
use crate::report::MatchReport;
use crate::shared::{self, PlayerSnap};
use crate::types::{MatchConfig, PlayerData, Position, Side, TeamData, Zone};

// ---------------------------------------------------------------------------
// MatchEngine — the core minute-by-minute simulator
// ---------------------------------------------------------------------------

/// Simulate a full match between two teams and return a detailed report.
pub fn simulate(home: &TeamData, away: &TeamData, config: &MatchConfig) -> MatchReport {
    let mut rng = rand::rng();
    simulate_with_rng(home, away, config, &mut rng)
}

/// Simulate with an explicit RNG (useful for deterministic tests).
pub fn simulate_with_rng<R: Rng>(
    home: &TeamData,
    away: &TeamData,
    config: &MatchConfig,
    rng: &mut R,
) -> MatchReport {
    let mut ctx = MatchContext::new(home, away, config);

    // Kick-off
    ctx.emit(MatchEvent::new(
        0,
        EventType::KickOff,
        Side::Home,
        Zone::Midfield,
    ));
    ctx.ball_zone = Zone::Midfield;
    ctx.possession = Side::Home;

    // --- First half (minutes 1–45 + stoppage) ---
    let first_half_stoppage = rng.random_range(0..=config.stoppage_time_max);
    let first_half_end = 45 + first_half_stoppage;
    for minute in 1..=first_half_end {
        simulate_minute(&mut ctx, minute, rng);
    }
    ctx.emit(MatchEvent::new(
        first_half_end,
        EventType::HalfTime,
        Side::Home,
        Zone::Midfield,
    ));

    // Reset ball position for second half
    let second_half_start = first_half_end + 1;
    ctx.ball_zone = Zone::Midfield;
    ctx.possession = Side::Away;
    ctx.emit(MatchEvent::new(
        second_half_start,
        EventType::SecondHalfStart,
        Side::Away,
        Zone::Midfield,
    ));

    // --- Second half (minutes 46–90 + stoppage) ---
    let second_half_stoppage = rng.random_range(0..=config.stoppage_time_max);
    let match_end = 90 + first_half_stoppage + second_half_stoppage;
    for minute in second_half_start..=match_end {
        simulate_minute(&mut ctx, minute, rng);
    }
    let total_minutes = match_end;
    ctx.emit(MatchEvent::new(
        match_end,
        EventType::FullTime,
        Side::Home,
        Zone::Midfield,
    ));

    let tracked_player_ids = home
        .players
        .iter()
        .chain(away.players.iter())
        .map(|player| player.id.clone())
        .collect();

    MatchReport::from_events_with_players(
        ctx.events,
        ctx.home_possession_ticks,
        ctx.away_possession_ticks,
        total_minutes,
        tracked_player_ids,
    )
}

// ---------------------------------------------------------------------------
// Internal context carried through the simulation
// ---------------------------------------------------------------------------

pub(crate) struct MatchContext<'a> {
    pub(crate) home: &'a TeamData,
    pub(crate) away: &'a TeamData,
    pub(crate) config: &'a MatchConfig,
    pub(crate) home_score: u8,
    pub(crate) away_score: u8,
    pub(crate) ball_zone: Zone,
    pub(crate) possession: Side,
    pub(crate) events: Vec<MatchEvent>,
    pub(crate) home_possession_ticks: u32,
    pub(crate) away_possession_ticks: u32,
    pub(crate) yellows: std::collections::HashMap<String, u8>,
    pub(crate) sent_off: std::collections::HashSet<String>,
    /// Team-level condition scalar (0.0–1.0). Starts from mean player condition, depletes per minute.
    pub(crate) home_condition: f64,
    pub(crate) away_condition: f64,
}

fn team_avg_condition(team: &TeamData) -> f64 {
    if team.players.is_empty() {
        return 1.0;
    }
    let sum: f64 = team.players.iter().map(|p| p.condition as f64).sum();
    (sum / team.players.len() as f64 / 100.0).clamp(0.5, 1.0)
}

impl<'a> MatchContext<'a> {
    fn new(home: &'a TeamData, away: &'a TeamData, config: &'a MatchConfig) -> Self {
        Self {
            home,
            away,
            config,
            home_score: 0,
            away_score: 0,
            ball_zone: Zone::Midfield,
            possession: Side::Home,
            events: Vec::with_capacity(200),
            home_possession_ticks: 0,
            away_possession_ticks: 0,
            yellows: std::collections::HashMap::new(),
            sent_off: std::collections::HashSet::new(),
            home_condition: team_avg_condition(home),
            away_condition: team_avg_condition(away),
        }
    }

    pub(crate) fn emit(&mut self, event: MatchEvent) {
        self.events.push(event);
    }

    pub(crate) fn team(&self, side: Side) -> &'a TeamData {
        match side {
            Side::Home => self.home,
            Side::Away => self.away,
        }
    }

    pub(crate) fn add_goal(&mut self, side: Side) {
        match side {
            Side::Home => self.home_score += 1,
            Side::Away => self.away_score += 1,
        }
    }
}

/// Pick a random player from a side, preferring a given position, and return
/// a snapshot so we don't hold a borrow on the context.
fn snap_player<R: Rng>(
    ctx: &MatchContext,
    side: Side,
    preferred: Position,
    rng: &mut R,
) -> PlayerSnap {
    let team = ctx.team(side);
    let available: Vec<&PlayerData> = team
        .players
        .iter()
        .filter(|p| !ctx.sent_off.contains(&p.id))
        .collect();

    let candidates: Vec<&PlayerData> = available
        .iter()
        .filter(|p| p.position == preferred)
        .copied()
        .collect();

    let pool = if candidates.is_empty() {
        &available
    } else {
        &candidates
    };

    if pool.is_empty() {
        return PlayerSnap::from(&team.players[0]);
    }
    PlayerSnap::from(pool[rng.random_range(0..pool.len())])
}

// ---------------------------------------------------------------------------
// Minute simulation
// ---------------------------------------------------------------------------

fn simulate_minute<R: Rng>(ctx: &mut MatchContext, minute: u8, rng: &mut R) {
    match ctx.possession {
        Side::Home => ctx.home_possession_ticks += 1,
        Side::Away => ctx.away_possession_ticks += 1,
    }

    // Deplete team condition ~0.18 over 90 minutes (floor at 0.70, but never increase
    // if condition is already below 0.70 at match start).
    let depletion = ctx.config.fatigue_per_minute / 100.0;
    ctx.home_condition = (ctx.home_condition - depletion).max(0.70_f64.min(ctx.home_condition));
    ctx.away_condition = (ctx.away_condition - depletion).max(0.70_f64.min(ctx.away_condition));

    let actions = rng.random_range(1..=3u8);
    for _ in 0..actions {
        resolution::resolve_action(ctx, minute, rng);
    }

    // Possession contest via midfield battle. Tempo (retention) and pressing
    // (ball-winning) weight the battle; transition dials act on the flip. Neutral
    // dials are ×1.0 / no-roll, so default sides match the pre-dial engine.
    let poss_side = ctx.possession;
    let def_side = poss_side.opposite();
    let poss_tactics = ctx.team(poss_side).tactics.clone();
    let def_tactics = ctx.team(def_side).tactics.clone();
    let mid_att = resolution::effective_midfield(ctx, poss_side)
        * shared::tactics_tempo_retention(&poss_tactics);
    let mid_def = resolution::effective_midfield(ctx, def_side)
        * shared::tactics_pressing_contest(&def_tactics);
    let retain = mid_att / (mid_att + mid_def);
    if rng.random_range(0.0..1.0f64) > retain {
        let rewin = shared::tactics_counter_press_rewin(&poss_tactics);
        if rewin > 0.0 && rng.random_range(0.0..1.0f64) < rewin {
            // Counter-press wins it straight back; nothing changes.
        } else {
            ctx.possession = def_side;
            let breakaway = shared::tactics_break_speed_counter(&def_tactics);
            if breakaway > 0.0 && rng.random_range(0.0..1.0f64) < breakaway {
                ctx.ball_zone = Zone::attacking_third(def_side);
            } else {
                ctx.ball_zone = Zone::Midfield;
            }
        }
    }
}
