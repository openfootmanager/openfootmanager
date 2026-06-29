use rand::{Rng, RngExt};

use crate::event::{EventType, MatchEvent};
use crate::shared::{
    PlayStylePhase, TraitContext, home_mod, play_style_modifier, role_attribute_modifier,
    tactics_buildup_mod, tactics_cross_probability, tactics_defensive_conversion_mod,
    tactics_foul_modifier, tactics_pressing_press, tactics_shape_modifier,
    tactics_tempo_progression, trait_bonus,
};
use crate::types::{Position, Side, Zone};

use super::MatchContext;
use super::fouls::{self, maybe_foul};
use super::snap_player;

// ---------------------------------------------------------------------------
// Action resolution per zone
// ---------------------------------------------------------------------------

pub(super) fn resolve_action<R: Rng>(ctx: &mut MatchContext, minute: u8, rng: &mut R) {
    let att_side = ctx.possession;
    let def_side = att_side.opposite();
    let zone = ctx.ball_zone;

    if zone.is_box_for(att_side) {
        resolve_shot(ctx, minute, att_side, rng);
        // resolve_shot manages ball_zone and possession for all outcomes
    } else if zone == Zone::attacking_third(att_side) {
        resolve_attacking_third(ctx, minute, att_side, def_side, rng);
    } else if zone == Zone::Midfield {
        resolve_midfield(ctx, minute, att_side, def_side, rng);
    } else {
        resolve_buildup(ctx, minute, att_side, def_side, rng);
    }
}

// ---------------------------------------------------------------------------
// Zone-specific resolution
// ---------------------------------------------------------------------------

fn resolve_buildup<R: Rng>(
    ctx: &mut MatchContext,
    minute: u8,
    att_side: Side,
    def_side: Side,
    rng: &mut R,
) {
    let passer = snap_player(ctx, att_side, Position::Defender, rng);
    let pass_skill = (passer.passing as f64
        + passer.vision as f64
        + passer.composure as f64
        + passer.teamwork as f64)
        / 4.0
        * trait_bonus(&passer, TraitContext::Passing);
    let press = effective_press(ctx, def_side);
    let ball_zone = ctx.ball_zone;

    let buildup_mod = tactics_buildup_mod(&ctx.team(att_side).tactics);
    let success_chance = (pass_skill * 1.3 * buildup_mod) / (pass_skill * 1.3 * buildup_mod + press);
    if rng.random_range(0.0..1.0f64) < success_chance {
        ctx.emit(
            MatchEvent::new(minute, EventType::PassCompleted, att_side, ball_zone)
                .with_player(&passer.id),
        );
        ctx.ball_zone = Zone::Midfield;
    } else {
        let interceptor = snap_player(ctx, def_side, Position::Midfielder, rng);
        ctx.emit(
            MatchEvent::new(minute, EventType::PassIntercepted, att_side, ball_zone)
                .with_player(&passer.id),
        );
        ctx.emit(
            MatchEvent::new(minute, EventType::Interception, def_side, ball_zone)
                .with_player(&interceptor.id),
        );
        ctx.possession = def_side;
    }
}

