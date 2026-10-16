use domain::player::{Player, PlayerAttributes, PlayerTrait, Position};
use ofm_core::player_rating::{
    generate_potential, growth_headroom_range, move_attributes_to, ovr_from_attributes,
    qualifies_for_wonderkid, refresh_player_derived, roll_growth_headroom,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn gk_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 40,
        stamina: 55,
        strength: 65,
        agility: 60,
        passing: 35,
        shooting: 20,
        tackling: 30,
        dribbling: 25,
        defending: 35,
        positioning: 70,
        vision: 60,
        decisions: 72,
        composure: 68,
        aggression: 45,
        teamwork: 65,
        leadership: 60,
        handling: 85,
        reflexes: 88,
        aerial: 75,
    }
}

fn striker_attrs() -> PlayerAttributes {
    PlayerAttributes {
        pace: 80,
        stamina: 70,
        strength: 72,
        agility: 75,
        passing: 60,
        shooting: 88,
        tackling: 35,
        dribbling: 78,
        defending: 25,
        positioning: 85,
        vision: 65,
        decisions: 76,
        composure: 80,
        aggression: 60,
        teamwork: 60,
        leadership: 55,
        handling: 10,
        reflexes: 15,
        aerial: 72,
    }
}

fn make_player(position: Position, attrs: PlayerAttributes, dob: &str) -> Player {
    let mut p = Player::new(
        "test-id".to_string(),
        "T. Player".to_string(),
        "Test Player".to_string(),
        dob.to_string(),
        "GB".to_string(),
        position,
        attrs,
    );
    p.team_id = Some("team1".to_string());
    p
}

// ---------------------------------------------------------------------------
// refresh_player_derived
// ---------------------------------------------------------------------------

#[test]
fn refresh_sets_nonzero_ovr_for_goalkeeper() {
    let mut player = make_player(Position::Goalkeeper, gk_attrs(), "2000-01-01");
    refresh_player_derived(&mut player, domain::age::season_midpoint(2026));

    assert!(
        player.ovr > 0,
        "OVR should be nonzero after refresh, got {}",
        player.ovr
    );
    assert!(player.ovr <= 99, "OVR should be <= 99, got {}", player.ovr);
}

#[test]
fn refresh_sets_nonzero_ovr_for_striker() {
    let mut player = make_player(Position::Forward, striker_attrs(), "1998-05-15");
    refresh_player_derived(&mut player, domain::age::season_midpoint(2026));

    assert!(player.ovr > 0);
    assert!(player.ovr <= 99);
}

#[test]
fn refresh_sets_potential_ge_ovr() {
    let mut player = make_player(Position::Forward, striker_attrs(), "2005-03-20");
    refresh_player_derived(&mut player, domain::age::season_midpoint(2026));

    assert!(
        player.potential >= player.ovr,
        "potential {} should be >= ovr {}",
        player.potential,
        player.ovr
    );
}

#[test]
fn refresh_preserves_existing_potential() {
    let mut player = make_player(Position::Forward, striker_attrs(), "1995-06-10");
    // Pre-set a high potential
    player.potential = 95;
    refresh_player_derived(&mut player, domain::age::season_midpoint(2026));

    assert_eq!(
        player.potential, 95,
        "Existing potential should be preserved"
    );
}

#[test]
fn refresh_does_not_lower_preserved_potential_below_ovr() {
    let mut player = make_player(Position::Forward, striker_attrs(), "1995-06-10");
    // Set a potential that is lower than what OVR would be (should be clamped)
    player.potential = 1;
    refresh_player_derived(&mut player, domain::age::season_midpoint(2026));

    assert!(
        player.potential >= player.ovr,
        "Preserved potential {} should be clamped to >= ovr {}",
        player.potential,
        player.ovr
    );
}

#[test]
fn refresh_awards_wonderkid_trait_for_young_high_potential_player() {
    let mut player = make_player(Position::Forward, striker_attrs(), "2007-01-01");
    player.ovr = 70;
    player.potential = 95;
    refresh_player_derived(&mut player, domain::age::season_midpoint(2026));

    assert!(
        player.potential >= 81 && player.potential.saturating_sub(player.ovr) >= 14,
        "Expected deterministic Wonderkid setup to qualify with potential={} ovr={}",
        player.potential,
        player.ovr
    );

    assert!(
        player.traits.contains(&PlayerTrait::Wonderkid),
        "Should have Wonderkid trait when potential={} ovr={} age~19",
        player.potential,
        player.ovr
    );
}

