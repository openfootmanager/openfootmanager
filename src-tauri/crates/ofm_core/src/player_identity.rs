use crate::game::Game;
use crate::player_rating::{formation_slots, natural_ovr};
use domain::player::{Footedness, Player, Position};
use std::collections::HashMap;

pub fn upgrade_game_player_identities(game: &mut Game) -> bool {
    let slot_map = build_assigned_slot_map(game);
    let mut changed = false;

    let on = game.clock.current_date.date_naive();

    for player in &mut game.players {
        if upgrade_player_identity(player, slot_map.get(&player.id)) {
            changed = true;
            // The upgrade re-rated him, and the wonderkid tag is a function of his rating.
            crate::player_rating::reconcile_wonderkid_trait(player, on);
        }
    }

    changed
}

pub fn upgrade_player_identity(player: &mut Player, assigned_slot: Option<&Position>) -> bool {
    if !needs_identity_upgrade(player) {
        return false;
    }

    let natural_position = infer_natural_position(player, assigned_slot);
    let alternate_positions = infer_alternate_positions(player, &natural_position, assigned_slot);
    let footedness = infer_footedness(player, &natural_position, assigned_slot);
    let weak_foot = infer_weak_foot(player, &alternate_positions, footedness);

    let changed = player.natural_position != natural_position
        || player.alternate_positions != alternate_positions
        || player.footedness != footedness
        || player.weak_foot != weak_foot;

    player.natural_position = natural_position;
    player.alternate_positions = alternate_positions;
    player.footedness = footedness;
    player.weak_foot = weak_foot;
    // The rating is the one the formula gives at the natural position, which is what the daily
    // refresh computes: a generated player was rated at his coarse slot position, and left so he
    // would jump (about 1.4 points on average) the first time the game refreshed him.
    //
    // A player with no rating at all (a save from before ratings existed) is left to the loader's
    // age-based backfill, which also draws his ceiling.
    let rated_changed = refresh_rating_to_natural_position(player);

    changed || rated_changed
}

/// Sets `player.ovr` to the formula's rating at his natural position and floors a ceiling he
/// already has at it, as the daily refresh does. `true` when either changed.
fn refresh_rating_to_natural_position(player: &mut Player) -> bool {
    if player.ovr == 0 {
        return false;
    }
    let rated = natural_ovr(player).round() as u8;
    let ceiling = if player.potential == 0 {
        0
    } else {
        player.potential.max(rated)
    };
    let changed = player.ovr != rated || player.potential != ceiling;
    player.ovr = rated;
    player.potential = ceiling;
    changed
}

fn needs_identity_upgrade(player: &Player) -> bool {
    // `player.position` intentionally stays as a coarse legacy bucket (the
    // player's group). Only trigger re-inference when the granular fields
    // — natural_position or alternate_positions — still hold a legacy value,
    // meaning the initial upgrade hasn't run for this player yet.
    //
    // Previously this also checked `player.position.is_legacy_bucket()`,
    // which is always true by design and made the upgrade re-run on every
    // save-load. That let `natural_position` flip between reloads when a
    // player's attribute scores or XI-slot assignment drifted across the
    // striker-vs-winger threshold (or similar).
    player.natural_position.is_legacy_bucket()
        || player
            .alternate_positions
            .iter()
            .any(Position::is_legacy_bucket)
}

fn build_assigned_slot_map(game: &Game) -> HashMap<String, Position> {
    let mut slot_map = HashMap::new();

    for team in &game.teams {
        let slots = formation_slots(&team.formation);
        for (index, player_id) in team.starting_xi_ids.iter().enumerate() {
            if let Some(slot) = slots.get(index) {
                slot_map.insert(player_id.clone(), slot.clone());
            }
        }
    }

    slot_map
}