fn resolve_midfield<R: Rng>(
    ctx: &mut MatchContext,
    minute: u8,
    att_side: Side,
    def_side: Side,
    rng: &mut R,
) {
    let attacker = snap_player(ctx, att_side, Position::Midfielder, rng);
    let defender = snap_player(ctx, def_side, Position::Midfielder, rng);

    let att_rating = (attacker.dribbling as f64
        + attacker.passing as f64
        + attacker.vision as f64
        + attacker.teamwork as f64)
        / 4.0
        * trait_bonus(&attacker, TraitContext::Midfield);
    let def_rating = (defender.tackling as f64
        + defender.positioning as f64
        + defender.decisions as f64
        + defender.teamwork as f64)
        / 4.0
        * trait_bonus(&defender, TraitContext::Tackling);

    let att_mod = play_style_modifier(
        ctx.team(att_side).play_style,
        PlayStylePhase::Midfield,
        true,
    ) * role_attribute_modifier(attacker.role, PlayStylePhase::Midfield);
    let def_mod = play_style_modifier(
        ctx.team(def_side).play_style,
        PlayStylePhase::Midfield,
        false,
    ) * role_attribute_modifier(defender.role, PlayStylePhase::Defense);
    let att_eff = att_rating
        * att_mod
        * home_mod(att_side, ctx.config)
        * tactics_tempo_progression(&ctx.team(att_side).tactics);
    let def_eff = def_rating * def_mod * home_mod(def_side, ctx.config);
    let success = att_eff / (att_eff + def_eff);

    if rng.random_range(0.0..1.0f64) < success {
        ctx.emit(
            MatchEvent::new(minute, EventType::PassCompleted, att_side, Zone::Midfield)
                .with_player(&attacker.id),
        );
        ctx.ball_zone = Zone::attacking_third(att_side);
    } else {
        if rng.random_range(0.0..1.0f64) < 0.6 {
            ctx.emit(
                MatchEvent::new(minute, EventType::Tackle, def_side, Zone::Midfield)
                    .with_player(&defender.id),
            );
            let foul_mod = tactics_foul_modifier(&ctx.team(def_side).tactics);
            let fouled = maybe_foul(
                ctx,
                minute,
                def_side,
                &attacker,
                &defender,
                Zone::Midfield,
                rng,
                foul_mod,
            );
            if fouled {
                // Fouled team (att_side) retains possession for the free kick
                ctx.possession = att_side;
                ctx.ball_zone = Zone::Midfield;
                return;
            }
        } else {
            ctx.emit(
                MatchEvent::new(minute, EventType::Interception, def_side, Zone::Midfield)
                    .with_player(&defender.id),
            );
        }
        ctx.possession = def_side;
        ctx.ball_zone = Zone::Midfield;
    }
}

