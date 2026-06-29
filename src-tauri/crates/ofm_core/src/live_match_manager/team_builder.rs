use crate::game::Game;
use crate::player_rating::{
    deployed_position, effective_rating_for_assignment, formation_slots, natural_ovr,
    positional_fit_for_assignment,
};
use domain::player::Position as DomainPosition;
use engine::{
    BreakSpeed, CounterPressDuration, DefensiveLine, DefensiveShape, MarkingStyle, PlayStyle,
    PlayerData, PlayerRole as EnginePlayerRole, Position, PressingIntensity, TacticsBuildUpStyle,
    TacticsConfig, TacticsPitchWidth, Tempo, TeamData,
};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Domain → Engine conversion with starting XI / bench split
// ---------------------------------------------------------------------------

pub(super) fn build_team_with_bench(game: &Game, team_id: &str) -> (TeamData, Vec<PlayerData>) {
    let team = game.teams.iter().find(|t| t.id == team_id);
    let (name, formation, play_style, tactics, saved_xi_ids) = match team {
        Some(t) => (
            t.name.clone(),
            t.formation.clone(),
            match t.play_style {
                domain::team::PlayStyle::Attacking => PlayStyle::Attacking,
                domain::team::PlayStyle::Defensive => PlayStyle::Defensive,
                domain::team::PlayStyle::Possession => PlayStyle::Possession,
                domain::team::PlayStyle::Counter => PlayStyle::Counter,
                domain::team::PlayStyle::HighPress => PlayStyle::HighPress,
                _ => PlayStyle::Balanced,
            },
            domain_to_engine_tactics(&t.tactics_phase),
            t.starting_xi_ids.as_slice(),
        ),
        None => (
            "Unknown".into(),
            "4-4-2".into(),
            PlayStyle::Balanced,
            TacticsConfig::default(),
            &[] as &[String],
        ),
    };

    // Collect all available (non-injured) players for this team
    let available_players: Vec<&domain::player::Player> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id) && p.injury.is_none())
        .collect();
    let player_roles = team.map(|t| &t.player_roles);
    // `deployed` is the granular slot the player occupies; `None` for the bench,
    // where the player's own position is used instead. The engine's coarse
    // position is derived from this so a player fielded out of position (e.g. a
    // striker at centre-back) is simulated in the position they actually play.
    let convert_player = |p: &domain::player::Player, deployed: Option<&DomainPosition>| {
        let role = player_roles
            .and_then(|roles| roles.get(&p.id))
            .map(domain_to_engine_role)
            .unwrap_or(EnginePlayerRole::Standard);
        to_engine_player(p, role, deployed)
    };

    // The user manages their own XI by hand (saved_xi_ids); AI clubs are managed
    // by a reputation-driven policy that picks a first-choice XI and rotates for
    // load management. Gate on the user team explicitly, NOT on "saved XI empty",
    // so the human's early-career auto-built XI stays reputation-independent.
    let is_user_team = game.manager.team_id.as_deref() == Some(team_id);
    let starting_players = if is_user_team {
        select_starting_xi(saved_xi_ids, &available_players, &formation)
    } else {
        let quality = team_management_quality(game, team);
        ai_select_starting_xi(&available_players, &formation, quality)
    };
    let used_ids: HashSet<String> = starting_players
        .iter()
        .map(|player| player.id.clone())
        .collect();
    let slots = formation_slots(&formation);
    let starting_xi = starting_players
        .into_iter()
        .enumerate()
        .map(|(slot_index, p)| {
            // The deployed slot must agree with deployed_position / the UI, which
            // key off the RAW starting_xi_ids. For the user team, select_starting_xi
            // compacts around unavailable (injured/sold) starters, so the list
            // index is NOT a reliable slot — resolve it from the saved XI instead
            // (replacements not in the saved XI fall back to their natural
            // position). The AI selection is built per-slot in order, so for it the
            // list index IS the slot.
            let deployed: Option<DomainPosition> = if is_user_team {
                team.and_then(|t| deployed_position(t, &p.id))
            } else {
                slots.get(slot_index).cloned()
            };
            convert_player(p, deployed.as_ref())
        })
        .collect();

    let mut bench_domain: Vec<&domain::player::Player> = available_players
        .into_iter()
        .filter(|player| !used_ids.contains(&player.id))
        .collect();
    bench_domain.sort_by(|left, right| {
        natural_ovr(right)
            .partial_cmp(&natural_ovr(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let bench = bench_domain.into_iter().map(|p| convert_player(p, None)).collect();

    let team_data = TeamData {
        id: team_id.to_string(),
        name,
        formation,
        play_style,
        players: starting_xi,
        tactics,
    };

    (team_data, bench)
}

fn select_starting_xi<'a>(
    saved_xi_ids: &[String],
    available_players: &[&'a domain::player::Player],
    formation: &str,
) -> Vec<&'a domain::player::Player> {
    let players_by_id: HashMap<&str, &domain::player::Player> = available_players
        .iter()
        .map(|player| (player.id.as_str(), *player))
        .collect();
    let mut saved_used_ids = HashSet::new();
    let mut valid_saved_players = Vec::with_capacity(11);

    for player_id in saved_xi_ids {
        let Some(player) = players_by_id.get(player_id.as_str()) else {
            continue;
        };

        if saved_used_ids.insert(player_id.clone()) {
            valid_saved_players.push(*player);
        }
    }

    if valid_saved_players.len() >= 8 {
        let mut selected = valid_saved_players;
        selected.truncate(11);
        let mut used_ids: HashSet<String> =
            selected.iter().map(|player| player.id.clone()).collect();
        let slots = formation_slots(formation);

        while selected.len() < 11 {
            let slot = slots.get(selected.len());
            let best_index = available_players
                .iter()
                .enumerate()
                .filter(|(_, player)| !used_ids.contains(&player.id))
                .max_by(|(_, left), (_, right)| {
                    let left_rating = slot.map_or_else(
                        || natural_ovr(left),
                        |slot| effective_rating_for_assignment(left, slot),
                    );
                    let right_rating = slot.map_or_else(
                        || natural_ovr(right),
                        |slot| effective_rating_for_assignment(right, slot),
                    );
                    left_rating
                        .partial_cmp(&right_rating)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(index, _)| index);

            let Some(best_index) = best_index else {
                break;
            };

            let player = available_players[best_index];
            used_ids.insert(player.id.clone());
            selected.push(player);
        }

        return selected;
    }

    auto_select_starting_xi(available_players, formation)
}

fn auto_select_starting_xi<'a>(
    available_players: &[&'a domain::player::Player],
    formation: &str,
) -> Vec<&'a domain::player::Player> {
    let slots = formation_slots(formation);
    let mut used_ids = HashSet::new();
    let mut starting_xi = Vec::with_capacity(11);

    for slot in slots.iter().take(11) {
        let best_player = available_players
            .iter()
            .copied()
            .filter(|player| !used_ids.contains(&player.id))
            .max_by(|left, right| {
                effective_rating_for_assignment(left, slot)
                    .partial_cmp(&effective_rating_for_assignment(right, slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some(player) = best_player else {
            break;
        };

        used_ids.insert(player.id.clone());
        starting_xi.push(player);
    }

    starting_xi
}

/// Maps a club's reputation to a 0.0–1.0 management-quality score. Generated club
/// reputations span roughly 300 (lower divisions) to 900 (elite). Quality drives
/// how proactively the AI rotates for player freshness.
fn management_quality(reputation: u32) -> f64 {
    (((reputation as f64) - 300.0) / 600.0).clamp(0.0, 1.0)
}

/// Maps a manager's overall rating (≈30–95) to a 0.0–1.0 management-quality score.
fn management_quality_from_rating(rating: u8) -> f64 {
    ((f64::from(rating) - 30.0) / 65.0).clamp(0.0, 1.0)
}

/// Resolves the AI management quality for a team: the linked manager's rating when
/// one is hired (manager-specific skill), otherwise the club's reputation as a
/// proxy. Defaults to a mid value when the team can't be found.
fn team_management_quality(game: &Game, team: Option<&domain::team::Team>) -> f64 {
    let Some(team) = team else {
        return management_quality(500);
    };

    if let Some(manager_id) = &team.manager_id {
        if let Some(manager) = game.managers.iter().find(|m| &m.id == manager_id) {
            return management_quality_from_rating(manager.rating());
        }
    }

    management_quality(team.reputation)
}

/// Reputation-aware AI lineup selection.
///
/// Step 1 picks the first-choice XI purely on condition-free positional fit, so a
/// club always fields its best players when fresh. Step 2 applies load management:
/// a tired starter is rested ONLY when (a) their condition is below a
/// quality-dependent fatigue threshold AND (b) a fresher squad option exists whose
/// quality is within a quality-dependent tolerance. Well-run clubs (high quality)
/// rest players earlier and accept a slightly larger quality drop to keep the
/// squad fresh; poorly-run clubs ride their best XI into the ground. The
/// gap-aware tolerance guarantees a strong starter is never benched for a much
/// weaker fresh player — better clubs still field better teams.
fn ai_select_starting_xi<'a>(
    available_players: &[&'a domain::player::Player],
    formation: &str,
    quality: f64,
) -> Vec<&'a domain::player::Player> {
    /// A rotation candidate must be at least this fresh to be worth considering.
    const FRESH_FLOOR: f64 = 60.0;
    /// And meaningfully fresher than the starter it would replace.
    const MIN_FRESHNESS_GAIN: i16 = 10;

    let slots = formation_slots(formation);
    let mut used_ids: HashSet<String> = HashSet::new();
    // (slot index, chosen player) so the rotation step can re-evaluate per slot.
    let mut selected: Vec<(usize, &'a domain::player::Player)> = Vec::with_capacity(11);

    // Step 1: first-choice XI by condition-free positional fit.
    for (slot_index, slot) in slots.iter().take(11).enumerate() {
        let best = available_players
            .iter()
            .copied()
            .filter(|player| !used_ids.contains(&player.id))
            .max_by(|left, right| {
                positional_fit_for_assignment(left, slot)
                    .partial_cmp(&positional_fit_for_assignment(right, slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some(player) = best else {
            break;
        };
        used_ids.insert(player.id.clone());
        selected.push((slot_index, player));
    }

    // Step 2: reputation-driven load management.
    let rest_threshold = 50.0 + 25.0 * quality; // 50 (poor) .. 75 (elite)
    let fit_tolerance = 12.0 * quality; // 0 (poor) .. 12 (elite)

    for entry in selected.iter_mut() {
        let slot = &slots[entry.0];
        let starter = entry.1;

        if f64::from(starter.condition) >= rest_threshold {
            continue; // Fresh enough — no reason to rotate.
        }

        let starter_fit = positional_fit_for_assignment(starter, slot);
        let starter_group = starter.position.to_group_position();
        let fresh_alternative = available_players
            .iter()
            .copied()
            .filter(|player| !used_ids.contains(&player.id))
            // Only rotate within the same position group, so load management never
            // skews the formation's distribution (e.g. fielding a 5th midfielder
            // in place of a defender, which would leave the XI a man short).
            .filter(|player| player.position.to_group_position() == starter_group)
            .filter(|player| f64::from(player.condition) >= FRESH_FLOOR)
            .filter(|player| {
                i16::from(player.condition) - i16::from(starter.condition) >= MIN_FRESHNESS_GAIN
            })
            .filter(|player| {
                positional_fit_for_assignment(player, slot) >= starter_fit - fit_tolerance
            })
            .max_by(|left, right| {
                positional_fit_for_assignment(left, slot)
                    .partial_cmp(&positional_fit_for_assignment(right, slot))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some(fresh) = fresh_alternative {
            used_ids.remove(&starter.id);
            used_ids.insert(fresh.id.clone());
            entry.1 = fresh;
        }
    }

    selected.into_iter().map(|(_, player)| player).collect()
}

pub(crate) fn domain_to_engine_role(role: &domain::team::PlayerRole) -> EnginePlayerRole {
    match role {
        domain::team::PlayerRole::Standard => EnginePlayerRole::Standard,
        domain::team::PlayerRole::BallPlayingKeeper => EnginePlayerRole::BallPlayingKeeper,
        domain::team::PlayerRole::SweeperKeeper => EnginePlayerRole::SweeperKeeper,
        domain::team::PlayerRole::Stopper => EnginePlayerRole::Stopper,
        domain::team::PlayerRole::CoverCB => EnginePlayerRole::CoverCB,
        domain::team::PlayerRole::BallPlayingCB => EnginePlayerRole::BallPlayingCB,
        domain::team::PlayerRole::AttackingFB => EnginePlayerRole::AttackingFB,
        domain::team::PlayerRole::DefensiveFB => EnginePlayerRole::DefensiveFB,
        domain::team::PlayerRole::InvertedFB => EnginePlayerRole::InvertedFB,
        domain::team::PlayerRole::WingBack => EnginePlayerRole::WingBack,
        domain::team::PlayerRole::AnchorMan => EnginePlayerRole::AnchorMan,
        domain::team::PlayerRole::BallWinner => EnginePlayerRole::BallWinner,
        domain::team::PlayerRole::DeepLyingPlaymaker => EnginePlayerRole::DeepLyingPlaymaker,
        domain::team::PlayerRole::BoxToBox => EnginePlayerRole::BoxToBox,
        domain::team::PlayerRole::Carrilero => EnginePlayerRole::Carrilero,
        domain::team::PlayerRole::Mezzala => EnginePlayerRole::Mezzala,
        domain::team::PlayerRole::AdvancedPlaymaker => EnginePlayerRole::AdvancedPlaymaker,
        domain::team::PlayerRole::ShadowStriker => EnginePlayerRole::ShadowStriker,
        domain::team::PlayerRole::WideForward => EnginePlayerRole::WideForward,
        domain::team::PlayerRole::InsideForward => EnginePlayerRole::InsideForward,
        domain::team::PlayerRole::InvertedWinger => EnginePlayerRole::InvertedWinger,
        domain::team::PlayerRole::Poacher => EnginePlayerRole::Poacher,
        domain::team::PlayerRole::TargetMan => EnginePlayerRole::TargetMan,
        domain::team::PlayerRole::DeepLyingForward => EnginePlayerRole::DeepLyingForward,
        domain::team::PlayerRole::False9 => EnginePlayerRole::False9,
        domain::team::PlayerRole::PressingForward => EnginePlayerRole::PressingForward,
        domain::team::PlayerRole::CompleteForward => EnginePlayerRole::CompleteForward,
    }
}

pub(crate) fn domain_to_engine_tactics(t: &domain::team::TacticsPhaseSettings) -> TacticsConfig {
    TacticsConfig {
        pressing_intensity: match t.pressing_intensity {
            domain::team::PressingIntensity::Passive => PressingIntensity::Passive,
            domain::team::PressingIntensity::Medium => PressingIntensity::Medium,
            domain::team::PressingIntensity::Aggressive => PressingIntensity::Aggressive,
        },
        defensive_line: match t.defensive_line {
            domain::team::DefensiveLine::VeryLow => DefensiveLine::VeryLow,
            domain::team::DefensiveLine::Low => DefensiveLine::Low,
            domain::team::DefensiveLine::Medium => DefensiveLine::Medium,
            domain::team::DefensiveLine::High => DefensiveLine::High,
        },
        width: match t.width {
            domain::team::PitchWidth::Narrow => TacticsPitchWidth::Narrow,
            domain::team::PitchWidth::Normal => TacticsPitchWidth::Normal,
            domain::team::PitchWidth::Wide => TacticsPitchWidth::Wide,
        },
        build_up_style: match t.build_up_style {
            domain::team::BuildUpStyle::Short => TacticsBuildUpStyle::Short,
            domain::team::BuildUpStyle::Mixed => TacticsBuildUpStyle::Mixed,
            domain::team::BuildUpStyle::Long => TacticsBuildUpStyle::Long,
        },
        marking_style: match t.marking_style {
            domain::team::MarkingStyle::Zonal => MarkingStyle::Zonal,
            domain::team::MarkingStyle::Mixed => MarkingStyle::Mixed,
            domain::team::MarkingStyle::ManToMan => MarkingStyle::ManToMan,
        },
        tempo: match t.tempo {
            domain::team::Tempo::Patient => Tempo::Patient,
            domain::team::Tempo::Direct => Tempo::Direct,
        },
        defensive_shape: match t.defensive_shape {
            domain::team::DefensiveShape::Stretched => DefensiveShape::Stretched,
            domain::team::DefensiveShape::Normal => DefensiveShape::Normal,
            domain::team::DefensiveShape::Compact => DefensiveShape::Compact,
        },
        counter_press_duration: match t.counter_press_duration {
            domain::team::CounterPressDuration::None => CounterPressDuration::None,
            domain::team::CounterPressDuration::Short => CounterPressDuration::Short,
            domain::team::CounterPressDuration::Long => CounterPressDuration::Long,
        },
        break_speed: match t.break_speed {
            domain::team::BreakSpeed::Slow => BreakSpeed::Slow,
            domain::team::BreakSpeed::Medium => BreakSpeed::Medium,
            domain::team::BreakSpeed::Fast => BreakSpeed::Fast,
        },
    }
}

fn to_engine_player(
    p: &domain::player::Player,
    role: EnginePlayerRole,
    deployed: Option<&DomainPosition>,
) -> PlayerData {
    // Fall back to the player's natural position (not `p.position`, which on
    // legacy saves can still hold a stale coarse bucket written by the old
    // set_formation stat-ranking).
    let group = deployed
        .cloned()
        .unwrap_or_else(|| p.natural_position.clone())
        .to_group_position();
    let pos = match group {
        DomainPosition::Goalkeeper => Position::Goalkeeper,
        DomainPosition::Defender => Position::Defender,
        DomainPosition::Midfielder => Position::Midfielder,
        DomainPosition::Forward => Position::Forward,
        _ => Position::Midfielder,
    };

    PlayerData {
        id: p.id.clone(),
        name: p.match_name.clone(),
        position: pos,
        ovr: p.ovr,
        condition: p.condition,
        fitness: p.fitness,
        pace: p.attributes.pace,
        stamina: p.attributes.stamina,
        strength: p.attributes.strength,
        agility: p.attributes.agility,
        passing: p.attributes.passing,
        shooting: p.attributes.shooting,
        tackling: p.attributes.tackling,
        dribbling: p.attributes.dribbling,
        defending: p.attributes.defending,
        positioning: p.attributes.positioning,
        vision: p.attributes.vision,
        decisions: p.attributes.decisions,
        composure: p.attributes.composure,
        aggression: p.attributes.aggression,
        teamwork: p.attributes.teamwork,
        leadership: p.attributes.leadership,
        handling: p.attributes.handling,
        reflexes: p.attributes.reflexes,
        aerial: p.attributes.aerial,
        traits: p.traits.iter().map(|t| format!("{:?}", t)).collect(),
        role,
    }
}

/// Auto-select set-piece takers from a set of player IDs.
/// Returns (captain_id, penalty_taker_id, free_kick_taker_id, corner_taker_id).
pub fn auto_select_set_pieces(
    game: &Game,
    player_ids: &[String],
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let players: Vec<&domain::player::Player> = player_ids
        .iter()
        .filter_map(|id| game.players.iter().find(|p| &p.id == id))
        .collect();

    if players.is_empty() {
        return (None, None, None, None);
    }

    // Captain: highest leadership + teamwork
    let captain = players
        .iter()
        .max_by_key(|p| (p.attributes.leadership as u16) + (p.attributes.teamwork as u16))
        .map(|p| p.id.clone());

    // Penalty taker: highest shooting + composure (exclude GK)
    let penalty = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| (p.attributes.shooting as u16) + (p.attributes.composure as u16))
        .map(|p| p.id.clone());

    // Free kick taker: highest passing + vision + shooting (exclude GK)
    let free_kick = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| {
            (p.attributes.passing as u16)
                + (p.attributes.vision as u16)
                + (p.attributes.shooting as u16) / 2
        })
        .map(|p| p.id.clone());

    // Corner taker: highest passing + vision (exclude GK, prefer different from FK)
    let corner = players
        .iter()
        .filter(|p| p.position != DomainPosition::Goalkeeper)
        .max_by_key(|p| {
            let base = (p.attributes.passing as u16) + (p.attributes.vision as u16);
            // Small penalty if same as free kick taker to encourage variety
            if free_kick.as_ref() == Some(&p.id) {
                base.saturating_sub(5)
            } else {
                base
            }
        })
        .map(|p| p.id.clone());

    (captain, penalty, free_kick, corner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::player::{Player, PlayerAttributes, Position as DomainPos};

    /// Uniform attributes: `weighted_score` averages attributes, so setting them
    /// all to `v` makes the condition-free positional fit ≈ `v` for any slot, with
    /// the per-slot compatibility/foot penalty identical across players (same
    /// position + footedness) — so it cancels in within-slot comparisons.
    fn attrs(v: u8) -> PlayerAttributes {
        PlayerAttributes {
            pace: v,
            stamina: v,
            strength: v,
            agility: v,
            passing: v,
            shooting: v,
            tackling: v,
            dribbling: v,
            defending: v,
            positioning: v,
            vision: v,
            decisions: v,
            composure: v,
            aggression: v,
            teamwork: v,
            leadership: v,
            handling: v,
            reflexes: v,
            aerial: v,
        }
    }

    fn mk(id: &str, attr: u8, condition: u8) -> Player {
        mk_pos(id, DomainPos::CenterBack, attr, condition)
    }

    fn mk_pos(id: &str, position: DomainPos, attr: u8, condition: u8) -> Player {
        let mut p = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "1998-01-01".to_string(),
            "GB".to_string(),
            position,
            attrs(attr),
        );
        p.condition = condition;
        p
    }

    #[test]
    fn management_quality_maps_reputation_to_unit_range() {
        assert_eq!(management_quality(300), 0.0);
        assert_eq!(management_quality(900), 1.0);
        assert!((management_quality(600) - 0.5).abs() < 1e-9);
        assert_eq!(management_quality(100), 0.0); // clamped below
        assert_eq!(management_quality(1200), 1.0); // clamped above
    }

    /// The discriminating test: an elite club must NOT bench a strong (but mildly
    /// tired) starter for a much weaker fresh player. Better clubs field better
    /// teams — the gap-aware tolerance enforces this.
    #[test]
    fn elite_club_keeps_strong_starters_over_fresh_scrubs() {
        let mut squad = Vec::new();
        for i in 0..11 {
            squad.push(mk(&format!("star{i}"), 80, 70)); // strong, mildly tired
        }
        for i in 0..3 {
            squad.push(mk(&format!("weak{i}"), 50, 100)); // weak, fully fresh
        }
        let refs: Vec<&Player> = squad.iter().collect();

        let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0);

        assert_eq!(xi.len(), 11);
        assert!(
            xi.iter().all(|p| p.id.starts_with("star")),
            "elite club fielded a weak fresh player over a strong starter: {:?}",
            xi.iter().map(|p| &p.id).collect::<Vec<_>>()
        );
    }

    /// An elite club rotates a tired starter for a *comparable* fresh deputy
    /// (within the fit tolerance), but still leaves the much-weaker scrubs benched.
    #[test]
    fn elite_club_rotates_tired_starter_for_comparable_fresh_player() {
        let mut squad = Vec::new();
        for i in 0..11 {
            squad.push(mk(&format!("star{i}"), 80, 65)); // strong, tired
        }
        squad.push(mk("deputy", 72, 100)); // comparable, fresh
        for i in 0..2 {
            squad.push(mk(&format!("weak{i}"), 50, 100)); // scrub, fresh
        }
        let refs: Vec<&Player> = squad.iter().collect();

        let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0);

        assert_eq!(xi.len(), 11);
        assert!(
            xi.iter().any(|p| p.id == "deputy"),
            "comparable fresh deputy should rotate in for a tired star"
        );
        assert_eq!(
            xi.iter().filter(|p| p.id.starts_with("star")).count(),
            10,
            "exactly one tired star should be rested"
        );
        assert!(
            xi.iter().all(|p| !p.id.starts_with("weak")),
            "scrubs are too far below tolerance to be rotated in"
        );
    }

    /// Same squad as above, but a poorly-run club: it rides its tired starters and
    /// does not rotate. Proves the reputation gradient.
    #[test]
    fn low_reputation_club_rides_tired_starters() {
        let mut squad = Vec::new();
        for i in 0..11 {
            squad.push(mk(&format!("star{i}"), 80, 65));
        }
        squad.push(mk("deputy", 72, 100));
        for i in 0..2 {
            squad.push(mk(&format!("weak{i}"), 50, 100));
        }
        let refs: Vec<&Player> = squad.iter().collect();

        let xi = ai_select_starting_xi(&refs, "4-4-2", management_quality(300)); // q = 0

        assert_eq!(xi.len(), 11);
        assert!(
            xi.iter().all(|p| p.id.starts_with("star")),
            "a low-reputation club should ride its tired first XI, not rotate"
        );
    }

    /// Regression: load-management rotation must not skew the formation's
    /// position distribution. A tired XI plus one fresh midfielder must still
    /// field 1 GK / 4 DEF / 4 MID / 2 FWD — never a defender short (which
    /// rendered only 10 players on the pitch).
    #[test]
    fn rotation_preserves_formation_position_distribution() {
        use DomainPos::{
            CentralMidfielder, CenterBack, Forward, Goalkeeper, LeftBack, LeftMidfielder,
            RightBack, RightMidfielder, Striker,
        };
        let squad = vec![
            mk_pos("gk", Goalkeeper, 75, 65),
            mk_pos("d1", CenterBack, 75, 65),
            mk_pos("d2", CenterBack, 75, 65),
            mk_pos("d3", LeftBack, 75, 65),
            mk_pos("d4", RightBack, 75, 65),
            mk_pos("m1", CentralMidfielder, 75, 65),
            mk_pos("m2", CentralMidfielder, 75, 65),
            mk_pos("m3", LeftMidfielder, 75, 65),
            mk_pos("m4", RightMidfielder, 75, 65),
            mk_pos("f1", Striker, 75, 65),
            mk_pos("f2", Striker, 75, 65),
            // Fresh midfielder load management will want to bring in.
            mk_pos("m_fresh", CentralMidfielder, 75, 100),
        ];
        let refs: Vec<&Player> = squad.iter().collect();

        let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0); // elite: rotates eagerly

        assert_eq!(xi.len(), 11);
        let group_count = |group: DomainPos| {
            xi.iter()
                .filter(|p| p.position.to_group_position() == group)
                .count()
        };
        assert_eq!(group_count(Goalkeeper), 1, "exactly one keeper");
        assert_eq!(group_count(DomainPos::Defender), 4, "must field four defenders");
        assert_eq!(group_count(DomainPos::Midfielder), 4, "must field four midfielders");
        assert_eq!(group_count(Forward), 2, "must field two forwards");
    }

    /// When a saved starter is unavailable the user XI compacts; the surviving
    /// starters must keep their real saved slot (matching deployed_position /
    /// the UI), not shift up into the vacated slot. Regression for an injured
    /// keeper turning an outfielder into the engine's goalkeeper.
    #[test]
    fn user_team_starter_keeps_saved_slot_when_xi_compacts() {
        use crate::clock::GameClock;
        use chrono::{TimeZone, Utc};
        use domain::manager::Manager;
        use domain::team::Team;

        let mut players: Vec<Player> = (1..=11)
            .map(|i| mk_pos(&format!("p{i}"), DomainPos::CenterBack, 70, 100))
            .collect();
        // p1 is a natural Forward saved into the LEFT-BACK slot (index 1).
        players[0] = mk_pos("p1", DomainPos::Forward, 70, 100);
        for player in players.iter_mut() {
            player.team_id = Some("user".to_string());
        }

        let mut team = Team::new(
            "user".to_string(),
            "User FC".to_string(),
            "USR".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            25_000,
        );
        team.formation = "4-4-2".to_string();
        // Slot 0 (GK) references a player that no longer exists, so
        // select_starting_xi drops it and the survivors compact.
        team.starting_xi_ids = std::iter::once("ghost-gk".to_string())
            .chain((1..=10).map(|i| format!("p{i}")))
            .collect();

        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("user".to_string());

        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let game = Game::new(clock, manager, vec![team], players, vec![], vec![]);

        let (team_data, _bench) = build_team_with_bench(&game, "user");

        let p1 = team_data
            .players
            .iter()
            .find(|p| p.id == "p1")
            .expect("p1 should be fielded");
        // Saved at the left-back slot -> simulated as a defender, never shoved
        // into the vacated goalkeeper slot.
        assert_eq!(p1.position, Position::Defender);
        assert_ne!(p1.position, Position::Goalkeeper);
    }
}
