//! The dice of a batch matchday come from the game's seed.

use domain::league::FixtureStatus;
use ofm_core::game::Game;
use ofm_core::turn;

use super::fixtures::make_game_with_match;

/// The scorelines of every club fixture that has been played.
fn scorelines(game: &Game) -> Vec<String> {
    let league = game.league.as_ref().expect("the helper builds a league");
    league
        .fixtures
        .iter()
        .filter(|fixture| fixture.status == FixtureStatus::Completed)
        .map(|fixture| {
            let result = fixture
                .result
                .as_ref()
                .expect("a completed fixture has a result");
            format!(
                "{}-{} {}-{}",
                fixture.home_team_id, fixture.away_team_id, result.home_goals, result.away_goals
            )
        })
        .collect()
}

fn one_day_with_seed(game: &Game, seed: u64) -> Vec<String> {
    let mut game = game.clone();
    game.seed = seed;
    turn::process_day(&mut game);
    scorelines(&game)
}

/// Given a game with a match today,
/// When the day is played twice from the same save and the same seed,
/// Then the match ends the same way both times.
#[test]
fn a_match_ends_the_same_way_from_the_same_seed() {
    let save = make_game_with_match();

    let first = one_day_with_seed(&save, 11);
    let second = one_day_with_seed(&save, 11);

    assert!(!first.is_empty(), "the day must have played the match");
    assert_eq!(first, second);
}

/// The control: seeds are what decide the match, or the scenario above could not
/// tell a seeded match from one that ignores its seed.
#[test]
fn the_seed_decides_how_a_match_ends() {
    let save = make_game_with_match();

    let outcomes: std::collections::BTreeSet<Vec<String>> =
        (0..40).map(|seed| one_day_with_seed(&save, seed)).collect();

    assert!(
        outcomes.len() > 1,
        "forty seeds all gave {:?}: the match is not reading the seed",
        outcomes.iter().next()
    );
}

/// What a match does to the people in it: their legs, their heads, and their clubs' form.
fn aftermath(game: &Game) -> Vec<String> {
    let mut lines: Vec<String> = game
        .players
        .iter()
        .map(|player| {
            format!(
                "{} cond{} fit{} morale{} inj{:?}",
                player.id, player.condition, player.fitness, player.morale, player.injury
            )
        })
        .collect();
    lines.extend(
        game.teams
            .iter()
            .map(|team| format!("{} form{:?}", team.id, team.form)),
    );
    lines
}

fn aftermath_after_one_day(game: &Game, seed: u64) -> Vec<String> {
    let mut game = game.clone();
    game.seed = seed;
    turn::process_day(&mut game);
    aftermath(&game)
}

/// Given a game with a match today,
/// When the day is played twice from the same save and the same seed,
/// Then the players come out of it with the same legs and moods, and the clubs the same form.
#[test]
fn a_match_leaves_the_same_bodies_and_moods_from_the_same_seed() {
    let save = make_game_with_match();

    let first = aftermath_after_one_day(&save, 11);
    let second = aftermath_after_one_day(&save, 11);

    assert_eq!(first, second);
}

/// The control, over the same projection.
#[test]
fn the_seed_decides_what_a_match_does_to_the_people_in_it() {
    let save = make_game_with_match();

    let outcomes: std::collections::BTreeSet<Vec<String>> = (0..40)
        .map(|seed| aftermath_after_one_day(&save, seed))
        .collect();

    assert!(
        outcomes.len() > 1,
        "forty seeds all left the same aftermath"
    );
}

/// Given two clubs on long streaks — one winning, one losing — so that a result in
///       line with the streak reaches the morale draw that only streaks make,
/// When the day is played twice from the same save, for forty different seeds,
/// Then every seed gives the same aftermath both times.
///
/// One day of a fresh save never reaches the streak branch, so the scenario above cannot
/// see whether it reads the seed.
#[test]
fn a_streak_moves_morale_the_same_way_from_the_same_seed() {
    let mut save = make_game_with_match();
    save.teams[0].form = vec!["W".to_string(); 5];
    save.teams[1].form = vec!["L".to_string(); 5];

    for seed in 0..40 {
        assert_eq!(
            aftermath_after_one_day(&save, seed),
            aftermath_after_one_day(&save, seed),
            "seed {seed}"
        );
    }
}