#[test]
fn refresh_does_not_award_wonderkid_for_old_player() {
    let mut player = make_player(Position::Forward, striker_attrs(), "1990-01-01");
    player.potential = 0;
    refresh_player_derived(&mut player, domain::age::season_midpoint(2026));

    assert!(
        !player.traits.contains(&PlayerTrait::Wonderkid),
        "Old player (age ~36) should not get Wonderkid trait"
    );
}

#[test]
fn wonderkid_qualification_accepts_exact_boundary() {
    assert!(qualifies_for_wonderkid(20, 81, 67));
}

#[test]
fn wonderkid_qualification_rejects_age_above_boundary() {
    assert!(!qualifies_for_wonderkid(21, 81, 67));
}

#[test]
fn wonderkid_qualification_rejects_potential_below_boundary() {
    assert!(!qualifies_for_wonderkid(20, 80, 66));
}

#[test]
fn wonderkid_qualification_rejects_growth_room_below_boundary() {
    assert!(!qualifies_for_wonderkid(20, 81, 68));
}

// ---------------------------------------------------------------------------
// generate_potential
// ---------------------------------------------------------------------------

fn rng() -> impl rand::Rng {
    ofm_core::seed::rng_for_seed(1, "player-rating-tests", "2030-01-01")
}

#[test]
fn generate_potential_always_ge_ovr() {
    for ovr in [40u8, 60, 75, 90] {
        for age in [16u32, 19, 21, 23, 25, 30] {
            let potential = generate_potential(ovr, age, &mut rng());
            assert!(
                potential >= ovr,
                "generate_potential({}, age {}) = {} should be >= ovr",
                ovr,
                age,
                potential
            );
        }
    }
}

#[test]
fn generate_potential_returns_ovr_for_old_players() {
    let ovr = 72u8;
    let potential = generate_potential(ovr, 32, &mut rng());
    assert_eq!(
        potential, ovr,
        "Players aged 32+ should have potential == ovr"
    );
}

#[test]
fn generate_potential_capped_at_99() {
    // Very high OVR for a teenager
    let potential = generate_potential(95, 16, &mut rng());
    assert!(potential <= 99, "Potential should never exceed 99");
}

#[test]
fn generate_potential_young_player_has_higher_ceiling() {
    // Run many times to account for randomness; young player should consistently
    // produce potential >= ovr (and usually higher for teens).
    let ovr = 60u8;
    let mut any_higher = false;
    let mut one_stream = rng();
    for _ in 0..50 {
        let p = generate_potential(ovr, 17, &mut one_stream);
        assert!(p >= ovr);
        if p > ovr {
            any_higher = true;
        }
    }
    assert!(
        any_higher,
        "At least some calls for a 17-year-old should produce potential > ovr"
    );
}

// ---------------------------------------------------------------------------
// growth headroom and lowering a player to a younger self
// ---------------------------------------------------------------------------

/// Given players of every age,
/// When the headroom each can still grow is asked for,
/// Then it only ever narrows with age, and a veteran has none.
#[test]
fn headroom_narrows_with_age_and_a_veteran_has_none() {
    let ages = [16u32, 18, 19, 20, 21, 22, 23, 25, 26, 33];
    for pair in ages.windows(2) {
        let (younger, older) = (
            growth_headroom_range(pair[0]),
            growth_headroom_range(pair[1]),
        );
        assert!(
            older.end() <= younger.end() && older.start() <= younger.start(),
            "age {} has {younger:?} but age {} has {older:?}",
            pair[0],
            pair[1]
        );
    }
    assert_eq!(growth_headroom_range(26), 0..=0);
    assert_eq!(growth_headroom_range(17), 15..=30);
}

/// Given a headroom drawn many times for a 17-year-old,
/// When the draws are collected,
/// Then every one is inside the 17-year-old's range and not all are equal.
#[test]
fn a_rolled_headroom_stays_inside_the_range_for_the_age() {
    let mut stream = rng();
    let draws: Vec<u8> = (0..200)
        .map(|_| roll_growth_headroom(17, &mut stream))
        .collect();
    assert!(draws.iter().all(|d| growth_headroom_range(17).contains(d)));
    assert!(draws.iter().any(|d| *d != draws[0]));
}

