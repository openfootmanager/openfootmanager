use rand::{Rng, RngExt};

use crate::event::{EventType, MatchEvent};
use crate::shared::{
    PlayStylePhase, TraitContext, home_mod, play_style_modifier, role_modifier,
    tactics_buildup_mod, tactics_cross_probability, tactics_defensive_conversion_mod,
    tactics_foul_modifier, tactics_pressing_press, tactics_shape_modifier,
    tactics_tempo_progression, tactics_width_versus_shape, trait_bonus,
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
    let success_chance =
        (pass_skill * 1.3 * buildup_mod) / (pass_skill * 1.3 * buildup_mod + press);
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
    ) * role_modifier(
        Position::Midfielder,
        attacker.role,
        PlayStylePhase::Midfield,
    );
    let def_mod = play_style_modifier(
        ctx.team(def_side).play_style,
        PlayStylePhase::Midfield,
        false,
    ) * role_modifier(Position::Midfielder, defender.role, PlayStylePhase::Defense);
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
        * role_modifier(Position::Forward, attacker.role, PlayStylePhase::Attack);
    let def_mod = play_style_modifier(ctx.team(def_side).play_style, PlayStylePhase::Defense, true)
        * role_modifier(Position::Defender, defender.role, PlayStylePhase::Defense);
    let att_eff = att_rating * att_mod * home_mod(att_side, ctx.config);
    let def_eff = def_rating
        * def_mod
        * home_mod(def_side, ctx.config)
        * tactics_shape_modifier(&ctx.team(def_side).tactics)
        * tactics_width_versus_shape(&ctx.team(att_side).tactics, &ctx.team(def_side).tactics);
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
            maybe_foul(
                ctx,
                minute,
                def_side,
                &attacker,
                &defender,
                zone,
                rng,
                tactics_foul_modifier(&ctx.team(def_side).tactics),
            )
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
            ctx.emit(MatchEvent::new(
                minute,
                EventType::PenaltyAwarded,
                att_side,
                zone,
            ));
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

    let att_cond = if att_side == Side::Home {
        ctx.home_condition
    } else {
        ctx.away_condition
    };
    let def_cond = if def_side == Side::Home {
        ctx.home_condition
    } else {
        ctx.away_condition
    };

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
    let conversion = (ctx.config.goal_conversion_base * def_line_mod
        + (shoot_rating - gk_rating) / 150.0)
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
    let forwards = team
        .players
        .iter()
        .filter(|p| p.position == Position::Forward && !ctx.sent_off.contains(&p.id));
    base * modifier
        * crate::shared::forward_press_factor(forwards)
        * tactics_pressing_press(&team.tactics)
        * home_mod(pressing_side, ctx.config)
}

#[cfg(test)]
mod press_tests {
    use super::*;
    use crate::test_support::{full_side, same};
    use crate::types::{MatchConfig, PlayStyle, PlayerRole, PressingIntensity, TeamData};

    fn press(home: &TeamData, away: &TeamData, side: Side) -> f64 {
        let config = MatchConfig::default();
        effective_press(&MatchContext::new(home, away, &config), side)
    }

    /// The Away side's press, with `change` made to its team: the ratio against the unchanged
    /// team isolates what the change multiplies it by (Away, so the home advantage is not in it).
    fn press_ratio(change: impl FnOnce(&mut TeamData)) -> f64 {
        let home = full_side("h");
        let plain = full_side("a");
        let mut changed = full_side("a");
        change(&mut changed);
        press(&home, &changed, Side::Away) / press(&home, &plain, Side::Away)
    }

    fn forwards_on(team: &mut TeamData, role: PlayerRole) {
        for player in team
            .players
            .iter_mut()
            .filter(|player| player.position == Position::Forward)
        {
            player.role = role;
        }
    }

    /// Given a side whose forwards are Pressing Forwards,
    /// When its press is read against the same side on Standard,
    /// Then it is 7.5% stronger.
    #[test]
    fn pressing_forwards_strengthen_the_press_by_their_share() {
        assert!(same(
            press_ratio(|team| forwards_on(team, PlayerRole::PressingForward)),
            1.075
        ));
    }

    /// Given a High Press side against a Balanced one,
    /// When its press is read,
    /// Then it is multiplied by the style's Press entry, 1.20.
    #[test]
    fn a_high_press_style_multiplies_the_press_by_its_entry() {
        assert!(same(
            press_ratio(|team| team.play_style = PlayStyle::HighPress),
            1.20
        ));
    }

    /// Given an aggressive pressing dial against a medium one,
    /// When the press is read,
    /// Then it is multiplied by 1.06.
    #[test]
    fn an_aggressive_dial_multiplies_the_press_by_its_figure() {
        assert!(same(
            press_ratio(|team| team.tactics.pressing_intensity = PressingIntensity::Aggressive),
            1.06
        ));
    }

    /// Given the same two sides,
    /// When the home side's press is read against the away side's,
    /// Then it is multiplied by the home advantage.
    #[test]
    fn the_home_side_presses_by_the_home_advantage_more() {
        let (home, away) = (full_side("h"), full_side("h"));

        assert!(same(
            press(&home, &away, Side::Home) / press(&home, &away, Side::Away),
            MatchConfig::default().home_advantage
        ));
    }

    /// Given Pressing Forward on a midfielder (the role is not a midfielder's, but the press reads
    /// the front line by position),
    /// When the press is read,
    /// Then it does not change: only forwards count.
    #[test]
    fn a_midfielder_on_the_forwards_role_adds_nothing() {
        assert!(same(
            press_ratio(|team| {
                for player in team
                    .players
                    .iter_mut()
                    .filter(|player| player.position == Position::Midfielder)
                {
                    player.role = PlayerRole::PressingForward;
                }
            }),
            1.0
        ));
    }

    /// Given both forwards Pressing Forwards and one of them sent off,
    /// When the press is read,
    /// Then only the one still on the pitch adds: 3.75%, not 7.5%.
    #[test]
    fn a_forward_sent_off_adds_nothing_to_the_press() {
        let home = full_side("h");
        let mut pressing = full_side("a");
        forwards_on(&mut pressing, PlayerRole::PressingForward);
        let plain = full_side("a");
        let config = MatchConfig::default();
        let mut ctx = MatchContext::new(&home, &pressing, &config);
        ctx.sent_off.insert("a_fwd0".to_string());

        let with_one_off = effective_press(&ctx, Side::Away);
        let standard = press(&home, &plain, Side::Away);

        assert!(same(with_one_off / standard, 1.0375));
    }
}
