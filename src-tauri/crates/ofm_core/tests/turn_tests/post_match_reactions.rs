use super::*;

fn morale_of(game: &Game, team_id: &str) -> Vec<(String, u8)> {
    game.players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .map(|player| (player.id.clone(), player.morale))
        .collect()
}

fn team1_with_form(form: &[&str]) -> Game {
    let mut game = make_game_with_match();
    let team = game.teams.iter_mut().find(|t| t.id == "team1").unwrap();
    team.form = form.iter().map(|result| result.to_string()).collect();
    game
}

/// Given the user's club and a board and fans both at 50
/// When it wins, draws or loses, narrowly or by three
/// Then the board moves by +2, -1 or -3, and the fans by +5, -2 or -8, three more either way
///      for a margin of three
#[test]
fn the_board_and_the_fans_answer_each_result_by_a_fixed_amount() {
    let reactions: Vec<((u8, u8), u8, u8)> = [(1, 0), (1, 1), (0, 1), (3, 0), (0, 3)]
        .into_iter()
        .map(|(home_goals, away_goals)| {
            let mut game = make_game_with_match();
            game.manager.satisfaction = 50;
            game.manager.fan_approval = 50;
            let report = empty_report(home_goals, away_goals);
            turn::apply_match_report(&mut game, 0, "team1", "team2", &report);
            (
                (home_goals, away_goals),
                game.manager.satisfaction,
                game.manager.fan_approval,
            )
        })
        .collect();

    assert_eq!(
        reactions,
        vec![
            ((1, 0), 52, 55),
            ((1, 1), 49, 48),
            ((0, 1), 47, 42),
            ((3, 0), 52, 58),
            ((0, 3), 47, 39),
        ]
    );
}

/// Given the same squad on the same day
/// When it loses 0-4 instead of 0-1
/// Then every player's morale ends 9 lower: three more for each goal of margin beyond one
#[test]
fn a_heavy_defeat_costs_each_player_more_morale_than_a_narrow_one() {
    let mut narrow = make_game_with_match();
    turn::apply_match_report(&mut narrow, 0, "team1", "team2", &empty_report(0, 1));
    let mut heavy = make_game_with_match();
    turn::apply_match_report(&mut heavy, 0, "team1", "team2", &empty_report(0, 4));

    let narrow_morale = morale_of(&narrow, "team1");
    let heavy_morale = morale_of(&heavy, "team1");
    assert!(!narrow_morale.is_empty());
    for ((id, after_narrow), (_, after_heavy)) in narrow_morale.iter().zip(&heavy_morale) {
        assert_eq!(
            i16::from(*after_narrow) - i16::from(*after_heavy),
            9,
            "{id}: {after_narrow} after 0-1, {after_heavy} after 0-4"
        );
    }
}

/// Given a club that lost its last two matches, and the control, a club whose last two read
/// W then L
/// When each loses again
/// Then only the club on three straight defeats drops a further 5 to 10 morale across the squad
#[test]
fn a_third_straight_defeat_drops_the_whole_squad() {
    let mut streak = team1_with_form(&["W", "L", "L"]);
    turn::apply_match_report(&mut streak, 0, "team1", "team2", &empty_report(0, 1));
    let mut broken = team1_with_form(&["L", "W", "L"]);
    turn::apply_match_report(&mut broken, 0, "team1", "team2", &empty_report(0, 1));

    let on_streak = morale_of(&streak, "team1");
    let control = morale_of(&broken, "team1");
    assert!(!on_streak.is_empty());
    for ((id, streak_morale), (_, control_morale)) in on_streak.iter().zip(&control) {
        let drop = i16::from(*control_morale) - i16::from(*streak_morale);
        assert!((5..=10).contains(&drop), "{id}: dropped {drop}");
    }
}

/// Given a club that won its last two matches, and the control, a club whose last two read
/// L then W
/// When each wins again
/// Then only the club on three straight wins gains a further 2 to 5 morale across the squad
#[test]
fn a_third_straight_win_lifts_the_whole_squad() {
    let mut streak = team1_with_form(&["L", "W", "W"]);
    turn::apply_match_report(&mut streak, 0, "team1", "team2", &empty_report(1, 0));
    let mut broken = team1_with_form(&["W", "L", "W"]);
    turn::apply_match_report(&mut broken, 0, "team1", "team2", &empty_report(1, 0));

    let on_streak = morale_of(&streak, "team1");
    let control = morale_of(&broken, "team1");
    assert!(!on_streak.is_empty());
    for ((id, streak_morale), (_, control_morale)) in on_streak.iter().zip(&control) {
        let lift = i16::from(*streak_morale) - i16::from(*control_morale);
        assert!((2..=5).contains(&lift), "{id}: lifted {lift}");
    }
}

fn with_team1_morale(mut game: Game, morale: u8) -> Game {
    for player in game
        .players
        .iter_mut()
        .filter(|player| player.team_id.as_deref() == Some("team1"))
    {
        player.morale = morale;
    }
    game
}

/// Given the user's squad on two straight defeats, once at morale 50 and once at morale 3
/// When it loses a third time, which costs each player at 50 the result and the streak
/// Then each player at 3 stays at 3: a defeat is no reason to lift anyone to 10
#[test]
fn a_defeat_never_lifts_a_player_whose_morale_is_below_ten() {
    let mut steady = with_team1_morale(team1_with_form(&["L", "L"]), 50);
    let mut unhappy = with_team1_morale(team1_with_form(&["L", "L"]), 3);

    turn::apply_match_report(&mut steady, 0, "team1", "team2", &empty_report(0, 2));
    turn::apply_match_report(&mut unhappy, 0, "team1", "team2", &empty_report(0, 2));

    let steady_morale = morale_of(&steady, "team1");
    assert!(!steady_morale.is_empty());
    assert!(
        steady_morale.iter().all(|(_, morale)| *morale < 50),
        "{steady_morale:?}"
    );
    let unhappy_morale = morale_of(&unhappy, "team1");
    assert!(
        unhappy_morale.iter().all(|(_, morale)| *morale == 3),
        "{unhappy_morale:?}"
    );
}

/// Given the user's squad on two straight wins, once at morale 50 and once at morale 0
/// When it wins a third time, on the same day
/// Then each player at 0 gains exactly what the same player at 50 gains, not a jump to 10
#[test]
fn a_win_lifts_a_very_unhappy_player_by_what_it_lifts_anyone_else() {
    let mut steady = with_team1_morale(team1_with_form(&["W", "W"]), 50);
    let mut unhappy = with_team1_morale(team1_with_form(&["W", "W"]), 0);

    turn::apply_match_report(&mut steady, 0, "team1", "team2", &empty_report(2, 0));
    turn::apply_match_report(&mut unhappy, 0, "team1", "team2", &empty_report(2, 0));

    let lifts: Vec<(String, i16, i16)> = morale_of(&steady, "team1")
        .into_iter()
        .zip(morale_of(&unhappy, "team1"))
        .map(|((id, steady), (_, unhappy))| (id, i16::from(steady) - 50, i16::from(unhappy)))
        .collect();
    assert!(
        lifts
            .iter()
            .any(|(_, from_50, _)| (1..10).contains(from_50)),
        "the win must lift someone by less than 10 to tell the floor apart: {lifts:?}"
    );
    assert!(
        lifts.iter().all(|(_, from_50, from_0)| from_50 == from_0),
        "{lifts:?}"
    );
}
