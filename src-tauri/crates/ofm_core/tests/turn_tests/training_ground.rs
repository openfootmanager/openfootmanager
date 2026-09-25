//! The training ground opens per club, not per world.

use super::*;

// ---------------------------------------------------------------------------
// The training ground is not closed because someone else has a fixture
//
// `has_match_today` was world-global: if any club in any active competition
// played, no club anywhere trained or recovered — including the eighteen with
// nothing on. The gate belongs per club, and it has to be applied at both day
// entry points, since `finish_live_match_day` ran no training at all.
// ---------------------------------------------------------------------------

#[test]
fn a_club_with_no_fixture_recovers_while_another_competition_plays() {
    let mut game = make_game_with_match();
    add_idle_club(&mut game, "idle", 50);
    let before = condition_of(&game, "idle");

    // 2025-06-15 is a Sunday: a rest day on every schedule, so an idle club can
    // only gain condition here.
    turn::process_day(&mut game);

    let after = condition_of(&game, "idle");
    assert!(
        after.iter().zip(&before).all(|(now, was)| now > was),
        "a club with nothing on should recover on someone else's matchday: {before:?} -> {after:?}"
    );
}

#[test]
fn a_club_playing_today_does_not_also_train() {
    let mut game = game_with_deep_squads();
    // Below 100, or a squad that wrongly trained on its matchday could recover
    // and still read 100, and this test could never see it.
    for player in game.players.iter_mut() {
        player.condition = 60;
    }
    let before: HashMap<String, u8> = game
        .players
        .iter()
        .map(|p| (p.id.clone(), p.condition))
        .collect();

    turn::process_day(&mut game);

    // The eleven who played are charged match wear; the rest of the squad were at
    // the ground doing nothing, and must not come out of it fresher than they
    // went in.
    for player in game.players.iter() {
        if player.team_id.as_deref() == Some("team1") && player.stats.appearances == 0 {
            assert_eq!(
                player.condition, before[&player.id],
                "{} neither played nor should have trained on its club's matchday",
                player.id
            );
        }
    }
}

#[test]
fn finishing_a_live_match_day_still_trains_the_clubs_that_were_not_playing() {
    let mut game = make_game_with_match();
    add_idle_club(&mut game, "idle", 50);
    let before = condition_of(&game, "idle");

    // The other entry point into a day: the user watched their own fixture. The
    // rest of the world still had a Sunday.
    turn::finish_live_match_day(&mut game);

    let after = condition_of(&game, "idle");
    assert!(
        after.iter().zip(&before).all(|(now, was)| now > was),
        "a club with nothing on should recover on the day the user plays live: {before:?} -> {after:?}"
    );
}