fn infer_natural_position(player: &Player, assigned_slot: Option<&Position>) -> Position {
    let group = player.position.to_group_position();

    if let Some(slot) = assigned_slot
        && !slot.is_legacy_bucket()
        && slot.to_group_position() == group
    {
        return slot.clone();
    }

    match group {
        Position::Goalkeeper => Position::Goalkeeper,
        Position::Defender => infer_defender_position(player, assigned_slot),
        Position::Midfielder => infer_midfielder_position(player, assigned_slot),
        Position::Forward => infer_forward_position(player, assigned_slot),
        granular => granular,
    }
}

fn infer_defender_position(player: &Player, assigned_slot: Option<&Position>) -> Position {
    let cb = score_position(player, &Position::CenterBack);
    let fb = score_position(player, &Position::RightBack);
    let wb = score_position(player, &Position::RightWingBack);
    let prefers_left = infer_left_side(player, assigned_slot);

    if cb >= fb.max(wb) + 6 {
        Position::CenterBack
    } else if wb > fb + 4 {
        if prefers_left {
            Position::LeftWingBack
        } else {
            Position::RightWingBack
        }
    } else if prefers_left {
        Position::LeftBack
    } else {
        Position::RightBack
    }
}

fn infer_midfielder_position(player: &Player, assigned_slot: Option<&Position>) -> Position {
    let dm = score_position(player, &Position::DefensiveMidfielder);
    let cm = score_position(player, &Position::CentralMidfielder);
    let am = score_position(player, &Position::AttackingMidfielder);
    let wide = score_position(player, &Position::RightMidfielder);
    let prefers_left = infer_left_side(player, assigned_slot);

    if wide > dm.max(cm).max(am) + 5 {
        if prefers_left {
            Position::LeftMidfielder
        } else {
            Position::RightMidfielder
        }
    } else if am >= dm.max(cm) + 4 {
        Position::AttackingMidfielder
    } else if dm > cm + 3 {
        Position::DefensiveMidfielder
    } else {
        Position::CentralMidfielder
    }
}

fn infer_forward_position(player: &Player, assigned_slot: Option<&Position>) -> Position {
    let striker = score_position(player, &Position::Striker);
    let wide = score_position(player, &Position::RightWinger);
    let prefers_left = infer_left_side(player, assigned_slot);

    if wide > striker + 5 {
        if prefers_left {
            Position::LeftWinger
        } else {
            Position::RightWinger
        }
    } else {
        Position::Striker
    }
}

fn infer_alternate_positions(
    player: &Player,
    natural_position: &Position,
    assigned_slot: Option<&Position>,
) -> Vec<Position> {
    let natural_score = score_position(player, natural_position);
    let candidates = candidate_alternate_positions(natural_position, assigned_slot);
    let mut alternates = Vec::new();

    for candidate in candidates {
        if candidate == *natural_position || alternates.contains(&candidate) {
            continue;
        }

        let candidate_score = score_position(player, &candidate);
        if candidate_score + 8 >= natural_score {
            alternates.push(candidate);
        }

        if alternates.len() == 2 {
            break;
        }
    }

    alternates
}

