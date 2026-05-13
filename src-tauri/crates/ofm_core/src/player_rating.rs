use domain::player::{Footedness, Player, PlayerTrait, Position};
use rand::RngExt;

pub fn formation_slots(formation: &str) -> Vec<Position> {
    formation_slot_rows(formation)
        .into_iter()
        .flatten()
        .collect()
}

/// Refresh a player's derived fields: `ovr`, `potential`, and `traits`.
///
/// - `ovr` is recomputed from the player's natural position using position-weighted attributes.
/// - `potential` is set only if it is currently 0 (unset), using a random bonus based on age.
///   Once set it is preserved so training gains can grow OVR toward the ceiling naturally.
/// - `traits` are recomputed from current attributes, and the `Wonderkid` trait is applied if
///   the player is young with a meaningful gap between current OVR and potential.
///
/// Pass `current_year` for accurate age calculation (use the game clock year).
pub fn refresh_player_derived(player: &mut Player, current_year: u32) {
    // 1. Compute position-weighted OVR
    let ovr_f = natural_ovr(player);
    let ovr = ovr_f.round() as u8;

    // 2. Compute potential if not yet set (initial generation or legacy saves)
    let age = player_age(&player.date_of_birth, current_year);
    let potential = if player.potential == 0 {
        generate_potential(ovr, age)
    } else {
        // Keep existing potential; clamp so it is always >= ovr
        player.potential.max(ovr)
    };

    // 3. Recompute attribute-based traits
    let mut traits = domain::player::compute_traits(&player.attributes, &player.natural_position);

    // 4. Award Wonderkid trait: young player whose ceiling far exceeds current ability
    let growth_room = potential.saturating_sub(ovr);
    if age <= 21 && potential >= 75 && growth_room >= 10 {
        if !traits.contains(&PlayerTrait::Wonderkid) {
            traits.push(PlayerTrait::Wonderkid);
        }
    }

    player.ovr = ovr;
    player.potential = potential;
    player.traits = traits;
}

/// Generate a potential rating for a newly-created player based on current OVR and age.
/// Returns a value in [1, 99] that is always >= `ovr`.
/// The lower bound of 1 (via `ovr.max(1)`) ensures potential is never 0 even when `ovr` is 0.
pub fn generate_potential(ovr: u8, age: u32) -> u8 {
    let mut rng = rand::rng();
    let bonus: u8 = match age {
        ..=18 => rng.random_range(15u8..=30),
        19..=20 => rng.random_range(8u8..=22),
        21..=22 => rng.random_range(4u8..=14),
        23..=25 => rng.random_range(0u8..=7),
        _ => 0,
    };
    (ovr.saturating_add(bonus)).min(99).max(ovr.max(1))
}

/// Parse birth year from a "YYYY-MM-DD" date string and return approximate age.
fn player_age(date_of_birth: &str, current_year: u32) -> u32 {
    let birth_year: u32 = date_of_birth
        .split('-')
        .next()
        .and_then(|y| y.parse().ok())
        .unwrap_or(2000);
    current_year.saturating_sub(birth_year)
}

fn formation_slot_rows(formation: &str) -> Vec<Vec<Position>> {
    let parts: Vec<usize> = formation
        .split('-')
        .filter_map(|part| part.parse::<usize>().ok())
        .collect();

    match parts.as_slice() {
        [defenders, midfielders, forwards] => vec![
            vec![Position::Goalkeeper],
            defender_line(*defenders),
            midfield_line(*midfielders),
            forward_line(*forwards),
        ],
        [defenders, deep_midfielders, attacking_midfielders, forwards] => vec![
            vec![Position::Goalkeeper],
            defender_line(*defenders),
            deep_midfield_line(*deep_midfielders),
            attacking_midfield_line(*attacking_midfielders),
            forward_line(*forwards),
        ],
        _ => formation_slot_rows("4-4-2"),
    }
}

pub fn natural_ovr(player: &Player) -> f64 {
    let natural_position = primary_position(player);
    ovr_for_position(player, &natural_position)
}

pub fn ovr_for_position(player: &Player, position: &Position) -> f64 {
    let canonical = canonical_position(position);
    let base = weighted_score(player, &canonical);
    let penalty = critical_penalty(player, &canonical);
    (base - penalty).clamp(1.0, 99.0)
}