fn resolve_attacking_third<R: Rng>(
    ctx: &mut MatchContext,
    minute: u8,
    att_side: Side,
    def_side: Side,
    rng: &mut R,
) {
    let attacker = snap_player(ctx, att_side, Position::Forward, rng);
    let defender = snap_player(ctx, def_side, Position::Defender, rng);

    let att_rating = (attacker.dribbling as f64
        + attacker.pace as f64
        + attacker.agility as f64
        + attacker.composure as f64)
        / 4.0
        * trait_bonus(&attacker, TraitContext::Dribbling);
    let def_rating = (defender.defending as f64
        + defender.tackling as f64
        + defender.positioning as f64
        + defender.aerial as f64)
        / 4.0
        * trait_bonus(&defender, TraitContext::Tackling);

    let att_mod = play_style_modifier(ctx.team(att_side).play_style, PlayStylePhase::Attack, true)
        * role_attribute_modifier(attacker.role, PlayStylePhase::Attack);
    let def_mod = play_style_modifier(
        ctx.team(def_side).play_style,
        PlayStylePhase::Defense,
        false,
    ) * role_attribute_modifier(defender.role, PlayStylePhase::Defense);
    let att_eff = att_rating * att_mod * home_mod(att_side, ctx.config);
    let def_eff = def_rating
        * def_mod
        * home_mod(def_side, ctx.config)
        * tactics_shape_modifier(&ctx.team(def_side).tactics);
    let success = att_eff / (att_eff + def_eff);
    let zone = Zone::attacking_third(att_side);
    let cross_prob = tactics_cross_probability(&ctx.team(att_side).tactics);

    if rng.random_range(0.0..1.0f64) < success {
        ctx.emit(
            MatchEvent::new(minute, EventType::Dribble, att_side, zone).with_player(&attacker.id),
        );
        if rng.random_range(0.0..1.0f64) < cross_prob {
            ctx.emit(
                MatchEvent::new(minute, EventType::Cross, att_side, zone).with_player(&attacker.id),
            );
            let header = snap_player(ctx, att_side, Position::Forward, rng);
            let def_header = snap_player(ctx, def_side, Position::Defender, rng);
            let aerial_att = header.aerial as f64;
            let aerial_def = def_header.aerial as f64;
            let aerial_win = aerial_att / (aerial_att + aerial_def);
            if rng.random_range(0.0..1.0f64) < aerial_win {
                ctx.ball_zone = Zone::attacking_box(att_side);
                resolve_shot(ctx, minute, att_side, rng);
            } else {
                ctx.emit(
                    MatchEvent::new(minute, EventType::Clearance, def_side, zone)
                        .with_player(&def_header.id),
                );
                ctx.possession = def_side;
                ctx.ball_zone = Zone::defensive_third(att_side);
            }
        } else {
            ctx.ball_zone = Zone::attacking_box(att_side);
        }
    } else {
        let is_tackle = rng.random_range(0.0..1.0f64) < 0.5;
        let fouled = if is_tackle {
            ctx.emit(
                MatchEvent::new(minute, EventType::DribbleTackled, att_side, zone)
                    .with_player(&attacker.id)
                    .with_secondary(&defender.id),
            );
            ctx.emit(
                MatchEvent::new(minute, EventType::Tackle, def_side, zone)
                    .with_player(&defender.id),
            );
            maybe_foul(ctx, minute, def_side, &attacker, &defender, zone, rng, tactics_foul_modifier(&ctx.team(def_side).tactics))
        } else {
            ctx.emit(
                MatchEvent::new(minute, EventType::Clearance, def_side, zone)
                    .with_player(&defender.id),
            );
            false
        };
        if fouled {
            // Fouled team (att_side) retains possession for the free kick in the attacking third
            ctx.possession = att_side;
            ctx.ball_zone = zone;
            return;
        }
        if rng.random_range(0.0..1.0f64) < 0.25 {
            ctx.emit(MatchEvent::new(minute, EventType::Corner, att_side, zone));
            if rng.random_range(0.0..1.0f64) < 0.30 {
                ctx.ball_zone = Zone::attacking_box(att_side);
                return;
            }
        }
        ctx.possession = def_side;
        ctx.ball_zone = Zone::defensive_third(att_side);
    }
}