fn candidate_alternate_positions(
    natural_position: &Position,
    assigned_slot: Option<&Position>,
) -> Vec<Position> {
    let mut candidates = match natural_position {
        Position::Goalkeeper => vec![],
        Position::RightBack => vec![
            Position::RightWingBack,
            Position::CenterBack,
            Position::LeftBack,
        ],
        Position::CenterBack => vec![
            Position::RightBack,
            Position::LeftBack,
            Position::DefensiveMidfielder,
        ],
        Position::LeftBack => vec![
            Position::LeftWingBack,
            Position::CenterBack,
            Position::RightBack,
        ],
        Position::RightWingBack => vec![
            Position::RightBack,
            Position::RightMidfielder,
            Position::LeftWingBack,
        ],
        Position::LeftWingBack => vec![
            Position::LeftBack,
            Position::LeftMidfielder,
            Position::RightWingBack,
        ],
        Position::DefensiveMidfielder => vec![Position::CentralMidfielder, Position::CenterBack],
        Position::CentralMidfielder => {
            vec![Position::DefensiveMidfielder, Position::AttackingMidfielder]
        }
        Position::AttackingMidfielder => vec![Position::CentralMidfielder, Position::Striker],
        Position::RightMidfielder => vec![
            Position::RightWinger,
            Position::CentralMidfielder,
            Position::LeftMidfielder,
        ],
        Position::LeftMidfielder => vec![
            Position::LeftWinger,
            Position::CentralMidfielder,
            Position::RightMidfielder,
        ],
        Position::RightWinger => vec![
            Position::Striker,
            Position::LeftWinger,
            Position::RightMidfielder,
        ],
        Position::LeftWinger => vec![
            Position::Striker,
            Position::RightWinger,
            Position::LeftMidfielder,
        ],
        Position::Striker => vec![
            Position::AttackingMidfielder,
            Position::RightWinger,
            Position::LeftWinger,
        ],
        Position::Defender => vec![Position::CenterBack],
        Position::Midfielder => vec![Position::CentralMidfielder],
        Position::Forward => vec![Position::Striker],
    };

    if let Some(slot) = assigned_slot
        && !slot.is_legacy_bucket()
        && !candidates.contains(slot)
        && *slot != *natural_position
    {
        candidates.insert(0, slot.clone());
    }

    candidates
}

fn infer_footedness(
    player: &Player,
    natural_position: &Position,
    assigned_slot: Option<&Position>,
) -> Footedness {
    if let Some(side_foot) = side_foot_from_position(natural_position) {
        return side_foot;
    }

    if let Some(slot) = assigned_slot
        && let Some(side_foot) = side_foot_from_position(slot)
    {
        return side_foot;
    }

    let hash = stable_hash(&player.id);
    if hash.is_multiple_of(20) {
        Footedness::Both
    } else if hash.is_multiple_of(5) {
        Footedness::Left
    } else {
        Footedness::Right
    }
}

fn infer_weak_foot(
    player: &Player,
    alternate_positions: &[Position],
    footedness: Footedness,
) -> u8 {
    if footedness == Footedness::Both {
        return 5;
    }

    let technical_balance = average(&[
        player.attributes.passing,
        player.attributes.dribbling,
        player.attributes.decisions,
        player.attributes.composure,
        player.attributes.teamwork,
    ]);

    if alternate_positions.len() >= 2 || technical_balance >= 78 {
        4
    } else if !alternate_positions.is_empty() || technical_balance >= 68 {
        3
    } else {
        2
    }
}

