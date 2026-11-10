//! MCP tool implementations: time

use mcp_results::time::{
    Blocker, Blockers, DayAdvanced, Outcome, PlayedMatch, SkipToMatchDay, StandingsUpdate,
    YourMatch,
};

use crate::application::time_blockers::stopping_blockers;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::require_game;
use crate::SaveManagerState;
use ofm_core::game::Game;
use ofm_core::state::StateManager;
use std::sync::{Arc, Mutex};

// ─── time_advance ───────────────────────────────────────────────────────────

pub fn time_advance(ctx: Arc<McpContext>) -> Result<DayAdvanced, String> {
    advance_gate(&require_game(&ctx.state_manager)?)?;

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

/// Time does not advance while a blocker stands: the screen's Continue stops on exactly this list,
/// and an agent resolves the squad, contract or other problem first, as a manager would.
pub(crate) fn advance_gate(game: &Game) -> Result<(), String> {
    let blockers = blockers_of(game, false).blockers;
    if blockers.is_empty() {
        return Ok(());
    }
    let list: Vec<String> = blockers.iter().map(|b| format!("- {b}")).collect();
    Err(format!(
        "Time cannot advance until these are resolved:\n{}\nUse time_check_blockers to see them again.",
        list.join("\n")
    ))
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
    let outcome = skip_to_match_day_in(&ctx.state_manager)?;
    if skip_moved_the_game(&outcome) {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }
    Ok(outcome)
}

/// Whether the skip moved the game on, so the screens must be told: any day it advanced, a
/// sacking and running out of days included. Standing still on a match day, or on a blocker that
/// stood at the start, changes nothing.
fn skip_moved_the_game(outcome: &SkipToMatchDay) -> bool {
    match outcome {
        SkipToMatchDay::MatchDayToday {} => false,
        SkipToMatchDay::Aborted { days_advanced }
        | SkipToMatchDay::Fired { days_advanced }
        | SkipToMatchDay::Skipped { days_advanced, .. }
        | SkipToMatchDay::Blocked { days_advanced, .. } => *days_advanced > 0,
    }
}

/// Skips to the next match day through the screen's own skip (`commands::time::skip_to_match_day_internal`),
/// so an agent stops where a player stops: on any day the user's club has a match scheduled, in
/// any competition, before it is played; after any day that brings a blocker; or when he is
/// sacked. The tool had its own loop that aimed at the next league fixture in the `game.league`
/// mirror and advanced in delegate mode, so a cup tie on the way, or the user's own match when
/// called on a match day, was simulated without him (#1193). Only the check that a blocker
/// already stands is its own: the screen makes it before offering the skip.
pub(crate) fn skip_to_match_day_in(state: &StateManager) -> Result<SkipToMatchDay, String> {
    crate::application::live_session::ensure_idle(state)?;
    let game = require_game(state)?;
    let blockers = blockers_of(&game, false).blockers;
    if !blockers.is_empty() {
        return Ok(SkipToMatchDay::Blocked {
            days_advanced: 0,
            blockers,
        });
    }

    let response =
        crate::commands::time::skip_to_match_day_internal(state, ofm_core::state::Career::Active)?;
    Ok(skip_outcome(&response))
}

/// The screen's skip `response`, as the agent's result. Read from the response alone, never from
/// the game afterwards: the skip has released the game by then, and a write in between could
/// change the date or the blockers. A sacking and blockers as the skip reported them; otherwise it
/// arrived, on a match day (none passed: match day today; some: skipped) or out of days, the skip's
/// 60-day limit, with none found (aborted).
fn skip_outcome(response: &serde_json::Value) -> SkipToMatchDay {
    let days_advanced = response["days_skipped"]
        .as_u64()
        .and_then(|days| u32::try_from(days).ok())
        .unwrap_or_default();
    match response["action"].as_str() {
        Some("fired") => SkipToMatchDay::Fired { days_advanced },
        Some("blocked") => SkipToMatchDay::Blocked {
            days_advanced,
            blockers: response["blockers"]
                .as_array()
                .map(|blockers| blockers.iter().map(blocker_of).collect())
                .unwrap_or_default(),
        },
        _ if response["reached_match_day"].as_bool() != Some(true) => {
            SkipToMatchDay::Aborted { days_advanced }
        }
        _ if days_advanced == 0 => SkipToMatchDay::MatchDayToday {},
        _ => SkipToMatchDay::Skipped {
            days_advanced,
            target_date: response["date"].as_str().unwrap_or_default().to_string(),
        },
    }
}

// ─── time_check_blockers ────────────────────────────────────────────────────

pub fn time_check_blockers(ctx: Arc<McpContext>) -> Result<Blockers, String> {
    let game = require_game(&ctx.state_manager)?;
    let live_match_in_progress = ctx.state_manager.with_live_match(|_| true).unwrap_or(false);
    Ok(blockers_of(&game, live_match_in_progress))
}

/// The blockers the UI's own check reports (`application::time_blockers::stopping_blockers`), so
/// an agent and a player are stopped by the same list with the same severities.
pub(crate) fn blockers_of(game: &Game, live_match_in_progress: bool) -> Blockers {
    Blockers {
        blockers: stopping_blockers(game).iter().map(blocker_of).collect(),
        live_match_in_progress,
    }
}

/// One blocker as the screen's check words it (`application::time_blockers`), as an agent reads it.
fn blocker_of(blocker: &serde_json::Value) -> Blocker {
    let text = |field: &str| {
        blocker
            .get(field)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    Blocker {
        id: text("id"),
        severity: text("severity"),
        tab: text("tab"),
        params: blocker.get("text_params").filter(|p| !p.is_null()).cloned(),
    }
}

// ─── transfer_market_browse ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{auto_save_if_due, skip_moved_the_game, skip_outcome, standings_update};
    use crate::commands::wrapper_testkit::WrapperApp;
    use chrono::TimeZone;
    use domain::league::{League, StandingEntry};
    use domain::manager::Manager;
    use mcp_results::time::SkipToMatchDay;
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

    /// Given the screen's skip answering that it arrived after 60 days without reaching a match
    /// When the agent's result is read from it
    /// Then it is aborted after 60 days: the skip ran out of days
    #[test]
    fn a_skip_that_ran_out_of_days_is_reported_as_aborted() {
        let response = serde_json::json!({
            "action": "arrived",
            "reached_match_day": false,
            "date": "2030-10-31",
            "days_skipped": 60,
        });

        assert_eq!(
            skip_outcome(&response),
            SkipToMatchDay::Aborted { days_advanced: 60 }
        );
    }

    /// Given the screen's skip answering that the board sacked the manager on its fourth day
    /// When the agent's result is read from it
    /// Then it is fired after 4 days, not aborted or skipped
    #[test]
    fn a_skip_that_ended_in_a_sacking_is_reported_as_fired() {
        let response = serde_json::json!({ "action": "fired", "days_skipped": 4 });

        assert_eq!(
            skip_outcome(&response),
            SkipToMatchDay::Fired { days_advanced: 4 }
        );
    }

    /// Given the screen's skip answering that it reached the user's match day after 3 days, on
    ///      2030-09-04
    /// When the agent's result is read from it
    /// Then it is skipped 3 days to 2030-09-04: the day the skip stopped on, as it reported it
    #[test]
    fn a_skip_that_reached_its_match_day_reports_the_day_it_stopped_on() {
        let response = serde_json::json!({
            "action": "arrived",
            "reached_match_day": true,
            "date": "2030-09-04",
            "days_skipped": 3,
        });

        assert_eq!(
            skip_outcome(&response),
            SkipToMatchDay::Skipped {
                days_advanced: 3,
                target_date: "2030-09-04".to_string(),
            }
        );
    }

    /// Given the screen's skip answering that it was blocked after 2 days by an injured starter
    /// When the agent's result is read from it
    /// Then the agent is told of that blocker, the one the skip found, not whatever the game holds
    ///      by the time the result is read
    #[test]
    fn a_blocked_skip_reports_the_blockers_the_skip_found() {
        let response = serde_json::json!({
            "action": "blocked",
            "days_skipped": 2,
            "blockers": [{
                "id": "injured_xi",
                "severity": "warn",
                "tab": "Squad",
                "text_params": { "count": "1" },
            }],
        });

        let SkipToMatchDay::Blocked {
            days_advanced,
            blockers,
        } = skip_outcome(&response)
        else {
            panic!("a blocked skip is reported as blocked");
        };
        assert_eq!(days_advanced, 2);
        let ids: Vec<&str> = blockers.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, vec!["injured_xi"]);
        assert_eq!(blockers[0].tab, "Squad");
        assert_eq!(blockers[0].severity, "warn");
        assert_eq!(
            blockers[0].params,
            Some(serde_json::json!({ "count": "1" }))
        );
    }

    /// Given each way a skip can end
    /// When the screens are to be told the game changed
    /// Then they are whenever a day passed (a sacking, running out of days, a skip, a blocker after
    ///      a day) and not when the skip stood still on a match day or a blocker already standing
    #[test]
    fn the_screens_are_told_whenever_the_skip_moved_the_game() {
        assert!(skip_moved_the_game(&SkipToMatchDay::Fired {
            days_advanced: 1
        }));
        assert!(skip_moved_the_game(&SkipToMatchDay::Aborted {
            days_advanced: 60
        }));
        assert!(skip_moved_the_game(&SkipToMatchDay::Skipped {
            days_advanced: 3,
            target_date: "2030-09-04".to_string(),
        }));
        assert!(skip_moved_the_game(&SkipToMatchDay::Blocked {
            days_advanced: 2,
            blockers: vec![],
        }));
        assert!(!skip_moved_the_game(&SkipToMatchDay::MatchDayToday {}));
        assert!(!skip_moved_the_game(&SkipToMatchDay::Blocked {
            days_advanced: 0,
            blockers: vec![],
        }));
    }
}