pub fn effective_rating_for_assignment(player: &Player, slot_position: &Position) -> f64 {
    let canonical_slot = canonical_position(slot_position);
    let base = ovr_for_position(player, &canonical_slot);
    let compatibility_penalty = compatibility_penalty(player, &canonical_slot);
    let foot_penalty = footedness_penalty(player, &canonical_slot);
    let adjusted = (base - compatibility_penalty - foot_penalty).max(1.0);
    adjusted * (player.condition as f64 / 100.0)
}

fn defender_line(count: usize) -> Vec<Position> {
    match count {
        3 => vec![
            Position::CenterBack,
            Position::CenterBack,
            Position::CenterBack,
        ],
        4 => vec![
            Position::LeftBack,
            Position::CenterBack,
            Position::CenterBack,
            Position::RightBack,
        ],
        5 => vec![
            Position::LeftWingBack,
            Position::CenterBack,
            Position::CenterBack,
            Position::CenterBack,
            Position::RightWingBack,
        ],
        _ => vec![Position::CenterBack; count],
    }
}

fn midfield_line(count: usize) -> Vec<Position> {
    match count {
        2 => vec![Position::CentralMidfielder, Position::CentralMidfielder],
        3 => vec![
            Position::DefensiveMidfielder,
            Position::CentralMidfielder,
            Position::AttackingMidfielder,
        ],
        4 => vec![
            Position::LeftMidfielder,
            Position::CentralMidfielder,
            Position::CentralMidfielder,
            Position::RightMidfielder,
        ],
        5 => vec![
            Position::LeftMidfielder,
            Position::DefensiveMidfielder,
            Position::CentralMidfielder,
            Position::AttackingMidfielder,
            Position::RightMidfielder,
        ],
        _ => vec![Position::CentralMidfielder; count],
    }
}

fn deep_midfield_line(count: usize) -> Vec<Position> {
    match count {
        1 => vec![Position::DefensiveMidfielder],
        2 => vec![Position::DefensiveMidfielder, Position::CentralMidfielder],
        _ => vec![Position::DefensiveMidfielder; count],
    }
}

fn attacking_midfield_line(count: usize) -> Vec<Position> {
    match count {
        1 => vec![Position::AttackingMidfielder],
        2 => vec![Position::AttackingMidfielder, Position::AttackingMidfielder],
        3 => vec![
            Position::LeftMidfielder,
            Position::AttackingMidfielder,
            Position::RightMidfielder,
        ],
        _ => vec![Position::AttackingMidfielder; count],
    }
}

fn forward_line(count: usize) -> Vec<Position> {
    match count {
        1 => vec![Position::Striker],
        2 => vec![Position::Striker, Position::Striker],
        3 => vec![
            Position::LeftWinger,
            Position::Striker,
            Position::RightWinger,
        ],
        _ => vec![Position::Striker; count],
    }
}

fn primary_position(player: &Player) -> Position {
    let preferred = if player.natural_position.is_legacy_bucket() {
        player.position.clone()
    } else {
        player.natural_position.clone()
    };

    canonical_position(&preferred)
}

fn canonical_position(position: &Position) -> Position {
    match position {
        Position::Goalkeeper => Position::Goalkeeper,
        Position::Defender => Position::CenterBack,
        Position::Midfielder => Position::CentralMidfielder,
        Position::Forward => Position::Striker,
        granular => granular.clone(),
    }
}

fn compatibility_penalty(player: &Player, slot_position: &Position) -> f64 {
    let primary = primary_position(player);
    if &primary == slot_position {
        return 0.0;
    }

    let alternates = player
        .alternate_positions
        .iter()
        .map(canonical_position)
        .collect::<Vec<_>>();

    if alternates.iter().any(|position| position == slot_position) {
        4.0
    } else if primary.to_group_position() == slot_position.to_group_position() {
        8.0
    } else {
        14.0
    }
}

fn footedness_penalty(player: &Player, slot_position: &Position) -> f64 {
    let Some(required_side) = slot_side(slot_position) else {
        return 0.0;
    };

    match (player.footedness, required_side) {
        (Footedness::Both, _) => 0.0,
        (Footedness::Left, Side::Left) | (Footedness::Right, Side::Right) => 0.0,
        _ => (10_i32 - (player.weak_foot.clamp(1, 5) as i32 * 2)).max(0) as f64,
    }
}

