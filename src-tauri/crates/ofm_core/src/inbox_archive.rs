//! Old mail leaves the live inbox but is never lost.
//!
//! `Game::messages` rides every day's response to the UI and grew about 0.35 MB a season, while
//! nothing in the game reads mail that is months old. Read mail older than [`ARCHIVE_AFTER_DAYS`]
//! moves to `Game::archived_messages`: skipped on IPC like the other archives, kept in the save,
//! and served a page at a time by [`page`]. Unread mail never moves, so the unread badge and any
//! message still waiting on an answer stay in the inbox.
//!
//! The sent-ledger (`Game::emitted_events`) is untouched, so archiving cannot make an event fire again.

use chrono::{Datelike, Duration, NaiveDate};
use domain::message::InboxMessage;
use serde::{Deserialize, Serialize};

use crate::game::Game;
use crate::slices::news::article_day;

/// A read message older than this many days is archived.
pub const ARCHIVE_AFTER_DAYS: i64 = 180;

/// The most a page returns, however many are asked for.
pub const MAX_PAGE_SIZE: usize = 200;

/// Where the previous page ended: the last message it returned, in newest-first order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveCursor {
    pub date: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchivedMessagesPage {
    pub items: Vec<InboxMessage>,
    /// Pass back as `before` for the next page; `None` when this was the last.
    pub next_before: Option<ArchiveCursor>,
}

/// Moves read mail older than [`ARCHIVE_AFTER_DAYS`] to the archive; returns how many moved.
///
/// A message whose date cannot be read stays where it is: guessing its age risks hiding mail.
pub fn archive_old_mail(game: &mut Game) -> usize {
    let cutoff = game.clock.current_date.date_naive() - Duration::days(ARCHIVE_AFTER_DAYS);
    let (old, live): (Vec<_>, Vec<_>) = std::mem::take(&mut game.messages)
        .into_iter()
        .partition(|message| message.read && is_before(&message.date, cutoff));
    game.messages = live;
    let moved = old.len();
    game.archived_messages.extend(old);
    moved
}

/// The monthly and season-end moment to run [`archive_old_mail`]: bounding the live inbox
/// to about half a year without touching it every day.
pub fn archive_on_the_first_of_the_month(game: &mut Game) {
    if game.clock.current_date.day() == 1 {
        archive_old_mail(game);
    }
}

fn is_before(date: &str, cutoff: NaiveDate) -> bool {
    NaiveDate::parse_from_str(article_day(date), "%Y-%m-%d").is_ok_and(|day| day < cutoff)
}

