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
