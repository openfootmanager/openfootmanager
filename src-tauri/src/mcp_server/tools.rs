use std::sync::Arc;

use rmcp::handler::server::tool::ToolRoute;
use rmcp::model::{CallToolResult, Content, Tool};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::formatting::translate_error;
use crate::mcp_server::tools_impl;

/// Type alias for our tool router.
pub type OfmToolRouter = rmcp::handler::server::tool::ToolRouter<Arc<McpContext>>;

// ─── Schema helpers ─────────────────────────────────────────────────────────

/// Default empty input schema for tools with no parameters.
fn empty_input_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    Arc::new(
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
        .as_object()
        .expect("JSON schema is always an object")
        .clone(),
    )
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
        .expect("JSON schema is always an object")
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

/// Build a JSON object schema from a list of (name, type, description, required) tuples.
fn build_schema(
    properties: &[(&str, &str, &str)],
    required: &[&str],
) -> Arc<serde_json::Map<String, serde_json::Value>> {
    let mut props = serde_json::Map::new();
    for (name, ty, desc) in properties {
        props.insert(
            name.to_string(),
            serde_json::json!({ "type": ty, "description": desc }),
        );
    }
    let mut schema = serde_json::json!({
        "type": "object",
        "properties": props,
    });
    if !required.is_empty() {
        schema["required"] = required.iter().map(|s| serde_json::Value::String(s.to_string())).collect();
    }
    Arc::new(schema.as_object().expect("JSON schema is always an object").clone())
}

/// Helper to create a simple tool with no parameters.
fn simple_tool(name: &'static str, description: &'static str) -> Tool {
    Tool::new(name, description, empty_input_schema())
}

// ─── Result helpers ─────────────────────────────────────────────────────────

fn error_result(msg: &str) -> CallToolResult {
    let mut result = CallToolResult::success(vec![Content::text(msg.to_string())]);
    result.is_error = Some(true);
    result
}

fn text_result(text: String) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text)])
}

fn err_result(e: &str) -> CallToolResult {
    error_result(&translate_error(e))
}

// ─── Parameter extraction helpers ───────────────────────────────────────────

