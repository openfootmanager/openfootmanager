use domain::player::{Player, PlayerAttributes, PlayerTrait, Position};
use ofm_core::player_rating::{generate_potential, refresh_player_derived};

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
    refresh_player_derived(&mut player, 2026);

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
    refresh_player_derived(&mut player, 2026);

    assert!(player.ovr > 0);
    assert!(player.ovr <= 99);
}

#[test]
fn refresh_sets_potential_ge_ovr() {
    let mut player = make_player(Position::Forward, striker_attrs(), "2005-03-20");
    refresh_player_derived(&mut player, 2026);

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
    refresh_player_derived(&mut player, 2026);

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
    refresh_player_derived(&mut player, 2026);

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
    refresh_player_derived(&mut player, 2026);

    assert!(
        player.potential >= 85 && player.potential.saturating_sub(player.ovr) >= 10,
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
    refresh_player_derived(&mut player, 2026);

    assert!(
        !player.traits.contains(&PlayerTrait::Wonderkid),
        "Old player (age ~36) should not get Wonderkid trait"
    );
}

// ---------------------------------------------------------------------------
// generate_potential
// ---------------------------------------------------------------------------

#[test]
fn generate_potential_always_ge_ovr() {
    for ovr in [40u8, 60, 75, 90] {
        for age in [16u32, 19, 21, 23, 25, 30] {
            let potential = generate_potential(ovr, age);
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
    let potential = generate_potential(ovr, 32);
    assert_eq!(
        potential, ovr,
        "Players aged 32+ should have potential == ovr"
    );
}

#[test]
fn generate_potential_capped_at_99() {
    // Very high OVR for a teenager
    let potential = generate_potential(95, 16);
    assert!(potential <= 99, "Potential should never exceed 99");
}

#[test]
fn generate_potential_young_player_has_higher_ceiling() {
    // Run many times to account for randomness; young player should consistently
    // produce potential >= ovr (and usually higher for teens).
    let ovr = 60u8;
    let mut any_higher = false;
    for _ in 0..50 {
        let p = generate_potential(ovr, 17);
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
