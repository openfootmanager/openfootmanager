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

/// Schema for tools taking a single `player_id` parameter.
fn player_id_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    param_schema("player_id", "Player entity ID")
}

/// Schema for tools taking a single `message_id` parameter.
fn message_id_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    param_schema("message_id", "Message entity ID")
}

fn param_schema(param_name: &str, description: &str) -> Arc<serde_json::Map<String, serde_json::Value>> {
    Arc::new(
        serde_json::json!({
            "type": "object",
            "properties": {
                param_name: { "type": "string", "description": description }
            },
            "required": [param_name]
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

    // ─── Phase 1: ping ──────────────────────────────────────────────────────

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

    // ─── Phase 2: Information tools ─────────────────────────────────────────

    macro_rules! real_tool {
        ($name:expr, $desc:expr, $fn:path) => {
            if !disabled.contains(&$name.to_string()) {
                let ctx = context.clone();
                router.add_route(ToolRoute::new_dyn(
                    simple_tool($name, $desc),
                    move |_context| {
                        let ctx = ctx.clone();
                        Box::pin(async move {
                            match $fn(ctx) {
                                Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                                Err(e) => Ok(error_result(&translate_error(&e))),
                            }
                        })
                    },
                ));
            }
        };
    }

    macro_rules! real_tool_schema {
        ($name:expr, $desc:expr, $schema:expr, $fn:path) => {
            if !disabled.contains(&$name.to_string()) {
                let ctx = context.clone();
                router.add_route(ToolRoute::new_dyn(
                    Tool::new($name, $desc, $schema),
                    move |tool_context| {
                        let ctx = ctx.clone();
                        Box::pin(async move {
                            match $fn(ctx, tool_context) {
                                Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                                Err(e) => Ok(error_result(&translate_error(&e))),
                            }
                        })
                    },
                ));
            }
        };
    }

    real_tool!("game_is_finished", "Check if the season/game is complete", tools_impl::game_is_finished);
    real_tool!("info_game_summary", "High-level game overview: date, position, finances, next match", tools_impl::info_game_summary);
    real_tool!("info_standings", "League table as formatted text", tools_impl::info_standings);
    real_tool!("info_fixtures", "Upcoming/past fixtures", tools_impl::info_fixtures);
    real_tool!("info_finances", "Financial overview + ledger", tools_impl::info_finances);
    real_tool!("info_news", "Recent news articles", tools_impl::info_news);
    real_tool!("info_season_context", "Season phase and transfer window status", tools_impl::info_season_context);
    real_tool!("info_match_preview", "Preview of next match (opponent, form, squad overview)", tools_impl::info_match_preview);
    real_tool!("training_get", "Current training settings + fitness overview", tools_impl::training_get);
    real_tool!("squad_get", "Squad overview with player IDs, stats, and formation", tools_impl::squad_get);
    real_tool!("inbox_mark_all_read", "Mark all messages as read", tools_impl::inbox_mark_all_read);
    real_tool!("inbox_clear_old", "Clear old messages", tools_impl::inbox_clear_old);
    real_tool!("staff_get", "List all staff (your team + available)", tools_impl::staff_get);
    real_tool!("season_check_complete", "Check if season is finished", tools_impl::season_check_complete);
    real_tool!("squad_auto_set_pieces", "Auto-assign best set-piece takers", tools_impl::squad_auto_set_pieces);
    real_tool!("season_advance", "Advance to next season (may be fired if objectives not met)", tools_impl::season_advance);
    real_tool!("game_save", "Persist current game", tools_impl::game_save);
    real_tool!("time_advance", "Advance one day (match forced to delegate mode). Includes round summary on match days", tools_impl::time_advance);

    // Tools that take player_id parameter
    {
        let ctx = context.clone();
        if !disabled.contains(&"info_player_profile".to_string()) {
            router.add_route(ToolRoute::new_dyn(
                Tool::new("info_player_profile", "Detailed player card (attributes, stats, contract, morale)", player_id_schema()),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        match tools_impl::info_player_profile(ctx, pid) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    {
        let ctx = context.clone();
        if !disabled.contains(&"inbox_mark_read".to_string()) {
            router.add_route(ToolRoute::new_dyn(
                Tool::new("inbox_mark_read", "Mark a message as read", message_id_schema()),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let msg_id = extract_string_param(&tool_context.arguments, "message_id").unwrap_or_default();
                        match tools_impl::inbox_mark_read(ctx, msg_id) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    {
        let ctx = context.clone();
        if !disabled.contains(&"inbox_delete".to_string()) {
            router.add_route(ToolRoute::new_dyn(
                Tool::new("inbox_delete", "Delete a message", message_id_schema()),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let msg_id = extract_string_param(&tool_context.arguments, "message_id").unwrap_or_default();
                        match tools_impl::inbox_delete(ctx, msg_id) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // ─── Phase 3: Core Gameplay Loop ────────────────────────────────────────

    // squad_set_formation
    {
        let ctx = context.clone();
        if !disabled.contains(&"squad_set_formation".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "formation": { "type": "string", "description": "Formation string (e.g. 4-4-2, 4-3-3, 3-5-2)" }
                },
                "required": ["formation"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("squad_set_formation", "Change formation (also reassigns outfield positions by defending ability)", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let formation = extract_string_param(&tool_context.arguments, "formation").unwrap_or_default();
                        match tools_impl::squad_set_formation(ctx, formation) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // squad_set_starting_xi
    {
        let ctx = context.clone();
        if !disabled.contains(&"squad_set_starting_xi".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "player_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Ordered list of 11 player IDs for the starting eleven"
                    }
                },
                "required": ["player_ids"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("squad_set_starting_xi", "Set starting eleven by player IDs", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pids = extract_string_array_param(&tool_context.arguments, "player_ids").unwrap_or_default();
                        match tools_impl::squad_set_starting_xi(ctx, pids) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // squad_set_play_style
    {
        let ctx = context.clone();
        if !disabled.contains(&"squad_set_play_style".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "play_style": {
                        "type": "string",
                        "enum": ["Attacking", "Defensive", "Possession", "Counter", "HighPress", "Balanced"],
                        "description": "Play style"
                    }
                },
                "required": ["play_style"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("squad_set_play_style", "Change play style", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let style = extract_string_param(&tool_context.arguments, "play_style").unwrap_or_default();
                        match tools_impl::squad_set_play_style(ctx, style) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // squad_set_match_roles
    {
        let ctx = context.clone();
        if !disabled.contains(&"squad_set_match_roles".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "captain": { "type": "string", "description": "Player ID for captain" },
                    "vice_captain": { "type": "string", "description": "Player ID for vice captain" },
                    "penalty_taker": { "type": "string", "description": "Player ID for penalty taker" },
                    "free_kick_taker": { "type": "string", "description": "Player ID for free kick taker" },
                    "corner_taker": { "type": "string", "description": "Player ID for corner taker" }
                }
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("squad_set_match_roles", "Set captain and set-piece takers by player ID", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let args = &tool_context.arguments;
                        match tools_impl::squad_set_match_roles(
                            ctx,
                            extract_string_param(args, "captain"),
                            extract_string_param(args, "vice_captain"),
                            extract_string_param(args, "penalty_taker"),
                            extract_string_param(args, "free_kick_taker"),
                            extract_string_param(args, "corner_taker"),
                        ) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    real_tool!("squad_auto_set_pieces", "Auto-assign best set-piece takers", tools_impl::squad_auto_set_pieces);

    // squad_set_player_role
    {
        let ctx = context.clone();
        if !disabled.contains(&"squad_set_player_role".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "player_id": { "type": "string", "description": "Player entity ID" },
                    "squad_role": { "type": "string", "enum": ["Senior", "Youth"], "description": "Squad role" }
                },
                "required": ["player_id", "squad_role"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("squad_set_player_role", "Set player squad role (Senior/Youth)", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        let role = extract_string_param(&tool_context.arguments, "squad_role").unwrap_or_default();
                        match tools_impl::squad_set_player_role(ctx, pid, role) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // training_set_focus_intensity
    {
        let ctx = context.clone();
        if !disabled.contains(&"training_set_focus_intensity".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "focus": { "type": "string", "enum": ["Physical", "Technical", "Tactical", "Defending", "Attacking", "Recovery"], "description": "Training focus" },
                    "intensity": { "type": "string", "enum": ["Low", "Medium", "High"], "description": "Training intensity" }
                },
                "required": ["focus", "intensity"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("training_set_focus_intensity", "Set team training focus and intensity", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let focus = extract_string_param(&tool_context.arguments, "focus").unwrap_or_default();
                        let intensity = extract_string_param(&tool_context.arguments, "intensity").unwrap_or_default();
                        match tools_impl::training_set_focus_intensity(ctx, focus, intensity) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // training_set_schedule
    {
        let ctx = context.clone();
        if !disabled.contains(&"training_set_schedule".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "schedule": { "type": "string", "enum": ["Intense", "Balanced", "Light"], "description": "Weekly training schedule" }
                },
                "required": ["schedule"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("training_set_schedule", "Set weekly training schedule (Intense/Balanced/Light)", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let schedule = extract_string_param(&tool_context.arguments, "schedule").unwrap_or_default();
                        match tools_impl::training_set_schedule(ctx, schedule) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // training_set_groups
    {
        let ctx = context.clone();
        if !disabled.contains(&"training_set_groups".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "groups_json": { "type": "string", "description": "JSON array of TrainingGroup objects" }
                },
                "required": ["groups_json"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("training_set_groups", "Set training groups with per-group focus overrides", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let groups_json = extract_string_param(&tool_context.arguments, "groups_json").unwrap_or_default();
                        match tools_impl::training_set_groups(ctx, groups_json) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // training_set_player_focus
    {
        let ctx = context.clone();
        if !disabled.contains(&"training_set_player_focus".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "player_id": { "type": "string", "description": "Player entity ID" },
                    "focus": { "type": "string", "description": "Individual training focus (omit to clear)" }
                },
                "required": ["player_id"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("training_set_player_focus", "Set individual player training focus", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        let focus = extract_string_param(&tool_context.arguments, "focus");
                        match tools_impl::training_set_player_focus(ctx, pid, focus) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // ─── Phase 4: Transfers, Contracts, etc. ────────────────────────────────

    // transfer_toggle_listed
    {
        let ctx = context.clone();
        if !disabled.contains(&"transfer_toggle_listed".to_string()) {
            router.add_route(ToolRoute::new_dyn(
                Tool::new("transfer_toggle_listed", "Toggle player transfer-listed status", player_id_schema()),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        match tools_impl::transfer_toggle_listed(ctx, pid) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // transfer_toggle_loan
    {
        let ctx = context.clone();
        if !disabled.contains(&"transfer_toggle_loan".to_string()) {
            router.add_route(ToolRoute::new_dyn(
                Tool::new("transfer_toggle_loan", "Toggle player loan-listed status", player_id_schema()),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        match tools_impl::transfer_toggle_loan(ctx, pid) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // transfer_make_bid
    {
        let ctx = context.clone();
        if !disabled.contains(&"transfer_make_bid".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "player_id": { "type": "string", "description": "Player entity ID to bid on" },
                    "fee": { "type": "integer", "description": "Transfer fee amount" }
                },
                "required": ["player_id", "fee"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("transfer_make_bid", "Make a transfer bid; includes negotiation feedback", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        let fee = extract_u64_param(&tool_context.arguments, "fee").unwrap_or(0);
                        match tools_impl::transfer_make_bid(ctx, pid, fee) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // transfer_preview_bid
    {
        let ctx = context.clone();
        if !disabled.contains(&"transfer_preview_bid".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "player_id": { "type": "string", "description": "Player entity ID" },
                    "fee": { "type": "integer", "description": "Proposed transfer fee" }
                },
                "required": ["player_id", "fee"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("transfer_preview_bid", "Preview financial impact of a transfer bid", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        let fee = extract_u64_param(&tool_context.arguments, "fee").unwrap_or(0);
                        match tools_impl::transfer_preview_bid(ctx, pid, fee) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // transfer_respond_to_offer
    {
        let ctx = context.clone();
        if !disabled.contains(&"transfer_respond_to_offer".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "player_id": { "type": "string", "description": "Player entity ID" },
                    "offer_id": { "type": "string", "description": "Offer ID to respond to" },
                    "accept": { "type": "boolean", "description": "True to accept, false to reject" }
                },
                "required": ["player_id", "offer_id", "accept"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("transfer_respond_to_offer", "Accept/reject an incoming offer", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        let oid = extract_string_param(&tool_context.arguments, "offer_id").unwrap_or_default();
                        let accept = extract_bool_param(&tool_context.arguments, "accept").unwrap_or(false);
                        match tools_impl::transfer_respond_to_offer(ctx, pid, oid, accept) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // transfer_counter_offer
    {
        let ctx = context.clone();
        if !disabled.contains(&"transfer_counter_offer".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "player_id": { "type": "string", "description": "Player entity ID" },
                    "offer_id": { "type": "string", "description": "Offer ID to counter" },
                    "requested_fee": { "type": "integer", "description": "Counter-offer fee amount" }
                },
                "required": ["player_id", "offer_id", "requested_fee"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("transfer_counter_offer", "Counter an incoming transfer offer", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        let oid = extract_string_param(&tool_context.arguments, "offer_id").unwrap_or_default();
                        let fee = extract_u64_param(&tool_context.arguments, "requested_fee").unwrap_or(0);
                        match tools_impl::transfer_counter_offer(ctx, pid, oid, fee) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // contract_propose_renewal
    {
        let ctx = context.clone();
        if !disabled.contains(&"contract_propose_renewal".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "player_id": { "type": "string", "description": "Player entity ID" },
                    "weekly_wage": { "type": "integer", "description": "Proposed weekly wage" },
                    "contract_years": { "type": "integer", "description": "Proposed contract length in years" }
                },
                "required": ["player_id", "weekly_wage", "contract_years"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("contract_propose_renewal", "Propose contract renewal; includes negotiation feedback", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        let wage = extract_u32_param(&tool_context.arguments, "weekly_wage").unwrap_or(0);
                        let years = extract_u32_param(&tool_context.arguments, "contract_years").unwrap_or(1);
                        match tools_impl::contract_propose_renewal(ctx, pid, wage, years) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // contract_delegate_renewals
    {
        let ctx = context.clone();
        if !disabled.contains(&"contract_delegate_renewals".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "player_ids": { "type": "array", "items": { "type": "string" }, "description": "Player IDs to renew (omit for all expiring)" },
                    "max_wage_increase_pct": { "type": "integer", "description": "Max wage increase percentage" },
                    "max_contract_years": { "type": "integer", "description": "Max contract years" }
                },
                "required": ["max_wage_increase_pct", "max_contract_years"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("contract_delegate_renewals", "Delegate contract renewals to assistant", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pids = extract_string_array_param(&tool_context.arguments, "player_ids");
                        let max_pct = extract_u32_param(&tool_context.arguments, "max_wage_increase_pct").unwrap_or(20);
                        let max_years = extract_u32_param(&tool_context.arguments, "max_contract_years").unwrap_or(3);
                        match tools_impl::contract_delegate_renewals(ctx, pids, max_pct, max_years) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // contract_preview_renewal
    {
        let ctx = context.clone();
        if !disabled.contains(&"contract_preview_renewal".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "player_id": { "type": "string", "description": "Player entity ID" },
                    "weekly_wage": { "type": "integer", "description": "Proposed weekly wage" }
                },
                "required": ["player_id", "weekly_wage"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("contract_preview_renewal", "Preview renewal financial impact", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        let wage = extract_u32_param(&tool_context.arguments, "weekly_wage").unwrap_or(0);
                        match tools_impl::contract_preview_renewal(ctx, pid, wage) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // contract_set_exit_intent
    {
        let ctx = context.clone();
        if !disabled.contains(&"contract_set_exit_intent".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "player_id": { "type": "string", "description": "Player entity ID" },
                    "reason": { "type": "string", "description": "Optional reason for exit intent" }
                },
                "required": ["player_id"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("contract_set_exit_intent", "Mark contract to let expire", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        let reason = extract_string_param(&tool_context.arguments, "reason");
                        match tools_impl::contract_set_exit_intent(ctx, pid, reason) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // contract_clear_exit_intent
    {
        let ctx = context.clone();
        if !disabled.contains(&"contract_clear_exit_intent".to_string()) {
            router.add_route(ToolRoute::new_dyn(
                Tool::new("contract_clear_exit_intent", "Remove exit intent from contract", player_id_schema()),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        match tools_impl::contract_clear_exit_intent(ctx, pid) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // contract_preview_termination
    {
        let ctx = context.clone();
        if !disabled.contains(&"contract_preview_termination".to_string()) {
            router.add_route(ToolRoute::new_dyn(
                Tool::new("contract_preview_termination", "Preview cost of terminating contract", player_id_schema()),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        match tools_impl::contract_preview_termination(ctx, pid) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // contract_terminate
    {
        let ctx = context.clone();
        if !disabled.contains(&"contract_terminate".to_string()) {
            router.add_route(ToolRoute::new_dyn(
                Tool::new("contract_terminate", "Terminate contract immediately", player_id_schema()),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let pid = extract_string_param(&tool_context.arguments, "player_id").unwrap_or_default();
                        match tools_impl::contract_terminate(ctx, pid) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // inbox_get_messages
    {
        let ctx = context.clone();
        if !disabled.contains(&"inbox_get_messages".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "category": { "type": "string", "description": "Filter by message category" },
                    "unread_only": { "type": "boolean", "description": "Show only unread messages" }
                }
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("inbox_get_messages", "Get messages (filterable by category, read status)", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let category = extract_string_param(&tool_context.arguments, "category");
                        let unread_only = extract_bool_param(&tool_context.arguments, "unread_only");
                        match tools_impl::inbox_get_messages(ctx, category, unread_only) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // inbox_resolve_action
    {
        let ctx = context.clone();
        if !disabled.contains(&"inbox_resolve_action".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "message_id": { "type": "string", "description": "Message ID" },
                    "action_id": { "type": "string", "description": "Action ID within the message" },
                    "option_id": { "type": "string", "description": "Option ID (if action has choices)" }
                },
                "required": ["message_id", "action_id"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("inbox_resolve_action", "Resolve a message action (job offers, events, etc.)", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let mid = extract_string_param(&tool_context.arguments, "message_id").unwrap_or_default();
                        let aid = extract_string_param(&tool_context.arguments, "action_id").unwrap_or_default();
                        let oid = extract_string_param(&tool_context.arguments, "option_id");
                        match tools_impl::inbox_resolve_action(ctx, mid, aid, oid) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // club_upgrade_facility
    {
        let ctx = context.clone();
        if !disabled.contains(&"club_upgrade_facility".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "facility": { "type": "string", "description": "Facility name to upgrade" }
                },
                "required": ["facility"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("club_upgrade_facility", "Upgrade a facility level", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let facility = extract_string_param(&tool_context.arguments, "facility").unwrap_or_default();
                        match tools_impl::club_upgrade_facility(ctx, facility) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // staff_hire
    {
        let ctx = context.clone();
        if !disabled.contains(&"staff_hire".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "staff_id": { "type": "string", "description": "Staff member entity ID" }
                },
                "required": ["staff_id"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("staff_hire", "Hire an unattached staff member", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let sid = extract_string_param(&tool_context.arguments, "staff_id").unwrap_or_default();
                        match tools_impl::staff_hire(ctx, sid) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // staff_release
    {
        let ctx = context.clone();
        if !disabled.contains(&"staff_release".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "staff_id": { "type": "string", "description": "Staff member entity ID" }
                },
                "required": ["staff_id"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("staff_release", "Release a staff member", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let sid = extract_string_param(&tool_context.arguments, "staff_id").unwrap_or_default();
                        match tools_impl::staff_release(ctx, sid) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // season_advance
    real_tool!("season_advance", "Advance to next season (may be fired if objectives not met)", tools_impl::season_advance);

    // help_find_tool
    {
        let ctx = context.clone();
        if !disabled.contains(&"help_find_tool".to_string()) {
            let schema = Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search keyword" }
                },
                "required": ["query"]
            }).as_object().unwrap().clone());
            router.add_route(ToolRoute::new_dyn(
                Tool::new("help_find_tool", "Search tools by keyword or description", schema),
                move |tool_context| {
                    let ctx = ctx.clone();
                    Box::pin(async move {
                        let query = extract_string_param(&tool_context.arguments, "query").unwrap_or_default();
                        match tools_impl::help_find_tool(ctx, query) {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(e) => Ok(error_result(&translate_error(&e))),
                        }
                    })
                },
            ));
        }
    }

    // help_list_categories
    if !disabled.contains(&"help_list_categories".to_string()) {
        router.add_route(ToolRoute::new_dyn(
            simple_tool("help_list_categories", "List all tool categories with counts"),
            |_tool_context| {
                Box::pin(async move {
                    Ok(CallToolResult::success(vec![Content::text(
                        tools_impl::help_list_categories(),
                    )]))
                })
            },
        ));
    }

    // ─── Remaining stubs ────────────────────────────────────────────────────

    let stubs: &[(&str, &str)] = &[
        // Game lifecycle (disabled in competition mode)
        ("game_new", "Create manager + generate/load world"),
        ("game_select_team", "Pick a team to manage"),
        ("game_load_save", "Load an existing save"),
        ("game_exit", "Save and return to menu"),
        ("game_export_world", "Export world to JSON file"),
        // Time
        ("time_skip_to_match_day", "Fast-forward to next fixture"),
        ("time_check_blockers", "Check if anything blocks time advancement"),
        // Transfers (remaining)
        ("transfer_market_browse", "Browse transfer/loan market with optional filters"),
        ("transfer_free_agent_offer", "Offer contract to a free agent"),
        ("transfer_free_agent_preview", "Preview free agent contract financial impact"),
        // Scouting
        ("scout_send", "Send scout to report on a player"),
        ("scout_get_reports", "Get completed scout reports"),
        ("scout_youth_start", "Start youth scouting assignment"),
        ("scout_youth_cancel", "Cancel youth scouting"),
        ("scout_youth_reassign", "Reassign youth scouting parameters"),
        // Info (remaining)
        ("info_player_stats", "Season + career stats for a player"),
        ("info_player_match_history", "Match-by-match stats for a player"),
        ("info_team_profile", "Detailed team view (squad, form, finances)"),
        ("info_team_stats", "Season stats for a team"),
        ("info_team_match_history", "Match-by-match stats for a team"),
        ("info_finance_snapshot", "Detailed financial snapshot"),
        // Club (remaining)
        ("club_request_board_support", "Request board financial support"),
        ("club_request_marketing", "Request marketing campaign"),
        ("club_request_sponsor_pitch", "Request sponsor pitch"),
        // Season (remaining)
        ("season_get_awards", "Get end-of-season awards"),
        // Jobs
        ("jobs_available", "List available job openings"),
        ("jobs_apply", "Apply for a job"),
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

// ─── Parameter extraction helpers ───────────────────────────────────────────

fn extract_string_param(args: &Option<serde_json::Map<String, serde_json::Value>>, key: &str) -> Option<String> {
    args.as_ref()?.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn extract_string_array_param(args: &Option<serde_json::Map<String, serde_json::Value>>, key: &str) -> Option<Vec<String>> {
    args.as_ref()?.get(key).and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
    })
}

fn extract_u64_param(args: &Option<serde_json::Map<String, serde_json::Value>>, key: &str) -> Option<u64> {
    args.as_ref()?.get(key).and_then(|v| v.as_u64())
}

fn extract_u32_param(args: &Option<serde_json::Map<String, serde_json::Value>>, key: &str) -> Option<u32> {
    args.as_ref()?.get(key).and_then(|v| v.as_u64()).map(|n| n as u32)
}

fn extract_bool_param(args: &Option<serde_json::Map<String, serde_json::Value>>, key: &str) -> Option<bool> {
    args.as_ref()?.get(key).and_then(|v| v.as_bool())
}