fn weighted_score(player: &Player, position: &Position) -> f64 {
    let attrs = &player.attributes;
    match position {
        Position::Goalkeeper => weighted_average(&[
            (attrs.handling, 28),
            (attrs.reflexes, 28),
            (attrs.aerial, 14),
            (attrs.positioning, 10),
            (attrs.decisions, 10),
            (attrs.composure, 5),
            (attrs.strength, 5),
        ]),
        Position::RightBack | Position::LeftBack => weighted_average(&[
            (attrs.pace, 18),
            (attrs.stamina, 16),
            (attrs.tackling, 17),
            (attrs.defending, 16),
            (attrs.positioning, 12),
            (attrs.passing, 10),
            (attrs.dribbling, 6),
            (attrs.decisions, 5),
        ]),
        Position::CenterBack => weighted_average(&[
            (attrs.defending, 24),
            (attrs.tackling, 18),
            (attrs.positioning, 18),
            (attrs.strength, 14),
            (attrs.aerial, 12),
            (attrs.decisions, 8),
            (attrs.composure, 6),
        ]),
        Position::RightWingBack | Position::LeftWingBack => weighted_average(&[
            (attrs.pace, 18),
            (attrs.stamina, 18),
            (attrs.tackling, 14),
            (attrs.defending, 12),
            (attrs.passing, 13),
            (attrs.dribbling, 11),
            (attrs.vision, 7),
            (attrs.decisions, 7),
        ]),
        Position::DefensiveMidfielder => weighted_average(&[
            (attrs.tackling, 18),
            (attrs.positioning, 18),
            (attrs.decisions, 16),
            (attrs.passing, 14),
            (attrs.defending, 12),
            (attrs.stamina, 10),
            (attrs.vision, 7),
            (attrs.strength, 5),
        ]),
        Position::CentralMidfielder => weighted_average(&[
            (attrs.passing, 20),
            (attrs.vision, 16),
            (attrs.decisions, 16),
            (attrs.stamina, 12),
            (attrs.dribbling, 10),
            (attrs.positioning, 9),
            (attrs.teamwork, 9),
            (attrs.tackling, 8),
        ]),
        Position::AttackingMidfielder => weighted_average(&[
            (attrs.vision, 20),
            (attrs.passing, 18),
            (attrs.dribbling, 16),
            (attrs.decisions, 14),
            (attrs.shooting, 10),
            (attrs.positioning, 8),
            (attrs.composure, 8),
            (attrs.pace, 6),
        ]),
        Position::RightMidfielder | Position::LeftMidfielder => weighted_average(&[
            (attrs.pace, 17),
            (attrs.stamina, 16),
            (attrs.passing, 15),
            (attrs.dribbling, 14),
            (attrs.vision, 10),
            (attrs.decisions, 10),
            (attrs.positioning, 10),
            (attrs.tackling, 8),
        ]),
        Position::RightWinger | Position::LeftWinger => weighted_average(&[
            (attrs.pace, 22),
            (attrs.dribbling, 22),
            (attrs.passing, 14),
            (attrs.shooting, 12),
            (attrs.vision, 10),
            (attrs.decisions, 8),
            (attrs.positioning, 6),
            (attrs.stamina, 6),
        ]),
        Position::Striker => weighted_average(&[
            (attrs.shooting, 26),
            (attrs.positioning, 18),
            (attrs.decisions, 14),
            (attrs.pace, 12),
            (attrs.dribbling, 10),
            (attrs.strength, 8),
            (attrs.composure, 8),
            (attrs.aerial, 4),
        ]),
        Position::Defender | Position::Midfielder | Position::Forward => unreachable!(),
    }
}

fn critical_penalty(player: &Player, position: &Position) -> f64 {
    let attrs = &player.attributes;
    let critical_min = match position {
        Position::Goalkeeper => attrs.handling.min(attrs.reflexes).min(attrs.positioning),
        Position::RightBack | Position::LeftBack => {
            attrs.tackling.min(attrs.defending).min(attrs.positioning)
        }
        Position::CenterBack => attrs.defending.min(attrs.tackling).min(attrs.positioning),
        Position::RightWingBack | Position::LeftWingBack => {
            attrs.pace.min(attrs.stamina).min(attrs.tackling)
        }
        Position::DefensiveMidfielder => attrs.tackling.min(attrs.positioning).min(attrs.passing),
        Position::CentralMidfielder => attrs.passing.min(attrs.vision).min(attrs.decisions),
        Position::AttackingMidfielder => attrs.vision.min(attrs.passing).min(attrs.dribbling),
        Position::RightMidfielder | Position::LeftMidfielder => {
            attrs.pace.min(attrs.passing).min(attrs.stamina)
        }
        Position::RightWinger | Position::LeftWinger => {
            attrs.pace.min(attrs.dribbling).min(attrs.passing)
        }
        Position::Striker => attrs.shooting.min(attrs.positioning).min(attrs.decisions),
        Position::Defender | Position::Midfielder | Position::Forward => 50,
    };

    if critical_min >= 45 {
        0.0
    } else {
        (45 - critical_min) as f64 * 0.6
    }
}

