use crate::game::Game;
use crate::player_rating::{effective_rating_for_assignment, formation_slots, natural_ovr};
use domain::player::Position as DomainPosition;
use engine::{
    DefensiveLine, MarkingStyle, PlayStyle, PlayerData, PlayerRole as EnginePlayerRole, Position,
    PressingIntensity, TacticsBuildUpStyle, TacticsConfig, TacticsPitchWidth, TeamData,
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
    let convert_player = |p: &domain::player::Player| {
        let role = player_roles
            .and_then(|roles| roles.get(&p.id))
            .map(domain_to_engine_role)
            .unwrap_or(EnginePlayerRole::Standard);
        to_engine_player(p, role)
    };

    let starting_players = select_starting_xi(saved_xi_ids, &available_players, &formation);
    let used_ids: HashSet<String> = starting_players
        .iter()
        .map(|player| player.id.clone())
        .collect();
    let starting_xi = starting_players.into_iter().map(|p| convert_player(p)).collect();

    let mut bench_domain: Vec<&domain::player::Player> = available_players
        .into_iter()
        .filter(|player| !used_ids.contains(&player.id))
        .collect();
    bench_domain.sort_by(|left, right| {
        natural_ovr(right)
            .partial_cmp(&natural_ovr(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let bench = bench_domain.into_iter().map(|p| convert_player(p)).collect();

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
    }
}

fn to_engine_player(p: &domain::player::Player, role: EnginePlayerRole) -> PlayerData {
    let pos = match p.position.to_group_position() {
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