fn score_position(player: &Player, position: &Position) -> i32 {
    let attrs = &player.attributes;
    match position {
        Position::Goalkeeper => weighted_sum(&[
            (attrs.handling, 30),
            (attrs.reflexes, 30),
            (attrs.aerial, 15),
            (attrs.positioning, 10),
            (attrs.decisions, 10),
            (attrs.strength, 5),
        ]),
        Position::RightBack | Position::LeftBack => weighted_sum(&[
            (attrs.pace, 22),
            (attrs.stamina, 18),
            (attrs.tackling, 18),
            (attrs.defending, 18),
            (attrs.passing, 12),
            (attrs.dribbling, 7),
            (attrs.positioning, 5),
        ]),
        Position::CenterBack => weighted_sum(&[
            (attrs.defending, 26),
            (attrs.tackling, 18),
            (attrs.positioning, 18),
            (attrs.strength, 16),
            (attrs.aerial, 12),
            (attrs.decisions, 10),
        ]),
        Position::RightWingBack | Position::LeftWingBack => weighted_sum(&[
            (attrs.pace, 20),
            (attrs.stamina, 20),
            (attrs.tackling, 14),
            (attrs.defending, 12),
            (attrs.passing, 14),
            (attrs.dribbling, 12),
            (attrs.vision, 8),
        ]),
        Position::DefensiveMidfielder => weighted_sum(&[
            (attrs.tackling, 20),
            (attrs.positioning, 20),
            (attrs.decisions, 18),
            (attrs.stamina, 14),
            (attrs.passing, 14),
            (attrs.strength, 9),
            (attrs.vision, 5),
        ]),
        Position::CentralMidfielder => weighted_sum(&[
            (attrs.passing, 22),
            (attrs.vision, 18),
            (attrs.decisions, 18),
            (attrs.stamina, 14),
            (attrs.dribbling, 10),
            (attrs.positioning, 10),
            (attrs.tackling, 8),
        ]),
        Position::AttackingMidfielder => weighted_sum(&[
            (attrs.vision, 22),
            (attrs.passing, 20),
            (attrs.dribbling, 18),
            (attrs.decisions, 14),
            (attrs.shooting, 10),
            (attrs.positioning, 8),
            (attrs.pace, 8),
        ]),
        Position::RightMidfielder | Position::LeftMidfielder => weighted_sum(&[
            (attrs.pace, 20),
            (attrs.stamina, 18),
            (attrs.passing, 16),
            (attrs.dribbling, 16),
            (attrs.vision, 12),
            (attrs.decisions, 10),
            (attrs.tackling, 8),
        ]),
        Position::RightWinger | Position::LeftWinger => weighted_sum(&[
            (attrs.pace, 24),
            (attrs.dribbling, 24),
            (attrs.passing, 15),
            (attrs.shooting, 12),
            (attrs.vision, 10),
            (attrs.decisions, 8),
            (attrs.stamina, 7),
        ]),
        Position::Striker => weighted_sum(&[
            (attrs.shooting, 30),
            (attrs.positioning, 20),
            (attrs.decisions, 15),
            (attrs.pace, 10),
            (attrs.dribbling, 10),
            (attrs.strength, 10),
            (attrs.aerial, 5),
        ]),
        Position::Defender => score_position(player, &Position::CenterBack),
        Position::Midfielder => score_position(player, &Position::CentralMidfielder),
        Position::Forward => score_position(player, &Position::Striker),
    }
}

fn weighted_sum(values: &[(u8, i32)]) -> i32 {
    values
        .iter()
        .map(|(value, weight)| *value as i32 * *weight)
        .sum::<i32>()
        / 100
}

fn average(values: &[u8]) -> i32 {
    values.iter().map(|value| *value as i32).sum::<i32>() / values.len() as i32
}

fn infer_left_side(player: &Player, assigned_slot: Option<&Position>) -> bool {
    if let Some(slot) = assigned_slot {
        match slot {
            Position::LeftBack
            | Position::LeftWingBack
            | Position::LeftMidfielder
            | Position::LeftWinger => return true,
            Position::RightBack
            | Position::RightWingBack
            | Position::RightMidfielder
            | Position::RightWinger => return false,
            _ => {}
        }
    }

    stable_hash(&player.id).is_multiple_of(2)
}

fn side_foot_from_position(position: &Position) -> Option<Footedness> {
    match position {
        Position::LeftBack
        | Position::LeftWingBack
        | Position::LeftMidfielder
        | Position::LeftWinger => Some(Footedness::Left),
        Position::RightBack
        | Position::RightWingBack
        | Position::RightMidfielder
        | Position::RightWinger => Some(Footedness::Right),
        _ => None,
    }
}

