//! Telling the user that a match of theirs was moved: one message per event, about their own club
//! only, listing every match the event moved.

use domain::message::{InboxMessage, MessageCategory, MessagePriority};

use crate::calendar_reconcile::Move;
use crate::game::Game;
use crate::inbox;

/// Sends the user one message about the matches of their club among `moves`, unless there are
/// none or `event_key` has been announced before. `event_key` names the event (a day's sweep, a
/// rollover) and is the message id, so the sent-ledger dedupes it.
pub(crate) fn announce_moved_matches(game: &mut Game, event_key: &str, moves: &[Move]) {
    let Some(user_team_id) = game.manager.team_id.clone() else {
        return;
    };
    let lines: Vec<String> = moves
        .iter()
        .filter(|planned| {
            planned.home_team_id == user_team_id || planned.away_team_id == user_team_id
        })
        .map(|planned| {
            format!(
                "{} – {}: {} → {}",
                game.team_name_or_id(&planned.home_team_id),
                game.team_name_or_id(&planned.away_team_id),
                planned.from.format("%Y-%m-%d"),
                planned.to.format("%Y-%m-%d"),
            )
        })
        .collect();
    if lines.is_empty() {
        return;
    }
    let date = game.clock.current_date.to_rfc3339();
    inbox::emit_once(game, event_key, || {
        moved_matches_message(event_key, &lines, &date)
    });
}

fn moved_matches_message(id: &str, lines: &[String], date: &str) -> InboxMessage {
    InboxMessage::new(
        id.to_string(),
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::LeagueInfo)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_i18n(
        "be.msg.matchesMoved.subject",
        "be.msg.matchesMoved.body",
        std::collections::HashMap::from([("matches".to_string(), lines.join("\n"))]),
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{NaiveDate, TimeZone, Utc};
    use domain::league::RescheduleReason;
    use domain::manager::Manager;

    fn game_for(user_team: &str) -> Game {
        let mut manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "ENG".into(),
        );
        manager.hire(user_team.to_string());
        let clock = GameClock::new(Utc.with_ymd_and_hms(2033, 3, 1, 12, 0, 0).unwrap());
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    fn moved(home: &str, away: &str, from_day: u32, to_day: u32) -> Move {
        Move {
            competition_id: "league".into(),
            fixture_id: format!("{home}-{away}"),
            home_team_id: home.into(),
            away_team_id: away.into(),
            from: NaiveDate::from_ymd_opt(2033, 3, from_day).unwrap(),
            to: NaiveDate::from_ymd_opt(2033, 3, to_day).unwrap(),
            reason: RescheduleReason::CupTie,
        }
    }

    /// Given three of the user's matches and one of another club moved in one event,
    /// When the event is announced, Then exactly one message lists the user's three.
    #[test]
    fn one_event_moving_three_of_the_users_matches_sends_one_message_with_all_three() {
        let mut game = game_for("me");
        let moves = [
            moved("me", "a", 10, 13),
            moved("b", "me", 17, 20),
            moved("me", "c", 24, 27),
            moved("x", "y", 10, 13),
        ];

        announce_moved_matches(&mut game, "postponed_2033-03-01", &moves);

        assert_eq!(game.messages.len(), 1);
        let body = &game.messages[0].i18n_params["matches"];
        assert_eq!(body.lines().count(), 3);
        assert!(!body.contains('x'));
    }

    /// Given only other clubs' matches moved, When announced, Then the user hears nothing.
    #[test]
    fn a_draw_that_moves_only_other_clubs_matches_sends_nothing() {
        let mut game = game_for("me");
        announce_moved_matches(
            &mut game,
            "postponed_2033-03-01",
            &[moved("x", "y", 10, 13)],
        );
        assert!(game.messages.is_empty());
    }

    /// Given an event already announced, When it is announced again, Then no second message is
    /// sent; a different event sends its own.
    #[test]
    fn the_same_event_twice_sends_one_message_and_two_events_send_two() {
        let mut game = game_for("me");
        let moves = [moved("me", "a", 10, 13)];

        announce_moved_matches(&mut game, "postponed_2033-03-01", &moves);
        announce_moved_matches(&mut game, "postponed_2033-03-01", &moves);
        assert_eq!(game.messages.len(), 1);

        announce_moved_matches(&mut game, "postponed_2033-03-02", &moves);
        assert_eq!(game.messages.len(), 2);
    }
}
