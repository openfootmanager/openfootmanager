//! MCP tool implementations: inbox

use mcp_results::inbox::{
    ActionResolved, AllMessagesMarkedRead, InboxMessage, InboxMessages, MessageDeleted,
    MessageMarkedRead, OldMessagesCleared,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::require_game;
use std::sync::Arc;

// ─── inbox_get_messages ─────────────────────────────────────────────────────

pub fn inbox_get_messages(
    ctx: Arc<McpContext>,
    category: Option<String>,
    unread_only: Option<bool>,
) -> Result<InboxMessages, String> {
    let game = require_game(&ctx.state_manager)?;

    // Agents see the same inbox the player does, which means mail dated ahead of
    // the clock is hidden from them too — an agent must not be able to act on an
    // event the player cannot see yet.
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let messages: Vec<_> = game
        .messages
        .iter()
        .filter(|m| ofm_core::slices::inbox::message_is_visible(&m.date, &today))
        .filter(|m| {
            if let Some(ref cat) = category {
                format!("{:?}", m.category) == *cat
            } else {
                true
            }
        })
        .filter(|m| {
            if let Some(true) = unread_only {
                !m.read
            } else {
                true
            }
        })
        .collect();

    Ok(InboxMessages {
        messages: messages
            .into_iter()
            .map(|m| InboxMessage {
                id: m.id.clone(),
                subject: m.subject.clone(),
                // Spelled as the `category` filter accepts it.
                category: format!("{:?}", m.category),
                read: m.read,
                date: m.date.clone(),
            })
            .collect(),
    })
}

// ─── inbox_mark_read ────────────────────────────────────────────────────────

// ─── inbox_mark_read ────────────────────────────────────────────────────────

pub fn inbox_mark_read(
    ctx: Arc<McpContext>,
    message_id: String,
) -> Result<MessageMarkedRead, String> {
    crate::commands::messages::mark_message_read_internal(&ctx.state_manager, &message_id)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(MessageMarkedRead { message_id })
}

// ─── inbox_mark_all_read ────────────────────────────────────────────────────

// ─── inbox_mark_all_read ────────────────────────────────────────────────────

pub fn inbox_mark_all_read(ctx: Arc<McpContext>) -> Result<AllMessagesMarkedRead, String> {
    crate::commands::messages::mark_all_messages_read_internal(&ctx.state_manager)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(AllMessagesMarkedRead {})
}

// ─── inbox_delete ───────────────────────────────────────────────────────────

// ─── inbox_delete ───────────────────────────────────────────────────────────

pub fn inbox_delete(ctx: Arc<McpContext>, message_id: String) -> Result<MessageDeleted, String> {
    crate::commands::messages::delete_message_internal(&ctx.state_manager, &message_id)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(MessageDeleted { message_id })
}

// ─── inbox_clear_old ────────────────────────────────────────────────────────

// ─── inbox_clear_old ────────────────────────────────────────────────────────

pub fn inbox_clear_old(ctx: Arc<McpContext>) -> Result<OldMessagesCleared, String> {
    crate::commands::messages::clear_old_messages_internal(&ctx.state_manager)?;

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
