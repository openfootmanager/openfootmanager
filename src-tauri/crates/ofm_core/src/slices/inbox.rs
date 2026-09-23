use crate::game::Game;
use domain::message::InboxMessage;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MessagesQuery {}

/// Whether a message dated `date` is visible at game-date `today` (formatted
/// `%Y-%m-%d`).
///
/// The mirror of [`crate::slices::news::article_is_visible`], which the news
/// feed has had since a future-dated article sat permanently atop it. Nothing
/// currently dates a message ahead of the clock, so this is a guard rather than
/// a fix — but the inbox had no such rule at all, and a message dated at a
/// fixture or a deadline would have pinned itself to the top of the list and
/// inflated the unread badge from the day it was created until the day it
/// referred to.
///
/// Compare on the day prefix: message dates come as both a bare `YYYY-MM-DD`
/// and an RFC3339 timestamp, and comparing whole strings would sort a
/// timestamped message after the bare day and hide same-day mail.
pub fn message_is_visible(date: &str, today: &str) -> bool {
    crate::slices::news::article_day(date) <= today
}

pub fn query_messages(game: &Game, _query: &MessagesQuery) -> Vec<InboxMessage> {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    game.messages
        .iter()
        .filter(|message| message_is_visible(&message.date, &today))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_dated_after_the_clock_is_not_visible() {
        assert!(!message_is_visible("2026-08-21", "2026-08-20"));
    }

    #[test]
    fn a_message_dated_today_is_visible_in_either_date_shape() {
        assert!(message_is_visible("2026-08-20", "2026-08-20"));
        assert!(message_is_visible(
            "2026-08-20T18:30:00+00:00",
            "2026-08-20"
        ));
    }

    #[test]
    fn a_message_dated_in_the_past_is_visible() {
        assert!(message_is_visible(
            "2026-07-01T09:00:00+00:00",
            "2026-08-20"
        ));
    }
}