fn resolve_shot<R: Rng>(ctx: &mut MatchContext, minute: u8, att_side: Side, rng: &mut R) {
    let def_side = att_side.opposite();
    let zone = Zone::attacking_box(att_side);

    // Box foul rate fixed at 3.6% per shot — independent of foul_probability (which tunes outfield fouls)
    if rng.random_range(0.0..1.0f64) < 0.036 {
        let fouler = snap_player(ctx, def_side, Position::Defender, rng);
        let fouled = snap_player(ctx, att_side, Position::Forward, rng);
        ctx.emit(
            MatchEvent::new(minute, EventType::Foul, def_side, zone)
                .with_player(&fouler.id)
                .with_secondary(&fouled.id),
        );
        if rng.random_range(0.0..1.0f64) < ctx.config.penalty_probability {
            ctx.emit(MatchEvent::new(minute, EventType::PenaltyAwarded, att_side, zone));
            fouls::resolve_penalty(ctx, minute, att_side, rng);
            fouls::maybe_card(ctx, minute, def_side, &fouler.id, zone, rng);
            ctx.ball_zone = Zone::Midfield;
            ctx.possession = def_side;
            return;
        }
        fouls::maybe_card(ctx, minute, def_side, &fouler.id, zone, rng);
        // Foul but no penalty: advantage played, shot continues
    }

    let shooter = snap_player(ctx, att_side, Position::Forward, rng);
    let assister = snap_player(ctx, att_side, Position::Midfielder, rng);
    let goalkeeper = snap_player(ctx, def_side, Position::Goalkeeper, rng);

    let att_cond = if att_side == Side::Home { ctx.home_condition } else { ctx.away_condition };
    let def_cond = if def_side == Side::Home { ctx.home_condition } else { ctx.away_condition };

    let shoot_rating =
        (shooter.shooting as f64 + shooter.composure as f64 + shooter.decisions as f64) / 3.0
            * trait_bonus(&shooter, TraitContext::Shooting)
            * att_cond;
    let gk_rating =
        (goalkeeper.handling as f64 + goalkeeper.reflexes as f64 + goalkeeper.positioning as f64)
            / 3.0
            * trait_bonus(&goalkeeper, TraitContext::Goalkeeping)
            * def_cond;

    let accuracy =
        (ctx.config.shot_accuracy_base + (shoot_rating - 50.0) / 200.0).clamp(0.15, 0.85);

    if rng.random_range(0.0..1.0f64) > accuracy {
        if rng.random_range(0.0..1.0f64) < 0.4 {
            ctx.emit(
                MatchEvent::new(minute, EventType::ShotBlocked, att_side, zone)
                    .with_player(&shooter.id),
            );
            // Blocked shot: ball stays in area, defender clears to midfield
            ctx.possession = def_side;
            ctx.ball_zone = Zone::Midfield;
        } else {
            ctx.emit(
                MatchEvent::new(minute, EventType::ShotOffTarget, att_side, zone)
                    .with_player(&shooter.id),
            );
            ctx.emit(MatchEvent::new(minute, EventType::GoalKick, def_side, zone));
            ctx.possession = def_side;
            ctx.ball_zone = Zone::defensive_third(def_side);
        }
        return;
    }

    let def_line_mod = tactics_defensive_conversion_mod(&ctx.team(def_side).tactics);
    let conversion =
        (ctx.config.goal_conversion_base * def_line_mod + (shoot_rating - gk_rating) / 150.0)
            .clamp(0.10, 0.70);

    if rng.random_range(0.0..1.0f64) < conversion {
        ctx.emit(
            MatchEvent::new(minute, EventType::Goal, att_side, zone)
                .with_player(&shooter.id)
                .with_secondary(&assister.id),
        );
        ctx.add_goal(att_side);
        ctx.possession = def_side;
        ctx.ball_zone = Zone::Midfield;
    } else {
        ctx.emit(
            MatchEvent::new(minute, EventType::ShotSaved, att_side, zone).with_player(&shooter.id),
        );
        // 40% of saves → corner (keeper parries wide), 60% → goal kick (keeper catches)
        if rng.random_range(0.0..1.0f64) < 0.40 {
            ctx.emit(MatchEvent::new(minute, EventType::Corner, att_side, zone));
            ctx.possession = att_side;
            ctx.ball_zone = Zone::attacking_box(att_side);
        } else {
            ctx.emit(MatchEvent::new(minute, EventType::GoalKick, def_side, zone));
            ctx.possession = def_side;
            ctx.ball_zone = Zone::defensive_third(def_side);
        }
    }
}

// ---------------------------------------------------------------------------
// Rating helpers
// ---------------------------------------------------------------------------

pub(super) fn effective_midfield(ctx: &MatchContext, side: Side) -> f64 {
    let base = ctx.team(side).midfield_rating();
    let modifier = play_style_modifier(ctx.team(side).play_style, PlayStylePhase::Midfield, true);
    base * modifier * home_mod(side, ctx.config)
}

fn effective_press(ctx: &MatchContext, pressing_side: Side) -> f64 {
    let team = ctx.team(pressing_side);
    let base = team.position_attr_avg(Position::Midfielder, |p| {
        ((p.stamina as u16 + p.tackling as u16 + p.pace as u16) / 3) as u8
    });
    let modifier = play_style_modifier(team.play_style, PlayStylePhase::Press, true);
    base * modifier
        * tactics_pressing_press(&team.tactics)
        * home_mod(pressing_side, ctx.config)
}
