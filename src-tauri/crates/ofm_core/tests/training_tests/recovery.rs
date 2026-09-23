//! The recovery ledger: what a club gets back between matches.
//!
//! The fatigue guard, the taper before a fixture, and the rest day. All three
//! decide whether a squad reaches its next match fit, and all three now apply
//! to every club alike — the player's included.

use super::*;

// ---------------------------------------------------------------------------
// Fatigue guard
// ---------------------------------------------------------------------------

/// Reproduces the fatigue spiral and verifies the guard breaks it for everyone.
///
/// An individually exhausted player on a team training at Medium intensity with a
/// non-recovery focus pays a flat condition cost (6) that exceeds their diminished
/// recovery — so without intervention they keep losing condition every training
/// day and never climb out. The guard used to auto-rest such players on AI teams
/// only; it now covers the user's club too, because being too tired to train is
/// not something a manager can decide their way out of.
#[test]
fn the_fatigue_guard_rests_an_exhausted_player_on_any_team() {
    let mut game = make_game(); // manager is hired to "team1" (the user team)

    // An AI club that is the user's club in every respect that training reads —
    // same settings and the same staff, so any divergence is the guard and not
    // a coaching or physio bonus.
    add_mirror_ai_team(&mut game);

    // Two identical exhausted players: one on the user team, one on the AI team.
    let mut user_tired = make_player("user_tired", "UserTired", "team1", "1998-06-10");
    user_tired.condition = 22;
    let mut ai_tired = make_player("ai_tired", "AiTired", "team2", "1998-06-10");
    ai_tired.condition = 22;
    game.players.push(user_tired);
    game.players.push(ai_tired);

    // Three consecutive training days (Monday is a training day under Balanced).
    for _ in 0..3 {
        training::process_training(&mut game, 0);
    }

    let condition_of = |id: &str| game.players.iter().find(|p| p.id == id).unwrap().condition;

    // An exhausted player physically cannot take a hard session. That is not an
    // AI concession, so both climb out of the spiral and they climb at the same rate.
    assert!(
        condition_of("ai_tired") > 22,
        "AI exhausted player should recover, got {}",
        condition_of("ai_tired")
    );
    assert!(
        condition_of("user_tired") > 22,
        "the user's exhausted player should recover too, got {}",
        condition_of("user_tired")
    );
    assert_eq!(
        condition_of("user_tired"),
        condition_of("ai_tired"),
        "identical players on identical settings must not diverge by who manages them"
    );
}

// ---------------------------------------------------------------------------
// process_training — the near-match taper
//
// No squad trains at full load two days before a game. That is a property of
// training, not a decision an AI manager makes, so it applies to every club.
// Without it the user's club, on the settings a new career starts with, loses
// about 26 condition a week and is on the floor inside two months.
// ---------------------------------------------------------------------------

#[test]
fn a_session_two_days_before_a_match_restores_condition_on_the_user_team() {
    let mut game = make_game(); // team1 is the user's, Physical / Medium / Balanced
    schedule_user_fixture_in(&mut game, 2);
    let before: Vec<u8> = game.players.iter().map(|p| p.condition).collect();

    training::process_training(&mut game, 0); // Monday, a training day under Balanced

    for (player, was) in game.players.iter().zip(before) {
        assert!(
            player.condition > was,
            "{} should finish a tapered session fresher than it started ({} → {})",
            player.id,
            was,
            player.condition
        );
    }
}

#[test]
fn a_session_far_from_the_next_match_still_costs_condition() {
    let mut game = make_game();
    schedule_user_fixture_in(&mut game, 6);
    let before: Vec<u8> = game.players.iter().map(|p| p.condition).collect();

    training::process_training(&mut game, 0);

    for (player, was) in game.players.iter().zip(before) {
        assert!(
            player.condition < was,
            "{} is nowhere near a fixture and should pay for the session ({} → {})",
            player.id,
            was,
            player.condition
        );
    }
}

