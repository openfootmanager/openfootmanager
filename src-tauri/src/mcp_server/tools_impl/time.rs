//! MCP tool implementations: time

use mcp_results::time::{
    Blockers, DayAdvanced, Outcome, PlayedMatch, SkipToMatchDay, StandingsUpdate, YourMatch,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{
    goal_difference, ranked, require_game, require_league,
};
use std::sync::{Arc, Mutex};

// ─── time_advance ───────────────────────────────────────────────────────────

pub fn time_advance(ctx: Arc<McpContext>) -> Result<DayAdvanced, String> {
    // Rate limiting: enforce minimum delay between advances
    if ctx.config.min_tick_delay_ms > 0 {
        // Simple approach: sleep for the configured delay
        // A more sophisticated approach would track last_advance timestamp
        std::thread::sleep(std::time::Duration::from_millis(
            ctx.config.min_tick_delay_ms,
        ));
    }

    // Use the delegate mode to force auto-simulation of matches
    let response = crate::application::time_advancement::advance_time_with_mode(
        &ctx.state_manager,
        "delegate",
    )?;

    let game = response.game.as_ref();
    let round_summary = response.round_summary.as_ref();
    let user_team_id = game.and_then(|game| game.manager.team_id.as_deref());

    let results: Vec<PlayedMatch> = round_summary
        .map(|summary| {
            summary
                .completed_results
                .iter()
                .map(|result| PlayedMatch {
                    home_team: result.home_team_name.clone(),
                    home_goals: result.home_goals,
                    away_goals: result.away_goals,
                    away_team: result.away_team_name.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    let day = DayAdvanced {
        date: game.map_or_else(
            || "Unknown".to_string(),
            |game| game.clock.current_date.format("%d %B %Y").to_string(),
        ),
        your_match: round_summary
            .zip(user_team_id)
            .and_then(|(summary, team_id)| your_match_in(&summary.completed_results, team_id)),
        standings: round_summary
            .zip(game)
            .and_then(|(_, game)| standings_update(game)),
        fired: game.is_some_and(|game| game.manager.team_id.is_none()),
        auto_saved: false,
        results,
    };

    let auto_saved = response.game.is_some() && auto_save_if_due(&ctx);

    // Notify GUI about state change
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(DayAdvanced { auto_saved, ..day })
}

fn your_match_in(
    results: &[ofm_core::turn::RoundResultSummary],
    team_id: &str,
) -> Option<YourMatch> {
    results
        .iter()
        .find(|result| result.home_team_id == team_id || result.away_team_id == team_id)
        .map(|result| {
            let at_home = result.home_team_id == team_id;
            let (your_goals, their_goals, opponent) = if at_home {
                (result.home_goals, result.away_goals, &result.away_team_name)
            } else {
                (result.away_goals, result.home_goals, &result.home_team_name)
            };
            YourMatch {
                outcome: match your_goals.cmp(&their_goals) {
                    std::cmp::Ordering::Greater => Outcome::Won,
                    std::cmp::Ordering::Less => Outcome::Lost,
                    std::cmp::Ordering::Equal => Outcome::Drew,
                },
                your_goals,
                their_goals,
                opponent: opponent.clone(),
                at_home,
            }
        })
}

fn standings_update(game: &ofm_core::game::Game) -> Option<StandingsUpdate> {
    let league = game.league.as_ref()?;
    let team_id = game.manager.team_id.as_deref()?;
    let standings = ranked(league);
    let position = standings.iter().position(|s| s.team_id == team_id)?;
    let standing = &standings[position];
    Some(StandingsUpdate {
        position: position + 1,
        points: standing.points,
        goal_difference: goal_difference(standing),
    })
}

/// Saves the game every `auto_save_interval_days` in-game days, counted per save.
/// Returns whether it saved.
fn auto_save_if_due(ctx: &McpContext) -> bool {
    if ctx.config.auto_save_interval_days == 0 {
        return false;
    }
    let Some(save_id) = ctx.state_manager.get_save_id() else {
        return false;
    };
    use std::collections::HashMap;
    use std::sync::LazyLock;
    static SAVE_DAY_COUNTERS: LazyLock<Mutex<HashMap<String, u32>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let Ok(mut counters) = SAVE_DAY_COUNTERS.lock() else {
        return false;
    };
    let days = counters.entry(save_id).or_insert(0);
    *days += 1;
    if *days < ctx.config.auto_save_interval_days {
        return false;
    }
    *days = 0;
    drop(counters); // release lock before save
    ctx.save_manager_state.0.lock().is_ok_and(|mut sm| {
        crate::commands::util::persist_active_game(&ctx.state_manager, &mut sm).is_ok()
    })
}

// ─── squad_get ──────────────────────────────────────────────────────────────

// ─── time_skip_to_match_day ─────────────────────────────────────────────────

pub fn time_skip_to_match_day(ctx: Arc<McpContext>) -> Result<SkipToMatchDay, String> {
    crate::application::live_session::ensure_idle(&ctx.state_manager)?;
    let game = require_game(&ctx.state_manager)?;
    let league = require_league(&game)?;
    let team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or("be.error.noTeamAssigned")?;

    // Find next fixture for user's team
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let next_fixture = league
        .fixtures
        .iter()
        .filter(|f| f.status != domain::league::FixtureStatus::Completed)
        .filter(|f| f.home_team_id == team_id || f.away_team_id == team_id)
        .filter(|f| f.date > today)
        .min_by_key(|f| &f.date);

    let Some(fixture) = next_fixture else {
        return Ok(SkipToMatchDay::NoUpcomingMatch {});
    };

    let target_date = fixture.date.clone();
    let days_to_skip = {
        let current = game.clock.current_date.date_naive();
        let target = chrono::NaiveDate::parse_from_str(&target_date, "%Y-%m-%d")
            .map_err(|e| format!("Date parse error: {}", e))?;
        (target - current).num_days()
    };

    if days_to_skip <= 0 {
        return Ok(SkipToMatchDay::MatchDayToday {});
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
        )?;

        advanced += 1;

        // Safety limit
        if advanced > 365 {
            return Ok(SkipToMatchDay::Aborted {
                days_advanced: advanced,
            });
        }
    }

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(SkipToMatchDay::Skipped {
        days_advanced: advanced,
        target_date,
    })
}

// ─── time_check_blockers ────────────────────────────────────────────────────

// ─── time_check_blockers ────────────────────────────────────────────────────

pub fn time_check_blockers(ctx: Arc<McpContext>) -> Result<Blockers, String> {
    let game = require_game(&ctx.state_manager)?;

    let mut blockers = Vec::new();

    // Check for live match in progress
    if ctx.state_manager.with_live_match(|_| true).unwrap_or(false) {
        blockers.push("Live match in progress — finish the match first".to_string());
    }

    // Check for pending transfer offers requiring response
    let team_id = game.manager.team_id.as_deref();
    if let Some(tid) = team_id {
        let pending_offers: Vec<_> = game
            .players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some(tid))
            .flat_map(|p| p.transfer_offers.iter())
            .filter(|o| o.status == domain::player::TransferOfferStatus::Pending)
            .collect();

        if !pending_offers.is_empty() {
            blockers.push(format!(
                "{} pending transfer offer(s) need response",
                pending_offers.len()
            ));
        }
    }

    // Check for contract renewal deadlines
    if let Some(_tid) = team_id {
        // Note: exit_intent is nested in player.morale_core.renewal_state.exit_intent
        // Skip this check for simplicity — agents can use info_player_profile to check
    }

    Ok(Blockers { blockers })
}

// ─── transfer_market_browse ─────────────────────────────────────────────────
