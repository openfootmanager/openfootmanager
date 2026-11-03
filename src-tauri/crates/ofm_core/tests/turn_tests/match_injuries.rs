use super::*;
use engine::{EventType, MatchEvent, Zone};

fn report_injuring(player_ids: &[&str]) -> MatchReport {
    let mut report = empty_report(0, 0);
    report.events = player_ids
        .iter()
        .map(|id| {
            MatchEvent::new(30, EventType::Injury, Side::Home, Zone::Midfield).with_player(id)
        })
        .collect();
    report
}

fn team1_player_ids(game: &Game) -> Vec<String> {
    game.players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some("team1"))
        .map(|player| player.id.clone())
        .collect()
}

fn injury_of(game: &Game, id: &str) -> Option<Injury> {
    game.players
        .iter()
        .find(|player| player.id == id)
        .and_then(|player| player.injury.clone())
}

/// Given a club match whose report records an injury to one of its players
/// When the match result is applied the way the day applies it
/// Then that player is injured: the post-match step is wired in, not just correct on its own.
#[test]
fn a_club_match_that_records_an_injury_injures_the_player() {
    let mut game = make_game_with_match();
    let hurt = team1_player_ids(&game)[0].clone();

    turn::apply_match_report(&mut game, 0, "team1", "team2", &report_injuring(&[&hurt]));

    assert!(injury_of(&game, &hurt).is_some());
}

/// Given two players of one club injured in the same match
/// When the result is applied under many different game seeds
/// Then the two do not always get the same injury: each is drawn from a stream of his own.
#[test]
fn two_players_hurt_in_one_match_are_drawn_independently() {
    let mut identical = 0;
    let seeds = 30;
    for seed in 0..seeds {
        let mut game = make_game_with_match();
        game.seed = seed;
        let ids = team1_player_ids(&game);
        let (first, second) = (ids[0].clone(), ids[1].clone());

        turn::apply_match_report(
            &mut game,
            0,
            "team1",
            "team2",
            &report_injuring(&[&first, &second]),
        );

        let (a, b) = (
            injury_of(&game, &first).expect("first injured"),
            injury_of(&game, &second).expect("second injured"),
        );
        if a.name == b.name && a.days_remaining == b.days_remaining {
            identical += 1;
        }
    }

    assert!(
        identical < seeds,
        "the two injuries were the same under every one of {seeds} seeds"
    );
}