/// The taper must be a window, not a permanent state. With one fixture a week a
/// Balanced club should load on Monday and Tuesday and taper on Thursday and
/// Friday — if every training day tapers, condition stops being a constraint.
#[test]
fn only_the_two_days_before_a_fixture_taper() {
    // Balanced trains Mon, Tue, Thu, Fri. The fixture is the coming Saturday.
    let expected_to_recover = [(0, false), (1, false), (3, true), (4, true)];

    for (weekday, should_recover) in expected_to_recover {
        let mut game = make_game();
        // Monday is weekday 0, so the Saturday fixture is (5 - weekday) days out.
        schedule_user_fixture_in(&mut game, 5 - i64::from(weekday));
        let before = game
            .players
            .iter()
            .find(|p| p.id == "p2")
            .unwrap()
            .condition;

        training::process_training(&mut game, weekday);

        let after = game
            .players
            .iter()
            .find(|p| p.id == "p2")
            .unwrap()
            .condition;
        if should_recover {
            assert!(
                after > before,
                "weekday {weekday} is inside the taper and should recover ({before} → {after})"
            );
        } else {
            assert!(
                after < before,
                "weekday {weekday} is outside the taper and should cost condition ({before} → {after})"
            );
        }
    }
}

#[test]
fn the_taper_reaches_the_user_team_and_the_ai_team_alike() {
    let mut game = make_game();
    add_mirror_ai_team(&mut game);
    game.players
        .push(make_player("ai_p", "AiPlayer", "team2", "1998-06-10"));
    // The fixture is team1 vs team2, so it is two days away for both clubs.
    schedule_user_fixture_in(&mut game, 2);

    let user_before = game
        .players
        .iter()
        .find(|p| p.id == "p2")
        .unwrap()
        .condition;
    let ai_before = game
        .players
        .iter()
        .find(|p| p.id == "ai_p")
        .unwrap()
        .condition;
    assert_eq!(user_before, ai_before, "helper players must start level");

    training::process_training(&mut game, 0);

    let user_after = game
        .players
        .iter()
        .find(|p| p.id == "p2")
        .unwrap()
        .condition;
    let ai_after = game
        .players
        .iter()
        .find(|p| p.id == "ai_p")
        .unwrap()
        .condition;
    assert_eq!(
        user_after, ai_after,
        "the taper is physics, so the same fixture must taper both clubs equally"
    );
    assert!(user_after > user_before, "both should have recovered");
}

// ---------------------------------------------------------------------------
// A day off is the best rest available
//
// Rest base was 7.0 against a Recovery session's 9.0, so the only thing a
// scheduled day off does — restore condition — it did worse than a training
// session that also nudges fitness and drifts attributes. A day off was strictly
// dominated, and the schedule with the fewest of them was the best schedule.
// ---------------------------------------------------------------------------

#[test]
fn a_rest_day_restores_more_than_a_recovery_session() {
    // Balanced rests on Wednesday (2) and trains on Monday (0).
    let rest = condition_after(TrainingSchedule::Balanced, TrainingFocus::Recovery, 2);
    let session = condition_after(TrainingSchedule::Balanced, TrainingFocus::Recovery, 0);

    assert!(
        rest > session,
        "a day off must restore more than a Recovery session, which also builds \
         fitness: rest gave {rest}, session gave {session}"
    );
}

#[test]
fn resting_more_beats_training_more_when_both_clubs_only_want_condition() {
    // One week, both clubs on Recovery focus so the only difference is how many
    // days off the schedule gives. Intense rests once, Light rests five times.
    let week = |schedule: TrainingSchedule| -> u8 {
        let mut game = make_game();
        game.teams[0].training_schedule = schedule;
        game.teams[0].training_focus = TrainingFocus::Recovery;
        // No physio, and read the veteran: the slowest recovery in the game, so a
        // week's worth of it stays clear of the 100 ceiling where the two
        // schedules would be indistinguishable rather than equal.
        game.staff.retain(|s| s.role != StaffRole::Physio);
        for p in game.players.iter_mut() {
            p.condition = 10;
        }
        for weekday in 0..7 {
            training::process_training(&mut game, weekday);
        }
        game.players
            .iter()
            .find(|p| p.id == "p3")
            .expect("veteran")
            .condition
    };

    let intense = week(TrainingSchedule::Intense);
    let light = week(TrainingSchedule::Light);

    assert!(
        light > intense,
        "a club that rests five days a week must end it fresher than one that \
         trains six, or nobody would ever schedule a day off: Light {light}, \
         Intense {intense}"
    );
}
