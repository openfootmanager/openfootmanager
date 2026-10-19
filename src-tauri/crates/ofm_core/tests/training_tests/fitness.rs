//! The squad fitness summary: what the screens and the staff warnings both read.

use super::*;
use domain::player::{Injury, SquadRole};
use training::FitnessStatus;

fn knock() -> Injury {
    Injury {
        name: "knock".to_string(),
        days_remaining: 5,
    }
}

/// `count` healthy seniors of the user's club at `condition`, appended to the game.
fn add_seniors(game: &mut Game, count: usize, condition: u8) {
    for index in 0..count {
        let mut player = make_player(&format!("sen{index}"), "Senior", "team1", "1995-01-01");
        player.condition = condition;
        game.players.push(player);
    }
}

/// Given a squad where one fit player, one exhausted player (39) and one injured player on 24 are
///       in the club
/// When the summary is asked for
/// Then the injured player is left out of every figure: two players, an average of 69.5, one
///      exhausted (below 40) and none critical (below 25)
#[test]
fn the_summary_leaves_injured_players_out_and_counts_below_the_thresholds() {
    let mut game = make_game();
    game.players[0].condition = 39;
    game.players[1].condition = 24;
    game.players[1].injury = Some(knock());
    game.players[2].condition = 100;

    let summary = training::squad_fitness(&game, "team1");

    assert_eq!(summary.players, 2);
    assert_eq!(summary.average_condition, Some(69.5));
    assert_eq!(summary.exhausted_count, 1);
    assert_eq!(summary.critical_count, 0);
}

/// Given players on 24, 25, 39 and 40
/// When the summary is asked for
/// Then critical is below 25 (one), exhausted is below 40 (three, the critical one among them): the
///      thresholds are exclusive
#[test]
fn the_thresholds_are_below_25_and_below_40() {
    let mut game = make_game();
    let mut fourth = game.players[0].clone();
    fourth.id = "p4".to_string();
    game.players.push(fourth);
    for (player, condition) in game.players.iter_mut().zip([24, 25, 39, 40]) {
        player.condition = condition;
    }

    let summary = training::squad_fitness(&game, "team1");

    assert_eq!(summary.critical_count, 1);
    assert_eq!(summary.exhausted_count, 3);
}

/// Given conditions on each side of both cut-offs
/// When each is given its status
/// Then 24 is critical, 25 and 39 are exhausted, and 40 is fit: the one function the counts use.
#[test]
fn a_condition_has_a_status_by_the_two_cut_offs() {
    assert_eq!(training::fitness_status(24), FitnessStatus::Critical);
    assert_eq!(training::fitness_status(25), FitnessStatus::Exhausted);
    assert_eq!(training::fitness_status(39), FitnessStatus::Exhausted);
    assert_eq!(training::fitness_status(40), FitnessStatus::Fit);
}

/// Given a club whose every player is injured
/// When the summary is asked for
/// Then nobody is counted and there is no average to show, but each player still has a status:
///      the training list colours the injured too.
#[test]
fn a_club_of_injured_players_has_no_average_but_every_player_a_status() {
    let mut game = make_game();
    for player in game.players.iter_mut() {
        player.injury = Some(knock());
        player.condition = 20;
    }

    let summary = training::squad_fitness(&game, "team1");

    assert_eq!(summary.players, 0);
    assert_eq!(summary.average_condition, None);
    assert_eq!(summary.critical_count, 0);
    assert_eq!(summary.statuses.len(), 3);
    assert!(
        summary
            .statuses
            .values()
            .all(|status| *status == FitnessStatus::Critical)
    );
}