fn weighted_average(values: &[(u8, i32)]) -> f64 {
    values
        .iter()
        .map(|(value, weight)| *value as f64 * *weight as f64)
        .sum::<f64>()
        / 100.0
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

fn slot_side(position: &Position) -> Option<Side> {
    match position {
        Position::LeftBack
        | Position::LeftWingBack
        | Position::LeftMidfielder
        | Position::LeftWinger => Some(Side::Left),
        Position::RightBack
        | Position::RightWingBack
        | Position::RightMidfielder
        | Position::RightWinger => Some(Side::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::player::PlayerAttributes;

    fn make_player(position: Position) -> Player {
        Player::new(
            "p-1".to_string(),
            "Test".to_string(),
            "Test Player".to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            position,
            PlayerAttributes {
                pace: 70,
                stamina: 70,
                strength: 70,
                agility: 70,
                passing: 70,
                shooting: 70,
                tackling: 70,
                dribbling: 70,
                defending: 70,
                positioning: 70,
                vision: 70,
                decisions: 70,
                composure: 70,
                aggression: 70,
                teamwork: 70,
                leadership: 70,
                handling: 20,
                reflexes: 20,
                aerial: 70,
            },
        )
    }

    #[test]
    fn formation_slots_return_exact_role_layout() {
        assert_eq!(
            formation_slots("4-4-2"),
            vec![
                Position::Goalkeeper,
                Position::LeftBack,
                Position::CenterBack,
                Position::CenterBack,
                Position::RightBack,
                Position::LeftMidfielder,
                Position::CentralMidfielder,
                Position::CentralMidfielder,
                Position::RightMidfielder,
                Position::Striker,
                Position::Striker,
            ]
        );
    }

    #[test]
    fn role_specific_rating_favors_matching_profile() {
        let mut player = make_player(Position::CenterBack);
        player.natural_position = Position::CenterBack;
        player.attributes.defending = 88;
        player.attributes.tackling = 84;
        player.attributes.positioning = 82;
        player.attributes.strength = 80;
        player.attributes.passing = 55;
        player.attributes.vision = 50;
        player.attributes.shooting = 40;
        player.attributes.dribbling = 44;

        assert!(
            ovr_for_position(&player, &Position::CenterBack)
                > ovr_for_position(&player, &Position::Striker)
        );
    }

    #[test]
    fn assignment_penalty_drops_wrong_side_fullback_more_with_poor_weak_foot() {
        let mut player = make_player(Position::RightBack);
        player.natural_position = Position::RightBack;
        player.footedness = Footedness::Right;
        player.weak_foot = 1;
        player.attributes.tackling = 82;
        player.attributes.defending = 80;
        player.attributes.positioning = 78;
        player.attributes.pace = 81;
        player.attributes.stamina = 79;

        let same_side = effective_rating_for_assignment(&player, &Position::RightBack);
        let wrong_side = effective_rating_for_assignment(&player, &Position::LeftBack);

        assert!(same_side > wrong_side);
    }

    #[test]
    fn alternate_positions_reduce_assignment_penalty() {
        let mut player = make_player(Position::CentralMidfielder);
        player.natural_position = Position::CentralMidfielder;
        player.alternate_positions = vec![Position::AttackingMidfielder];
        player.attributes.passing = 82;
        player.attributes.vision = 84;
        player.attributes.decisions = 78;
        player.attributes.dribbling = 76;

        let alternate_role =
            effective_rating_for_assignment(&player, &Position::AttackingMidfielder);
        let out_of_group_role = effective_rating_for_assignment(&player, &Position::RightBack);

        assert!(alternate_role > out_of_group_role);
    }
}
