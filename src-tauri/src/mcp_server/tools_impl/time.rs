//! MCP tool implementations: time

use mcp_results::time::{
    Blockers, DayAdvanced, Outcome, PlayedMatch, SkipToMatchDay, StandingsUpdate, YourMatch,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game, require_league};
use crate::SaveManagerState;
use ofm_core::state::StateManager;
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
        ofm_core::state::Career::Active,
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

    let auto_saved = response.game.is_some()
        && auto_save_if_due(
            &ctx.state_manager,
            &ctx.save_manager_state,
            ctx.config.auto_save_interval_days,
        );

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
    let standings = league.sorted_standings();
    let position = standings.iter().position(|s| s.team_id == team_id)?;
    let standing = &standings[position];
    Some(StandingsUpdate {
        position: position + 1,
        points: standing.points,
        goal_difference: i64::from(standing.goal_difference()),
    })
}

/// Saves the game every `interval_days` in-game days, counted per save; 0 turns it off.
/// Returns whether it saved.
fn auto_save_if_due(state: &StateManager, saves: &SaveManagerState, interval_days: u32) -> bool {
    if interval_days == 0 {
        return false;
    }
    let Some(save_id) = state.get_save_id() else {
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
    if *days < interval_days {
        return false;
    }
    *days = 0;
    drop(counters); // release lock before save
    saves.0.lock().is_ok_and(|mut sm| {
        crate::commands::util::persist_active_game(state, &mut sm, ofm_core::state::Career::Active)
            .is_ok()
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
            ofm_core::state::Career::Active,
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

#[cfg(test)]
mod tests {
    use super::{auto_save_if_due, standings_update};
    use crate::commands::wrapper_testkit::WrapperApp;
    use chrono::TimeZone;
    use domain::league::{League, StandingEntry};
    use domain::manager::Manager;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;

    fn entry(team_id: &str, goals_for: u32, goals_against: u32) -> StandingEntry {
        StandingEntry {
            team_id: team_id.to_string(),
            played: 3,
            won: 1,
            drawn: 0,
            lost: 2,
            goals_for,
            goals_against,
            points: 3,
        }
    }

    /// Given two clubs level on points, one with more goals scored and the other the better goal difference
    /// When the standings update for the better goal difference's club is built
    /// Then it is first, because the game ranks by points, goal difference, then goals scored.
    #[test]
    fn a_tie_on_points_is_split_by_goal_difference_not_goals_scored() {
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("tidy".to_string());
        let clock = GameClock::new(chrono::Utc.with_ymd_and_hms(2030, 9, 1, 12, 0, 0).unwrap());
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        let mut league = League::new(
            "league".to_string(),
            "League".to_string(),
            2030,
            &["open".to_string(), "tidy".to_string()],
        );
        league.standings = vec![entry("open", 8, 8), entry("tidy", 3, 1)];
        game.league = Some(league);

        let update = standings_update(&game).expect("the manager's club is in the table");

        assert_eq!(update.position, 1);
        assert_eq!(update.goal_difference, 2);
    }

    fn game_to_autosave() -> Game {
        let manager = Manager::new(
            "mgr".to_string(),
            "Original".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let clock = GameClock::new(chrono::Utc.with_ymd_and_hms(2030, 9, 1, 12, 0, 0).unwrap());
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    /// Changes the live game after its save was made, so a save shows up as a different name on disk.
    fn rename_in_memory(app: &WrapperApp) {
        app.state()
            .update_game(|game| game.manager.first_name = "Renamed".to_string())
            .expect("game");
    }

    /// Given an interval of 0 (auto-save off) and a changed game
    /// When a day passes
    /// Then it reports no save and the file on disk is unchanged.
    #[test]
    fn an_interval_of_zero_never_saves() {
        let app = WrapperApp::new(game_to_autosave(), true);
        rename_in_memory(&app);

        let saved = auto_save_if_due(&app.state(), &app.saves(), 0);

        assert!(!saved);
        assert_eq!(app.saved_game().manager.first_name, "Original");
    }

    /// Given a game that has no save behind it
    /// When a day passes with auto-save on
    /// Then it reports no save, rather than claiming one it cannot make.
    #[test]
    fn a_game_without_a_save_is_not_auto_saved() {
        let app = WrapperApp::new(game_to_autosave(), false);

        assert!(!auto_save_if_due(&app.state(), &app.saves(), 1));
    }

    /// Given an interval of 1 and a changed game
    /// When a day passes
    /// Then it reports a save and the file on disk holds the change.
    #[test]
    fn an_interval_of_one_saves_every_day() {
        let app = WrapperApp::new(game_to_autosave(), true);
        rename_in_memory(&app);

        let saved = auto_save_if_due(&app.state(), &app.saves(), 1);

        assert!(saved);
        assert_eq!(app.saved_game().manager.first_name, "Renamed");
    }

    /// Given an interval of 2 and a changed game
    /// When two days pass
    /// Then the first day saves nothing and the second saves.
    #[test]
    fn an_interval_of_two_saves_on_the_second_day_only() {
        let app = WrapperApp::new(game_to_autosave(), true);
        rename_in_memory(&app);

        let first = auto_save_if_due(&app.state(), &app.saves(), 2);
        let on_disk_after_first = app.saved_game().manager.first_name;
        let second = auto_save_if_due(&app.state(), &app.saves(), 2);

        assert!(!first);
        assert_eq!(on_disk_after_first, "Original");
        assert!(second);
        assert_eq!(app.saved_game().manager.first_name, "Renamed");
    }
}
