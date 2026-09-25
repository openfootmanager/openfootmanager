//! The matches nobody is watching.
//!
//! Every fixture the player is not sitting through is resolved by `process_day`
//! without a UI, and for a long time without much else: the whole squad was
//! handed to the engine, and every one of them was charged for a full match.
//! These tests pin who actually takes the field in an unwatched match, and who
//! pays for it.

use super::*;

// ---------------------------------------------------------------------------
// Instant simulation fields an eleven, not a squad list
//
// Every fixture the player is not watching goes through `build_engine_team`,
// which used to hand the engine every player on the books — injured included —
// and the report then credited each of them a full match. A squad was charged
// roughly twice the condition it should have been, every fixture, and reserves
// banked appearances for games they never played.
// ---------------------------------------------------------------------------

#[test]
fn an_instant_match_charges_eleven_players_a_side_not_the_whole_squad() {
    let mut game = game_with_deep_squads();
    let before: HashMap<String, u8> = game
        .players
        .iter()
        .map(|p| (p.id.clone(), p.condition))
        .collect();

    turn::process_day(&mut game);

    for team_id in ["team1", "team2"] {
        let played = game
            .players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some(team_id))
            .filter(|p| p.condition < before[&p.id])
            .count();
        assert_eq!(
            played, 11,
            "{team_id} should have charged exactly its eleven starters, not {played} players"
        );
    }
}

#[test]
fn an_instant_match_never_fields_an_injured_player() {
    let mut game = game_with_deep_squads();
    // Injure one first-choice player per slot group on team1.
    let injured_ids: Vec<String> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .take(3)
        .map(|p| p.id.clone())
        .collect();
    for player in game.players.iter_mut() {
        if injured_ids.contains(&player.id) {
            player.injury = Some(Injury {
                name: "common.injuries.calfStrain".to_string(),
                days_remaining: 10,
            });
        }
    }
    let before: HashMap<String, u8> = game
        .players
        .iter()
        .map(|p| (p.id.clone(), p.condition))
        .collect();

    turn::process_day(&mut game);

    for id in &injured_ids {
        let player = game.players.iter().find(|p| &p.id == id).unwrap();
        assert!(
            player.condition >= before[id],
            "injured {id} must not be charged for a match it could not play \
             ({} → {})",
            before[id],
            player.condition
        );
    }
}

/// team2 is an AI club, so this covers the `ai_select_starting_xi` branch; the
/// short-squad test below runs the same top-up through the user's own club.
#[test]
fn an_instant_match_still_fields_a_side_when_every_player_is_injured() {
    let mut game = game_with_deep_squads();
    for player in game.players.iter_mut() {
        if player.team_id.as_deref() == Some("team2") {
            player.injury = Some(Injury {
                name: "common.injuries.calfStrain".to_string(),
                days_remaining: 10,
            });
        }
    }

    // Must not panic: an empty side makes the engine index a player that is not
    // there. A club with bodies puts a side out, however sore.
    turn::process_day(&mut game);

    assert_eq!(
        fielded_count(&game, "team2"),
        11,
        "a club with no fit players must still field eleven"
    );
}

/// team1 is the manager's own club, so this runs the top-up behind
/// `select_starting_xi` rather than the AI policy.
#[test]
fn a_squad_too_short_to_field_eleven_is_topped_up_from_the_injured() {
    let mut game = game_with_deep_squads();
    let fit_ids: Vec<String> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some("team1"))
        .take(7)
        .map(|p| p.id.clone())
        .collect();
    for player in game.players.iter_mut() {
        if player.team_id.as_deref() == Some("team1") && !fit_ids.contains(&player.id) {
            player.injury = Some(Injury {
                name: "common.injuries.calfStrain".to_string(),
                days_remaining: 10,
            });
        }
    }

    turn::process_day(&mut game);

    assert_eq!(
        fielded_count(&game, "team1"),
        11,
        "seven fit players must be made up to eleven, not fielded as a short side"
    );
    for id in &fit_ids {
        let player = game.players.iter().find(|p| &p.id == id).unwrap();
        assert!(
            player.stats.appearances > 0,
            "fit {id} must start ahead of any injured player"
        );
    }
}

// ---------------------------------------------------------------------------
// A dormant match is a scoreline, not a workout
//
// Competitions outside the player's active scope are resolved by a
// scoreline-only model that charges nobody any condition. Their clubs have not
// played ninety minutes in any physical sense, so the training ground's "you
// played today" gate must not close on them — or they lose the day's recovery
// every matchday and get nothing for it.
// ---------------------------------------------------------------------------

#[test]
fn a_club_in_a_dormant_competition_still_recovers_on_its_matchday() {
    let mut game = make_game_with_match();
    add_idle_club(&mut game, "dormant_home", 50);
    add_idle_club(&mut game, "dormant_away", 50);

    let active = game.league.clone().expect("the fixture's league");
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let dormant = League {
        id: "dormant_league".to_string(),
        name: "Somewhere Else".to_string(),
        season: 1,
        fixtures: vec![Fixture {
            id: "dormant_fix".to_string(),
            matchday: 1,
            date: today,
            home_team_id: "dormant_home".to_string(),
            away_team_id: "dormant_away".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            ..Default::default()
        }],
        standings: vec![
            StandingEntry::new("dormant_home".to_string()),
            StandingEntry::new("dormant_away".to_string()),
        ],
        ..Default::default()
    };
    game.active_competition_ids = vec![active.id.clone()];
    game.competitions = vec![active, dormant];
    let before = condition_of(&game, "dormant_home");

    // 2025-06-15 is a Sunday: a rest day on every schedule.
    turn::process_day(&mut game);

    let dormant_played = game.competitions[1]
        .fixtures
        .iter()
        .all(|f| f.status == FixtureStatus::Completed);
    assert!(
        dormant_played,
        "the dormant fixture should have been resolved"
    );
    let after = condition_of(&game, "dormant_home");
    assert!(
        after.iter().zip(&before).all(|(now, was)| now > was),
        "a scoreline-only match costs nothing, so its rest day must still restore: \
         {before:?} -> {after:?}"
    );
}
