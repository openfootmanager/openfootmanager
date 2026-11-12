//! MCP tool implementations: inbox

use mcp_results::inbox::{
    ActionResolved, AllMessagesMarkedRead, ArchivedMessage, ArchivedMessages, InboxAction,
    InboxActionOption, InboxMessage, InboxMessages, MessageDeleted, MessageMarkedRead,
    OldMessagesCleared,
};

use crate::mcp_server::backend_text;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::require_game;
use ofm_core::game::Game;
use ofm_core::inbox_archive::{ArchiveCursor, ArchivedMessagesPage};
use ofm_core::state::Career;
use std::sync::Arc;

/// Archive messages per page when the agent does not say.
const DEFAULT_ARCHIVE_PAGE: usize = 50;

// ─── inbox_get_messages ─────────────────────────────────────────────────────

pub fn inbox_get_messages(
    ctx: Arc<McpContext>,
    category: Option<String>,
    unread_only: Option<bool>,
) -> Result<InboxMessages, String> {
    let game = require_game(&ctx.state_manager)?;
    Ok(inbox_messages_result(
        &game,
        category.as_deref(),
        unread_only,
    ))
}

/// The inbox as the player sees it, in words: the subject, body and sender are the English text
/// of the keys the backend wrote, and every action carries the ids `inbox_resolve_action` takes.
fn inbox_messages_result(
    game: &Game,
    category: Option<&str>,
    unread_only: Option<bool>,
) -> InboxMessages {
    // Agents see the same inbox the player does, which means mail dated ahead of
    // the clock is hidden from them too — an agent must not be able to act on an
    // event the player cannot see yet.
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    InboxMessages {
        messages: game
            .messages
            .iter()
            .filter(|m| ofm_core::slices::inbox::message_is_visible(&m.date, &today))
            .filter(|m| category.is_none_or(|cat| format!("{:?}", m.category) == cat))
            .filter(|m| !(unread_only == Some(true) && m.read))
            .map(inbox_message_result)
            .collect(),
    }
}

fn inbox_message_result(message: &domain::message::InboxMessage) -> InboxMessage {
    let text = backend_text::resolve_message(message);
    InboxMessage {
        id: message.id.clone(),
        subject: text.subject,
        body: text.body,
        sender: text.sender,
        sender_role: text.sender_role,
        // Spelled as the `category` filter accepts it.
        category: format!("{:?}", message.category),
        read: message.read,
        date: message.date.clone(),
        actions: text
            .actions
            .into_iter()
            .map(|action| InboxAction {
                id: action.id,
                label: action.label,
                kind: action.kind.to_string(),
                resolved: action.resolved,
                options: action
                    .options
                    .into_iter()
                    .map(|option| InboxActionOption {
                        id: option.id,
                        label: option.label,
                        description: option.description,
                    })
                    .collect(),
            })
            .collect(),
        details: text.details,
    }
}

// ─── inbox_get_archived ─────────────────────────────────────────────────────

/// Read mail older than half a year, which `inbox_get_messages` no longer lists. Newest first.
pub fn inbox_get_archived(
    ctx: Arc<McpContext>,
    before_date: Option<String>,
    before_id: Option<String>,
    limit: Option<u64>,
) -> Result<ArchivedMessages, String> {
    let before = ArchiveCursor::from_parts(before_date, before_id)?;
    let limit = limit.map_or(DEFAULT_ARCHIVE_PAGE, |n| n as usize);
    let page = crate::commands::messages::get_archived_messages_internal(
        &ctx.state_manager,
        before,
        limit,
    )?;
    Ok(archived_messages_result(page))
}

/// The page as an agent reads it: the cursor split into the two values it passes back.
fn archived_messages_result(page: ArchivedMessagesPage) -> ArchivedMessages {
    ArchivedMessages {
        messages: page
            .items
            .into_iter()
            .map(|m| ArchivedMessage {
                id: m.id,
                subject: m.subject,
                body: m.body,
                category: format!("{:?}", m.category),
                date: m.date,
            })
            .collect(),
        next_before_date: page.next_before.as_ref().map(|c| c.date.clone()),
        next_before_id: page.next_before.map(|c| c.id),
    }
}

// ─── inbox_mark_read ────────────────────────────────────────────────────────

// ─── inbox_mark_read ────────────────────────────────────────────────────────

pub fn inbox_mark_read(
    ctx: Arc<McpContext>,
    message_id: String,
) -> Result<MessageMarkedRead, String> {
    crate::commands::messages::mark_message_read_internal(
        &ctx.state_manager,
        Career::Active,
        &message_id,
    )?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(MessageMarkedRead { message_id })
}

// ─── inbox_mark_all_read ────────────────────────────────────────────────────

// ─── inbox_mark_all_read ────────────────────────────────────────────────────

pub fn inbox_mark_all_read(ctx: Arc<McpContext>) -> Result<AllMessagesMarkedRead, String> {
    crate::commands::messages::mark_all_messages_read_internal(&ctx.state_manager, Career::Active)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(AllMessagesMarkedRead {})
}