fn extract_string_param(args: &Option<serde_json::Map<String, serde_json::Value>>, key: &str) -> Option<String> {
    args.as_ref()?.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Extract a required string parameter. Returns an error result if missing or empty.
fn require_string_param(
    args: &Option<serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Result<String, CallToolResult> {
    extract_string_param(args, key)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error_result(&format!("Missing required parameter: {}", key)))
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
    args.as_ref()?.get(key).and_then(|v| v.as_u64()).and_then(|n| u32::try_from(n).ok())
}

fn extract_bool_param(args: &Option<serde_json::Map<String, serde_json::Value>>, key: &str) -> Option<bool> {
    args.as_ref()?.get(key).and_then(|v| v.as_bool())
}

/// Extract a required u64 parameter. Returns an error result if missing.
fn require_u64_param(
    args: &Option<serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Result<u64, CallToolResult> {
    extract_u64_param(args, key)
        .ok_or_else(|| error_result(&format!("Missing required parameter: {}", key)))
}

/// Extract a required u32 parameter. Returns an error result if missing or out of range.
fn require_u32_param(
    args: &Option<serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Result<u32, CallToolResult> {
    extract_u32_param(args, key)
        .ok_or_else(|| error_result(&format!("Missing required parameter: {}", key)))
}

/// Extract a required bool parameter. Returns an error result if missing.
fn require_bool_param(
    args: &Option<serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Result<bool, CallToolResult> {
    extract_bool_param(args, key)
        .ok_or_else(|| error_result(&format!("Missing required parameter: {}", key)))
}

// ─── Tool router builder ────────────────────────────────────────────────────
//
// ⚠️  When adding a new tool here, also add it to tool_catalog() below and
//     to docs/MCP_SERVER.md. See the checklist at tool_catalog().

/// Build the tool router, omitting any tools whose names appear in `disabled`.
pub fn build_tool_router(context: &Arc<McpContext>, disabled: &[String]) -> OfmToolRouter {
    let mut router = OfmToolRouter::new();

    // ─── Macros for concise tool registration ───────────────────────────────
    //
    // `real_tool!`   — no-param tools: just name, desc, impl function
    // `id_tool!`     — single required string ID param (player_id, message_id, etc.)
    // `custom_tool!` — custom schema + custom extraction body

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
                                Ok(text) => Ok(text_result(text)),
                                Err(e) => Ok(err_result(&e)),
                            }
                        })
                    },
                ));
            }
        };
    }

    /// Register a tool that takes a single required string parameter.
    /// The schema key and the impl function's second parameter name must match.
    macro_rules! id_tool {
        ($name:expr, $desc:expr, $schema:expr, $param_key:ident, $fn:path) => {
            if !disabled.contains(&$name.to_string()) {
                let ctx = context.clone();
                router.add_route(ToolRoute::new_dyn(
                    Tool::new($name, $desc, $schema),
                    move |tool_context| {
                        let ctx = ctx.clone();
                        Box::pin(async move {
                            let $param_key = match require_string_param(&tool_context.arguments, stringify!($param_key)) {
                                Ok(v) => v,
                                Err(e) => return Ok(e),
                            };
                            match $fn(ctx, $param_key) {
                                Ok(text) => Ok(text_result(text)),
                                Err(e) => Ok(err_result(&e)),
                            }
                        })
                    },
                ));
            }
        };
    }

    /// Register a tool with a custom schema and custom param extraction body.
    /// The `$body` closure receives `ctx: Arc<McpContext>` and `args` (the cloned tool arguments).
    macro_rules! custom_tool {
        ($name:expr, $desc:expr, $schema:expr, $ctx:ident, $args:ident, $body:expr) => {
            if !disabled.contains(&$name.to_string()) {
                let ctx_clone = context.clone();
                router.add_route(ToolRoute::new_dyn(
                    Tool::new($name, $desc, $schema),
                    move |tool_context| {
                        let $ctx = ctx_clone.clone();
                        let $args = tool_context.arguments.clone();
                        Box::pin(async move {
                            let $args = &$args;
                            $body
                        })
                    },
                ));
            }
        };
    }

    // ─── Phase 1: ping ──────────────────────────────────────────────────────

    if !disabled.contains(&"ping".to_string()) {
        router.add_route(ToolRoute::new_dyn(
            simple_tool("ping", "Check if the MCP server is alive and responding"),
            |_context| {
                Box::pin(async {
                    Ok(text_result("Pong! OpenFoot Manager MCP server is alive.".to_string()))
                })
            },
        ));
    }

    // ─── Phase 2: Information tools (no params) ─────────────────────────────

    real_tool!("game_is_finished", "Check if the season/game is complete", tools_impl::info::game_is_finished);
    real_tool!("info_game_summary", "High-level game overview: date, position, finances, next match", tools_impl::info::info_game_summary);
    real_tool!("info_standings", "League table as formatted text", tools_impl::info::info_standings);
    real_tool!("info_fixtures", "Upcoming/past fixtures", tools_impl::info::info_fixtures);
    real_tool!("info_finances", "Financial overview + ledger", tools_impl::info::info_finances);
    real_tool!("info_news", "Recent news articles", tools_impl::info::info_news);
    real_tool!("info_season_context", "Season phase and transfer window status", tools_impl::info::info_season_context);
    real_tool!("info_match_preview", "Preview of next match (opponent, form, squad overview)", tools_impl::info::info_match_preview);
    real_tool!("training_get", "Current training settings + fitness overview", tools_impl::training::training_get);
    real_tool!("squad_get", "Squad overview with player IDs, stats, and formation", tools_impl::squad::squad_get);
    real_tool!("inbox_mark_all_read", "Mark all messages as read", tools_impl::inbox::inbox_mark_all_read);
    real_tool!("inbox_clear_old", "Clear old messages", tools_impl::inbox::inbox_clear_old);
    real_tool!("staff_get", "List all staff (your team + available)", tools_impl::club::staff_get);
    real_tool!("season_check_complete", "Check if season is finished", tools_impl::season::season_check_complete);
    real_tool!("game_save", "Persist current game", tools_impl::game::game_save);
    real_tool!("time_advance", "Advance one day (match forced to delegate mode). Includes round summary on match days", tools_impl::time::time_advance);
    real_tool!("squad_auto_set_pieces", "Auto-assign best set-piece takers", tools_impl::squad::squad_auto_set_pieces);
    real_tool!("season_advance", "Advance to next season (may be fired if objectives not met)", tools_impl::season::season_advance);
    real_tool!("time_skip_to_match_day", "Fast-forward to next fixture", tools_impl::time::time_skip_to_match_day);
    real_tool!("time_check_blockers", "Check if anything blocks time advancement", tools_impl::time::time_check_blockers);
    real_tool!("club_request_board_support", "Request board financial support", tools_impl::club::club_request_board_support);
    real_tool!("club_request_marketing", "Request marketing campaign", tools_impl::club::club_request_marketing);
    real_tool!("club_request_sponsor_pitch", "Request sponsor pitch", tools_impl::club::club_request_sponsor_pitch);
    real_tool!("scout_get_reports", "Get completed scout reports", tools_impl::scouting::scout_get_reports);
    real_tool!("season_get_awards", "Get end-of-season awards", tools_impl::season::season_get_awards);
    real_tool!("jobs_available", "List available job openings. Employed managers see only clubs that are a step up in reputation.", tools_impl::season::jobs_available);
    real_tool!("game_exit", "Save and return to menu", tools_impl::game::game_exit);

    // ─── Phase 3: Single-ID-param tools ─────────────────────────────────────

    id_tool!("info_player_profile", "Detailed player card (attributes, stats, contract, morale)", player_id_schema(), player_id, tools_impl::info::info_player_profile);
    id_tool!("info_player_stats", "Season + career stats for a player", player_id_schema(), player_id, tools_impl::info::info_player_stats);
    id_tool!("inbox_mark_read", "Mark a message as read", message_id_schema(), message_id, tools_impl::inbox::inbox_mark_read);
    id_tool!("inbox_delete", "Delete a message", message_id_schema(), message_id, tools_impl::inbox::inbox_delete);
    id_tool!("transfer_toggle_listed", "Toggle player transfer-listed status", player_id_schema(), player_id, tools_impl::transfers::transfer_toggle_listed);
    id_tool!("transfer_toggle_loan", "Toggle player loan-listed status", player_id_schema(), player_id, tools_impl::transfers::transfer_toggle_loan);
    id_tool!("contract_clear_exit_intent", "Remove exit intent from contract", player_id_schema(), player_id, tools_impl::contracts::contract_clear_exit_intent);
    id_tool!("contract_preview_termination", "Preview cost of terminating contract", player_id_schema(), player_id, tools_impl::contracts::contract_preview_termination);
    id_tool!("contract_terminate", "Terminate contract immediately", player_id_schema(), player_id, tools_impl::contracts::contract_terminate);
    id_tool!("staff_hire", "Hire an unattached staff member", staff_id_schema(), staff_id, tools_impl::club::staff_hire);
    id_tool!("staff_release", "Release a staff member", staff_id_schema(), staff_id, tools_impl::club::staff_release);
    id_tool!("game_select_team", "Pick a team to manage", team_id_schema(), team_id, tools_impl::game::game_select_team);
    id_tool!("game_load_save", "Load an existing save", save_id_schema(), save_id, tools_impl::game::game_load_save);
    id_tool!("jobs_apply", "Apply for a job. Employed managers can only apply to better clubs; applying to your own team or a worse club returns an error.", team_id_schema(), team_id, tools_impl::season::jobs_apply);
    id_tool!("scout_youth_cancel", "Cancel youth scouting", assignment_id_schema(), assignment_id, tools_impl::scouting::scout_youth_cancel);

    // ─── Phase 4: Custom-schema tools ───────────────────────────────────────

    // squad_set_formation
    custom_tool!("squad_set_formation", "Change formation (also reassigns outfield positions by defending ability)",
        build_schema(&[("formation", "string", "Formation string (e.g. 4-4-2, 4-3-3, 3-5-2)")], &["formation"]),
        ctx, args, {
            let formation = match require_string_param(args, "formation") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::squad::squad_set_formation(ctx, formation) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // squad_set_starting_xi
    custom_tool!("squad_set_starting_xi", "Set starting eleven by player IDs",
        build_schema(&[("player_ids", "array", "Ordered list of 11 player IDs for the starting eleven")], &["player_ids"]),
        ctx, args, {
            let pids = extract_string_array_param(args, "player_ids").unwrap_or_default();
            match tools_impl::squad::squad_set_starting_xi(ctx, pids) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // squad_set_play_style
    custom_tool!("squad_set_play_style", "Change play style",
        build_schema(&[("play_style", "string", "Play style: Attacking, Defensive, Possession, Counter, HighPress, Balanced")], &["play_style"]),
        ctx, args, {
            let style = match require_string_param(args, "play_style") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::squad::squad_set_play_style(ctx, style) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // squad_set_match_roles
    custom_tool!("squad_set_match_roles", "Set captain and set-piece takers by player ID",
        build_schema(&[
            ("captain", "string", "Player ID for captain"),
            ("vice_captain", "string", "Player ID for vice captain"),
            ("penalty_taker", "string", "Player ID for penalty taker"),
            ("free_kick_taker", "string", "Player ID for free kick taker"),
            ("corner_taker", "string", "Player ID for corner taker"),
        ], &[]),
        ctx, args, {
            match tools_impl::squad::squad_set_match_roles(
                ctx,
                extract_string_param(args, "captain"),
                extract_string_param(args, "vice_captain"),
                extract_string_param(args, "penalty_taker"),
                extract_string_param(args, "free_kick_taker"),
                extract_string_param(args, "corner_taker"),
            ) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // squad_set_player_role
    custom_tool!("squad_set_player_role", "Set player squad role (Senior/Youth)",
        build_schema(&[("player_id", "string", "Player entity ID"), ("squad_role", "string", "Squad role: Senior or Youth")], &["player_id", "squad_role"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let role = match require_string_param(args, "squad_role") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::squad::squad_set_player_role(ctx, pid, role) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // training_set_focus_intensity
    custom_tool!("training_set_focus_intensity", "Set team training focus and intensity",
        build_schema(&[
            ("focus", "string", "Training focus: Physical, Technical, Tactical, Defending, Attacking, Recovery"),
            ("intensity", "string", "Training intensity: Low, Medium, High"),
        ], &["focus", "intensity"]),
        ctx, args, {
            let focus = match require_string_param(args, "focus") { Ok(v) => v, Err(e) => return Ok(e) };
            let intensity = match require_string_param(args, "intensity") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::training::training_set_focus_intensity(ctx, focus, intensity) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // training_set_schedule
    custom_tool!("training_set_schedule", "Set weekly training schedule (Intense/Balanced/Light)",
        build_schema(&[("schedule", "string", "Weekly training schedule: Intense, Balanced, Light")], &["schedule"]),
        ctx, args, {
            let schedule = match require_string_param(args, "schedule") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::training::training_set_schedule(ctx, schedule) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // training_set_groups
    custom_tool!("training_set_groups", "Set training groups with per-group focus overrides",
        build_schema(&[("groups_json", "string", "JSON array of TrainingGroup objects")], &["groups_json"]),
        ctx, args, {
            let groups_json = match require_string_param(args, "groups_json") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::training::training_set_groups(ctx, groups_json) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // training_set_player_focus
    custom_tool!("training_set_player_focus", "Set individual player training focus",
        build_schema(&[("player_id", "string", "Player entity ID"), ("focus", "string", "Individual training focus (omit to clear)")], &["player_id"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let focus = extract_string_param(args, "focus");
            match tools_impl::training::training_set_player_focus(ctx, pid, focus) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // transfer_make_bid
    custom_tool!("transfer_make_bid", "Make a transfer bid; includes negotiation feedback",
        build_schema(&[("player_id", "string", "Player entity ID to bid on"), ("fee", "integer", "Transfer fee amount")], &["player_id", "fee"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let fee = match require_u64_param(args, "fee") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::transfers::transfer_make_bid(ctx, pid, fee) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // transfer_preview_bid
    custom_tool!("transfer_preview_bid", "Preview financial impact of a transfer bid",
        build_schema(&[("player_id", "string", "Player entity ID"), ("fee", "integer", "Proposed transfer fee")], &["player_id", "fee"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let fee = match require_u64_param(args, "fee") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::transfers::transfer_preview_bid(ctx, pid, fee) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // transfer_respond_to_offer
    custom_tool!("transfer_respond_to_offer", "Accept/reject an incoming offer",
        build_schema(&[
            ("player_id", "string", "Player entity ID"),
            ("offer_id", "string", "Offer ID to respond to"),
            ("accept", "boolean", "True to accept, false to reject"),
        ], &["player_id", "offer_id", "accept"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let oid = match require_string_param(args, "offer_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let accept = match require_bool_param(args, "accept") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::transfers::transfer_respond_to_offer(ctx, pid, oid, accept) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // transfer_counter_offer
    custom_tool!("transfer_counter_offer", "Counter an incoming transfer offer",
        build_schema(&[
            ("player_id", "string", "Player entity ID"),
            ("offer_id", "string", "Offer ID to counter"),
            ("requested_fee", "integer", "Counter-offer fee amount"),
        ], &["player_id", "offer_id", "requested_fee"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let oid = match require_string_param(args, "offer_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let fee = match require_u64_param(args, "requested_fee") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::transfers::transfer_counter_offer(ctx, pid, oid, fee) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // transfer_market_browse
    custom_tool!("transfer_market_browse", "Browse transfer/loan market with optional filters",
        build_schema(&[
            ("position", "string", "Filter by position (e.g. GK, CB, ST)"),
            ("max_price", "integer", "Max annual cost estimate"),
            ("listed_only", "boolean", "Only show transfer/loan listed players"),
        ], &[]),
        ctx, args, {
            let pos = extract_string_param(args, "position");
            let max_price = extract_u64_param(args, "max_price");
            let listed_only = extract_bool_param(args, "listed_only");
            match tools_impl::transfers::transfer_market_browse(ctx, pos, max_price, listed_only) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // transfer_free_agent_offer
    custom_tool!("transfer_free_agent_offer", "Offer contract to a free agent",
        build_schema(&[
            ("player_id", "string", "Player entity ID"),
            ("weekly_wage", "integer", "Offered weekly wage"),
            ("contract_years", "integer", "Contract length in years"),
        ], &["player_id", "weekly_wage", "contract_years"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let wage = match require_u32_param(args, "weekly_wage") { Ok(v) => v, Err(e) => return Ok(e) };
            let years = match require_u32_param(args, "contract_years") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::transfers::transfer_free_agent_offer(ctx, pid, wage, years) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // transfer_free_agent_preview
    custom_tool!("transfer_free_agent_preview", "Preview free agent contract financial impact",
        build_schema(&[("player_id", "string", "Player entity ID"), ("weekly_wage", "integer", "Proposed weekly wage")], &["player_id", "weekly_wage"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let wage = match require_u32_param(args, "weekly_wage") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::transfers::transfer_free_agent_preview(ctx, pid, wage) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // contract_propose_renewal
    custom_tool!("contract_propose_renewal", "Propose contract renewal; includes negotiation feedback",
        build_schema(&[
            ("player_id", "string", "Player entity ID"),
            ("weekly_wage", "integer", "Proposed weekly wage"),
            ("contract_years", "integer", "Proposed contract length in years"),
        ], &["player_id", "weekly_wage", "contract_years"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let wage = match require_u32_param(args, "weekly_wage") { Ok(v) => v, Err(e) => return Ok(e) };
            let years = match require_u32_param(args, "contract_years") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::contracts::contract_propose_renewal(ctx, pid, wage, years) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // contract_delegate_renewals
    custom_tool!("contract_delegate_renewals", "Delegate contract renewals to assistant",
        build_schema(&[
            ("player_ids", "array", "Player IDs to renew (omit for all expiring)"),
            ("max_wage_increase_pct", "integer", "Max wage increase percentage"),
            ("max_contract_years", "integer", "Max contract years"),
        ], &["max_wage_increase_pct", "max_contract_years"]),
        ctx, args, {
            let pids = extract_string_array_param(args, "player_ids");
            let max_pct = match require_u32_param(args, "max_wage_increase_pct") { Ok(v) => v, Err(e) => return Ok(e) };
            let max_years = match require_u32_param(args, "max_contract_years") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::contracts::contract_delegate_renewals(ctx, pids, max_pct, max_years) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // contract_preview_renewal
    custom_tool!("contract_preview_renewal", "Preview renewal financial impact",
        build_schema(&[("player_id", "string", "Player entity ID"), ("weekly_wage", "integer", "Proposed weekly wage")], &["player_id", "weekly_wage"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let wage = match require_u32_param(args, "weekly_wage") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::contracts::contract_preview_renewal(ctx, pid, wage) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // contract_set_exit_intent
    custom_tool!("contract_set_exit_intent", "Mark contract to let expire",
        build_schema(&[("player_id", "string", "Player entity ID"), ("reason", "string", "Optional reason for exit intent")], &["player_id"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let reason = extract_string_param(args, "reason");
            match tools_impl::contracts::contract_set_exit_intent(ctx, pid, reason) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // inbox_get_messages
    custom_tool!("inbox_get_messages", "Get messages (filterable by category, read status)",
        build_schema(&[("category", "string", "Filter by message category"), ("unread_only", "boolean", "Show only unread messages")], &[]),
        ctx, args, {
            let category = extract_string_param(args, "category");
            let unread_only = extract_bool_param(args, "unread_only");
            match tools_impl::inbox::inbox_get_messages(ctx, category, unread_only) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // inbox_resolve_action
    custom_tool!("inbox_resolve_action", "Resolve a message action (job offers, events, etc.)",
        build_schema(&[
            ("message_id", "string", "Message ID"),
            ("action_id", "string", "Action ID within the message"),
            ("option_id", "string", "Option ID (if action has choices)"),
        ], &["message_id", "action_id"]),
        ctx, args, {
            let mid = match require_string_param(args, "message_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let aid = match require_string_param(args, "action_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let oid = extract_string_param(args, "option_id");
            match tools_impl::inbox::inbox_resolve_action(ctx, mid, aid, oid) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // club_upgrade_facility
    custom_tool!("club_upgrade_facility", "Upgrade a facility level",
        build_schema(&[("facility", "string", "Facility name to upgrade")], &["facility"]),
        ctx, args, {
            let facility = match require_string_param(args, "facility") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::club::club_upgrade_facility(ctx, facility) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // help_find_tool
    custom_tool!("help_find_tool", "Search tools by keyword or description",
        build_schema(&[("query", "string", "Search keyword")], &["query"]),
        ctx, args, {
            let query = match require_string_param(args, "query") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::help::help_find_tool(ctx, query) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // help_list_categories
    if !disabled.contains(&"help_list_categories".to_string()) {
        router.add_route(ToolRoute::new_dyn(
            simple_tool("help_list_categories", "List all tool categories with counts"),
            |_tool_context| {
                Box::pin(async move {
                    Ok(text_result(tools_impl::help::help_list_categories()))
                })
            },
        ));
    }

    // info_player_match_history
    custom_tool!("info_player_match_history", "Match-by-match stats for a player",
        build_schema(&[("player_id", "string", "Player entity ID"), ("limit", "integer", "Max matches to return")], &["player_id"]),
        ctx, args, {
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let limit = extract_u64_param(args, "limit").map(|n| n as usize);
            match tools_impl::info::info_player_match_history(ctx, pid, limit) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // info_team_profile
    id_tool!("info_team_profile", "Detailed team view (squad, form, finances)", team_id_schema(), team_id, tools_impl::info::info_team_profile);

    // info_team_stats
    id_tool!("info_team_stats", "Season stats for a team", team_id_schema(), team_id, tools_impl::info::info_team_stats);

    // info_team_match_history
    custom_tool!("info_team_match_history", "Match-by-match stats for a team",
        build_schema(&[("team_id", "string", "Team entity ID"), ("limit", "integer", "Max matches to return")], &["team_id"]),
        ctx, args, {
            let tid = match require_string_param(args, "team_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let limit = extract_u64_param(args, "limit").map(|n| n as usize);
            match tools_impl::info::info_team_match_history(ctx, tid, limit) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // info_finance_snapshot
    custom_tool!("info_finance_snapshot", "Detailed financial snapshot",
        build_schema(&[("team_id", "string", "Team entity ID (omit for own team)")], &[]),
        ctx, args, {
            let tid = extract_string_param(args, "team_id");
            match tools_impl::info::info_finance_snapshot(ctx, tid) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // scout_send
    custom_tool!("scout_send", "Send scout to report on a player",
        build_schema(&[("scout_id", "string", "Staff member ID to send"), ("player_id", "string", "Player ID to scout")], &["scout_id", "player_id"]),
        ctx, args, {
            let sid = match require_string_param(args, "scout_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let pid = match require_string_param(args, "player_id") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::scouting::scout_send(ctx, sid, pid) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // scout_youth_start
    custom_tool!("scout_youth_start", "Start youth scouting assignment",
        build_schema(&[
            ("scout_id", "string", "Staff member ID"),
            ("region", "string", "Scouting region: Domestic or International"),
            ("objective", "string", "Scouting objective: Balanced, HighPotential, ReadySoon"),
            ("target_position", "string", "Target position (GK, DF, MF, FW)"),
        ], &["scout_id"]),
        ctx, args, {
            let sid = match require_string_param(args, "scout_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let region = extract_string_param(args, "region");
            let objective = extract_string_param(args, "objective");
            let target_position = extract_string_param(args, "target_position");
            match tools_impl::scouting::scout_youth_start(ctx, sid, region, objective, target_position) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // scout_youth_reassign
    custom_tool!("scout_youth_reassign", "Reassign youth scouting parameters",
        build_schema(&[("assignment_id", "string", "Assignment ID"), ("scout_id", "string", "New staff member ID")], &["assignment_id", "scout_id"]),
        ctx, args, {
            let aid = match require_string_param(args, "assignment_id") { Ok(v) => v, Err(e) => return Ok(e) };
            let sid = match require_string_param(args, "scout_id") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::scouting::scout_youth_reassign(ctx, aid, sid) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // game_new
    custom_tool!("game_new", "Create manager + generate/load world + optionally select team",
        build_schema(&[
            ("first_name", "string", "Manager first name"),
            ("last_name", "string", "Manager last name"),
            ("nationality", "string", "Manager nationality"),
            ("world_source", "string", "World JSON path (omit for random)"),
            ("team_id", "string", "Team to manage (required if world has no pre-assigned manager)"),
        ], &["first_name", "last_name", "nationality"]),
        ctx, args, {
            let first = match require_string_param(args, "first_name") { Ok(v) => v, Err(e) => return Ok(e) };
            let last = match require_string_param(args, "last_name") { Ok(v) => v, Err(e) => return Ok(e) };
            let nat = match require_string_param(args, "nationality") { Ok(v) => v, Err(e) => return Ok(e) };
            let world = extract_string_param(args, "world_source");
            let team = extract_string_param(args, "team_id");
            match tools_impl::game::game_new(ctx, first, last, nat, world, team) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // game_export_world — export path is server-controlled for security
    if !disabled.contains(&"game_export_world".to_string()) {
        let ctx = context.clone();
        router.add_route(ToolRoute::new_dyn(
            Tool::new("game_export_world", "Export world to JSON (saved in app data directory with auto-generated filename)", empty_input_schema()),
            move |_tool_context| {
                let ctx = ctx.clone();
                Box::pin(async move {
                    match tools_impl::game::game_export_world_safe(ctx) {
                        Ok(text) => Ok(text_result(text)),
                        Err(e) => Ok(err_result(&e)),
                    }
                })
            },
        ));
    }

    // ─── New tools: game state, saves, worlds ────────────────────────────────

    // info_game_state — raw JSON game state dump (disabled in Competition mode)
    real_tool!("info_game_state", "Full game state as JSON (useful for programmatic access; disabled in competition mode)", tools_impl::info::info_game_state);

    // game_list_saves
    real_tool!("game_list_saves", "List all saved games with manager name and date", tools_impl::game::game_list_saves);

    // game_delete_save
    id_tool!("game_delete_save", "Permanently delete a saved game", save_id_schema(), save_id, tools_impl::game::game_delete_save);

    // game_list_world_databases
    real_tool!("game_list_world_databases", "List available world databases (built-in random + user JSON files)", tools_impl::game::game_list_world_databases);

    // ─── Live match tools ──────────────────────────────────────────────────────

    // match_start
    custom_tool!("match_start", "Start a live match for a fixture",
        build_schema(&[
            ("fixture_index", "integer", "Index of the fixture in the league fixture list"),
            ("mode", "string", "Match mode: live, spectator, or instant"),
            ("allows_extra_time", "boolean", "Allow extra time if draw (default: true)"),
        ], &["fixture_index", "mode"]),
        ctx, args, {
            let fixture_index = match require_u32_param(args, "fixture_index") { Ok(v) => v, Err(e) => return Ok(e) };
            let mode = match require_string_param(args, "mode") { Ok(v) => v, Err(e) => return Ok(e) };
            let allows_extra_time = extract_bool_param(args, "allows_extra_time");
            match tools_impl::live_match::match_start(ctx, fixture_index, mode, allows_extra_time) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // match_step
    custom_tool!("match_step", "Advance live match by up to N minutes. Stops early at half time, before a penalty shootout, and at full time, so fewer minutes may pass than requested; the response says how many did.",
        build_schema(&[("minutes", "integer", "Number of minutes to advance")], &["minutes"]),
        ctx, args, {
            let minutes = match require_u32_param(args, "minutes") { Ok(v) => v as u16, Err(e) => return Ok(e) };
            match tools_impl::live_match::match_step(ctx, minutes) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // match_command
    custom_tool!("match_command", "Apply a tactical command during a live match (substitution, formation change, etc.)",
        build_schema(&[("command_json", "string", "Match command as JSON (see engine MatchCommand type)")], &["command_json"]),
        ctx, args, {
            let command_json = match require_string_param(args, "command_json") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::live_match::match_command(ctx, command_json) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // match_snapshot
    real_tool!("match_snapshot", "Get current live match state without advancing time", tools_impl::live_match::match_snapshot);

    // match_finish
    real_tool!("match_finish", "Finish the live match, apply results, and clean up", tools_impl::live_match::match_finish);

    // match_team_talk
    custom_tool!("match_team_talk", "Apply a team talk during half-time or full-time break",
        build_schema(&[
            ("tone", "string", "Talk tone: calm, motivational, assertive, aggressive, praise, disappointed"),
            ("context", "string", "Match context: winning, losing, drawing"),
        ], &["tone", "context"]),
        ctx, args, {
            let tone = match require_string_param(args, "tone") { Ok(v) => v, Err(e) => return Ok(e) };
            let context = match require_string_param(args, "context") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::live_match::match_team_talk(ctx, tone, context) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    // match_press_conference — derives team/score from game state to prevent fabrication
    custom_tool!("match_press_conference", "Submit press conference answers after a match (team and score derived from game state)",
        build_schema(&[
            ("answers_json", "string", "JSON array of answer objects with question_id, response_id, response_text, and optionally player_id"),
        ], &["answers_json"]),
        ctx, args, {
            let answers_json = match require_string_param(args, "answers_json") { Ok(v) => v, Err(e) => return Ok(e) };
            match tools_impl::live_match::match_press_conference(ctx, answers_json) {
                Ok(text) => Ok(text_result(text)),
                Err(e) => Ok(err_result(&e)),
            }
        });

    router
}

// ─── Additional schema helpers for id_tool! ─────────────────────────────────

fn staff_id_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    param_schema("staff_id", "Staff member entity ID")
}

fn team_id_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    param_schema("team_id", "Team entity ID")
}

fn save_id_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    param_schema("save_id", "Save ID to load")
}

fn assignment_id_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    param_schema("assignment_id", "Scouting assignment ID")
}

// ─── Tool catalog for help tools ─────────────────────────────────────────────
// Single source of truth for tool names, descriptions, and categories.
//
// ⚠️  WHEN ADDING A NEW MCP TOOL:
//     1. Add the route registration in build_tool_router() above
//     2. Add the (name, description, category) entry here in tool_catalog()
//     3. Add the implementation in the appropriate tools_impl/ sub-module
//     4. If the tool mutates state, emit "game-state-changed" via ctx.app_handle
//     5. If it should be disabled in competition mode, add it to config.rs disabled_tools()
//     6. Update the tool tables in docs/MCP_SERVER.md
//
// The catalog and the router MUST stay in sync — help_find_tool searches only
// the catalog, so a missing entry means the tool is invisible to agents.

/// Returns the full catalog of MCP tools as (name, description, category) tuples.
pub fn tool_catalog() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // Ping
        ("ping", "Check if the MCP server is alive", "Utility"),

        // Information
        ("info_game_summary", "High-level overview: date, league position, finances, next match, unread messages", "Information"),
        ("info_game_state", "Full game state as JSON (programmatic access)", "Information"),
        ("info_standings", "Full league table with goal difference", "Information"),
        ("info_fixtures", "Upcoming fixtures and recent results for your team", "Information"),
        ("info_match_preview", "Next opponent details, form, and standings comparison", "Information"),
        ("info_player_profile", "Detailed player card with attributes and contract", "Information"),
        ("info_player_stats", "Player season and career statistics", "Information"),
        ("info_team_profile", "Detailed team view with squad summary", "Information"),
        ("info_team_stats", "Team season statistics", "Information"),
        ("info_finances", "Financial overview with budget breakdown", "Information"),
        ("info_news", "Recent news articles", "Information"),
        ("info_season_context", "Season phase and transfer window status", "Information"),
        ("info_finance_snapshot", "Detailed financial snapshot", "Information"),
        ("info_player_match_history", "Match-by-match stats for a player", "Information"),
        ("info_team_match_history", "Match-by-match stats for a team", "Information"),

        // Time
        ("time_advance", "Advance one day", "Time"),
        ("time_skip_to_match_day", "Fast-forward to next match day", "Time"),
        ("time_check_blockers", "Check if anything blocks time advancement", "Time"),

        // Squad
        ("squad_get", "Squad overview with starting XI and bench", "Squad"),
        ("squad_set_formation", "Change formation", "Squad"),
        ("squad_set_starting_xi", "Set starting eleven", "Squad"),
        ("squad_set_play_style", "Change play style", "Squad"),
        ("squad_set_match_roles", "Set captain and set-piece takers", "Squad"),
        ("squad_auto_set_pieces", "Auto-assign set-piece takers based on attributes", "Squad"),
        ("squad_set_player_role", "Set player squad role (starter/substitute/reserve)", "Squad"),

        // Training
        ("training_get", "Training settings overview", "Training"),
        ("training_set_focus_intensity", "Set training focus and intensity", "Training"),
        ("training_set_schedule", "Set weekly training schedule", "Training"),
        ("training_set_groups", "Set training group assignments", "Training"),
        ("training_set_player_focus", "Set individual player training focus", "Training"),

        // Transfers
        ("transfer_market_browse", "Browse transfer market", "Transfers"),
        ("transfer_make_bid", "Make a transfer bid", "Transfers"),
        ("transfer_preview_bid", "Preview financial impact of a transfer bid", "Transfers"),
        ("transfer_respond_to_offer", "Accept or reject a transfer offer", "Transfers"),
        ("transfer_counter_offer", "Counter a transfer offer", "Transfers"),
        ("transfer_toggle_listed", "Toggle transfer-listed status", "Transfers"),
        ("transfer_toggle_loan", "Toggle loan-listed status", "Transfers"),
        ("transfer_free_agent_offer", "Sign a free agent", "Transfers"),
        ("transfer_free_agent_preview", "Preview financial impact of signing a free agent", "Transfers"),

        // Contracts
        ("contract_propose_renewal", "Propose contract renewal", "Contracts"),
        ("contract_delegate_renewals", "Delegate contract renewals to staff", "Contracts"),
        ("contract_set_exit_intent", "Set exit intent on a player contract", "Contracts"),
        ("contract_clear_exit_intent", "Clear exit intent on a player contract", "Contracts"),
        ("contract_terminate", "Terminate a player contract", "Contracts"),
        ("contract_preview_renewal", "Preview financial impact of contract renewal", "Contracts"),
        ("contract_preview_termination", "Preview cost of terminating contract", "Contracts"),

        // Inbox
        ("inbox_get_messages", "Get inbox messages", "Inbox"),
        ("inbox_mark_read", "Mark message as read", "Inbox"),
        ("inbox_mark_all_read", "Mark all messages as read", "Inbox"),
        ("inbox_delete", "Delete a message", "Inbox"),
        ("inbox_clear_old", "Clear old messages", "Inbox"),
        ("inbox_resolve_action", "Resolve an inbox action item", "Inbox"),

        // Club
        ("club_upgrade_facility", "Upgrade a club facility", "Club"),
        ("club_request_board_support", "Request board support for transfer budget or wage bill", "Club"),
        ("club_request_marketing", "Request marketing campaign", "Club"),
        ("club_request_sponsor_pitch", "Request sponsor pitch", "Club"),
        ("staff_hire", "Hire staff member", "Club"),
        ("staff_release", "Release staff member", "Club"),
        ("staff_get", "List all staff (your team + available)", "Club"),

        // Scouting
        ("scout_youth_start", "Start youth scouting assignment", "Scouting"),
        ("scout_youth_cancel", "Cancel youth scouting assignment", "Scouting"),
        ("scout_youth_reassign", "Reassign youth scouting parameters", "Scouting"),
        ("scout_get_reports", "Get completed scout reports", "Scouting"),
        ("scout_send", "Send scout to report on a player", "Scouting"),

        // Season
        ("season_check_complete", "Check if season is complete and ready to advance", "Season"),
        ("season_advance", "Advance to next season", "Season"),
        ("season_get_awards", "Get end-of-season awards", "Season"),

        // Game Lifecycle
        ("game_new", "Create a new manager and generate/load a world", "Game Lifecycle"),
        ("game_select_team", "Pick a team to manage", "Game Lifecycle"),
        ("game_load_save", "Load an existing save", "Game Lifecycle"),
        ("game_save", "Persist the current game", "Game Lifecycle"),
        ("game_exit", "Auto-save and return to menu", "Game Lifecycle"),
        ("game_export_world", "Export world data to JSON", "Game Lifecycle"),
        ("game_list_saves", "List all saved games", "Game Lifecycle"),
        ("game_delete_save", "Permanently delete a saved game", "Game Lifecycle"),
        ("game_list_world_databases", "List available world databases", "Game Lifecycle"),
        ("game_is_finished", "Check if game is finished", "Game Lifecycle"),

        // Live Match
        ("match_start", "Start a live match for a fixture", "Live Match"),
        ("match_step", "Advance the live match by N minutes", "Live Match"),
        ("match_command", "Apply a tactical command during a live match", "Live Match"),
        ("match_snapshot", "Get current match state without advancing time", "Live Match"),
        ("match_finish", "Finish the match, apply results", "Live Match"),
        ("match_team_talk", "Apply a team talk during a match break", "Live Match"),
        ("match_press_conference", "Submit press conference answers after a match", "Live Match"),

        // Jobs
        ("jobs_available", "List current job openings", "Jobs"),
        ("jobs_apply", "Apply for a job", "Jobs"),

        // Help
        ("help_find_tool", "Search tools by keyword", "Help"),
        ("help_list_categories", "List all tool categories with tool counts", "Help"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// This module's own source. The router is built from closures capturing an
    /// `Arc<McpContext>`, which owns a `tauri::AppHandle`, so building a real router in a unit
    /// test would mean standing up a mock Tauri application just to read a list of names. The
    /// registrations are plain, regular source text, so they are read from there instead.
    ///
    /// The hazard with any textual check is that it quietly stops matching and passes on an empty
    /// parse. `router_registrations_are_all_recognised` exists to make that impossible.
    ///
    /// This parser goes away when the router can report its registered names without a constructed
    /// `McpContext` — that is, without a `tauri::AppHandle`.
    const FULL_SOURCE: &str = include_str!("tools.rs");

    /// Everything above this test module.
    ///
    /// Necessary because the tests below quote the very patterns they count: the first draft
    /// counted ten route registrations where the router has six, having matched its own comments.
    /// Anything after the module marker is this test's text, not a registration.
    fn source() -> &'static str {
        // Split on the *last* such marker, and name `mod tests {` in it: a `#[cfg(test)]` helper
        // added higher up the file would otherwise cut production code short, and every check
        // below would start reporting on registrations it never read.
        FULL_SOURCE
            .rsplit_once("\n#[cfg(test)]\nmod tests {")
            .map(|(production_code, _)| production_code)
            .expect("this test module is introduced by `#[cfg(test)] mod tests {`")
    }

    /// The macros every bulk registration goes through.
    const REGISTRATION_MACROS: &[&str] = &["real_tool!(", "id_tool!(", "custom_tool!("];

    /// Route registrations that live inside a `macro_rules!` definition rather than being a
    /// registration in their own right — one per macro above.
    const REGISTRATIONS_INSIDE_MACRO_DEFINITIONS: usize = REGISTRATION_MACROS.len();

    fn first_string_literal(line: &str) -> Option<&str> {
        let after_quote = line.find('"')? + 1;
        let rest = &line[after_quote..];
        let closing = rest.find('"')?;
        Some(&rest[..closing])
    }

    fn is_macro_invocation(line: &str) -> bool {
        REGISTRATION_MACROS
            .iter()
            .any(|macro_name| line.starts_with(macro_name))
    }

    /// The first string literal at or after `start`, searching at most a few lines.
    ///
    /// The bound matters: without it a registration whose name went missing would silently adopt
    /// the next literal anywhere below it, which is how a text parser turns a real defect into a
    /// plausible wrong answer. Nothing in this file puts more than three lines between a
    /// registration and its name.
    fn literal_at_or_after(lines: &[&str], start: usize) -> Option<String> {
        const LOOKAHEAD: usize = 3;

        lines
            .iter()
            .skip(start)
            .take(LOOKAHEAD + 1)
            .find_map(|line| first_string_literal(line))
            .map(String::from)
    }

    /// Tools registered through one of the registration macros, in registration order.
    ///
    /// The name is taken from the invocation line *or the lines just below it*. Requiring it on
    /// the invocation line looks simpler and is a trap: `rustfmt` splits
    /// `real_tool!("name", "desc", path);` across four lines, and it does so for 82 of the 86
    /// invocations in this file. A same-line parser therefore loses 82 names the moment the
    /// pending repo-wide format sweep lands — every check here fails at once, for no defect.
    fn macro_registered_names_in(source: &str) -> Vec<String> {
        let lines: Vec<&str> = source.lines().map(str::trim).collect();

        lines
            .iter()
            .enumerate()
            .filter(|(_, line)| is_macro_invocation(line))
            .filter_map(|(index, _)| literal_at_or_after(&lines, index))
            .collect()
    }

    fn macro_registered_names() -> Vec<String> {
        macro_registered_names_in(source())
    }

    fn macro_invocation_line_count() -> usize {
        source()
            .lines()
            .map(str::trim)
            .filter(|line| is_macro_invocation(line))
            .count()
    }

    /// The name each hand-rolled block *guards* on, paired with the name it actually *registers*.
    ///
    /// These are two independent spellings of the same tool — `if !disabled.contains(&"ping")`
    /// and `simple_tool("ping", …)` — and only the second one reaches an agent. Reading just the
    /// guard, as this module first did, means a block whose two spellings disagree passes every
    /// check while the router serves a name the catalog has never heard of.
    ///
    /// Inside the macro definitions the same guard reads `$name`, so it does not match here.
    fn hand_rolled_guard_and_registered_names_in(source: &str) -> Vec<(String, Option<String>)> {
        // The `Tool` sits within a few lines of its guard — at most guard, `let ctx = …`,
        // `router.add_route(ToolRoute::new_dyn(`, then the name. Searching further would let a
        // guard that wraps no registration at all pair up with the *next* block's tool, which
        // reads as a name mismatch somewhere it isn't.
        const BLOCK: usize = 4;
        const GUARD: &str = "if !disabled.contains(&\"";
        const REGISTRARS: &[&str] = &["simple_tool(", "Tool::new("];

        let lines: Vec<&str> = source.lines().map(str::trim).collect();

        lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with(GUARD))
            .filter_map(|(index, line)| {
                let guard = String::from(first_string_literal(line)?);

                let registered = lines
                    .iter()
                    .enumerate()
                    .skip(index)
                    .take(BLOCK + 1)
                    .find(|(_, below)| REGISTRARS.iter().any(|start| below.starts_with(start)))
                    .and_then(|(at, _)| literal_at_or_after(&lines, at));

                Some((guard, registered))
            })
            .collect()
    }

    /// Tools registered by hand rather than through a macro, named as the router names them.
    ///
    /// A guard whose registration could not be read is dropped here rather than guessed at, which
    /// makes `router_registrations_are_all_recognised` fall out of balance and say so.
    fn hand_rolled_names() -> Vec<String> {
        hand_rolled_guard_and_registered_names_in(source())
            .into_iter()
            .filter_map(|(_, registered)| registered)
            .collect()
    }

    /// Every routed name, in registration order and including any repeats.
    ///
    /// `routed_names` collapses these into a set, which is what the catalog comparisons need but
    /// is also exactly what hides a name registered twice.
    fn routed_names_in_order() -> Vec<String> {
        macro_registered_names()
            .into_iter()
            .chain(hand_rolled_names())
            .collect()
    }

    fn routed_names() -> BTreeSet<String> {
        routed_names_in_order().into_iter().collect()
    }

    fn catalogued_names() -> BTreeSet<String> {
        tool_catalog()
            .into_iter()
            .map(|(name, _, _)| String::from(name))
            .collect()
    }

    /// Guards every other test in this module against parsing nothing and passing anyway.
    ///
    /// A registration written in a style the parser does not recognise still adds a route, so
    /// pinning those two counts against each other turns an unrecognised style into a loud
    /// failure rather than a silently uncounted tool.
    #[test]
    fn router_registrations_are_all_recognised() {
        let registrations = source().matches("add_route(").count();
        let hand_rolled = hand_rolled_names().len();

        assert_eq!(
            registrations,
            REGISTRATIONS_INSIDE_MACRO_DEFINITIONS + hand_rolled,
            "found {registrations} route registrations but recognised {hand_rolled} hand-rolled \
             ones plus {REGISTRATIONS_INSIDE_MACRO_DEFINITIONS} inside macro definitions. A tool \
             was registered in a style this test cannot see — teach it the new style rather than \
             deleting the assertion, or the catalog checks below start passing for tools they \
             never examined.",
        );

        assert_eq!(
            macro_registered_names().len(),
            macro_invocation_line_count(),
            "a registration macro was invoked without a tool name in the three lines below it, so \
             its name could not be extracted",
        );
    }

    /// The other half of the recognition guard: every *textual* use of a registration macro has
    /// to be one this parser counted.
    ///
    /// `is_macro_invocation` asks whether the trimmed line *starts with* the macro name, so
    /// anything sharing that line — `#[cfg(feature = "x")] real_tool!("secret", …)` — is invisible
    /// and its tool would be routed without ever being compared against the catalog. Counting
    /// occurrences instead of trusting the line shape turns that into a failure.
    #[test]
    fn every_macro_invocation_is_recognised() {
        let occurrences: usize = REGISTRATION_MACROS
            .iter()
            .map(|macro_name| source().matches(macro_name).count())
            .sum();

        assert_eq!(
            occurrences,
            macro_invocation_line_count(),
            "found {occurrences} uses of a registration macro but only \
             {} of them begin a line. One is written in a shape this parser skips — most likely \
             an attribute or another statement sharing the line — so its tool is routed without \
             ever being checked against the catalog.",
            macro_invocation_line_count(),
        );
    }

    /// A hand-rolled tool spells its name twice: once in the disable guard and once in the `Tool`
    /// handed to `add_route`. Only the second reaches an agent, and nothing makes them agree.
    ///
    /// This is the gap that made the rest of this module honest-looking rather than honest: the
    /// checks below all read the guard, so renaming only the registered tool left every one of
    /// them green while `help_find_tool` advertised a name the router no longer served.
    #[test]
    fn hand_rolled_guards_match_the_names_they_register() {
        let pairs = hand_rolled_guard_and_registered_names_in(source());

        let unreadable: Vec<&String> = pairs
            .iter()
            .filter(|(_, registered)| registered.is_none())
            .map(|(guard, _)| guard)
            .collect();

        assert!(
            unreadable.is_empty(),
            "no `simple_tool(\"…\")` or `Tool::new(\"…\")` found beneath these disable guards: \
             {unreadable:?}. Either the guard wraps no registration, or the registration is \
             written in a shape this parser cannot read — in which case its tool is routed \
             without ever being compared against the catalog.",
        );

        let disagreements: Vec<(&String, &String)> = pairs
            .iter()
            .filter_map(|(guard, registered)| Some((guard, registered.as_ref()?)))
            .filter(|(guard, registered)| guard != registered)
            .collect();

        assert!(
            disagreements.is_empty(),
            "these hand-rolled tools guard on one name and register another (guard, registered): \
             {disagreements:?}. Agents call the registered name; the disable list and this test \
             read the guard. Make them the same literal.",
        );
    }

    /// A hand-rolled block with only one route in it, so the fixtures below stay readable.
    fn hand_rolled_block(guard: &str, registered: &str) -> String {
        format!(
            "if !disabled.contains(&\"{guard}\".to_string()) {{\n    \
             router.add_route(ToolRoute::new_dyn(\n        \
             simple_tool(\"{registered}\", \"desc\"),\n    ));\n}}\n"
        )
    }

    /// Proves the guard/registration comparison actually bites. Without it the assertion above
    /// would pass just as happily against a parser that compared a name with itself.
    #[test]
    fn a_guard_that_disagrees_with_its_registration_is_caught() {
        let agreeing = hand_rolled_block("ping", "ping");
        let disagreeing = hand_rolled_block("ping", "pong");

        assert_eq!(
            hand_rolled_guard_and_registered_names_in(&agreeing),
            vec![(String::from("ping"), Some(String::from("ping")))],
        );
        assert_eq!(
            hand_rolled_guard_and_registered_names_in(&disagreeing),
            vec![(String::from("ping"), Some(String::from("pong")))],
            "the parser must report the registered name, not the guard name twice",
        );
    }

    /// A guard that wraps no registration must come back empty-handed rather than adopting the
    /// next block's tool.
    ///
    /// Searching the whole remaining file for a `Tool` looks harmless and is not: a stray guard
    /// then pairs with a registration far below it, and the mismatch is reported against a block
    /// that is perfectly correct. It also lets the recognition count stay balanced — one
    /// unreadable registration cancelling one stray guard — which is precisely the arithmetic
    /// that check exists to prevent.
    #[test]
    fn a_guard_with_no_registration_beneath_it_pairs_with_nothing() {
        let stray = format!(
            "if !disabled.contains(&\"stray\".to_string()) {{\n    tidy_up();\n}}\n\n{}",
            hand_rolled_block("ping", "ping"),
        );

        assert_eq!(
            hand_rolled_guard_and_registered_names_in(&stray),
            vec![
                (String::from("stray"), None),
                (String::from("ping"), Some(String::from("ping"))),
            ],
        );
    }

    /// Proves the name extraction survives `rustfmt`.
    ///
    /// The second fixture is the exact shape the formatter produces, and it is not hypothetical:
    /// running `rustfmt` over this file today reflows 82 of its 86 invocations into it. Before
    /// this test existed the parser read only the invocation line and would have found nothing
    /// there.
    #[test]
    fn a_registration_split_across_lines_is_still_read() {
        let same_line = "real_tool!(\"game_status\", \"desc\", tools_impl::info::game_status);\n";
        let reflowed = "real_tool!(\n    \"game_status\",\n    \"desc\",\n    \
                        tools_impl::info::game_status\n);\n";

        assert_eq!(macro_registered_names_in(same_line), vec!["game_status"]);
        assert_eq!(
            macro_registered_names_in(reflowed),
            vec!["game_status"],
            "rustfmt splits long registrations across lines; a same-line parser loses the name",
        );
    }

    /// `ToolRouter::add_route` stores routes with `HashMap::insert` (rmcp 1.7,
    /// `handler/server/router/tool.rs`), so registering a name twice does not fail — the second
    /// registration silently replaces the first handler, and the tool starts answering with
    /// another tool's implementation.
    ///
    /// Nothing else here can see that. The set comparisons below dedupe, and the recognition
    /// count rises consistently because both the invocation count and the extracted-name count go
    /// up together.
    #[test]
    fn no_tool_is_routed_twice() {
        let mut seen = BTreeSet::new();
        let duplicates: Vec<String> = routed_names_in_order()
            .into_iter()
            .filter(|name| !seen.insert(name.clone()))
            .collect();

        assert!(
            duplicates.is_empty(),
            "these tools are registered more than once: {duplicates:?}. The router keeps the last \
             registration, so the earlier handler is unreachable and the tool answers as whatever \
             was registered after it.",
        );
    }

    /// `help_find_tool` searches only the catalog, so a routed tool missing from it is callable
    /// but undiscoverable. Until now this was prevented only by a ⚠️ in a comment.
    #[test]
    fn every_routed_tool_is_in_the_catalog() {
        let catalogued = catalogued_names();
        let missing: Vec<String> = routed_names()
            .into_iter()
            .filter(|name| !catalogued.contains(name))
            .collect();

        assert!(
            missing.is_empty(),
            "registered in build_tool_router() but absent from tool_catalog():\n\n  {missing:?}\n\n\
             help_find_tool searches only the catalog, so an agent can never discover these. Add a \
             (name, description, category) entry — see the checklist above tool_catalog().",
        );
    }

    /// The reverse direction, which is worse: a catalogued tool with no route is advertised to
    /// agents and then fails when called.
    #[test]
    fn every_catalogued_tool_has_a_route() {
        let routed = routed_names();
        let missing: Vec<String> = catalogued_names()
            .into_iter()
            .filter(|name| !routed.contains(name))
            .collect();

        assert!(
            missing.is_empty(),
            "listed in tool_catalog() but never registered in build_tool_router():\n\n  \
             {missing:?}\n\n\
             Agents will find these via help_find_tool and get an unknown-tool error on call.",
        );
    }

    #[test]
    fn the_catalog_has_no_duplicate_entries() {
        let catalog = tool_catalog();
        let unique: BTreeSet<&str> = catalog.iter().map(|(name, _, _)| *name).collect();

        assert_eq!(
            catalog.len(),
            unique.len(),
            "tool_catalog() lists {} entries but only {} distinct names; a duplicate makes \
             help_find_tool report the same tool twice",
            catalog.len(),
            unique.len(),
        );
    }
}
