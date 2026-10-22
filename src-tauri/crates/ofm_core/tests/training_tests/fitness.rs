//! The squad fitness summary: what the screens and the staff warnings both read.

use super::*;
use domain::player::{Injury, SquadRole};
use ofm_core::rated_tone::RatedTone;
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

/// Given healthy players on 70 and 75, an average of 72.5
/// When the summary is asked for
/// Then the average is painted from the same band rule as each player: Tiring, fair, not the green
///      a 70-up threshold gave it above players marked Tiring.
#[test]
fn the_average_has_a_band_from_the_same_rule() {
    let mut game = make_game();
    game.players.truncate(2);
    game.players[0].condition = 70;
    game.players[1].condition = 75;

    let summary = training::squad_fitness(&game, "team1");

    assert_eq!(summary.average_condition, Some(72.5));
    let average = summary.average_status.expect("an average to paint");
    assert_eq!(average.band, FitnessStatus::Tiring);
    assert_eq!(average.tone, RatedTone::Fair);
    assert_eq!(average.label_key, "be.condition.tiring");
}

/// Given healthy players on 74 and 75, an average of 74.5 that the screens show as 75
/// When the summary is asked for
/// Then the average's band is the band of the number shown, Fresh: the word never disagrees with
///      the figure beside it.
#[test]
fn the_average_band_is_the_band_of_the_rounded_figure() {
    let mut game = make_game();
    game.players.truncate(2);
    game.players[0].condition = 74;
    game.players[1].condition = 75;

    let summary = training::squad_fitness(&game, "team1");

    assert_eq!(summary.average_condition, Some(74.5));
    assert_eq!(
        summary.average_status.map(|status| status.band),
        Some(FitnessStatus::Fresh)
    );
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

/// Given conditions on each side of the three cut-offs
/// When each is given its band
/// Then 24 is critical, 25 and 39 exhausted, 40 and 74 tiring, 75 and 100 fresh: the one function
///      the counts, the words and the colours all use.
#[test]
fn a_condition_falls_in_one_of_four_bands() {
    assert_eq!(training::fitness_status(0), FitnessStatus::Critical);
    assert_eq!(training::fitness_status(24), FitnessStatus::Critical);
    assert_eq!(training::fitness_status(25), FitnessStatus::Exhausted);
    assert_eq!(training::fitness_status(39), FitnessStatus::Exhausted);
    assert_eq!(training::fitness_status(40), FitnessStatus::Tiring);
    assert_eq!(training::fitness_status(74), FitnessStatus::Tiring);
    assert_eq!(training::fitness_status(75), FitnessStatus::Fresh);
    assert_eq!(training::fitness_status(100), FitnessStatus::Fresh);
}

/// Given a condition in each band
/// When the screens ask how to show it
/// Then each band brings its own word key and its step on the good-to-bad scale, so no screen
///      compares the number against a threshold of its own.
#[test]
fn each_band_carries_its_word_and_its_tone() {
    let shown = |condition| {
        let status = training::condition_status(condition);
        (status.band, status.label_key, status.tone)
    };
    assert_eq!(
        shown(90),
        (FitnessStatus::Fresh, "be.condition.fresh", RatedTone::Good)
    );
    assert_eq!(
        shown(60),
        (
            FitnessStatus::Tiring,
            "be.condition.tiring",
            RatedTone::Fair
        )
    );
    assert_eq!(
        shown(30),
        (
            FitnessStatus::Exhausted,
            "be.condition.exhausted",
            RatedTone::Low
        )
    );
    assert_eq!(
        shown(10),
        (
            FitnessStatus::Critical,
            "be.condition.critical",
            RatedTone::Bad
        )
    );
}

/// Given healthy seniors on 40 and 74, the edges of the tiring band
/// When the summary is asked for
/// Then neither is counted as exhausted: the new band splits the old "fit" in two and leaves the
///      exhausted and critical counts, and the warnings that read them, exactly where they were.
#[test]
fn tiring_players_are_not_counted_as_exhausted() {
    let mut game = make_game();
    game.players[0].condition = 40;
    game.players[1].condition = 74;
    game.players[2].condition = 39;

    let summary = training::squad_fitness(&game, "team1");

    assert_eq!(summary.exhausted_count, 1);
    assert_eq!(summary.critical_count, 0);
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
    assert_eq!(summary.average_status, None);
    assert_eq!(summary.critical_count, 0);
    assert_eq!(summary.statuses.len(), 3);
    assert!(
        summary
            .statuses
            .values()
            .all(|status| status.band == FitnessStatus::Critical)
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

    assert_eq!(summary.statuses["p1"].band, FitnessStatus::Critical);
    assert_eq!(summary.statuses["p2"].band, FitnessStatus::Exhausted);
    assert_eq!(summary.statuses["p3"].band, FitnessStatus::Fresh);
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
    assert_eq!(summary.statuses["academy"].band, FitnessStatus::Critical);
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
