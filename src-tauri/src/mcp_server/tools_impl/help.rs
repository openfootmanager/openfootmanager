//! MCP tool implementations: help

use std::sync::Arc;
use crate::mcp_server::context::McpContext;

// ─── help_find_tool ─────────────────────────────────────────────────────────

pub fn help_find_tool(_ctx: Arc<McpContext>, query: String) -> Result<String, String> {
    // Simple keyword search across all tool names and descriptions
    let query_lower = query.to_lowercase();
    let all_tools: Vec<(&str, &str)> = vec![
        ("info_game_summary", "Overview of current game state"),
        ("info_standings", "League table"),
        ("info_fixtures", "Upcoming and recent fixtures"),
        ("info_player_profile", "Detailed player card"),
        ("info_player_stats", "Player season/career stats"),
        ("info_team_profile", "Detailed team view"),
        ("info_team_stats", "Team season stats"),
        ("info_finances", "Financial overview"),
        ("info_news", "Recent news"),
        ("info_season_context", "Season phase and transfer window"),
        ("info_match_preview", "Next match preview"),
        ("time_advance", "Advance one day"),
        ("time_skip_to_match_day", "Fast-forward to match day"),
        ("squad_get", "Squad overview"),
        ("squad_set_formation", "Change formation"),
        ("squad_set_starting_xi", "Set starting eleven"),
        ("squad_set_play_style", "Change play style"),
        ("squad_set_match_roles", "Set captain and set-piece takers"),
        ("squad_auto_set_pieces", "Auto-assign set-piece takers"),
        ("squad_set_player_role", "Set player squad role"),
        ("training_get", "Training settings"),
        ("training_set_focus_intensity", "Set training focus and intensity"),
        ("transfer_make_bid", "Make a transfer bid"),
        ("transfer_toggle_listed", "Toggle transfer-listed status"),
        ("transfer_toggle_loan", "Toggle loan-listed status"),
        ("contract_propose_renewal", "Propose contract renewal"),
        ("contract_delegate_renewals", "Delegate contract renewals"),
        ("inbox_get_messages", "Get inbox messages"),
        ("club_upgrade_facility", "Upgrade facility"),
        ("staff_hire", "Hire staff"),
        ("season_advance", "Advance season"),
        ("game_save", "Save current game"),
        ("game_is_finished", "Check if game is finished"),
    ];

    let matches: Vec<_> = all_tools.iter()
        .filter(|(name, desc)| name.contains(&query_lower) || desc.to_lowercase().contains(&query_lower))
        .collect();

    if matches.is_empty() {
        return Ok(format!("## Tool Search: '{}'\n\nNo tools found matching your query. Try `help_list_categories`.", query));
    }

    let mut output = format!("## Tool Search: '{}'\n\n| Tool | Description |\n|------|-------------|\n", query);
    for (name, desc) in matches {
        output.push_str(&format!("| {} | {} |\n", name, desc));
    }

    Ok(output)
}

// ─── help_list_categories ───────────────────────────────────────────────────

// ─── help_list_categories ───────────────────────────────────────────────────

pub fn help_list_categories() -> String {
    let categories: Vec<(&str, &[&str])> = vec![
        ("📋 Information", &["info_game_summary", "info_standings", "info_fixtures", "info_player_profile", "info_finances", "info_news", "info_season_context", "info_match_preview"]),
        ("⏰ Time", &["time_advance", "time_skip_to_match_day", "time_check_blockers"]),
        ("⚽ Squad", &["squad_get", "squad_set_formation", "squad_set_starting_xi", "squad_set_play_style", "squad_set_match_roles", "squad_auto_set_pieces"]),
        ("🏋️ Training", &["training_get", "training_set_focus_intensity", "training_set_schedule", "training_set_groups"]),
        ("💰 Transfers", &["transfer_make_bid", "transfer_toggle_listed", "transfer_toggle_loan", "transfer_respond_to_offer", "transfer_counter_offer", "transfer_free_agent_offer"]),
        ("📝 Contracts", &["contract_propose_renewal", "contract_delegate_renewals", "contract_set_exit_intent", "contract_terminate"]),
        ("📬 Inbox", &["inbox_get_messages", "inbox_mark_read", "inbox_mark_all_read", "inbox_resolve_action"]),
        ("🏢 Club", &["club_upgrade_facility", "staff_hire", "staff_release"]),
        ("🗓️ Season", &["season_check_complete", "season_advance"]),
        ("💾 Game", &["game_save", "game_is_finished"]),
        ("❓ Help", &["help_find_tool", "help_list_categories"]),
    ];

    let mut output = String::from("## Tool Categories\n\n");
    for (cat, tools) in &categories {
        output.push_str(&format!("**{}** ({} tools): {}\n\n", cat, tools.len(), tools.join(", ")));
    }

    output
}

// ─── time_skip_to_match_day ─────────────────────────────────────────────────
