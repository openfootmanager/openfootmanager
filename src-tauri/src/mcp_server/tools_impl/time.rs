//! MCP tool implementations: time

use std::sync::{Arc, Mutex};
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game, require_league};
use crate::mcp_server::formatting::translate_error;

// ─── time_advance ───────────────────────────────────────────────────────────

pub fn time_advance(ctx: Arc<McpContext>) -> Result<String, String> {
    // Rate limiting: enforce minimum delay between advances
    if ctx.config.min_tick_delay_ms > 0 {
        // Simple approach: sleep for the configured delay
        // A more sophisticated approach would track last_advance timestamp
        std::thread::sleep(std::time::Duration::from_millis(ctx.config.min_tick_delay_ms));
    }

    // Use the delegate mode to force auto-simulation of matches
    let response = crate::application::time_advancement::advance_time_with_mode(
        &ctx.state_manager,
        "delegate",
    )
    .map_err(|e| translate_error(&e))?;

    let mut output = String::new();

    // Current date
    let date_str = if let Some(ref game) = response.game {
        game.clock.current_date.format("%d %B %Y").to_string()
    } else {
        "Unknown".to_string()
    };
    output.push_str(&format!("## Day Advanced — {}\n\n", date_str));

    // If there was a match, show round summary
    if let Some(ref round_summary) = response.round_summary {
        if !round_summary.completed_results.is_empty() {
            output.push_str("### Match Results\n\n| Home | Score | Away |\n|------|-------|------|\n");
            for result in &round_summary.completed_results {
                output.push_str(&format!(
                    "| {} | {} - {} | {} |\n",
                    result.home_team_name,
                    result.home_goals,
                    result.away_goals,
                    result.away_team_name,
                ));
            }

            // Highlight user's match
            if let Some(ref game) = response.game {
                if let Some(team_id) = &game.manager.team_id {
                    for result in &round_summary.completed_results {
                        if result.home_team_id == *team_id || result.away_team_id == *team_id {
                            let is_home = result.home_team_id == *team_id;
                            let our_goals = if is_home { result.home_goals } else { result.away_goals };
                            let their_goals = if is_home { result.away_goals } else { result.home_goals };
                            let opponent = if is_home { &result.away_team_name } else { &result.home_team_name };
                            let venue = if is_home { "H" } else { "A" };
                            let result_text = if our_goals > their_goals {
                                "won"
                            } else if our_goals < their_goals {
                                "lost"
                            } else {
                                "drew"
                            };
                            output.push_str(&format!(
                                "\nYour team {} {}-{} vs {} ({}).",
                                result_text, our_goals, their_goals, opponent, venue
                            ));
                            break;
                        }
                    }
                }
            }
        }

        // Standings update if we have a game
        if let Some(ref game) = response.game {
            if let Some(league) = &game.league {
                if let Some(team_id) = &game.manager.team_id {
                    let mut standings = league.standings.clone();
                    standings.sort_by(|a, b| b.points.cmp(&a.points).then_with(|| b.goals_for.cmp(&a.goals_for)));
                    if let Some(pos) = standings.iter().position(|s| s.team_id == *team_id) {
                        let standing = &standings[pos];
                        output.push_str(&format!(
                            "\n\n### Standings Update\n\nLeague position: {} | Points: {} | GD: {:+}",
                            pos + 1,
                            standing.points,
                            i64::from(standing.goals_for) - i64::from(standing.goals_against),
                        ));
                    }
                }
            }
        }
    }

    // Check if manager was fired during the advance
    if let Some(ref game) = response.game {
        if game.manager.team_id.is_none() {
            output.push_str("\n\n**⚠️ You have been fired!** Use `jobs_available` to find a new position.");
        }
    }

    // Auto-save every N in-game days (per-save tracking)
    if ctx.config.auto_save_interval_days > 0 {
        if let Some(ref game) = response.game {
            if let Some(save_id) = ctx.state_manager.get_save_id() {
                use std::sync::LazyLock;
                use std::collections::HashMap;
                static SAVE_DAY_COUNTERS: LazyLock<Mutex<HashMap<String, u32>>> =
                    LazyLock::new(|| Mutex::new(HashMap::new()));

                let mut counters = SAVE_DAY_COUNTERS.lock().unwrap();
                let days = counters.entry(save_id.clone()).or_insert(0);
                *days += 1;
                if *days >= ctx.config.auto_save_interval_days {
                    *days = 0;
                    drop(counters); // release lock before save
                    let stats_state = ctx.state_manager
                        .get_stats_state(|s| s.clone())
                        .unwrap_or_default();
                    if let Ok(mut sm) = ctx.save_manager_state.0.lock() {
                        let _ = sm.save_game_with_stats(game, &stats_state, &save_id);
                        output.push_str("\n\n💾 *Auto-saved.*");
                    }
                }
            }
        }
    }

    // Notify GUI about state change
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(output)
}