// ─── inbox_delete ───────────────────────────────────────────────────────────

// ─── inbox_delete ───────────────────────────────────────────────────────────

pub fn inbox_delete(ctx: Arc<McpContext>, message_id: String) -> Result<MessageDeleted, String> {
    crate::commands::messages::delete_message_internal(
        &ctx.state_manager,
        Career::Active,
        &message_id,
    )?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(MessageDeleted { message_id })
}

// ─── inbox_clear_old ────────────────────────────────────────────────────────

// ─── inbox_clear_old ────────────────────────────────────────────────────────

pub fn inbox_clear_old(ctx: Arc<McpContext>) -> Result<OldMessagesCleared, String> {
    crate::commands::messages::clear_old_messages_internal(&ctx.state_manager, Career::Active)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(OldMessagesCleared {})
}

// ─── inbox_resolve_action ───────────────────────────────────────────────────

// ─── inbox_resolve_action ───────────────────────────────────────────────────

pub fn inbox_resolve_action(
    ctx: Arc<McpContext>,
    message_id: String,
    action_id: String,
    option_id: Option<String>,
) -> Result<ActionResolved, String> {
    crate::commands::messages::resolve_message_action_internal(
        &ctx.state_manager,
        Career::Active,
        &message_id,
        &action_id,
        option_id.as_deref(),
    )?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(ActionResolved {
        message_id,
        action_id,
    })
}

