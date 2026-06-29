use rand::{Rng, RngExt};

use crate::event::{EventType, MatchEvent};
use crate::shared::{PlayerSnap, TraitContext, trait_bonus};
use crate::types::{Position, Side, Zone};

use super::MatchContext;
use super::snap_player;

/// `fouled_snap` is the player who was fouled; `fouler_snap` committed the foul.
/// `fouling_side` is the side that committed the foul.
/// `tactics_mod` is a pre-computed multiplier from the fouling team's pressing/marking settings.
/// Returns `true` if a foul was committed.
pub(super) fn maybe_foul<R: Rng>(
    ctx: &mut MatchContext,
    minute: u8,
    fouling_side: Side,
    fouled_snap: &PlayerSnap,
    fouler_snap: &PlayerSnap,
    zone: Zone,
    rng: &mut R,
    tactics_mod: f64,
) -> bool {
    let aggression_mod = fouler_snap.aggression as f64 / 100.0;
    let foul_chance = ctx.config.foul_probability
        * (0.6 + aggression_mod * 0.8)
        * trait_bonus(fouler_snap, TraitContext::Foul)
        * tactics_mod;
    if rng.random_range(0.0..1.0f64) >= foul_chance {
        return false;
    }

    ctx.emit(
        MatchEvent::new(minute, EventType::Foul, fouling_side, zone)
            .with_player(&fouler_snap.id)
            .with_secondary(&fouled_snap.id),
    );

    let att_side = fouling_side.opposite();

    if zone.is_box_for(att_side) && rng.random_range(0.0..1.0f64) < ctx.config.penalty_probability {
        ctx.emit(MatchEvent::new(
            minute,
            EventType::PenaltyAwarded,
            att_side,
            zone,
        ));
        resolve_penalty(ctx, minute, att_side, rng);
    } else {
        ctx.emit(MatchEvent::new(minute, EventType::FreeKick, att_side, zone));
    }

    maybe_card(ctx, minute, fouling_side, &fouler_snap.id, zone, rng);

    if rng.random_range(0.0..1.0f64) < ctx.config.injury_probability {
        ctx.emit(
            MatchEvent::new(minute, EventType::Injury, att_side, zone).with_player(&fouled_snap.id),
        );
    }

    true
}

pub(super) fn maybe_card<R: Rng>(
    ctx: &mut MatchContext,
    minute: u8,
    side: Side,
    fouler_id: &str,
    zone: Zone,
    rng: &mut R,
) {
    let aggression_factor = ctx
        .team(side)
        .players
        .iter()
        .find(|p| p.id == fouler_id)
        .map(|p| p.aggression as f64 / 100.0)
        .unwrap_or(0.5);
    let card_chance = ctx.config.yellow_card_probability * (0.5 + aggression_factor);
    if rng.random_range(0.0..1.0f64) >= card_chance {
        return;
    }

    if rng.random_range(0.0..1.0f64) < ctx.config.red_card_probability {
        ctx.emit(MatchEvent::new(minute, EventType::RedCard, side, zone).with_player(fouler_id));
        ctx.sent_off.insert(fouler_id.to_string());
        return;
    }

    let current_yellows = ctx.yellows.entry(fouler_id.to_string()).or_insert(0);
    *current_yellows += 1;

    if *current_yellows >= 2 {
        ctx.emit(
            MatchEvent::new(minute, EventType::SecondYellow, side, zone).with_player(fouler_id),
        );
        ctx.sent_off.insert(fouler_id.to_string());
    } else {
        ctx.emit(MatchEvent::new(minute, EventType::YellowCard, side, zone).with_player(fouler_id));
    }
}

pub(super) fn resolve_penalty<R: Rng>(ctx: &mut MatchContext, minute: u8, att_side: Side, rng: &mut R) {
    let taker = snap_player(ctx, att_side, Position::Forward, rng);
    let gk = snap_player(ctx, att_side.opposite(), Position::Goalkeeper, rng);

    let shoot_skill = (taker.shooting as f64 + taker.decisions as f64) / 2.0;
    let gk_skill = (gk.positioning as f64 + gk.decisions as f64) / 2.0;
    let conversion = (0.75 + (shoot_skill - gk_skill) / 300.0).clamp(0.55, 0.92);
    let zone = Zone::attacking_box(att_side);

    if rng.random_range(0.0..1.0f64) < conversion {
        ctx.emit(
            MatchEvent::new(minute, EventType::PenaltyGoal, att_side, zone).with_player(&taker.id),
        );
        ctx.add_goal(att_side);
    } else {
        ctx.emit(
            MatchEvent::new(minute, EventType::PenaltyMiss, att_side, zone).with_player(&taker.id),
        );
    }
}
