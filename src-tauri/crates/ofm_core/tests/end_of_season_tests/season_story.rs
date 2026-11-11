//! What the player is told and credited at the end of a season: the review, the payout, the
//! movement message and the manager's record.

use super::*;

fn user_messages_starting(game: &Game, prefix: &str) -> Vec<String> {
    game.messages
        .iter()
        .filter(|message| message.id.starts_with(prefix))
        .map(|message| message.id.clone())
        .collect()
}

/// Given a split-season country whose two halves finished on the same day, the user's club
///       first in the opening half and last in the closing one
/// When the season ends
/// Then the summary, the club's record and the payout all come from the same half: the club is
///      told it finished first, recorded first, and paid the champion's 5,000,000
#[test]
fn split_season_halves_ending_on_the_same_day_pay_from_the_table_the_summary_reports() {
    let mut game = make_completed_season_game();
    let mut apertura = first_division("ar-d1-apertura", "AR", "america", &["team1", "team2"]);
    apertura.priority = 0;
    apertura.fixtures = vec![make_completed_fixture("ap", "team1", "team2", 2, 0)];
    let mut clausura = first_division("ar-d1-clausura", "AR", "america", &["team2", "team1"]);
    clausura.priority = 1;
    clausura.fixtures = vec![make_completed_fixture("cl", "team2", "team1", 2, 0)];
    game.league = Some(apertura.clone());
    game.competitions = vec![apertura, clausura];
    let finance_before = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .expect("team1")
        .finance;

    let summary = process_end_of_season(&mut game);

    let team1 = game
        .teams
        .iter()
        .find(|team| team.id == "team1")
        .expect("team1");
    let payout = game
        .messages
        .iter()
        .find(|message| message.id.starts_with("season_payout_"))
        .expect("a payout message");
    assert_eq!(
        (
            summary.user_position,
            team1.history[0].league_position,
            payout.i18n_params.get("position").map(String::as_str),
            payout.i18n_params.get("amount").map(String::as_str),
            team1.finance - finance_before,
        ),
        (1, 1, Some("1"), Some("5000000"), 5_000_000)
    );
}

/// Given the user's club in a top division, and a feeder of the same rank whose champion is
///       promoted into it along a berth
/// When the season ends and the user's club, bottom of the top division, drops into the feeder
/// Then the board tells the user they were relegated: a move between equal ranks that no berth
///      made is a relegation, not a promotion
#[test]
fn a_berth_relegation_into_an_equal_rank_feeder_is_reported_as_a_relegation() {
    let mut game = make_completed_season_game();
    game.manager.hire("team2".to_string());
    for id in ["team3", "team4"] {
        if !game.teams.iter().any(|team| team.id == id) {
            game.teams.push(make_team(id, &format!("{id} FC")));
        }
    }
    let central = League {
        id: "central".to_string(),
        name: "Central League".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team1".to_string(), "team2".to_string()],
        fixtures: vec![make_completed_fixture("cf", "team1", "team2", 2, 0)],
        standings: vec![
            make_standing("team1", 1, 0, 0, 2, 0),
            make_standing("team2", 0, 0, 1, 0, 2),
        ],
        ..Default::default()
    };
    let mut feeder = League {
        id: "feeder".to_string(),
        name: "Feeder League".to_string(),
        country_id: Some("ENG".to_string()),
        priority: 0,
        season: 1,
        participant_ids: vec!["team3".to_string(), "team4".to_string()],
        fixtures: vec![make_completed_fixture("ff", "team3", "team4", 3, 0)],
        standings: vec![
            make_standing("team3", 1, 0, 0, 3, 0),
            make_standing("team4", 0, 0, 1, 0, 3),
        ],
        ..Default::default()
    };
    feeder.berths = vec![Berth {
        target: "central".to_string(),
        rule: BerthRule::PositionRange { from: 1, to: 1 },
        fallback_to: None,
    }];
    game.league = Some(central.clone());
    game.competitions = vec![central, feeder];

    process_end_of_season(&mut game);

    let feeder_now = game
        .competitions
        .iter()
        .find(|competition| competition.id == "feeder")
        .expect("feeder");
    assert!(
        feeder_now.participant_ids.contains(&"team2".to_string()),
        "precondition: the user's club went down into the feeder: {:?}",
        feeder_now.participant_ids
    );
    assert_eq!(
        (
            user_messages_starting(&game, "relegation_").len(),
            user_messages_starting(&game, "promotion_").len(),
        ),
        (1, 0)
    );
}

/// Given the user's club ended the season with one win, one draw and one defeat
/// When the season ends
/// Then the manager's career record counts the draw and the defeat as well as the win
#[test]
fn manager_career_stats_count_draws_and_losses() {
    let mut game = make_completed_season_game();
    if let Some(league) = &mut game.league {
        league.fixtures = vec![
            make_completed_fixture("f1", "team1", "team2", 2, 1),
            make_completed_fixture("f2", "team2", "team1", 1, 1),
            make_completed_fixture("f3", "team2", "team1", 1, 0),
        ];
        league.standings = vec![
            make_standing("team1", 1, 1, 1, 3, 3),
            make_standing("team2", 1, 1, 1, 3, 3),
        ];
    }

    process_end_of_season(&mut game);

    let stats = &game.manager.career_stats;
    assert_eq!(
        (stats.matches_managed, stats.wins, stats.draws, stats.losses),
        (3, 1, 1, 1)
    );
}

/// Given a manager without a club when the season ends
/// When the season ends
/// Then no prize-money message is sent: there is no club to have been paid
#[test]
fn an_unemployed_manager_is_not_told_of_a_prize_payout() {
    let mut game = make_completed_season_game();
    game.manager.team_id = None;

    process_end_of_season(&mut game);

    assert_eq!(
        user_messages_starting(&game, "season_payout_"),
        Vec::<String>::new()
    );
}

/// Given the user manages Test FC, which played its rival Rival FC
/// When the season ends
/// Then the board's season review names Test FC, the user's club
#[test]
fn the_season_review_names_the_users_club() {
    let mut game = make_completed_season_game();

    process_end_of_season(&mut game);

    let review = game
        .messages
        .iter()
        .find(|message| message.id == "season_end_1")
        .expect("a season review");
    assert_eq!(
        review.i18n_params.get("team").map(String::as_str),
        Some("Test FC")
    );
}

/// Given a league whose last match was played on 1 June, with a fixture still scheduled for later
/// When the season ends
/// Then the season review is dated on the last match played, not on the scheduled one
#[test]
fn season_messages_are_dated_on_the_last_league_match_played() {
    let mut game = make_completed_season_game();
    if let Some(league) = &mut game.league {
        let mut unplayed = make_completed_fixture("f3", "team1", "team2", 0, 0);
        unplayed.date = "2025-07-15".to_string();
        unplayed.status = FixtureStatus::Scheduled;
        unplayed.result = None;
        league.fixtures.push(unplayed);
    }

    process_end_of_season(&mut game);

    let review = game
        .messages
        .iter()
        .find(|message| message.id == "season_end_1")
        .expect("a season review");
    assert_eq!(review.date, "2025-06-01");
}
