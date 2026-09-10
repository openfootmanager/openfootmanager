//! The sent-ledger: the single way to put a message in the player's inbox.
//!
//! Generators used to guard themselves with `game.messages.iter().any(|m| m.id
//! == msg_id)`. That reads the mailbox, and the mailbox is the player's — they
//! delete from it, bulk-delete from it, and clear it. So the guard answered "is
//! this still in the inbox", never "did we already send this", and every
//! generator re-fired as soon as its message was removed. The World Cup
//! champion announcement made it visible because it re-evaluates its condition
//! every simulated day for the eleven months the tournament stays in
//! `competitions` (issue #520).
//!
//! [`emit_once`] answers the real question against [`Game::emitted_events`],
//! which the player cannot touch and which is persisted with the save.

use crate::game::Game;
use domain::message::InboxMessage;

/// Send `build()` to the inbox unless `key` has already been sent, and return
/// whether it was sent.
///
/// The key is also the message id, so a message and its ledger entry always
/// agree. Callers that carry a side effect — the contract-concern morale hit is
/// the one that bit us — must gate that effect on the returned `bool`, never on
/// the message's presence in the inbox.
///
/// Choosing a key is choosing how often the event may recur. A key that names
/// only an entity (`morale_talk_{player_id}`) means *once per save, ever*; if
/// the event should be able to happen again, the key needs the scope it recurs
/// on (a season, a contract, a day).
pub fn emit_once(game: &mut Game, key: &str, build: impl FnOnce() -> InboxMessage) -> bool {
    if game.emitted_events.contains(key) {
        return false;
    }
    let message = build();
    debug_assert_eq!(
        message.id, key,
        "emit_once key must be the message id, or the ledger and the inbox disagree"
    );
    emit(game, message)
}

/// Send `message` unless its id has been sent before, and return whether it was
/// sent.
///
/// The id is the ledger key. Use this where the message is already built;
/// [`emit_once`] where building it is the expensive part.
pub fn emit(game: &mut Game, message: InboxMessage) -> bool {
    if game.emitted_events.contains(&message.id) {
        return false;
    }
    game.emitted_events.insert(message.id.clone());
    game.messages.push(message);
    true
}

/// Send each of `messages` whose id has not been sent before, and return how
/// many were sent.
///
/// For generators that build a batch and extend the inbox in one go. Ids
/// repeated inside `messages` are sent once.
pub fn emit_all(game: &mut Game, messages: Vec<InboxMessage>) -> usize {
    let mut sent = 0;
    for message in messages {
        if emit(game, message) {
            sent += 1;
        }
    }
    sent
}

/// The season to scope a recurring event's key by.
///
/// Some events are conditions rather than moments — a player is unhappy, a
/// squad member is not getting minutes. Keyed on the player alone they would
/// enter the ledger once and never be raised again; keyed on the day they would
/// arrive as often as the dice allow. The season is the unit a manager thinks
/// in, so `morale_talk_{player}_{season}` means "he'll raise it once a season",
/// which is roughly what the old accidental throttle delivered.
pub fn recurrence_season(game: &Game) -> u32 {
    game.primary_competition()
        .map(|competition| competition.season)
        .unwrap_or_else(|| {
            game.clock
                .current_date
                .format("%Y")
                .to_string()
                .parse()
                .unwrap_or(0)
        })
}

/// Whether `key` has already been announced.
///
/// For callers that need to skip expensive work before they can build the
/// message. Prefer [`emit_once`] where the message is cheap to build.
pub fn already_emitted(game: &Game, key: &str) -> bool {
    game.emitted_events.contains(key)
}