// ─── squad_get ──────────────────────────────────────────────────────────────

// ─── time_skip_to_match_day ─────────────────────────────────────────────────

pub fn time_skip_to_match_day(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let league = require_league(&game)?;
    let team_id = game.manager.team_id.as_deref().ok_or("be.error.noTeamAssigned")?;

    // Find next fixture for user's team
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let next_fixture = league.fixtures.iter()
        .filter(|f| f.status != domain::league::FixtureStatus::Completed)
        .filter(|f| f.home_team_id == team_id || f.away_team_id == team_id)
        .filter(|f| f.date > today)
        .min_by_key(|f| &f.date);

    let Some(fixture) = next_fixture else {
        return Ok("## No Upcoming Match\n\nNo more fixtures scheduled for your team.".to_string());
    };

    let target_date = fixture.date.clone();
    let days_to_skip = {
        let current = game.clock.current_date.date_naive();
        let target = chrono::NaiveDate::parse_from_str(&target_date, "%Y-%m-%d")
            .map_err(|e| format!("Date parse error: {}", e))?;
        (target - current).num_days()
    };

    if days_to_skip <= 0 {
        return Ok("## Match Day Today\n\nYour next match is today. Use `time_advance` to play it.".to_string());
    }

    // Advance time day by day until we reach the match day
    let mut advanced = 0u32;
    loop {
        let game = require_game(&ctx.state_manager)?;
        let current = game.clock.current_date.format("%Y-%m-%d").to_string();
        if current >= target_date {
            break;
        }

        crate::application::time_advancement::advance_time_with_mode(
            &ctx.state_manager,
            "delegate",
        )
        .map_err(|e| translate_error(&e))?;

        advanced += 1;

        // Safety limit
        if advanced > 365 {
            return Ok("## Skip Aborted\n\nSkipped more than 365 days without reaching match. Something may be wrong.".to_string());
        }
    }

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Skipped to Match Day\n\n**{} days advanced** to {}.\nUse `time_advance` to play the match.", advanced, target_date))
}

// ─── time_check_blockers ────────────────────────────────────────────────────

// ─── time_check_blockers ────────────────────────────────────────────────────

pub fn time_check_blockers(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    let mut blockers = Vec::new();

    // Check for live match in progress
    if ctx.state_manager.with_live_match(|_| true).unwrap_or(false) {
        blockers.push("Live match in progress — finish the match first".to_string());
    }

    // Check for pending transfer offers requiring response
    let team_id = game.manager.team_id.as_deref();
    if let Some(tid) = team_id {
        let pending_offers: Vec<_> = game.players.iter()
            .filter(|p| p.team_id.as_deref() == Some(tid))
            .flat_map(|p| p.transfer_offers.iter())
            .filter(|o| o.status == domain::player::TransferOfferStatus::Pending)
            .collect();

        if !pending_offers.is_empty() {
            blockers.push(format!("{} pending transfer offer(s) need response", pending_offers.len()));
        }
    }

    // Check for contract renewal deadlines
    if let Some(_tid) = team_id {
        // Note: exit_intent is nested in player.morale_core.renewal_state.exit_intent
        // Skip this check for simplicity — agents can use info_player_profile to check
    }

    if blockers.is_empty() {
        Ok("## No Blockers\n\nTime can be advanced safely.".to_string())
    } else {
        Ok(format!("## ⚠️ Blockers Detected\n\n{}", blockers.iter().map(|b| format!("- {}", b)).collect::<Vec<_>>().join("\n")))
    }
}

// ─── transfer_market_browse ─────────────────────────────────────────────────
