//! A generated club arrives with an identity: tactics that are its play style
//! expressed in the nine dials the engine reads, and a squad with jobs rather
//! than only names. Both are applied inside the generator — tactics in
//! `build_team`, roles in `normalize_generated_team` — so these tests live here.

use super::*;

/// A generated squad must arrive with jobs, not just names. Roles are
/// assigned in `normalize_generated_team` rather than in `build_club`,
/// because the package path runs it again *after* replacing generated
/// players with authored ones — and a role belongs to the squad that
/// finished, not the one that was built first.
#[test]
fn a_generated_squad_is_given_something_to_do() {
    let mut rng = StdRng::seed_from_u64(29);
    let mut tdef = test_team_def();
    tdef.play_style = "Possession".to_string();
    let names_def = default_names_definition();
    let country_codes = generation::nationality_distribution();

    let (team, players, _) = build_club(&tdef, country_codes, 2026, &names_def, &mut rng);

    assert!(
        !team.player_roles.is_empty(),
        "every player converted to Standard at the engine boundary"
    );
    for (player_id, role) in &team.player_roles {
        let player = players
            .iter()
            .find(|p| p.id == *player_id)
            .expect("a role was stored for somebody who is not in the squad");
        assert!(
            player.position.admits_role(role),
            "{:?} was given {role:?}",
            player.position
        );
    }
}

/// Every club the world builds — procedural, package-authored or filler —
/// comes through `build_team`, which is why the blueprint is applied here
/// and not at one of the several call sites above it.
#[test]
fn a_built_club_takes_its_style_onto_the_pitch_with_it() {
    let mut rng = StdRng::seed_from_u64(19);
    let mut tdef = test_team_def();
    tdef.play_style = "HighPress".to_string();

    let team = build_team(&tdef, &mut rng);

    assert_eq!(team.play_style, domain::team::PlayStyle::HighPress);
    assert_eq!(
        team.tactics_phase,
        crate::ai_tactics::blueprint_for(&domain::team::PlayStyle::HighPress),
        "a club's stored tactics must be the blueprint for its style"
    );
    assert_ne!(
        team.tactics_phase,
        domain::team::TacticsPhaseSettings::default(),
        "the whole point is that it is no longer neutral"
    );
}

/// Two clubs of different styles must be distinguishable on the pitch, not
/// merely in a label the engine never reads.
#[test]
fn two_clubs_of_different_styles_do_not_play_the_same_way() {
    let mut rng = StdRng::seed_from_u64(23);
    let mut counter_def = test_team_def();
    counter_def.play_style = "Counter".to_string();
    let mut possession_def = test_team_def();
    possession_def.play_style = "Possession".to_string();

    let counter = build_team(&counter_def, &mut rng);
    let possession = build_team(&possession_def, &mut rng);

    assert_ne!(counter.tactics_phase, possession.tactics_phase);
}
