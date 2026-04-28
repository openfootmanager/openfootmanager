use crate::game::Game;
use crate::player_rating::{effective_rating_for_assignment, formation_slots, natural_ovr};
use domain::player::Position as DomainPosition;
use engine::{NaturalPosition, PlayStyle, PlayerData, Position, TeamData};

// ---------------------------------------------------------------------------
// Domain → Engine conversion with starting XI / bench split
// ---------------------------------------------------------------------------

pub(super) fn build_team_with_bench(game: &Game, team_id: &str) -> (TeamData, Vec<PlayerData>) {
    let team = game.teams.iter().find(|t| t.id == team_id);
    let (name, formation, play_style, saved_xi_ids) = match team {
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
            t.starting_xi_ids.clone(),
        ),
        None => ("Unknown".into(), "4-4-2".into(), PlayStyle::Balanced, Vec::new()),
    };

    // Collect all available (non-injured) players for this team
    let available_players: Vec<&domain::player::Player> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id) && p.injury.is_none())
        .collect();

    let by_id: std::collections::HashMap<&str, &domain::player::Player> = available_players
        .iter()
        .map(|player| (player.id.as_str(), *player))
        .collect();

    let mut used_ids = std::collections::HashSet::new();
    let mut starting_xi = Vec::with_capacity(11);
    let slots = formation_slots(&formation);

    // Use saved starting XI if available and valid (at least 8 players)
    let mut valid_saved_ids = Vec::new();
    for id in saved_xi_ids {
        if by_id.contains_key(id.as_str()) && used_ids.insert(id.clone()) {
            valid_saved_ids.push(id.clone());
        }
    }

    if valid_saved_ids.len() >= 8 {
        // Use saved XI as base
        for id in &valid_saved_ids {
            if let Some(player) = by_id.get(id.as_str()) {
                starting_xi.push(to_engine_player(player));
            }
        }

        // Fill remaining slots with best available players
        let mut remaining_players: Vec<&domain::player::Player> = available_players
            .iter()
            .copied()
            .filter(|player| !used_ids.contains(&player.id))
            .collect();
        remaining_players.sort_by(|left, right| {
            natural_ovr(right)
                .partial_cmp(&natural_ovr(left))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        while starting_xi.len() < 11 {
            let slot = slots.get(starting_xi.len());
            let best_index = remaining_players
                .iter()
                .enumerate()
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

            let player = remaining_players.remove(best_index);
            used_ids.insert(player.id.clone());
            starting_xi.push(to_engine_player(player));
        }
    } else {
        // Auto-select best players by rating (legacy behavior for empty/invalid XI)
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
            starting_xi.push(to_engine_player(player));
        }
    }

    let mut bench_domain: Vec<&domain::player::Player> = available_players
        .into_iter()
        .filter(|player| !used_ids.contains(&player.id))
        .collect();
    bench_domain.sort_by(|left, right| {
        natural_ovr(right)
            .partial_cmp(&natural_ovr(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let bench = bench_domain.into_iter().map(to_engine_player).collect();

    let team_data = TeamData {
        id: team_id.to_string(),
        name,
        formation,
        play_style,
        players: starting_xi,
    };

    (team_data, bench)
}

fn to_engine_player(p: &domain::player::Player) -> PlayerData {
    // Map granular domain position to engine NaturalPosition
    let natural_pos = match p.position {
        DomainPosition::Goalkeeper => NaturalPosition::Goalkeeper,
        DomainPosition::RightBack => NaturalPosition::RightBack,
        DomainPosition::CenterBack => NaturalPosition::CenterBack,
        DomainPosition::LeftBack => NaturalPosition::LeftBack,
        DomainPosition::RightWingBack => NaturalPosition::RightWingBack,
        DomainPosition::LeftWingBack => NaturalPosition::LeftWingBack,
        DomainPosition::DefensiveMidfielder => NaturalPosition::DefensiveMidfielder,
        DomainPosition::CentralMidfielder => NaturalPosition::CentralMidfielder,
        DomainPosition::AttackingMidfielder => NaturalPosition::AttackingMidfielder,
        DomainPosition::RightMidfielder => NaturalPosition::RightMidfielder,
        DomainPosition::LeftMidfielder => NaturalPosition::LeftMidfielder,
        DomainPosition::RightWinger => NaturalPosition::RightWinger,
        DomainPosition::LeftWinger => NaturalPosition::LeftWinger,
        DomainPosition::Striker => NaturalPosition::Striker,
        // Legacy bucket positions — fallback to a reasonable default
        DomainPosition::Defender => NaturalPosition::CenterBack,
        DomainPosition::Midfielder => NaturalPosition::CentralMidfielder,
        DomainPosition::Forward => NaturalPosition::Striker,
    };

    // Derive group position from natural position
    let pos = natural_pos.to_group_position();

    PlayerData {
        id: p.id.clone(),
        name: p.match_name.clone(),
        position: pos,
        natural_position: natural_pos,
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
