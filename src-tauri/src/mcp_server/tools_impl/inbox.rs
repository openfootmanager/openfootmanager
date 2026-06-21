//! MCP tool implementations: inbox

use std::sync::Arc;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game};
use crate::mcp_server::formatting::translate_error;

// ─── inbox_get_messages ─────────────────────────────────────────────────────

pub fn inbox_get_messages(ctx: Arc<McpContext>, category: Option<String>, unread_only: Option<bool>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    let messages: Vec<_> = game.messages.iter()
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

    if messages.is_empty() {
        return Ok("## Inbox\n\nNo messages.".to_string());
    }

    let mut output = format!("## Inbox ({} messages)\n\n| ID | Subject | Category | Read | Date |\n|----|---------|----------|------|------|\n", messages.len());
    for m in messages.iter().take(20) {
        let read_marker = if m.read { "✓" } else { "●" };
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            m.id,
            m.subject,
            format!("{:?}", m.category),
            read_marker,
            m.date,
        ));
    }
    if messages.len() > 20 {
        output.push_str(&format!("\n... and {} more.", messages.len() - 20));
    }

    Ok(output)
}

// ─── inbox_mark_read ────────────────────────────────────────────────────────

// ─── inbox_mark_read ────────────────────────────────────────────────────────

pub fn inbox_mark_read(ctx: Arc<McpContext>, message_id: String) -> Result<String, String> {
    crate::commands::messages::mark_message_read_internal(&ctx.state_manager, &message_id)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("Message marked as read.".to_string())
}

// ─── inbox_mark_all_read ────────────────────────────────────────────────────

// ─── inbox_mark_all_read ────────────────────────────────────────────────────

pub fn inbox_mark_all_read(ctx: Arc<McpContext>) -> Result<String, String> {
    crate::commands::messages::mark_all_messages_read_internal(&ctx.state_manager)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("All messages marked as read.".to_string())
}

// ─── inbox_delete ───────────────────────────────────────────────────────────

// ─── inbox_delete ───────────────────────────────────────────────────────────

pub fn inbox_delete(ctx: Arc<McpContext>, message_id: String) -> Result<String, String> {
    crate::commands::messages::delete_message_internal(&ctx.state_manager, &message_id)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("Message deleted.".to_string())
}

// ─── inbox_clear_old ────────────────────────────────────────────────────────

// ─── inbox_clear_old ────────────────────────────────────────────────────────

pub fn inbox_clear_old(ctx: Arc<McpContext>) -> Result<String, String> {
    crate::commands::messages::clear_old_messages_internal(&ctx.state_manager)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("Old messages cleared.".to_string())
}

// ─── inbox_resolve_action ───────────────────────────────────────────────────

// ─── inbox_resolve_action ───────────────────────────────────────────────────

pub fn inbox_resolve_action(ctx: Arc<McpContext>, message_id: String, action_id: String, option_id: Option<String>) -> Result<String, String> {
    crate::commands::messages::resolve_message_action_internal(
        &ctx.state_manager,
        &message_id,
        &action_id,
        option_id.as_deref(),
    )
    .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Action Resolved\n\nMessage {} — action {} completed.", message_id, action_id))
}

// ─── info_player_profile ────────────────────────────────────────────────────
