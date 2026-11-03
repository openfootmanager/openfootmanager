//! When a benched player complains, and how many player messages a day can bring. Each test
//! sweeps world seeds, since one seed is one roll of the day's dice.

use super::*;

fn fixture(index: u32, home: &str, away: &str, status: FixtureStatus) -> Fixture {
    Fixture {
        id: format!("fix{index}"),
        matchday: index + 1,
        date: format!("2025-06-{:02}", 1 + index),
        home_team_id: home.to_string(),
        away_team_id: away.to_string(),
        competition: FixtureCompetition::League,
        status,
        ..Default::default()
    }
}

/// The squad of `make_game` with these fixtures, and p_fwd0 benched: rated 70, morale 40,
/// `appearances` games played. Everyone else is unrated, so only he can complain.
fn benched_forward_with(fixtures: Vec<Fixture>, appearances: u32) -> Game {
    let mut game = make_game();
    game.league = Some(League {
        id: "league1".to_string(),
        name: "Test League".to_string(),
        season: 1,
        fixtures,
        standings: vec![StandingEntry::new("team1".to_string())],
        ..Default::default()
    });
    let player = game.players.iter_mut().find(|p| p.id == "p_fwd0").unwrap();
    player.ovr = 70;
    player.morale = 40;
    player.stats.appearances = appearances;
    game
}

/// On how many of seeds 0..200 a day of player events brings a message starting with `prefix`.
fn days_with(game: &Game, prefix: &str) -> usize {
    (0..200)
        .filter(|&seed| {
            let mut day = game.clone();
            day.seed = seed;
            player_events::check_player_events(&mut day);
            day.messages.iter().any(|m| m.id.starts_with(prefix))
        })
        .count()
}

fn completed(count: u32, home: &str, away: &str) -> Vec<Fixture> {
    (0..count)
        .map(|i| fixture(i, home, away, FixtureStatus::Completed))
        .collect()
}

/// Given the benched forward and five completed matches, all the user's club away from home
/// When days of player events go by
/// Then he complains on some of them: away matches count toward the five
#[test]
fn bench_complaints_count_the_users_away_matches() {
    let game = benched_forward_with(completed(5, "team2", "team1"), 0);

    assert!(days_with(&game, "bench_complaint_") > 0);
}

/// Given the benched forward and five completed matches between two other clubs
/// When days of player events go by
/// Then he never complains: his own club has played nothing
#[test]
fn bench_complaints_ignore_matches_between_other_clubs() {
    let game = benched_forward_with(completed(5, "team2", "team3"), 0);

    assert_eq!(days_with(&game, "bench_complaint_"), 0);
}

/// Given the benched forward, four completed matches and ten scheduled ones
/// When days of player events go by
/// Then he never complains: only five matches played count
#[test]
fn bench_complaints_wait_for_five_completed_matches_not_scheduled_ones() {
    let mut fixtures = completed(4, "team1", "team2");
    fixtures.extend((4..14).map(|i| fixture(i, "team1", "team2", FixtureStatus::Scheduled)));
    let game = benched_forward_with(fixtures, 0);

    assert_eq!(days_with(&game, "bench_complaint_"), 0);
}

/// Given the forward with three appearances in ten completed matches: thirty percent
/// When days of player events go by
/// Then he never complains: only a player in under thirty percent of the matches does
#[test]
fn a_squad_player_with_thirty_percent_of_appearances_does_not_complain() {
    let game = benched_forward_with(completed(10, "team1", "team2"), 3);

    assert_eq!(days_with(&game, "bench_complaint_"), 0);
}

/// Given the benched forward and five completed home matches
/// When 200 days of player events go by, one per seed
/// Then he complains on between 5 and 45 of them: about one day in ten
#[test]
fn bench_complaints_come_about_one_day_in_ten() {
    let game = benched_forward_with(completed(5, "team1", "team2"), 0);

    let days = days_with(&game, "bench_complaint_");

    assert!((5..=45).contains(&days), "{days} days of 200");
}