/// Given an injured player on 10, a fit one on 30 and one on 90
/// When the summary is asked for
/// Then the statuses name every club player, the injured one critical although the counts leave him
///      out.
#[test]
fn the_statuses_cover_the_injured_the_counts_leave_out() {
    let mut game = make_game();
    game.players[0].condition = 10;
    game.players[0].injury = Some(knock());
    game.players[1].condition = 30;
    game.players[2].condition = 90;

    let summary = training::squad_fitness(&game, "team1");

    assert_eq!(summary.statuses["p1"], FitnessStatus::Critical);
    assert_eq!(summary.statuses["p2"], FitnessStatus::Exhausted);
    assert_eq!(summary.statuses["p3"], FitnessStatus::Fit);
    assert_eq!(summary.critical_count, 0);
}

/// Given eighteen fit seniors and an academy player on 10 who has not been called up
/// When the summary is asked for
/// Then the academy player is outside every figure (he is not in the match-day pool), but he has a
///      status for the list.
#[test]
fn an_academy_player_who_is_not_called_up_is_not_counted() {
    let mut game = make_game();
    game.players.clear();
    add_seniors(&mut game, 18, 90);
    let mut academy = make_player("academy", "Academy", "team1", "2008-01-01");
    academy.squad_role = SquadRole::Youth;
    academy.condition = 10;
    game.players.push(academy);

    let summary = training::squad_fitness(&game, "team1");

    assert_eq!(summary.players, 18);
    assert_eq!(summary.critical_count, 0);
    assert_eq!(summary.average_condition, Some(90.0));
    assert_eq!(summary.statuses["academy"], FitnessStatus::Critical);
}

/// Given eighteen fit seniors and three academy players on 10, none called up
/// When the staff's fitness warning runs
/// Then it stays quiet: the academy does not drive the warning.
#[test]
fn academy_players_who_are_not_called_up_do_not_trigger_the_warning() {
    let mut game = make_game();
    game.players.clear();
    add_seniors(&mut game, 18, 90);
    for index in 0..3 {
        let mut academy = make_player(&format!("acad{index}"), "Academy", "team1", "2008-01-01");
        academy.squad_role = SquadRole::Youth;
        academy.condition = 10;
        game.players.push(academy);
    }

    training::check_squad_fitness_warnings(&mut game);

    assert!(
        game.messages
            .iter()
            .all(|message| !message.id.starts_with("fitness_warn_"))
    );
}

/// Given injured players on a very low condition and a fit rest of the squad
/// When the staff's fitness warning runs
/// Then it stays quiet: the warning reads the same summary, which does not count them
#[test]
fn injured_exhausted_players_do_not_trigger_the_warning() {
    let mut game = make_game();
    game.players[0].condition = 90;
    for player in game.players.iter_mut().skip(1) {
        player.condition = 10;
        player.injury = Some(knock());
    }

    training::check_squad_fitness_warnings(&mut game);

    assert!(
        game.messages
            .iter()
            .all(|message| !message.id.starts_with("fitness_warn_"))
    );
}

/// Given three players on 10 and a fourth on exactly 25
/// When the staff's critical fitness warning runs
/// Then it names the three, and not the one on 25: critical is below 25, in the names as in the count
#[test]
fn the_critical_warning_names_only_players_below_25() {
    let mut game = make_game();
    let mut fourth = game.players[0].clone();
    fourth.id = "p4".to_string();
    fourth.match_name = "Quarter".to_string();
    game.players.push(fourth);
    for (player, condition) in game.players.iter_mut().zip([10, 10, 10, 25]) {
        player.condition = condition;
    }

    training::check_squad_fitness_warnings(&mut game);

    let warning = game
        .messages
        .iter()
        .find(|message| message.id.starts_with("fitness_warn_"))
        .expect("a critical warning");
    let named: Vec<String> = warning.i18n_params["players"]
        .lines()
        .map(str::to_string)
        .collect();
    let expected: Vec<String> = game
        .players
        .iter()
        .take(3)
        .map(|player| format!("{} (10%)", player.match_name))
        .collect();
    assert_eq!(warning.i18n_params["criticalCount"], "3");
    assert_eq!(named, expected);
}
