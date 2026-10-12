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
// Every fixture the player was not watching used to go through
// `build_engine_team`, which handed the engine every player on the books —
// injured included — and the report then credited each of them a full match. A
// squad was charged roughly twice the condition it should have been, every
// fixture, and reserves banked appearances for games they never played. Both
// match paths now build their sides with `turn::squad::build_team_with_bench`.
// ---------------------------------------------------------------------------

/// Eleven start and at most five come on, so a side spends between eleven and
/// sixteen of its twenty-two. The assertion that survives the arrival of the
/// bench is the one the bug was about: a player who did not take the field is
/// not charged for the match.
#[test]
fn an_instant_match_charges_the_players_who_took_the_field_and_nobody_else() {
    let mut game = game_with_deep_squads();
    let before: HashMap<String, u8> = game
        .players
        .iter()
        .map(|p| (p.id.clone(), p.condition))
        .collect();

    turn::process_day(&mut game);

    for team_id in ["team1", "team2"] {
        for player in game
            .players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some(team_id))
        {
            assert!(
                player.stats.appearances > 0 || player.condition >= before[&player.id],
                "unused {} was charged for a match it never played ({} → {})",
                player.id,
                before[&player.id],
                player.condition
            );
        }

        let used = fielded_count(&game, team_id);
        assert!(
            (11..=16).contains(&used),
            "{team_id} put out eleven and may bring on five, so {used} players is not a team sheet"
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

// ---------------------------------------------------------------------------
// A match nobody watches is still a match somebody manages
//
// Every fixture in the world bar the one the player sat through was resolved by
// `engine::simulate`: a one-shot with no command loop, so eleven players a side
// played ninety minutes and the bench never moved. The manager the AI slices
// taught to read a game only ever managed one club a matchday.
// ---------------------------------------------------------------------------

/// Both squads flat out, so the branch that fires every minute rather than at a
/// checkpoint is certain to reach for the bench whatever the scoreline does.
/// Substitutions average under three a match; a level game between fresh sides
/// can honestly produce none, and a red line must not turn on the dice.
fn spend_every_squad(game: &mut Game) {
    for player in game.players.iter_mut() {
        player.condition = 40;
    }
}

#[test]
fn a_match_nobody_watches_still_uses_the_bench() {
    let mut game = game_with_deep_squads();
    spend_every_squad(&mut game);

    turn::process_day(&mut game);

    for team_id in ["team1", "team2"] {
        let used = fielded_count(&game, team_id);
        assert!(
            used > 11,
            "{team_id} played a match on its knees and finished with the eleven \
             it started ({used} players credited with an appearance)"
        );
    }
}

/// team1 is the manager's own club. Nobody is watching this fixture either — the
/// player advanced past it — so it gets a touchline like everyone else. The side
/// the player happens to own is not the side that goes unmanaged.
#[test]
fn the_players_own_club_is_managed_when_the_player_is_not_watching() {
    let mut game = game_with_deep_squads();
    spend_every_squad(&mut game);

    turn::process_day(&mut game);

    assert!(
        fielded_count(&game, "team1") > 11,
        "the manager's own club was left without a touchline"
    );
}

/// The bench path now runs for every club in the world, so it meets clubs that
/// have no bench: `make_game_with_match` gives each side exactly eleven. They
/// must play the match out — not panic, and not field a short side.
#[test]
fn a_club_with_exactly_eleven_players_plays_the_match_out() {
    let mut game = make_game_with_match();
    spend_every_squad(&mut game);

    turn::process_day(&mut game);

    for team_id in ["team1", "team2"] {
        assert_eq!(
            fielded_count(&game, team_id),
            11,
            "{team_id} has nobody to bring on and eleven who must finish"
        );
    }
}

// ---------------------------------------------------------------------------
// A level knockout tie that nobody watched
//
// These used to be settled with a strength-weighted coin toss at 90'. They
// are now played by the live engine, which goes to extra time and only then
// to penalties, as the player's own knockout matches always have.
// ---------------------------------------------------------------------------

#[test]
fn simulate_other_matches_settles_knockout_draws_with_shootout() {
    // Regression: an AI-simulated knockout tie that ended level persisted with
    // no shootout score, so advance_knockout_competition_round always advanced
    // the home team. The full-engine sim path must resolve level knockout
    // ties with a simulated shootout.
    let mut saw_draw = false;
    // Each attempt is a different game: one seed would be one outcome two hundred times.
    for attempt in 0..200 {
        let mut game = make_game_with_match();
        game.seed = attempt;
        {
            let league = game.league.as_mut().unwrap();
            league.fixtures[0].competition = FixtureCompetition::Cup;
            league.knockout_rounds = vec![KnockoutRoundState {
                id: "round-1".to_string(),
                name: "Final".to_string(),
                fixture_ids: vec!["fix1".to_string()],
                bye_team_ids: Vec::new(),
                completed: false,
            }];
        }
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        turn::simulate_other_matches(&mut game, &today, None);

        let result = game.league.as_ref().unwrap().fixtures[0]
            .result
            .as_ref()
            .expect("fixture should have a result");
        if result.home_goals == result.away_goals {
            saw_draw = true;
            let home_pens = result.home_penalties.expect("level knockout needs pens");
            let away_pens = result.away_penalties.expect("level knockout needs pens");
            assert_ne!(home_pens, away_pens, "shootout must have a winner");
            // The shootout is now the end of a tie that went to extra time
            // rather than a coin toss thrown at 90'. Neither side has a bench,
            // so every starter is still on and must be credited with the extra
            // half hour; ninety minutes here means the tie never played it.
            let longest = game
                .players
                .iter()
                .map(|player| player.stats.minutes_played)
                .max()
                .unwrap_or(0);
            assert!(
                longest >= 120,
                "a level knockout tie goes to extra time before penalties, \
                 but the longest shift was {longest} minutes"
            );
            break;
        }
        assert!(
            result.home_penalties.is_none() && result.away_penalties.is_none(),
            "decisive results must not carry a shootout"
        );
    }
    assert!(saw_draw, "expected at least one drawn knockout in 200 sims");
}

/// Given a league fixture nobody watches, when it is played many times over and
/// some of those end level, then none of them goes to extra time or to penalties:
/// a league draw is a draw. The counterpart to the knockout tie above, and the
/// shape of #601, where a league match was handed to the engine as a knockout.
#[test]
fn a_level_league_match_nobody_watches_ends_at_full_time() {
    let game = make_game_with_match();
    assert!(
        !game.league.as_ref().unwrap().is_knockout_fixture("fix1"),
        "the fixture must be a league match for this to prove anything"
    );
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let mut draws = 0;
    for attempt in 0..200 {
        let mut game = game.clone();
        game.seed = attempt;
        turn::simulate_other_matches(&mut game, &today, None);

        let result = game.league.as_ref().unwrap().fixtures[0]
            .result
            .clone()
            .expect("fixture should have a result");
        assert!(
            result.home_penalties.is_none() && result.away_penalties.is_none(),
            "a league match went to penalties: {result:?}"
        );
        let longest = game
            .players
            .iter()
            .map(|player| player.stats.minutes_played)
            .max()
            .unwrap_or(0);
        assert!(
            longest < 120,
            "a league match played extra time: the longest shift was {longest} minutes"
        );
        if result.home_goals == result.away_goals {
            draws += 1;
            if draws >= 5 {
                return;
            }
        }
    }
    panic!("expected at least five level league matches in 200, saw {draws}");
}