/// Given an outfielder and a keeper,
/// When their attributes are moved to a rating 20 below and 10 above their own,
/// Then each lands within a point of it.
#[test]
fn moving_attributes_lands_within_a_point_of_the_target() {
    for (attributes, position) in [
        (gk_attrs(), Position::Goalkeeper),
        (striker_attrs(), Position::Forward),
    ] {
        let own = ovr_from_attributes(&attributes, &position).round() as u8;
        for target in [own - 20, own + 10] {
            let mut moved = attributes.clone();

            move_attributes_to(&mut moved, &position, target);

            let landed = ovr_from_attributes(&moved, &position);
            assert!(
                (landed - f64::from(target)).abs() <= 1.0,
                "{landed} is not within a point of {target}"
            );
        }
    }
}

/// Given attributes already at the target,
/// When they are moved to it,
/// Then nothing changes.
#[test]
fn moving_to_the_rating_they_have_changes_nothing() {
    let mut attributes = striker_attrs();
    let before = attributes.clone();
    let own = ovr_from_attributes(&attributes, &Position::Forward).round() as u8;

    move_attributes_to(&mut attributes, &Position::Forward, own);

    assert_eq!(attributes, before);
}

/// Given a target no attribute block can reach in either direction,
/// When attributes are moved toward it,
/// Then the move ends, with every attribute clamped inside 1..=99.
#[test]
fn moving_toward_an_unreachable_target_still_ends() {
    for target in [1u8, 99] {
        let mut attributes = striker_attrs();

        move_attributes_to(&mut attributes, &Position::Forward, target);

        let landed = ovr_from_attributes(&attributes, &Position::Forward);
        assert!((1.0..=99.0).contains(&landed), "{landed}");
    }
}

/// Given the age bands the headroom is set by,
/// When the range for the first and last age of each band is asked for,
/// Then each band has its own range.
#[test]
fn every_age_band_has_its_own_headroom() {
    for (ages, range) in [
        ([16u32, 18], 15..=30),
        ([19, 20], 8..=22),
        ([21, 22], 4..=14),
        ([23, 25], 0..=7),
        ([26, 40], 0..=0),
    ] {
        for age in ages {
            assert_eq!(growth_headroom_range(age), range, "age {age}");
        }
    }
}

/// Given attribute blocks and targets between the lowest and highest rating a shift can give,
/// When they are moved to the target,
/// Then the result is the closest rating any equal shift of the block could reach.
#[test]
fn moving_attributes_lands_on_the_closest_reachable_rating() {
    for (attributes, position) in [
        (gk_attrs(), Position::Goalkeeper),
        (striker_attrs(), Position::Forward),
    ] {
        for target in (30u8..=90).step_by(3) {
            let mut moved = attributes.clone();

            move_attributes_to(&mut moved, &position, target);

            let landed = (ovr_from_attributes(&moved, &position) - f64::from(target)).abs();
            let best_reachable = (-60i16..=60)
                .map(|shift| {
                    let mut shifted = attributes.clone();
                    shift_all(&mut shifted, shift);
                    (ovr_from_attributes(&shifted, &position) - f64::from(target)).abs()
                })
                .fold(f64::MAX, f64::min);
            assert!(
                landed <= best_reachable + 1e-9,
                "target {target}: landed {landed:.2} away, best possible {best_reachable:.2}"
            );
        }
    }
}

fn shift_all(attributes: &mut PlayerAttributes, shift: i16) {
    let apply = |value: &mut u8| *value = (i16::from(*value) + shift).clamp(1, 99) as u8;
    apply(&mut attributes.pace);
    apply(&mut attributes.stamina);
    apply(&mut attributes.strength);
    apply(&mut attributes.agility);
    apply(&mut attributes.passing);
    apply(&mut attributes.shooting);
    apply(&mut attributes.tackling);
    apply(&mut attributes.dribbling);
    apply(&mut attributes.defending);
    apply(&mut attributes.positioning);
    apply(&mut attributes.vision);
    apply(&mut attributes.decisions);
    apply(&mut attributes.composure);
    apply(&mut attributes.aggression);
    apply(&mut attributes.teamwork);
    apply(&mut attributes.leadership);
    apply(&mut attributes.handling);
    apply(&mut attributes.reflexes);
    apply(&mut attributes.aerial);
}
