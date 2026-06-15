//! MCP tool implementations: season

use std::sync::Arc;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game};
use crate::mcp_server::formatting::translate_error;

// ─── season_check_complete ──────────────────────────────────────────────────

pub fn season_check_complete(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    if let Some(league) = &game.league {
        let incomplete = league.fixtures.iter()
            .filter(|f| f.status != domain::league::FixtureStatus::Completed)
            .count();
        if incomplete == 0 && !league.fixtures.is_empty() {
            return Ok("## Season Status: Complete ✅\n\nAll fixtures played. Use `season_advance` to proceed.".to_string());
        }
        return Ok(format!("## Season Status: In Progress\n\n**Remaining fixtures**: {}", incomplete));
    }

    Ok("## Season Status: No league active.".to_string())
}

// ─── season_advance ─────────────────────────────────────────────────────────

// ─── season_advance ─────────────────────────────────────────────────────────

pub fn season_advance(ctx: Arc<McpContext>) -> Result<String, String> {
    // The season advance is handled by advancing time through the off-season.
    // In competition mode, this uses delegate mode.
    let response = crate::application::time_advancement::advance_time_with_mode(
        &ctx.state_manager,
        "delegate",
    )
    .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    if let Some(ref game) = response.game {
        if game.manager.team_id.is_none() {
            return Ok("## Season Advance\n\n**⚠️ You have been fired!** Use `jobs_available` to find a new position.".to_string());
        }

        Ok(format!("## Day Advanced\n\nDate: {}. Continue advancing to reach next season.", game.clock.current_date.format("%d %B %Y")))
    } else {
        Ok("## Season Advance\n\nGame state lost during advance.".to_string())
    }
}

// ─── help_find_tool ─────────────────────────────────────────────────────────

// ─── season_get_awards ──────────────────────────────────────────────────────

pub fn season_get_awards(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let awards = ofm_core::season_awards::compute_season_awards(&game);

    let mut output = String::from("## Season Awards\n\n");

    let categories = [
        ("🏆 Golden Boot", &awards.golden_boot),
        ("🅰️ Assist King", &awards.assist_king),
        ("⭐ Player of the Year", &awards.player_of_year),
        ("🧤 Clean Sheet King", &awards.clean_sheet_king),
        ("📋 Most Appearances", &awards.most_appearances),
        ("🌟 Young Player", &awards.young_player),
    ];

    for (title, entries) in &categories {
        if !entries.is_empty() {
            output.push_str(&format!("### {}\n\n| # | Player | Team | Value |\n|---|--------|------|-------|\n", title));
            for (i, e) in entries.iter().enumerate() {
                output.push_str(&format!("| {} | {} | {} | {:.1} |\n", i + 1, e.player_name, e.team_name, e.value));
            }
            output.push('\n');
        }
    }

    if !awards.manager_of_season.is_empty() {
        output.push_str("### 👔 Manager of the Season\n\n| # | Manager | Team | Value |\n|---|---------|------|-------|\n");
        for (i, e) in awards.manager_of_season.iter().enumerate() {
            output.push_str(&format!("| {} | {} | {} | {:.1} |\n", i + 1, e.manager_name, e.team_name, e.value));
        }
    }

    Ok(output)
}

// ─── jobs_available ─────────────────────────────────────────────────────────

// ─── jobs_available ─────────────────────────────────────────────────────────

pub fn jobs_available(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let jobs = ofm_core::job_offers::get_available_jobs(&game);

    if jobs.is_empty() {
        return Ok("## Available Jobs\n\nNo job openings available right now.".to_string());
    }

    let mut output = format!("## Available Jobs ({} openings)\n\n| # | Team | City | Reputation | Last Position |\n|---|------|------|------------|---------------|\n", jobs.len());
    for (i, j) in jobs.iter().enumerate() {
        let pos = j.last_league_position.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
        output.push_str(&format!("| {} | {} ({}) | {} | {} | {} |\n",
            i + 1, j.team_name, j.team_id, j.city, j.reputation, pos));
    }

    Ok(output)
}

// ─── jobs_apply ──────────────────────────────────────────────────────────────

// ─── jobs_apply ──────────────────────────────────────────────────────────────

pub fn jobs_apply(ctx: Arc<McpContext>, team_id: String) -> Result<String, String> {
    let mut game = require_game(&ctx.state_manager)?;
    let result = ofm_core::job_offers::apply_for_job(&mut game, &team_id);
    ctx.state_manager.set_game(game);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let result_text = match result {
        ofm_core::job_offers::JobApplicationResult::Hired => "✅ Hired! You are now the manager of this team.",
        ofm_core::job_offers::JobApplicationResult::Rejected => "❌ Rejected. The team chose another candidate.",
        ofm_core::job_offers::JobApplicationResult::InvalidTeam => "⚠️ Invalid team — no opening available.",
        ofm_core::job_offers::JobApplicationResult::AlreadyEmployed => "⚠️ You already have a team. Resign first.",
        ofm_core::job_offers::JobApplicationResult::SameTeam => "⚠️ You are already managing this team.",
        ofm_core::job_offers::JobApplicationResult::NotBetterClub => "⚠️ This club is not a step up from your current position. Only better clubs will consider an employed manager.",
    };

    Ok(format!("## Job Application Result\n\n{}", result_text))
}

// ─── game_new ───────────────────────────────────────────────────────────────