/// Given p_fwd0 at morale exactly 30, the rest of the squad content
/// When 200 days of player events go by
/// Then he never asks for a crisis talk: that needs morale below 30
#[test]
fn a_player_at_exactly_thirty_morale_does_not_ask_for_a_crisis_talk() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 30;

    assert_eq!(days_with(&game, "morale_talk_"), 0);
}

/// Given two player messages already sent today, five completed matches, and a squad all at
///       morale 95 (each with a chance of a happy message every day)
/// When 200 days of player events go by
/// Then no further player message is ever sent that day: two a day is the cap
#[test]
fn two_player_messages_already_today_hold_back_every_other_player_message() {
    let mut game = benched_forward_with(completed(5, "team1", "team2"), 0);
    for player in game.players.iter_mut() {
        player.morale = 95;
    }
    inject_player_message(&mut game, "happy_player_x", "p_def0", "respond");
    inject_player_message(&mut game, "bench_complaint_y", "p_def1", "respond");

    let days_with_a_third = (0..200)
        .filter(|&seed| {
            let mut day = game.clone();
            day.seed = seed;
            player_events::check_player_events(&mut day);
            day.messages.len() > game.messages.len()
                && day.messages.iter().skip(game.messages.len()).any(|m| {
                    m.id.starts_with("morale_talk_")
                        || m.id.starts_with("bench_complaint_")
                        || m.id.starts_with("happy_player_")
                })
        })
        .count();

    assert_eq!(days_with_a_third, 0);
}

/// Given two player messages already sent today and the whole squad in crisis (morale 10)
/// When 200 days of player events go by
/// Then no crisis talk is asked for on any of them: two a day is the cap for crisis talks too
#[test]
fn two_player_messages_already_today_hold_back_crisis_talks() {
    let mut game = make_game();
    for player in game.players.iter_mut() {
        player.morale = 10;
    }
    inject_player_message(&mut game, "happy_player_x", "p_def0", "respond");
    inject_player_message(&mut game, "bench_complaint_y", "p_def1", "respond");

    assert_eq!(days_with(&game, "morale_talk_"), 0);
}

/// Given p_fwd0 in crisis (morale 20) and the rest of the squad content
/// When 200 days of player events go by
/// Then he asks for a talk on between 15 and 70 of them: about one day in five
#[test]
fn a_player_in_crisis_asks_for_a_talk_about_one_day_in_five() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 20;

    let days = days_with(&game, "morale_talk_");

    assert!((15..=70).contains(&days), "{days} days of 200");
}

/// Given p_fwd0 delighted (morale 95) and the rest of the squad content
/// When 200 days of player events go by
/// Then he tells the manager so on at most 12 of them: about one day in a hundred
#[test]
fn a_delighted_player_says_so_about_one_day_in_a_hundred() {
    let mut game = make_game();
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 95;

    let days = days_with(&game, "happy_player_");

    assert!(days <= 12, "{days} days of 200");
}

/// Given the benched forward at morale exactly 50, five completed matches in
/// When 200 days of player events go by
/// Then he never complains: only a player below 50 morale does
#[test]
fn a_benched_player_at_exactly_fifty_morale_does_not_complain() {
    let mut game = benched_forward_with(completed(5, "team1", "team2"), 0);
    game.players
        .iter_mut()
        .find(|p| p.id == "p_fwd0")
        .unwrap()
        .morale = 50;

    assert_eq!(days_with(&game, "bench_complaint_"), 0);
}

/// Given two player messages already sent today and the benched forward due a complaint
/// When 200 days of player events go by
/// Then he never complains on any of them: two a day is the cap for bench complaints too
#[test]
fn two_player_messages_already_today_hold_back_bench_complaints() {
    let mut game = benched_forward_with(completed(5, "team1", "team2"), 0);
    inject_player_message(&mut game, "happy_player_x", "p_def0", "respond");
    inject_player_message(&mut game, "morale_talk_y", "p_def1", "respond");

    assert_eq!(days_with(&game, "bench_complaint_p_fwd0"), 0);
}