fn stable_hash(value: &str) -> u64 {
    value.bytes().fold(0_u64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as u64)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::PlayerAttributes;
    use domain::team::Team;

    fn make_player(id: &str, position: Position, attrs: PlayerAttributes) -> Player {
        Player::new(
            id.to_string(),
            format!("{}. Test", id),
            format!("{} Test", id),
            "2000-01-01".to_string(),
            "GB".to_string(),
            position,
            attrs,
        )
    }

    fn make_team() -> Team {
        Team::new(
            "team-1".to_string(),
            "Test FC".to_string(),
            "TFC".to_string(),
            "GB".to_string(),
            "London".to_string(),
            "Test Stadium".to_string(),
            25000,
        )
    }

    fn make_manager() -> Manager {
        Manager::new(
            "mgr-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        )
    }

    /// Given a generated defender whose stored overall was set at his coarse position (here
    ///       deliberately far from what his attributes score),
    /// When he is given his specific natural position,
    /// Then his stored overall is what the rating formula computes for that position, so the
    ///      first rating refresh has nothing to change.
    #[test]
    fn upgrading_a_players_identity_refreshes_his_overall_to_the_natural_position() {
        let attrs = PlayerAttributes {
            pace: 86,
            stamina: 84,
            strength: 66,
            agility: 74,
            passing: 62,
            shooting: 40,
            tackling: 78,
            dribbling: 63,
            defending: 73,
            positioning: 68,
            vision: 55,
            decisions: 64,
            composure: 61,
            aggression: 66,
            teamwork: 72,
            leadership: 50,
            handling: 20,
            reflexes: 20,
            aerial: 48,
        };
        let mut player = make_player("stale-rb", Position::Defender, attrs);
        player.ovr = 50;
        player.potential = 50;

        upgrade_player_identity(&mut player, Some(&Position::RightBack));

        let natural = crate::player_rating::natural_ovr(&player).round() as u8;
        assert_ne!(
            natural, 50,
            "the fixture must tell the stale value from the formula's"
        );
        assert_eq!(player.ovr, natural);
        assert!(
            player.potential >= player.ovr,
            "a ceiling is never below the rating"
        );
    }

    /// Given a goalkeeper (whose specific position is his coarse one) with a stale stored overall,
    /// When his identity is upgraded,
    /// Then his overall is corrected and the call reports a change, so a save loader persists it.
    #[test]
    fn a_rating_correction_alone_is_reported_as_a_change() {
        let attrs = PlayerAttributes {
            pace: 50,
            stamina: 60,
            strength: 60,
            agility: 70,
            passing: 55,
            shooting: 30,
            tackling: 30,
            dribbling: 30,
            defending: 40,
            positioning: 70,
            vision: 55,
            decisions: 66,
            composure: 62,
            aggression: 50,
            teamwork: 60,
            leadership: 50,
            handling: 78,
            reflexes: 80,
            aerial: 72,
        };
        let mut player = make_player("stale-gk", Position::Goalkeeper, attrs);
        let formula = crate::player_rating::natural_ovr(&player).round() as u8;
        player.ovr = formula.saturating_sub(7);

        let changed = upgrade_player_identity(&mut player, None);

        assert_eq!(player.ovr, formula);
        assert!(changed, "a corrected rating must be reported");
    }

    /// Given a legacy player with no rating and no ceiling,
    /// When his identity is upgraded,
    /// Then both stay unset, so the loader's age-based backfill draws his ceiling.
    #[test]
    fn a_player_with_no_rating_yet_keeps_it_unset() {
        let attrs = PlayerAttributes {
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
            aerial: 60,
        };
        let mut player = make_player("pre-rating", Position::Midfielder, attrs);
        player.ovr = 0;
        player.potential = 0;

        upgrade_player_identity(&mut player, None);

        assert_eq!((player.ovr, player.potential), (0, 0));
    }

    #[test]
    fn upgrade_player_identity_infers_granular_defender_profile() {
        let attrs = PlayerAttributes {
            pace: 86,
            stamina: 84,
            strength: 66,
            agility: 74,
            passing: 62,
            shooting: 40,
            tackling: 78,
            dribbling: 63,
            defending: 73,
            positioning: 68,
            vision: 55,
            decisions: 64,
            composure: 61,
            aggression: 66,
            teamwork: 72,
            leadership: 50,
            handling: 20,
            reflexes: 20,
            aerial: 48,
        };
        let mut player = make_player("legacy-rb", Position::Defender, attrs);

        let changed = upgrade_player_identity(&mut player, Some(&Position::RightBack));

        assert!(changed);
        assert_eq!(player.natural_position, Position::RightBack);
        assert_eq!(player.footedness, Footedness::Right);
        assert!(player.weak_foot >= 2);
    }

    #[test]
    fn upgrade_player_identity_keeps_specialists_narrow() {
        let attrs = PlayerAttributes {
            pace: 58,
            stamina: 70,
            strength: 84,
            agility: 55,
            passing: 48,
            shooting: 35,
            tackling: 81,
            dribbling: 40,
            defending: 86,
            positioning: 82,
            vision: 44,
            decisions: 68,
            composure: 60,
            aggression: 73,
            teamwork: 64,
            leadership: 58,
            handling: 20,
            reflexes: 20,
            aerial: 80,
        };
        let mut player = make_player("legacy-cb", Position::Defender, attrs);

        upgrade_player_identity(&mut player, Some(&Position::CenterBack));

        assert_eq!(player.natural_position, Position::CenterBack);
        assert!(player.alternate_positions.len() <= 1);
        assert!(player.footedness != Footedness::Both);
    }

    #[test]
    fn upgrade_player_identity_does_not_rerun_when_natural_position_is_already_granular() {
        // Regression for the Kevin Müller flip: `player.position` stays as a
        // coarse legacy bucket (Forward) by design, but a granular
        // `natural_position` (LeftWinger) means the initial upgrade has
        // already happened. The guard must not re-run inference — otherwise
        // a border-line attribute score or a new XI-slot assignment can
        // flip the natural on every save-load.
        let attrs = PlayerAttributes {
            pace: 80,
            stamina: 74,
            strength: 66,
            agility: 78,
            passing: 70,
            shooting: 75,
            tackling: 40,
            dribbling: 82,
            defending: 34,
            positioning: 74,
            vision: 68,
            decisions: 70,
            composure: 72,
            aggression: 58,
            teamwork: 66,
            leadership: 50,
            handling: 20,
            reflexes: 20,
            aerial: 60,
        };
        let mut player = make_player("p-lw", Position::Forward, attrs);
        // Simulate a save where the initial upgrade has already run.
        player.natural_position = Position::LeftWinger;
        player.alternate_positions = vec![Position::Striker, Position::RightWinger];
        player.footedness = Footedness::Left;
        player.weak_foot = 3;

        // Even with an assigned XI slot that would score to a *different*
        // natural (Striker) under fresh inference, the upgrade must be a
        // no-op because natural_position is already granular.
        let changed = upgrade_player_identity(&mut player, Some(&Position::Striker));

        assert!(
            !changed,
            "should not re-upgrade a player with a granular natural"
        );
        assert_eq!(player.natural_position, Position::LeftWinger);
        assert_eq!(
            player.alternate_positions,
            vec![Position::Striker, Position::RightWinger]
        );
    }

    #[test]
    fn upgrade_player_identity_still_runs_when_alternates_contain_legacy_bucket() {
        // If a save was written by an older build that put a coarse bucket
        // into alternate_positions, the upgrade should still run so the
        // player ends up with fully granular fields.
        let attrs = PlayerAttributes {
            pace: 84,
            stamina: 82,
            strength: 63,
            agility: 72,
            passing: 64,
            shooting: 40,
            tackling: 77,
            dribbling: 62,
            defending: 72,
            positioning: 66,
            vision: 58,
            decisions: 64,
            composure: 60,
            aggression: 64,
            teamwork: 74,
            leadership: 44,
            handling: 20,
            reflexes: 20,
            aerial: 46,
        };
        let mut player = make_player("p-mixed", Position::Defender, attrs);
        player.natural_position = Position::RightBack;
        // Legacy bucket sneaking in — must trigger re-inference.
        player.alternate_positions = vec![Position::Midfielder];

        let changed = upgrade_player_identity(&mut player, Some(&Position::RightBack));

        assert!(changed);
        assert!(
            player
                .alternate_positions
                .iter()
                .all(|position| !position.is_legacy_bucket()),
            "alternates should be fully granular after upgrade",
        );
    }

    /// Given a 19-year-old with an S-tier ceiling whose stored overall was stale-high (so he did
    ///       not qualify as a wonderkid), and whose rating at his natural position is much lower,
    /// When the game's players are given their identities,
    /// Then his wonderkid tag follows his new rating: he is tagged, as the rule says.
    #[test]
    fn re_rating_a_player_at_career_open_reconciles_his_wonderkid_tag() {
        let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let attrs = PlayerAttributes {
            pace: 62,
            stamina: 62,
            strength: 62,
            agility: 62,
            passing: 62,
            shooting: 62,
            tackling: 62,
            dribbling: 62,
            defending: 62,
            positioning: 62,
            vision: 62,
            decisions: 62,
            composure: 62,
            aggression: 62,
            teamwork: 62,
            leadership: 62,
            handling: 20,
            reflexes: 20,
            aerial: 62,
        };
        let mut prospect = make_player("prospect", Position::Midfielder, attrs);
        prospect.date_of_birth = "2007-03-01".to_string();
        prospect.team_id = Some("team-1".to_string());
        prospect.ovr = 80;
        prospect.potential = 85;
        prospect
            .traits
            .retain(|t| *t != domain::player::PlayerTrait::Wonderkid);
        let mut game = Game::new(
            GameClock::new(start),
            make_manager(),
            vec![make_team()],
            vec![prospect],
            vec![],
            vec![],
        );

        upgrade_game_player_identities(&mut game);

        let player = &game.players[0];
        let age = domain::age::playing_age(start.date_naive(), &player.date_of_birth);
        assert!(
            player.ovr < 71,
            "the fixture must re-rate him well below his ceiling"
        );
        assert!(crate::player_rating::qualifies_for_wonderkid(
            age,
            player.potential,
            player.ovr
        ));
        assert!(
            player
                .traits
                .contains(&domain::player::PlayerTrait::Wonderkid),
            "his tag must follow his new rating"
        );
    }

    #[test]
    fn upgrade_game_player_identities_uses_team_slot_context() {
        let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let clock = GameClock::new(start);
        let mut team = make_team();
        team.formation = "4-4-2".to_string();
        team.starting_xi_ids = vec![
            "p-gk".to_string(),
            "p-lb".to_string(),
            "p-cb1".to_string(),
            "p-cb2".to_string(),
            "p-rb".to_string(),
            "p-lm".to_string(),
            "p-cm1".to_string(),
            "p-cm2".to_string(),
            "p-rm".to_string(),
            "p-st1".to_string(),
            "p-st2".to_string(),
        ];

        let mut right_back = make_player(
            "p-rb",
            Position::Defender,
            PlayerAttributes {
                pace: 84,
                stamina: 82,
                strength: 63,
                agility: 72,
                passing: 64,
                shooting: 40,
                tackling: 77,
                dribbling: 62,
                defending: 72,
                positioning: 66,
                vision: 58,
                decisions: 64,
                composure: 60,
                aggression: 64,
                teamwork: 74,
                leadership: 44,
                handling: 20,
                reflexes: 20,
                aerial: 46,
            },
        );
        right_back.team_id = Some("team-1".to_string());

        let mut striker = make_player(
            "p-st1",
            Position::Forward,
            PlayerAttributes {
                pace: 78,
                stamina: 70,
                strength: 76,
                agility: 68,
                passing: 56,
                shooting: 84,
                tackling: 32,
                dribbling: 71,
                defending: 36,
                positioning: 83,
                vision: 58,
                decisions: 74,
                composure: 70,
                aggression: 66,
                teamwork: 62,
                leadership: 40,
                handling: 20,
                reflexes: 20,
                aerial: 68,
            },
        );
        striker.team_id = Some("team-1".to_string());

        let game = &mut Game::new(
            clock,
            make_manager(),
            vec![team],
            vec![right_back, striker],
            vec![],
            vec![],
        );

        let changed = upgrade_game_player_identities(game);

        assert!(changed);
        assert_eq!(game.players[0].natural_position, Position::RightBack);
        assert_eq!(game.players[1].natural_position, Position::Striker);
    }
}