// ─── info_player_profile ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::message::{ActionOption, ActionType, InboxMessage, MessageAction};
    use ofm_core::clock::GameClock;
    use std::collections::HashMap;

    fn game_with(messages: Vec<InboxMessage>) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        game.messages = messages;
        game
    }

    /// A message the way most of the backend writes one: no plain text, keys and params.
    fn keyed(id: &str, subject_key: &str, body_key: &str, params: &[(&str, &str)]) -> InboxMessage {
        InboxMessage::new(
            id.to_string(),
            String::new(),
            String::new(),
            String::new(),
            "2026-07-30".to_string(),
        )
        .with_i18n(
            subject_key,
            body_key,
            params
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<HashMap<_, _>>(),
        )
    }

    /// Given the messages the game writes with keys and params (the board's season objectives,
    ///       an incoming offer, a completed transfer, the assistant's advice)
    /// When an agent lists the inbox
    /// Then every subject and body is English text: not blank, not the key, no placeholder left
    ///      (#1195: agents saw blank subjects for nearly every message)
    #[test]
    fn an_agent_reads_words_not_blanks_or_keys_for_the_messages_the_game_writes() {
        let game = game_with(vec![
            ofm_core::messages::board_expectations_message("Northfield", "t1", "2026-07-30"),
            ofm_core::messages::incoming_transfer_offer_message(
                "o1",
                "p1",
                "Jo Doe",
                "Southgate",
                1_250_000,
                "2026-07-30",
            ),
            ofm_core::messages::transfer_complete_message("Jo Doe", 800_001, "2026-07-30"),
            ofm_core::messages::staff_advice_message("Northfield", "t1", "2026-07-30"),
        ]);

        let listed = inbox_messages_result(&game, None, None).messages;

        assert_eq!(listed.len(), 4);
        for message in &listed {
            for (field, text) in [("subject", &message.subject), ("body", &message.body)] {
                assert!(!text.trim().is_empty(), "{}: blank {field}", message.id);
                assert!(
                    !text.contains("{{"),
                    "{}: {field} has a placeholder: {text}",
                    message.id
                );
                assert!(
                    !text.starts_with("be."),
                    "{}: {field} is a raw key: {text}",
                    message.id
                );
            }
        }
        assert_eq!(listed[0].subject, "Northfield — Season Objectives");
        assert_eq!(listed[1].subject, "Incoming Offer for Jo Doe");
        assert_eq!(
            listed[1].body,
            "Southgate have submitted an offer of €1.25M for Jo Doe. Review the bid in the Transfers tab to accept or reject it."
        );
        assert_eq!(listed[0].sender, "Board of Directors");
    }

    /// Given a message that asks for a choice, and one that asks to be acknowledged
    /// When an agent lists the inbox
    /// Then each action is listed with the id `inbox_resolve_action` takes, its label in words,
    ///      its kind and whether it is already answered, and a choice lists its options' ids with
    ///      their labels and descriptions
    #[test]
    fn an_agent_is_shown_the_action_and_option_ids_it_can_resolve() {
        let mut choice = keyed(
            "sponsor_1",
            "be.msg.sponsor.subject",
            "be.msg.sponsor.body",
            &[
                ("sponsor", "Acme"),
                ("team", "Northfield"),
                ("amount", "900000"),
            ],
        );
        choice.actions.push(MessageAction {
            id: "respond".to_string(),
            label: String::new(),
            action_type: ActionType::ChooseOption {
                options: vec![ActionOption {
                    id: "accept".to_string(),
                    label: String::new(),
                    description: String::new(),
                    label_key: Some("be.msg.sponsor.options.accept.label".to_string()),
                    description_key: Some("be.msg.sponsor.options.accept.description".to_string()),
                }],
            },
            resolved: false,
            label_key: Some("be.msg.event.respond".to_string()),
        });
        let game = game_with(vec![
            choice,
            ofm_core::messages::board_expectations_message("Northfield", "t1", "2026-07-30"),
        ]);

        let listed = inbox_messages_result(&game, None, None).messages;

        let sponsor = &listed[0].actions[0];
        assert_eq!(
            (sponsor.id.as_str(), sponsor.kind.as_str()),
            ("respond", "choose_option")
        );
        assert!(
            !sponsor.label.is_empty(),
            "the action label is words: {sponsor:?}"
        );
        assert!(!sponsor.resolved);
        let accept = &sponsor.options[0];
        assert_eq!(accept.id, "accept");
        assert!(
            !accept.label.is_empty() && !accept.label.starts_with("be."),
            "{accept:?}"
        );
        assert!(!accept.description.is_empty(), "{accept:?}");
        let board = &listed[1].actions[0];
        assert_eq!(
            (board.id.as_str(), board.kind.as_str(), board.label.as_str()),
            ("ack_objectives", "acknowledge", "Accept Objectives")
        );
    }

    /// Given a message dated after the game's clock and a read one
    /// When an agent lists the inbox, then lists only the unread, then filters by category
    /// Then the future mail is never shown and the filters still apply: reading the text did not
    ///      change which messages are listed
    #[test]
    fn the_listing_still_hides_future_mail_and_honours_its_filters() {
        let mut future = keyed(
            "later",
            "be.msg.sponsor.subject",
            "be.msg.sponsor.body",
            &[],
        );
        future.date = "2026-09-01".to_string();
        let mut read = ofm_core::messages::staff_advice_message("Northfield", "t1", "2026-07-30");
        read.read = true;
        let game = game_with(vec![
            future,
            read,
            ofm_core::messages::board_expectations_message("Northfield", "t1", "2026-07-30"),
        ]);

        let ids = |category: Option<&str>, unread_only: Option<bool>| -> Vec<String> {
            inbox_messages_result(&game, category, unread_only)
                .messages
                .into_iter()
                .map(|m| m.id)
                .collect()
        };
        let all = ids(None, None);
        let unread = ids(None, Some(true));
        let directives = ids(Some("BoardDirective"), None);

        assert_eq!(all, ["staff_advice_1", "board_expect_1"]);
        assert_eq!(unread, ["board_expect_1"]);
        assert_eq!(directives, ["board_expect_1"]);
    }

    /// Given a page of archived mail with more behind it
    /// When it is turned into the agent's result
    /// Then the messages keep their text and category, and the cursor is split for the next call.
    #[test]
    fn an_archive_page_becomes_a_result_with_the_cursor_split() {
        let page = ArchivedMessagesPage {
            items: vec![InboxMessage::new(
                "m1".into(),
                "Old news".into(),
                "Body".into(),
                "Board".into(),
                "2030-01-01".into(),
            )],
            next_before: Some(ArchiveCursor {
                date: "2030-01-01".into(),
                id: "m1".into(),
            }),
        };

        let result = archived_messages_result(page);

        assert_eq!(result.messages[0].subject, "Old news");
        assert_eq!(result.messages[0].body, "Body");
        assert_eq!(result.messages[0].category, "System");
        assert_eq!(result.next_before_date.as_deref(), Some("2030-01-01"));
        assert_eq!(result.next_before_id.as_deref(), Some("m1"));
    }

    /// Given archived mail written with keys (its plain subject and body empty, as the backend
    ///       writes most mail)
    /// When it is turned into the agent's result
    /// Then the subject and the body are the English the live inbox reads, not blank (#1388)
    #[test]
    fn archived_mail_written_with_keys_reads_in_words() {
        let page = ArchivedMessagesPage {
            items: vec![ofm_core::messages::board_expectations_message(
                "Northfield",
                "t1",
                "2026-07-30",
            )],
            next_before: None,
        };

        let result = archived_messages_result(page);

        assert!(
            result.messages[0].subject.contains("Season Objectives"),
            "{:?}",
            result.messages[0].subject
        );
        assert!(
            result.messages[0]
                .body
                .contains("The board has set the following expectations"),
            "{:?}",
            result.messages[0].body
        );
    }

    /// Given an archived message and the same message in the live inbox
    /// When each is turned into the agent's result
    /// Then the two read the same: one resolver, not two
    #[test]
    fn an_archived_message_reads_as_it_did_in_the_live_inbox() {
        let message =
            ofm_core::messages::board_expectations_message("Northfield", "t1", "2026-07-30");
        let live = inbox_message_result(&message);
        let archived = archived_messages_result(ArchivedMessagesPage {
            items: vec![message],
            next_before: None,
        });

        assert_eq!(archived.messages[0].subject, live.subject);
        assert_eq!(archived.messages[0].body, live.body);
    }
}
