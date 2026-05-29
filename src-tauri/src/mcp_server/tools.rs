use std::sync::Arc;

use rmcp::handler::server::tool::ToolRoute;
use rmcp::model::{CallToolResult, Content, Tool};
use rmcp::ErrorData as McpError;

use crate::mcp_server::context::McpContext;
use crate::mcp_server::formatting::translate_error;
use crate::mcp_server::tools_impl;

/// Type alias for our tool router.
pub type OfmToolRouter = rmcp::handler::server::tool::ToolRouter<Arc<McpContext>>;

/// Default empty input schema for tools with no parameters.
fn empty_input_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    Arc::new(
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
        .as_object()
        .unwrap()
        .clone(),
    )
}

/// Helper to create a simple tool with no parameters.
fn simple_tool(name: &'static str, description: &'static str) -> Tool {
    Tool::new(name, description, empty_input_schema())
}

/// Helper to make a "not yet implemented" response.
fn not_implemented(name: &str) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(
        format!(
            "Tool `{}` is not yet implemented. Coming in a future phase.",
            name
        ),
    )]))
}

fn error_result(msg: &str) -> CallToolResult {
    let mut result = CallToolResult::success(vec![Content::text(msg.to_string())]);
    result.is_error = Some(true);
    result
}

/// Build the tool router, omitting any tools whose names appear in `disabled`.
pub fn build_tool_router(context: &Arc<McpContext>, disabled: &[String]) -> OfmToolRouter {
    let mut router = OfmToolRouter::new();

    // Phase 1: ping
    if !disabled.contains(&"ping".to_string()) {
        router.add_route(ToolRoute::new_dyn(
            simple_tool("ping", "Check if the MCP server is alive and responding"),
            |_context| {
                Box::pin(async {
                    Ok(CallToolResult::success(vec![Content::text(
                        "Pong! OpenFoot Manager MCP server is alive.".to_string(),
                    )]))
                })
            },
        ));
    }

    // Phase 2: Real implementations for key tools
    if !disabled.contains(&"game_is_finished".to_string()) {
        let ctx = context.clone();
        router.add_route(ToolRoute::new_dyn(
            simple_tool("game_is_finished", "Check if the season/game is complete"),
            move |_context| {
                let ctx = ctx.clone();
                Box::pin(async move {
                    match tools_impl::game_is_finished(ctx) {
                        Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                        Err(e) => Ok(error_result(&translate_error(&e))),
                    }
                })
            },
        ));
    }

    if !disabled.contains(&"info_game_summary".to_string()) {
        let ctx = context.clone();
        router.add_route(ToolRoute::new_dyn(
            simple_tool("info_game_summary", "High-level game overview: date, position, finances, next match"),
            move |_context| {
                let ctx = ctx.clone();
                Box::pin(async move {
                    match tools_impl::info_game_summary(ctx) {
                        Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                        Err(e) => Ok(error_result(&translate_error(&e))),
                    }
                })
            },
        ));
    }

    if !disabled.contains(&"info_standings".to_string()) {
        let ctx = context.clone();
        router.add_route(ToolRoute::new_dyn(
            simple_tool("info_standings", "League table as formatted text"),
            move |_context| {
                let ctx = ctx.clone();
                Box::pin(async move {
                    match tools_impl::info_standings(ctx) {
                        Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                        Err(e) => Ok(error_result(&translate_error(&e))),
                    }
                })
            },
        ));
    }

    if !disabled.contains(&"info_fixtures".to_string()) {
        let ctx = context.clone();
        router.add_route(ToolRoute::new_dyn(
            simple_tool("info_fixtures", "Upcoming/past fixtures"),
            move |_context| {
                let ctx = ctx.clone();
                Box::pin(async move {
                    match tools_impl::info_fixtures(ctx) {
                        Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                        Err(e) => Ok(error_result(&translate_error(&e))),
                    }
                })
            },
        ));
    }

    if !disabled.contains(&"time_advance".to_string()) {
        let ctx = context.clone();
        router.add_route(ToolRoute::new_dyn(
            Tool::new(
                "time_advance",
                "Advance one day (match forced to delegate mode). Includes round summary on match days",
                empty_input_schema(),
            ),
            move |_context| {
                let ctx = ctx.clone();
                Box::pin(async move {
                    match tools_impl::time_advance(ctx) {
                        Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                        Err(e) => Ok(error_result(&translate_error(&e))),
                    }
                })
            },
        ));
    }

    // Stub tools for phases 2-4 (remaining tools without real implementations).
    let stubs: &[(&str, &str)] = &[
        // Phase 2 — Game Lifecycle (remaining)
        ("game_new", "Create manager + generate/load world"),
        ("game_select_team", "Pick a team to manage"),
        ("game_load_save", "Load an existing save"),
        ("game_save", "Persist current game"),
        ("game_exit", "Save and return to menu"),
        ("game_export_world", "Export world to JSON file"),
        // Phase 3 — Core Gameplay Loop (remaining stubs)
        ("time_skip_to_match_day", "Fast-forward to next fixture"),
        ("time_check_blockers", "Check if anything blocks time advancement"),
        ("squad_get", "Squad overview with player IDs, stats, and formation"),
        ("squad_set_formation", "Change formation (also reassigns outfield positions by defending ability)"),
        ("squad_set_starting_xi", "Set starting eleven by player IDs"),
        ("squad_set_play_style", "Change play style"),
        ("training_get", "Current training settings + fitness overview"),
        ("training_set_focus_intensity", "Set team training focus and intensity"),
        // Phase 4 — Full Tool Surface
        ("squad_set_match_roles", "Set captain and set-piece takers by player ID"),
        ("squad_set_player_role", "Set player squad role (Senior/Youth)"),
        ("squad_auto_set_pieces", "Auto-assign best set-piece takers"),
        ("training_set_schedule", "Set weekly training schedule (Intense/Balanced/Light)"),
        ("training_set_groups", "Set training groups with per-group focus overrides"),
        ("training_set_player_focus", "Set individual player training focus"),
        ("transfer_market_browse", "Browse transfer/loan market with optional filters"),
        ("transfer_make_bid", "Make a transfer bid; includes negotiation feedback"),
        ("transfer_preview_bid", "Preview financial impact of a transfer bid"),
        ("transfer_respond_to_offer", "Accept/reject/withdraw an incoming offer"),
        ("transfer_counter_offer", "Counter an incoming transfer offer"),
        ("transfer_toggle_listed", "Toggle player transfer-listed status"),
        ("transfer_toggle_loan", "Toggle player loan-listed status"),
        ("transfer_free_agent_offer", "Offer contract to a free agent"),
        ("transfer_free_agent_preview", "Preview free agent contract financial impact"),
        ("contract_propose_renewal", "Propose contract renewal; includes negotiation feedback"),
        ("contract_delegate_renewals", "Delegate contract renewals to assistant"),
        ("contract_preview_renewal", "Preview renewal financial impact"),
        ("contract_set_exit_intent", "Mark contract to let expire"),
        ("contract_clear_exit_intent", "Remove exit intent from contract"),
        ("contract_preview_termination", "Preview cost of terminating contract"),
        ("contract_terminate", "Terminate contract immediately"),
        ("scout_send", "Send scout to report on a player"),
        ("scout_get_reports", "Get completed scout reports"),
        ("scout_youth_start", "Start youth scouting assignment"),
        ("scout_youth_cancel", "Cancel youth scouting"),
        ("scout_youth_reassign", "Reassign youth scouting parameters"),
        ("staff_get", "List all staff (your team + available)"),
        ("staff_hire", "Hire an unattached staff member"),
        ("staff_release", "Release a staff member"),
        ("inbox_get_messages", "Get messages (filterable by category, read status)"),
        ("inbox_mark_read", "Mark a message as read"),
        ("inbox_mark_all_read", "Mark all messages as read"),
        ("inbox_delete", "Delete message(s)"),
        ("inbox_clear_old", "Clear old messages"),
        ("inbox_resolve_action", "Resolve a message action (job offers, events, etc.)"),
        ("info_player_profile", "Detailed player card (attributes, stats, contract, morale)"),
        ("info_player_stats", "Season + career stats for a player"),
        ("info_player_match_history", "Match-by-match stats for a player"),
        ("info_team_profile", "Detailed team view (squad, form, finances)"),
        ("info_team_stats", "Season stats for a team"),
        ("info_team_match_history", "Match-by-match stats for a team"),
        ("info_finances", "Financial overview + ledger"),
        ("info_finance_snapshot", "Detailed financial snapshot"),
        ("info_news", "Recent news articles"),
        ("info_season_context", "Season phase and transfer window status"),
        ("info_match_preview", "Preview of next match (opponent, form, squad overview)"),
        ("club_upgrade_facility", "Upgrade a facility level"),
        ("club_request_board_support", "Request board financial support"),
        ("club_request_marketing", "Request marketing campaign"),
        ("club_request_sponsor_pitch", "Request sponsor pitch"),
        ("season_check_complete", "Check if season is finished"),
        ("season_advance", "Advance to next season (may be fired if objectives not met)"),
        ("season_get_awards", "Get end-of-season awards"),
        ("jobs_available", "List available job openings"),
        ("jobs_apply", "Apply for a job"),
        ("help_find_tool", "Search tools by keyword or description"),
        ("help_list_categories", "List all tool categories with counts"),
    ];

    for (name, desc) in stubs {
        if disabled.contains(&name.to_string()) {
            continue;
        }
        let name_owned = name.to_string();
        router.add_route(ToolRoute::new_dyn(
            simple_tool(name, desc),
            move |_context| {
                let n = name_owned.clone();
                Box::pin(async move { not_implemented(&n) })
            },
        ));
    }

    router
}
