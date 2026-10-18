//! The squad fitness summary: what the screens and the staff warnings both read.

use super::*;

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
    game.players[1].injury = Some(domain::player::Injury {
        name: "knock".to_string(),
        days_remaining: 5,
    });
    game.players[2].condition = 100;

    let summary = training::squad_fitness(&game, "team1").expect("a summary");

    assert_eq!(summary.players, 2);
    assert_eq!(summary.average_condition, 69.5);
    assert_eq!(summary.exhausted_count, 1);
    assert_eq!(summary.critical_count, 0);
}

/// Given players on 24, 25, 39 and 40
/// When the summary is asked for
/// Then critical is below 25 (one), exhausted is below 40 (three): the thresholds are exclusive
#[test]
fn the_thresholds_are_below_25_and_below_40() {
    let mut game = make_game();
    let mut fourth = game.players[0].clone();
    fourth.id = "p4".to_string();
    game.players.push(fourth);
    for (player, condition) in game.players.iter_mut().zip([24, 25, 39, 40]) {
        player.condition = condition;
    }

    let summary = training::squad_fitness(&game, "team1").expect("a summary");

    assert_eq!(summary.critical_count, 1);
    assert_eq!(summary.exhausted_count, 3);
}

/// Given a club whose every player is injured
/// When the summary is asked for
/// Then there is none: there is no squad to average
#[test]
fn a_club_of_injured_players_has_no_summary() {
    let mut game = make_game();
    for player in game.players.iter_mut() {
        player.injury = Some(domain::player::Injury {
            name: "knock".to_string(),
            days_remaining: 5,
        });
    }

    assert!(training::squad_fitness(&game, "team1").is_none());
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
        player.injury = Some(domain::player::Injury {
            name: "knock".to_string(),
            days_remaining: 5,
        });
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
    let named = warning.i18n_params["players"].lines().count();
    assert_eq!(warning.i18n_params["criticalCount"], "3");
    assert_eq!(named, 3);
    assert!(!warning.i18n_params["players"].contains("(25%)"));
}