/// A page of the archive, newest first, starting after `before`.
///
/// Keyed on (date, id) rather than an offset so mail archived between two requests cannot shift
/// the page under the reader.
pub fn page(game: &Game, before: Option<&ArchiveCursor>, limit: usize) -> ArchivedMessagesPage {
    let limit = limit.clamp(1, MAX_PAGE_SIZE);
    let mut newest_first: Vec<&InboxMessage> = game
        .archived_messages
        .iter()
        .filter(|message| {
            before.is_none_or(|cursor| {
                (message.date.as_str(), message.id.as_str())
                    < (cursor.date.as_str(), cursor.id.as_str())
            })
        })
        .collect();
    newest_first.sort_by(|a, b| (&b.date, &b.id).cmp(&(&a.date, &a.id)));

    let more = newest_first.len() > limit;
    newest_first.truncate(limit);
    let next_before = more
        .then(|| newest_first.last())
        .flatten()
        .map(|last| ArchiveCursor {
            date: last.date.clone(),
            id: last.id.clone(),
        });
    ArchivedMessagesPage {
        items: newest_first.into_iter().cloned().collect(),
        next_before,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;

    fn game_on(year: i32, month: u32, day: u32) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    fn mail(id: &str, date: &str, read: bool) -> InboxMessage {
        let mut message = InboxMessage::new(
            id.into(),
            "Subject".into(),
            "Body".into(),
            "Board".into(),
            date.into(),
        );
        message.read = read;
        message
    }

    fn ids(messages: &[InboxMessage]) -> Vec<&str> {
        messages.iter().map(|m| m.id.as_str()).collect()
    }

    /// Given read and unread mail, old and recent, on 2031-07-01
    /// When old mail is archived
    /// Then only read mail older than the window moves; unread and recent mail stays live.
    #[test]
    fn only_old_read_mail_is_archived() {
        let mut game = game_on(2031, 7, 1);
        game.messages = vec![
            mail("old-read", "2030-12-01", true),
            mail("old-unread", "2030-12-01", false),
            mail("recent-read", "2031-06-01", true),
        ];

        let moved = archive_old_mail(&mut game);

        assert_eq!(moved, 1);
        assert_eq!(ids(&game.archived_messages), ["old-read"]);
        assert_eq!(ids(&game.messages), ["old-unread", "recent-read"]);
    }

    /// Given read mail exactly at the window's edge and one day past it
    /// When old mail is archived
    /// Then the day on the edge stays live and the day past it moves.
    #[test]
    fn the_window_edge_is_inclusive_of_the_live_inbox() {
        let mut game = game_on(2031, 7, 1);
        // 2031-07-01 minus 180 days is 2031-01-02.
        game.messages = vec![
            mail("on-the-edge", "2031-01-02", true),
            mail("a-day-past", "2031-01-01", true),
        ];

        archive_old_mail(&mut game);

        assert_eq!(ids(&game.messages), ["on-the-edge"]);
        assert_eq!(ids(&game.archived_messages), ["a-day-past"]);
    }

    /// Given mail dated as an RFC3339 timestamp and mail with an unreadable date
    /// When old mail is archived
    /// Then the timestamp is read by its day, and the unreadable date stays live.
    #[test]
    fn dates_are_read_by_their_day_and_unreadable_ones_stay_put() {
        let mut game = game_on(2031, 7, 1);
        game.messages = vec![
            mail("stamped", "2030-11-05T09:30:00+00:00", true),
            mail("garbled", "last spring", true),
        ];

        archive_old_mail(&mut game);

        assert_eq!(ids(&game.archived_messages), ["stamped"]);
        assert_eq!(ids(&game.messages), ["garbled"]);
    }

    /// Given the same old read mail on the 1st and on another day
    /// When the monthly step runs
    /// Then it archives on the 1st only.
    #[test]
    fn the_monthly_step_runs_on_the_first_only() {
        let mut second = game_on(2031, 7, 2);
        second.messages = vec![mail("old", "2030-01-01", true)];
        archive_on_the_first_of_the_month(&mut second);
        assert!(second.archived_messages.is_empty());

        let mut first = game_on(2031, 7, 1);
        first.messages = vec![mail("old", "2030-01-01", true)];
        archive_on_the_first_of_the_month(&mut first);
        assert_eq!(ids(&first.archived_messages), ["old"]);
    }

    /// Given an archive of five messages, two sharing a date
    /// When it is read two at a time
    /// Then pages come newest first, the shared date is split without loss or repeat by id, and the last
    ///      page has no next cursor.
    #[test]
    fn pages_run_newest_first_without_loss_or_repeat() {
        let mut game = game_on(2031, 7, 1);
        game.archived_messages = vec![
            mail("a", "2030-01-01", true),
            mail("b", "2030-03-01", true),
            mail("c", "2030-03-01", true),
            mail("d", "2030-02-01", true),
            mail("e", "2029-12-01", true),
        ];

        let first = page(&game, None, 2);
        assert_eq!(ids(&first.items), ["c", "b"]);
        let second = page(&game, first.next_before.as_ref(), 2);
        assert_eq!(ids(&second.items), ["d", "a"]);
        let third = page(&game, second.next_before.as_ref(), 2);
        assert_eq!(ids(&third.items), ["e"]);
        assert_eq!(third.next_before, None);
    }

    /// Given an archive and absurd limits
    /// When it is paged
    /// Then zero returns one message and a huge limit is capped at the maximum page.
    #[test]
    fn the_page_size_is_kept_within_bounds() {
        let mut game = game_on(2031, 7, 1);
        game.archived_messages = (0..MAX_PAGE_SIZE + 5)
            .map(|n| mail(&format!("m{n:04}"), "2030-01-01", true))
            .collect();

        assert_eq!(page(&game, None, 0).items.len(), 1);
        let big = page(&game, None, usize::MAX);
        assert_eq!(big.items.len(), MAX_PAGE_SIZE);
        assert!(big.next_before.is_some());
    }

    /// Given a message the player already saw, now archived
    /// When the sent-ledger is consulted
    /// Then its event is still sent, so archiving cannot make it fire again.
    #[test]
    fn archiving_does_not_reopen_the_sent_ledger() {
        let mut game = game_on(2031, 7, 1);
        crate::inbox::emit(&mut game, mail("event-1", "2030-01-01", true));
        archive_old_mail(&mut game);

        assert!(game.messages.is_empty());
        assert!(!crate::inbox::emit(
            &mut game,
            mail("event-1", "2031-07-01", false)
        ));
    }
}