/// Populate the ledger of a save written before it existed.
///
/// Runs on load. Without it, the first advance after upgrading would find an
/// empty ledger and re-announce everything still sitting in the inbox. Message
/// ids are the ledger's keys, so seeding is a straight copy.
///
/// World Cup champions get an extra pass: that message may already have been
/// purged from the mailbox, and the archive remembers what the inbox forgot.
pub fn seed_ledger_from_save(game: &mut Game) {
    if !game.emitted_events.is_empty() {
        return;
    }
    let ids: Vec<String> = game.messages.iter().map(|m| m.id.clone()).collect();
    game.emitted_events.extend(ids);
    let champions: Vec<u32> = game
        .world_history
        .world_cup_champions
        .iter()
        .map(|record| record.year)
        .collect();
    for year in champions {
        game.emitted_events
            .insert(format!("world_cup_champion_{year}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use domain::world_history::WorldCupChampionRecord;

    fn game() -> Game {
        let start = chrono::DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        Game::new(
            GameClock::new(start),
            domain::manager::Manager::new(
                "mgr".to_string(),
                "Test".to_string(),
                "Manager".to_string(),
                "1980-01-01".to_string(),
                "BRA".to_string(),
            ),
            vec![],
            vec![],
            vec![],
            vec![],
        )
    }

    fn message(id: &str) -> InboxMessage {
        InboxMessage::new(
            id.to_string(),
            String::new(),
            String::new(),
            String::new(),
            "2026-06-01".to_string(),
        )
    }

    #[test]
    fn emit_once_sends_the_first_time_only() {
        let mut game = game();
        assert!(emit_once(&mut game, "evt", || message("evt")));
        assert!(!emit_once(&mut game, "evt", || message("evt")));
        assert_eq!(game.messages.len(), 1);
    }

    #[test]
    fn deleting_the_message_does_not_resurrect_the_event() {
        // The whole point: the mailbox is the player's, the ledger is ours.
        let mut game = game();
        emit_once(&mut game, "evt", || message("evt"));
        game.messages.clear();
        assert!(!emit_once(&mut game, "evt", || message("evt")));
        assert!(game.messages.is_empty());
    }

    #[test]
    fn seeding_adopts_the_inbox_of_a_save_written_before_the_ledger() {
        let mut game = game();
        game.messages.push(message("old_event"));
        seed_ledger_from_save(&mut game);
        assert!(!emit_once(&mut game, "old_event", || message("old_event")));
        assert_eq!(game.messages.len(), 1);
    }

    #[test]
    fn seeding_recovers_a_champion_whose_message_was_already_purged() {
        let mut game = game();
        game.world_history
            .record_world_cup_champion(WorldCupChampionRecord {
                year: 2026,
                nation_code: "BRA".to_string(),
                nation_name: "Brazil".to_string(),
            });
        seed_ledger_from_save(&mut game);
        assert!(already_emitted(&game, "world_cup_champion_2026"));
    }

    /// No generator may dedupe against the mailbox again.
    ///
    /// This is a source check because the mistake is invisible at runtime: code
    /// that asks the inbox "have I sent this?" behaves correctly until the day a
    /// player deletes the message, and no ordinary test constructs that state.
    /// Nine generators carried the pattern before #520, in two spellings, and
    /// each looked perfectly reasonable in isolation.
    ///
    /// Deliberately narrow: it bans comparing a message *id*, not reading the
    /// inbox. `has_pending_sponsor_offer` scans for unresolved actions and
    /// `job_offers` matches an id prefix to resolve one — both are questions
    /// about the mailbox's present contents, which is what the mailbox is for.
    #[test]
    fn no_generator_dedupes_against_the_mailbox() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("readable source directory") {
                let path = entry.expect("readable entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                // This module documents the banned pattern, so it is exempt.
                if path == root.join("inbox.rs") {
                    continue;
                }
                let source = std::fs::read_to_string(&path).expect("readable source file");
                for (number, line) in source.lines().enumerate() {
                    let squashed: String = line
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect::<String>();
                    let scans_mailbox = squashed.contains("messages.iter()")
                        || squashed.contains("messages.iter().map(");
                    if scans_mailbox
                        && (squashed.contains(".id==") || squashed.contains("id.clone()"))
                    {
                        offenders.push(format!(
                            "{}:{}",
                            path.strip_prefix(&root).unwrap_or(&path).display(),
                            number + 1
                        ));
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "these dedupe against the inbox instead of the sent-ledger, so deleting \
             the message re-sends the event (#520). Use inbox::emit / emit_once / \
             already_emitted:\n  {}",
            offenders.join("\n  ")
        );
    }

    #[test]
    fn seeding_leaves_a_populated_ledger_alone() {
        let mut game = game();
        emit_once(&mut game, "evt", || message("evt"));
        game.messages.clear();
        game.messages.push(message("stray"));
        seed_ledger_from_save(&mut game);
        assert!(!already_emitted(&game, "stray"));
    }
}
