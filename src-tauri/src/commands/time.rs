use log::info;
use std::sync::Arc;
use tauri::State;

use crate::application::auto_save;
use crate::application::time_advancement::advance_time_with_mode as advance_time_with_mode_service;
pub use crate::application::time_advancement::AdvanceTimeWithModeResponse;
use crate::application::time_blockers::stopping_blockers;
use ofm_core::advance_results::collect_advance_results;
use ofm_core::game::Game;
use ofm_core::state::{Career, StateManager};

pub fn advance_time_internal(state: &StateManager, career: Career) -> Result<Game, String> {
    let _operation = crate::application::live_session::idle_operation(state)?;
    // Process the day in place (no upfront clone of the whole world) and clone
    // only once for the response. Captures are appended after the game lock is
    // released to avoid holding two state locks at once.
    let mut captures = Vec::new();
    let advanced = state.update_game_for(career, |game| {
        info!(
            "[cmd] advance_time: date={}",
            game.clock.current_date.format("%Y-%m-%d")
        );
        ofm_core::turn::process_day_with_capture(game, &mut |capture| {
            captures.push(capture);
        });
        game.clone()
    })?;

    for capture in captures {
        state.append_stats_state(capture);
    }

    Ok(advanced)
}

pub fn advance_time_with_mode_internal(
    state: &StateManager,
    career: Career,
    mode: &str,
) -> Result<AdvanceTimeWithModeResponse, String> {
    advance_time_with_mode_service(state, career, mode)
}

/// Advance time with a specific match mode.
/// mode: "live" | "spectator" | "delegate" | "instant"
/// If mode is "live" or "spectator" and there's a user match today,
/// it sets up the live match session instead of auto-simulating.
///
/// Runs on a blocking worker so the day's simulation never freezes the webview.
#[tauri::command]
pub async fn advance_time_with_mode<R: tauri::Runtime>(
    state: State<'_, Arc<StateManager>>,
    saves: State<'_, Arc<crate::SaveManagerState>>,
    app_handle: tauri::AppHandle<R>,
    mode: String,
    career_token: u64,
) -> Result<AdvanceTimeWithModeResponse, String> {
    let state = state.inner().clone();
    let saves = saves.inner().clone();
    run_off_thread(move || {
        let career = Career::Token(career_token);
        let before = auto_save::game_date(&state);
        let mut response = advance_time_with_mode_internal(&state, career, &mode)?;
        let enabled = || crate::commands::settings::auto_save_enabled(&app_handle);
        response.auto_save_error =
            auto_save::save_if_a_new_week(&state, &saves.0, enabled, before, career).error();
        Ok(response)
    })
    .await
}

#[tauri::command]
pub async fn advance_time(
    state: State<'_, Arc<StateManager>>,
    career_token: u64,
) -> Result<Game, String> {
    let state = state.inner().clone();
    run_off_thread(move || advance_time_internal(&state, Career::Token(career_token))).await
}

/// Run blocking simulation work on Tauri's blocking thread pool so the webview
/// (main) thread stays responsive while a turn is processed.
async fn run_off_thread<T, F>(work: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|err| {
            log::error!("Simulation worker failed: {err}");
            "be.error.taskJoinFailed".to_string()
        })?
}

#[tauri::command]
pub fn check_blocking_actions(
    state: State<'_, Arc<StateManager>>,
) -> Result<serde_json::Value, String> {
    log::debug!("[cmd] check_blocking_actions");
    let game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession")?;

    let blockers = stopping_blockers(&game);
    info!(
        "[cmd] check_blocking_actions: date={}, blocker_count={}",
        game.clock.current_date.format("%Y-%m-%d"),
        blockers.len()
    );
    Ok(serde_json::json!(blockers))
}

#[tauri::command]
pub async fn skip_to_match_day<R: tauri::Runtime>(
    state: State<'_, Arc<StateManager>>,
    saves: State<'_, Arc<crate::SaveManagerState>>,
    app_handle: tauri::AppHandle<R>,
    career_token: u64,
) -> Result<serde_json::Value, String> {
    let state = state.inner().clone();
    let saves = saves.inner().clone();
    run_off_thread(move || {
        let career = Career::Token(career_token);
        let before = auto_save::game_date(&state);
        let mut response = skip_to_match_day_internal(&state, career)?;
        let enabled = || crate::commands::settings::auto_save_enabled(&app_handle);
        auto_save::save_if_a_new_week(&state, &saves.0, enabled, before, career)
            .note_on(&mut response);
        Ok(response)
    })
    .await
}

pub fn skip_to_match_day_internal(
    state: &StateManager,
    career: Career,
) -> Result<serde_json::Value, String> {
    let _operation = crate::application::live_session::idle_operation(state)?;
    info!("[cmd] skip_to_match_day");
    // Precondition: manager must be employed at entry — guarantees that any later
    // `team_id.is_none()` inside the loop is a real firing transition, not a stale state.
    // Clock date before any processing — every match dated on/after this was
    // played during the skip, which is what the results recap shows.
    let (user_team_id, start_date) = state
        .get_game(|g| {
            (
                g.manager.team_id.clone(),
                g.clock.current_date.format("%Y-%m-%d").to_string(),
            )
        })
        .ok_or("be.error.noActiveGameSession")?;
    let user_team_id = user_team_id.ok_or("be.error.noTeamAssigned")?;
    info!(
        "[cmd] skip_to_match_day: start_date={}, user_team_id={}",
        start_date, user_team_id
    );

    // One day per update_game call: each day is processed and persisted
    // atomically under the game lock, so concurrent GUI/MCP writes between
    // days are seen by the next iteration instead of being clobbered by a
    // single stale whole-loop clone — while never holding the lock for the
    // entire (up to 60-day) skip.
    enum Step {
        /// Stop before processing: it's the user's match day.
        MatchDay,
        /// The skip ends here with this response (fired / blocked).
        Terminal(serde_json::Value),
        /// Quiet day processed; keep skipping.
        Advanced,
    }

    let mut days_skipped = 0u32;
    loop {
        if days_skipped >= 60 {
            break;
        }

        let mut captures = Vec::new();
        let step = state.update_game_for(career, |game| {
            let today = game.clock.current_date.format("%Y-%m-%d").to_string();

            // Stop on the user's match day so it can be played interactively
            // rather than auto-simulated. Detect it across the user's
            // competitions (the source of truth) — not the legacy `league`
            // mirror, which misses cups.
            if game.user_has_scheduled_match_on(&today) {
                info!(
                    "[cmd] skip_to_match_day: found match_day={}, days_skipped={}",
                    today, days_skipped
                );
                return Step::MatchDay;
            }

            ofm_core::turn::process_day_with_capture(game, &mut |capture| {
                captures.push(capture);
            });

            // Detect a firing that happened *during* this skip. Because the
            // function errors out above when the manager starts unemployed,
            // seeing `team_id.is_none()` here can only mean a real
            // employed → unemployed transition.
            if game.manager.team_id.is_none() {
                info!(
                    "[cmd] skip_to_match_day: manager fired after {} days",
                    days_skipped + 1
                );
                let results = collect_advance_results(game, &start_date);
                return Step::Terminal(serde_json::json!({
                    "action": "fired",
                    "game": game.clone(),
                    "days_skipped": days_skipped + 1,
                    "results": results
                }));
            }

            // After processing, check if blocking actions arose
            let blockers = stopping_blockers(game);
            if !blockers.is_empty() {
                info!(
                    "[cmd] skip_to_match_day: blocked_after_days={}, date={}, blocker_count={}",
                    days_skipped + 1,
                    game.clock.current_date.format("%Y-%m-%d"),
                    blockers.len()
                );
                let results = collect_advance_results(game, &start_date);
                return Step::Terminal(serde_json::json!({
                    "action": "blocked",
                    "game": game.clone(),
                    "blockers": blockers,
                    "days_skipped": days_skipped + 1,
                    "results": results
                }));
            }

            Step::Advanced
        })?;
        for capture in captures {
            state.append_stats_state(capture);
        }

        match step {
            Step::MatchDay => break,
            Step::Terminal(response) => return Ok(response),
            Step::Advanced => days_skipped += 1,
        }
    }

    let game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession")?;
    info!(
        "[cmd] skip_to_match_day: arrived_after_days={}, final_date={}",
        days_skipped,
        game.clock.current_date.format("%Y-%m-%d")
    );
    let results = collect_advance_results(&game, &start_date);
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();
    Ok(serde_json::json!({
        "action": "arrived",
        "reached_match_day": arrived_on_the_match_day(&game),
        "date": date,
        "game": game,
        "days_skipped": days_skipped,
        "results": results
    }))
}

/// Whether the skip ended on the user's match day, rather than out of days (the 60-day limit). Read
/// from the game the skip ends on, under its idle operation, never from the loop's own flag: the
/// 60th day can be the match day, and the loop stops on the limit before it checks it.
fn arrived_on_the_match_day(game: &Game) -> bool {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    game.user_has_scheduled_match_on(&today)
}

/// Advance exactly one day, surfacing what happened so the frontend can build a
/// per-day digest feed instead of a silent batch spinner.
///
/// Stop conditions are checked **before** advancing so the user can never
/// auto-simulate their own match or skip past an unresolved blocker:
///   - `match_day`  — user has a scheduled fixture today; game state unchanged
///   - `blocked`    — action-required blocker exists; game state unchanged
///   - `fired`      — manager was dismissed during today's processing
///   - `advanced`   — quiet day processed successfully; game state updated
pub fn advance_one_day_internal(
    state: &StateManager,
    career: Career,
) -> Result<serde_json::Value, String> {
    let _operation = crate::application::live_session::idle_operation(state)?;
    // The whole check-process-respond sequence runs under the game lock
    // (update_game) so a concurrent GUI/MCP write is never clobbered by a
    // stale clone written back afterwards.
    let mut captures = Vec::new();
    let response = state.update_game_for(career, |game| {
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        info!("[cmd] advance_one_day: date={}", today);

        if game.user_has_scheduled_match_on(&today) {
            info!("[cmd] advance_one_day: match_day={}", today);
            return serde_json::json!({
                "action": "match_day",
                "game": game.clone(),
                "date": today,
                "results": []
            });
        }

        let blockers = stopping_blockers(game);
        if !blockers.is_empty() {
            info!(
                "[cmd] advance_one_day: blocked date={} count={}",
                today,
                blockers.len()
            );
            return serde_json::json!({
                "action": "blocked",
                "game": game.clone(),
                "date": today,
                "blockers": blockers,
                "results": []
            });
        }

        // Unlike the multi-day handlers there is no employment guard at
        // entry — this is the unemployed manager's only way to advance
        // time — so "fired" must mean a transition during today's
        // processing, not a dismissal from an earlier day.
        let was_employed = game.manager.team_id.is_some();
        ofm_core::turn::process_day_with_capture(game, &mut |capture| {
            captures.push(capture);
        });

        if was_employed && game.manager.team_id.is_none() {
            info!("[cmd] advance_one_day: manager fired on date={}", today);
            let results = collect_advance_results(game, &today);
            return serde_json::json!({
                "action": "fired",
                "game": game.clone(),
                "date": today,
                "results": results
            });
        }

        let results = collect_advance_results(game, &today);
        info!(
            "[cmd] advance_one_day: advanced from={} to={}",
            today,
            game.clock.current_date.format("%Y-%m-%d")
        );
        serde_json::json!({
            "action": "advanced",
            "game": game.clone(),
            "date": today,
            "results": results
        })
    })?;
    for capture in captures {
        state.append_stats_state(capture);
    }
    Ok(response)
}

#[tauri::command]
pub async fn advance_one_day<R: tauri::Runtime>(
    state: State<'_, Arc<StateManager>>,
    saves: State<'_, Arc<crate::SaveManagerState>>,
    app_handle: tauri::AppHandle<R>,
    career_token: u64,
) -> Result<serde_json::Value, String> {
    let state = state.inner().clone();
    let saves = saves.inner().clone();
    run_off_thread(move || {
        let career = Career::Token(career_token);
        let before = auto_save::game_date(&state);
        let mut response = advance_one_day_internal(&state, career)?;
        let enabled = || crate::commands::settings::auto_save_enabled(&app_handle);
        auto_save::save_if_a_new_week(&state, &saves.0, enabled, before, career)
            .note_on(&mut response);
        Ok(response)
    })
    .await
}

fn messages_needing_attention(game: &Game) -> std::collections::HashSet<(String, bool)> {
    game.messages
        .iter()
        .filter(|message| message.priority.needs_attention())
        .map(|message| (message.id.clone(), message.read))
        .collect()
}

/// Whether the day just processed produced something the user should pause on:
/// a transfer deadline, the window opening, or a fresh High or Urgent inbox item.
/// (Match days and blockers are handled separately in the loop.)
fn continue_reached_attention_event(
    transfer_status: &domain::season::TransferWindowStatus,
    opens_on: Option<&str>,
    today: &str,
    attention_before: &std::collections::HashSet<(String, bool)>,
    attention_after: &std::collections::HashSet<(String, bool)>,
) -> bool {
    *transfer_status == domain::season::TransferWindowStatus::DeadlineDay
        || opens_on == Some(today)
        || attention_after.iter().any(|(id, read)| {
            !attention_before.contains(&(id.clone(), *read))
                && (!read || !attention_before.contains(&(id.clone(), false)))
        })
}

#[tauri::command]
pub async fn advance_to_next_event(
    state: State<'_, Arc<StateManager>>,
    career_token: u64,
) -> Result<serde_json::Value, String> {
    let state = state.inner().clone();
    run_off_thread(move || advance_to_next_event_internal(&state, Career::Token(career_token)))
        .await
}

/// Roll the clock forward day by day until the next thing the user should see:
/// their next match, an action-required blocker, a transfer deadline / window
/// opening, or a new high-priority inbox item (capped at 60 days). Mirrors
/// `skip_to_match_day` but with the broader stop conditions of an opt-in
/// "smart Continue".
pub fn advance_to_next_event_internal(
    state: &StateManager,
    career: Career,
) -> Result<serde_json::Value, String> {
    let _operation = crate::application::live_session::idle_operation(state)?;
    info!("[cmd] advance_to_next_event");
    // Require an employed manager at entry so a later `team_id.is_none()` in the
    // loop is a genuine firing transition.
    let (user_team_id, start_date) = state
        .get_game(|g| {
            (
                g.manager.team_id.clone(),
                g.clock.current_date.format("%Y-%m-%d").to_string(),
            )
        })
        .ok_or("be.error.noActiveGameSession")?;
    let _user_team_id = user_team_id.ok_or("be.error.noTeamAssigned")?;

    // Per-day atomic steps under the game lock — see skip_to_match_day for why.
    enum Step {
        /// Stop before processing: it's the user's match day.
        MatchDay,
        /// The skip ends here with this response (fired / blocked).
        Terminal(serde_json::Value),
        /// Day processed and it produced an attention event: stop after it.
        AdvancedThenStop,
        /// Quiet day processed; keep going.
        Advanced,
    }

    let mut days_skipped = 0u32;
    loop {
        if days_skipped >= 60 {
            break;
        }

        let mut captures = Vec::new();
        let step = state.update_game_for(career, |game| {
            let today = game.clock.current_date.format("%Y-%m-%d").to_string();

            // Stop *on* the user's match day so it can be played interactively
            // rather than auto-simulated — detected across the user's competitions.
            if game.user_has_scheduled_match_on(&today) {
                return Step::MatchDay;
            }

            let attention_before = messages_needing_attention(game);

            ofm_core::turn::process_day_with_capture(game, &mut |capture| {
                captures.push(capture);
            });

            if game.manager.team_id.is_none() {
                let results = collect_advance_results(game, &start_date);
                return Step::Terminal(serde_json::json!({
                    "action": "fired",
                    "game": game.clone(),
                    "days_skipped": days_skipped + 1,
                    "results": results
                }));
            }

            let blockers = stopping_blockers(game);
            if !blockers.is_empty() {
                let results = collect_advance_results(game, &start_date);
                return Step::Terminal(serde_json::json!({
                    "action": "blocked",
                    "game": game.clone(),
                    "blockers": blockers,
                    "days_skipped": days_skipped + 1,
                    "results": results
                }));
            }

            let new_today = game.clock.current_date.format("%Y-%m-%d").to_string();
            if continue_reached_attention_event(
                &game.season_context.transfer_window.status,
                game.season_context.transfer_window.opens_on.as_deref(),
                &new_today,
                &attention_before,
                &messages_needing_attention(game),
            ) {
                return Step::AdvancedThenStop;
            }

            Step::Advanced
        })?;
        for capture in captures {
            state.append_stats_state(capture);
        }

        match step {
            Step::MatchDay => break,
            Step::Terminal(response) => return Ok(response),
            Step::AdvancedThenStop => {
                days_skipped += 1;
                break;
            }
            Step::Advanced => days_skipped += 1,
        }
    }

    let game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession")?;
    info!(
        "[cmd] advance_to_next_event: stopped_after_days={}, final_date={}",
        days_skipped,
        game.clock.current_date.format("%Y-%m-%d")
    );
    let results = collect_advance_results(&game, &start_date);
    Ok(serde_json::json!({
        "action": "arrived",
        "game": game,
        "days_skipped": days_skipped,
        "results": results
    }))
}

#[cfg(test)]
mod tests {
    use super::{advance_time_with_mode_internal, continue_reached_attention_event};
    use crate::application::time_blockers::compute_blocking_actions;
    use chrono::{TimeZone, Utc};
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus};
    use domain::manager::Manager;
    use domain::message::{InboxMessage, MessagePriority};
    use domain::player::{
        ContractExitIntent, ContractRenewalState, Injury, Player, PlayerAttributes, Position,
        RenewalSessionStatus,
    };
    use domain::season::TransferWindowStatus;
    use domain::stats::StatsState;
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::StateManager;
    use serde_json::Value;

    /// Given a simulation worker that panics, when it is joined, then the error is a translation key.
    #[test]
    fn panicking_simulation_returns_a_translation_key() {
        let result =
            tauri::async_runtime::block_on(super::run_off_thread(|| -> Result<(), String> {
                panic!("simulation failed")
            }));
        assert_eq!(result, Err("be.error.taskJoinFailed".to_string()));
        let english: Value =
            serde_json::from_str(include_str!("../../../src/i18n/locales/en.json"))
                .expect("English locale is valid JSON");
        assert!(english["be"]["error"]["taskJoinFailed"].is_string());
    }

    fn default_attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 60,
            shooting: 60,
            tackling: 60,
            dribbling: 60,
            defending: 60,
            positioning: 60,
            vision: 60,
            decisions: 60,
            composure: 60,
            aggression: 60,
            teamwork: 60,
            leadership: 60,
            handling: 60,
            reflexes: 60,
            aerial: 60,
        }
    }

    fn make_player(id: &str, name: &str, team_id: &str, position: Position) -> Player {
        let mut player = Player::new(
            id.to_string(),
            name.to_string(),
            name.to_string(),
            "2000-01-01".to_string(),
            "England".to_string(),
            position,
            default_attrs(),
        );
        player.team_id = Some(team_id.to_string());
        player
    }

    fn make_game(roster_size: usize) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());

        let players: Vec<Player> = (1..=roster_size)
            .map(|idx| {
                let position = if idx == 1 {
                    Position::Goalkeeper
                } else if idx <= 5 {
                    Position::Defender
                } else if idx <= 9 {
                    Position::Midfielder
                } else {
                    Position::Forward
                };

                make_player(
                    &format!("p{}", idx),
                    &format!("Player {}", idx),
                    "team1",
                    position,
                )
            })
            .collect();

        let mut team = Team::new(
            "team1".to_string(),
            "Test FC".to_string(),
            "TST".to_string(),
            "England".to_string(),
            "Testville".to_string(),
            "Test Ground".to_string(),
            20_000,
        );
        team.starting_xi_ids = players
            .iter()
            .take(11)
            .map(|player| player.id.clone())
            .collect();

        Game::new(clock, manager, vec![team], players, vec![], vec![])
    }

    fn make_game_with_matchday() -> Game {
        let mut game = make_game(22);
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        let mut opponent_team = Team::new(
            "team2".to_string(),
            "Rival FC".to_string(),
            "RIV".to_string(),
            "England".to_string(),
            "Rivaltown".to_string(),
            "Rival Ground".to_string(),
            21_000,
        );
        opponent_team.starting_xi_ids = game
            .players
            .iter()
            .skip(11)
            .take(11)
            .map(|p| p.id.clone())
            .collect();
        game.teams.push(opponent_team);

        for player in game.players.iter_mut().skip(11) {
            player.team_id = Some("team2".to_string());
        }

        game.teams[0].starting_xi_ids =
            game.players.iter().take(11).map(|p| p.id.clone()).collect();
        game.league = Some(domain::league::League {
            id: "league-1".to_string(),
            name: "League".to_string(),
            season: 2025,
            fixtures: vec![Fixture {
                postponement: None,
                id: "fixture-1".to_string(),
                competition_id: "league-1".to_string(),
                matchday: 1,
                date: today,
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Scheduled,
                result: None,
            }],
            standings: vec![
                domain::league::StandingEntry::new("team1".to_string()),
                domain::league::StandingEntry::new("team2".to_string()),
            ],
            transfer_log: vec![],
            transfer_rumours: vec![],
            ..domain::league::League::default()
        });
        game
    }

    /// Like `make_game_with_matchday` but stores the fixture in `game.competitions`
    /// (the source of truth for `user_has_scheduled_match_on`).
    fn make_game_with_competition_matchday() -> Game {
        let mut game = make_game(22);
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        let mut opponent_team = Team::new(
            "team2".to_string(),
            "Rival FC".to_string(),
            "RIV".to_string(),
            "England".to_string(),
            "Rivaltown".to_string(),
            "Rival Ground".to_string(),
            21_000,
        );
        opponent_team.starting_xi_ids = game
            .players
            .iter()
            .skip(11)
            .take(11)
            .map(|p| p.id.clone())
            .collect();
        game.teams.push(opponent_team);
        for player in game.players.iter_mut().skip(11) {
            player.team_id = Some("team2".to_string());
        }
        game.teams[0].starting_xi_ids =
            game.players.iter().take(11).map(|p| p.id.clone()).collect();

        game.competitions = vec![domain::league::League {
            id: "comp-1".to_string(),
            name: "League".to_string(),
            season: 2025,
            fixtures: vec![Fixture {
                postponement: None,
                id: "fx-1".to_string(),
                competition_id: "comp-1".to_string(),
                matchday: 1,
                date: today,
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Scheduled,
                result: None,
            }],
            standings: vec![
                domain::league::StandingEntry::new("team1".to_string()),
                domain::league::StandingEntry::new("team2".to_string()),
            ],
            ..domain::league::League::default()
        }];
        game.league = None;
        game
    }

    #[test]
    fn user_match_day_is_detected_from_competitions_not_legacy_league() {
        let mut game = make_game(2);
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        // The user's fixture lives in a competition; the legacy mirror is empty,
        // so the old game.league check would have missed this match day and the
        // skip/continue loops would have auto-simulated the user's match.
        game.league = None;
        game.competitions = vec![domain::league::League {
            id: "comp-1".to_string(),
            name: "League".to_string(),
            season: 2025,
            fixtures: vec![Fixture {
                postponement: None,
                id: "fx-1".to_string(),
                competition_id: "comp-1".to_string(),
                matchday: 2,
                date: today.clone(),
                home_team_id: "team1".to_string(),
                away_team_id: "team2".to_string(),
                competition: FixtureCompetition::League,
                status: FixtureStatus::Scheduled,
                result: None,
            }],
            ..domain::league::League::default()
        }];

        assert!(
            game.user_has_scheduled_match_on(&today),
            "a match day held only in a competition must be detected"
        );
        assert!(!game.user_has_scheduled_match_on("2099-01-01"));
    }

    fn make_message(id: &str, priority: MessagePriority, read: bool) -> InboxMessage {
        let mut message = InboxMessage::new(
            id.to_string(),
            "Subject".to_string(),
            "Body".to_string(),
            "Board".to_string(),
            "2025-06-15".to_string(),
        )
        .with_priority(priority);
        message.read = read;
        message
    }

    /// Given a read transfer digest, when a second club's offer replaces it,
    /// then Continue stops for the reopened digest even though its ID is unchanged.
    #[test]
    fn continue_stops_when_a_transfer_digest_is_reopened_by_another_offer() {
        let mut game = make_game(11);
        let digest = ofm_core::messages::transfer_interest_digest_message;
        let mut first = digest("player", "Player", 1, "First club", 100, "2025-06-15");
        first.read = true;
        game.messages = vec![first];
        let before = super::messages_needing_attention(&game);
        game.messages[0] = digest("player", "Player", 2, "Second club", 200, "2025-06-16");
        assert!(!game.messages[0].read);
        assert!(continue_reached_attention_event(
            &TransferWindowStatus::Closed,
            None,
            "2025-06-16",
            &before,
            &super::messages_needing_attention(&game),
        ));
    }

    /// Given an unread high-priority offer, when it is resolved and marked read,
    /// then Continue does not stop for that already-seen offer.
    #[test]
    fn continue_does_not_stop_when_an_unread_offer_is_marked_read() {
        let mut game = make_game(11);
        game.messages = vec![make_message("job-offer", MessagePriority::High, false)];
        let before = super::messages_needing_attention(&game);
        game.messages[0].read = true;
        assert!(!continue_reached_attention_event(
            &TransferWindowStatus::Closed,
            None,
            "2025-06-16",
            &before,
            &super::messages_needing_attention(&game),
        ));
    }

    /// Given old attention mail and an exhausted squad on the first, when Continue processes
    /// the day, then it stops for the new urgent warning even though old mail is archived.
    #[test]
    fn continue_stops_for_new_urgent_mail_when_old_attention_mail_is_archived() {
        let mut game = make_game(22);
        game.seed = 1;
        game.clock = GameClock::new(Utc.with_ymd_and_hms(2025, 10, 1, 12, 0, 0).unwrap());
        for player in game.players.iter_mut().skip(11) {
            player.condition = 1;
        }
        let mut high = make_message("old-high", MessagePriority::High, true);
        high.date = "2025-01-01".to_string();
        let mut urgent = make_message("old-urgent", MessagePriority::Urgent, true);
        urgent.date = "2025-01-01".to_string();
        game.messages = vec![high, urgent];
        for index in 0..50 {
            let mut old = make_message(&format!("old-{index}"), MessagePriority::High, true);
            old.date = "2025-01-01".to_string();
            game.messages.push(old);
        }
        let state = StateManager::new();
        state.set_game(game);
        state.set_stats_state(StatsState::default());

        let result =
            super::advance_to_next_event_internal(&state, ofm_core::state::Career::Active).unwrap();
        state
            .get_game(|game| {
                assert!(game.archived_messages.iter().any(|m| m.id == "old-high"));
                assert!(game.archived_messages.iter().any(|m| m.id == "old-urgent"));
                assert!(game
                    .messages
                    .iter()
                    .any(|m| m.id == "fitness_warn_2025-10-01"
                        && m.priority == MessagePriority::Urgent));
            })
            .unwrap();
        assert_eq!(result["days_skipped"], 1);
        assert_eq!(result["action"], "arrived", "{}", result["blockers"]);
    }

    fn blocker_by_id<'a>(blockers: &'a [Value], id: &str) -> Option<&'a Value> {
        blockers
            .iter()
            .find(|blocker| blocker.get("id").and_then(Value::as_str) == Some(id))
    }

    fn mark_player_let_expire(game: &mut Game, player_id: &str) {
        let player = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
            .unwrap();
        player.stage_contract_end(Some("2025-08-01".to_string()));
        player.morale_core.renewal_state = Some(ContractRenewalState {
            status: RenewalSessionStatus::Blocked,
            manager_blocked_until: None,
            last_attempt_date: Some("2025-06-15".to_string()),
            last_assistant_attempt_date: None,
            last_outcome: None,
            conversation_round: 0,
            exit_intent: Some(ContractExitIntent::LetExpire {
                set_on: "2025-06-15".to_string(),
                reason: Some("test".to_string()),
            }),
        });
    }

    #[test]
    fn continue_stops_on_deadline_open_day_or_new_high_priority_mail() {
        // Deadline day always stops.
        assert!(continue_reached_attention_event(
            &TransferWindowStatus::DeadlineDay,
            None,
            "2026-08-31",
            &Default::default(),
            &Default::default(),
        ));
        // The window's opening day stops.
        assert!(continue_reached_attention_event(
            &TransferWindowStatus::Open,
            Some("2026-07-01"),
            "2026-07-01",
            &Default::default(),
            &Default::default(),
        ));
        // A freshly arrived high-priority message stops.
        assert!(continue_reached_attention_event(
            &TransferWindowStatus::Closed,
            None,
            "2026-07-05",
            &[("old".to_string(), false)].into_iter().collect(),
            &[("old".to_string(), false), ("new".to_string(), false)]
                .into_iter()
                .collect(),
        ));
        // A quiet day mid-window with no new mail keeps rolling.
        assert!(!continue_reached_attention_event(
            &TransferWindowStatus::Open,
            Some("2026-07-01"),
            "2026-07-10",
            &[("old".to_string(), false)].into_iter().collect(),
            &[("old".to_string(), false)].into_iter().collect(),
        ));
    }

    /// Whether a continue stops after a quiet day on which one unread message of `priority`
    /// arrived, compared by ID before and after the day.
    fn continue_stops_for_new_mail_of(priority: MessagePriority) -> bool {
        let mut game = make_game(11);
        game.messages.clear();
        let before = super::messages_needing_attention(&game);
        game.messages
            .push(make_message("arrived-today", priority, false));
        continue_reached_attention_event(
            &TransferWindowStatus::Closed,
            None,
            "2025-06-16",
            &before,
            &super::messages_needing_attention(&game),
        )
    }

    /// Given a continue rolling through quiet days
    /// When an Urgent message arrives, as a High one would
    /// Then the continue stops after that day, as it does for High: Urgent is the more pressing
    ///      of the two, and it used to roll straight past it (maintainer ruling: High or Urgent).
    #[test]
    fn urgent_mail_stops_a_continue_as_high_mail_does() {
        assert!(continue_stops_for_new_mail_of(MessagePriority::High));
        assert!(continue_stops_for_new_mail_of(MessagePriority::Urgent));
    }

    /// Given the same
    /// When a Normal or a Low message arrives
    /// Then the continue keeps rolling.
    #[test]
    fn normal_and_low_mail_do_not_stop_a_continue() {
        assert!(!continue_stops_for_new_mail_of(MessagePriority::Normal));
        assert!(!continue_stops_for_new_mail_of(MessagePriority::Low));
    }

    #[test]
    fn advance_time_records_match_history_in_active_stats_state() {
        let state = StateManager::new();
        state.set_game(make_game_with_matchday());
        state.set_stats_state(StatsState::default());

        let advanced =
            super::advance_time_internal(&state, ofm_core::state::Career::Active).unwrap();
        let stats = state.get_stats_state(|current| current.clone()).unwrap();

        assert_eq!(
            advanced.clock.current_date.date_naive(),
            Utc.with_ymd_and_hms(2025, 6, 16, 12, 0, 0)
                .unwrap()
                .date_naive()
        );
        assert!(
            !stats.player_matches.is_empty(),
            "expected player match history to be recorded"
        );
        assert_eq!(stats.team_matches.len(), 2);
        assert_eq!(stats.player_matches[0].fixture_id, "fixture-1");
    }

    /// Given career B is active and a screen still holds career A's token
    /// When the screen's Continue arrives through the command
    /// Then it is refused with the career-changed key and the date has not moved.
    #[test]
    fn the_continue_command_for_a_replaced_career_is_refused_and_the_date_stays() {
        use crate::commands::wrapper_testkit::WrapperApp;
        let app = WrapperApp::new(make_game(11), false);
        let date = |app: &WrapperApp| app.state().get_game(|g| g.clock.current_date).unwrap();
        let before = date(&app);

        let refused = tauri::async_runtime::block_on(super::advance_one_day(
            app.state(),
            app.saves(),
            app.handle(),
            app.token + 1,
        ));

        assert_eq!(refused, Err("be.error.careerChanged".to_string()));
        assert_eq!(date(&app), before);
    }

    /// The saved career's date after the screen's Continue goes through `command` on a saved
    /// career that opens on Sunday 15 June 2025, with Auto-Save at its default (on).
    fn saved_date_after_a_continue(command: &str) -> chrono::NaiveDate {
        use crate::commands::wrapper_testkit::WrapperApp;
        let app = WrapperApp::new(make_game(11), true);
        let outcome = match command {
            "advance_one_day" => tauri::async_runtime::block_on(super::advance_one_day(
                app.state(),
                app.saves(),
                app.handle(),
                app.token,
            ))
            .map(|_| ()),
            _ => tauri::async_runtime::block_on(super::advance_time_with_mode(
                app.state(),
                app.saves(),
                app.handle(),
                "delegate".to_string(),
                app.token,
            ))
            .map(|_| ()),
        };
        assert_eq!(outcome, Ok(()), "{command}");
        app.saved_game().clock.current_date.date_naive()
    }

    /// Given a saved career on a Sunday with Auto-Save on
    /// When the screen's day-by-day Continue (advance_one_day) brings Monday
    /// Then the save on disk reads Monday: the command saved the week it crossed (#1181).
    #[test]
    fn the_day_by_day_continue_saves_the_week_it_crosses() {
        assert_eq!(
            saved_date_after_a_continue("advance_one_day"),
            chrono::NaiveDate::from_ymd_opt(2025, 6, 16).unwrap()
        );
    }

    /// Given the same
    /// When the screen's Continue goes through advance_time_with_mode
    /// Then the save on disk reads Monday too.
    #[test]
    fn the_mode_continue_saves_the_week_it_crosses() {
        assert_eq!(
            saved_date_after_a_continue("advance_time_with_mode"),
            chrono::NaiveDate::from_ymd_opt(2025, 6, 16).unwrap()
        );
    }

    /// Given a saved career on a Sunday whose save has since been deleted
    /// When the screen's day-by-day Continue brings Monday
    /// Then the day still advances, and the response carries the auto-save error the screen shows.
    #[test]
    fn the_day_by_day_continue_reports_a_week_it_could_not_save() {
        use crate::commands::wrapper_testkit::WrapperApp;
        let app = WrapperApp::new(make_game(11), true);
        let save_id = app.save_id.clone().expect("built with a save");
        app.saves()
            .0
            .lock()
            .expect("save manager")
            .delete_save(&save_id)
            .expect("delete the save");

        let response = tauri::async_runtime::block_on(super::advance_one_day(
            app.state(),
            app.saves(),
            app.handle(),
            app.token,
        ))
        .expect("the day advances");

        assert_eq!(response["auto_save_error"], "be.error.autoSave.failed");
        assert_eq!(
            app.state()
                .get_game(|game| game.clock.current_date.date_naive())
                .unwrap(),
            chrono::NaiveDate::from_ymd_opt(2025, 6, 16).unwrap()
        );
    }

    /// Given a saved career on Sunday 15 June 2025 with Auto-Save on
    /// When the screen's skip to the next match day (skip_to_match_day) moves the game on
    /// Then the save on disk reads the day the skip stopped on: any later day is a new week, and
    ///      this command saves it through its own call, not the other two commands' (#1181 review).
    #[test]
    fn the_skip_to_match_day_saves_the_week_it_crosses() {
        use crate::commands::wrapper_testkit::WrapperApp;
        let app = WrapperApp::new(make_game(11), true);

        tauri::async_runtime::block_on(super::skip_to_match_day(
            app.state(),
            app.saves(),
            app.handle(),
            app.token,
        ))
        .expect("the skip runs");

        let stopped_on = app
            .state()
            .get_game(|game| game.clock.current_date.date_naive())
            .unwrap();
        assert!(stopped_on > chrono::NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        assert_eq!(app.saved_game().clock.current_date.date_naive(), stopped_on);
    }

    /// Given a game with nobody injured, and one where two starters are injured
    /// When the user skips to their next match day, and when they continue to the next event
    /// Then the injured game stops after a day with the injured-XI blocker listed (days_skipped 1),
    ///      and whatever stops the healthy game over the days, it is never an informational blocker:
    ///      the unread-urgent-mail blocker does not stop time.
    #[test]
    fn skipping_and_continuing_stop_on_a_blocker_and_only_then() {
        use ofm_core::state::Career;
        let blocker_ids = |outcome: &Value| -> Vec<String> {
            outcome["blockers"]
                .as_array()
                .map(|blockers| {
                    blockers
                        .iter()
                        .map(|blocker| blocker["id"].as_str().unwrap().to_string())
                        .collect()
                })
                .unwrap_or_default()
        };
        let blocker_severities = |outcome: &Value| -> Vec<String> {
            outcome["blockers"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|blocker| blocker["severity"].as_str().unwrap().to_string())
                .collect()
        };
        let blocked_game = || {
            let mut game = make_game(11);
            for player_id in ["p2", "p5"] {
                let player = game.players.iter_mut().find(|p| p.id == player_id).unwrap();
                player.injury = Some(Injury {
                    name: "Hamstring".to_string(),
                    days_remaining: 7,
                });
            }
            game
        };
        let run = |game: Game, advance: fn(&StateManager, Career) -> Result<Value, String>| {
            let state = StateManager::new();
            state.set_game(game);
            advance(&state, Career::Active).expect("the advance answers")
        };

        for advance in [
            super::skip_to_match_day_internal as fn(&StateManager, Career) -> Result<Value, String>,
            super::advance_to_next_event_internal,
        ] {
            let blocked = run(blocked_game(), advance);
            assert_eq!(blocked["action"], "blocked", "{blocked}");
            assert_eq!(blocked["days_skipped"], 1, "{blocked}");
            assert!(
                blocker_ids(&blocked).contains(&"injured_xi".to_string()),
                "{blocked}"
            );

            // The healthy game meets real blockers within days (injuries, a thin XI); whatever
            // stops it, it is never the informational mail.
            let healthy = run(make_game(11), advance);
            assert!(
                blocker_severities(&healthy).iter().all(|s| s != "info"),
                "an informational blocker stopped time: {healthy}"
            );
        }
    }

    /// Given a healthy game whose only blocker is unread urgent mail, which is informational
    /// When the blocker check runs, the user continues one day, skips, or continues to the next event
    /// Then the check lists nothing, the single day is advanced, and neither multi-day advance is
    ///      ever stopped by the mail (a real blocker may stop them later, but never an "info" one).
    #[test]
    fn unread_urgent_mail_alone_never_stops_time() {
        use crate::commands::wrapper_testkit::WrapperApp;
        use ofm_core::state::Career;
        let mail_only = || {
            let mut game = make_game(11);
            game.messages = vec![make_message("urgent-1", MessagePriority::Urgent, false)];
            game
        };
        assert!(
            blocker_by_id(&compute_blocking_actions(&mail_only()), "urgent_messages").is_some(),
            "the fixture should have the informational blocker"
        );

        let app = WrapperApp::new(mail_only(), false);
        let listed = super::check_blocking_actions(app.state()).expect("the check answers");
        assert_eq!(listed, serde_json::json!([]), "{listed}");

        let state = StateManager::new();
        state.set_game(mail_only());
        let one_day = super::advance_one_day_internal(&state, Career::Active).expect("one day");
        assert_eq!(one_day["action"], "advanced", "{one_day}");

        for advance in [
            super::skip_to_match_day_internal as fn(&StateManager, Career) -> Result<Value, String>,
            super::advance_to_next_event_internal,
        ] {
            let state = StateManager::new();
            state.set_game(mail_only());
            let outcome = advance(&state, Career::Active).expect("the advance answers");
            for blocker in outcome["blockers"].as_array().into_iter().flatten() {
                assert_ne!(blocker["severity"], "info", "{outcome}");
            }
        }
    }

    /// Given a game with an injured pair of starters and unread urgent mail
    /// When the UI's blocker check and the MCP blocker tool read it
    /// Then both list the same blockers with the same severities and tabs (the injured XI, and no
    ///      informational mail), and the MCP flags a live match on its own.
    #[cfg(feature = "mcp")]
    #[test]
    fn the_mcp_blockers_are_the_ones_the_ui_gets() {
        use crate::commands::wrapper_testkit::WrapperApp;
        use crate::mcp_server::tools_impl::time::blockers_of;
        let mut game = make_game(11);
        for player_id in ["p2", "p5"] {
            let player = game.players.iter_mut().find(|p| p.id == player_id).unwrap();
            player.injury = Some(Injury {
                name: "Hamstring".to_string(),
                days_remaining: 7,
            });
        }
        game.messages = vec![make_message("urgent-1", MessagePriority::Urgent, false)];
        let app = WrapperApp::new(game.clone(), false);
        let ui = super::check_blocking_actions(app.state()).expect("the check answers");
        let ui: Vec<(String, String, String)> = ui
            .as_array()
            .unwrap()
            .iter()
            .map(|b| {
                let field = |name: &str| b[name].as_str().unwrap().to_string();
                (field("id"), field("severity"), field("tab"))
            })
            .collect();

        let mcp = blockers_of(&game, false);
        let mcp_list: Vec<(String, String, String)> = mcp
            .blockers
            .iter()
            .map(|b| (b.id.clone(), b.severity.clone(), b.tab.clone()))
            .collect();

        assert!(!ui.is_empty(), "the fixture should have a real blocker");
        assert!(ui.iter().any(|(id, ..)| id == "injured_xi"), "{ui:?}");
        assert!(
            ui.iter().all(|(_, severity, _)| severity != "info"),
            "{ui:?}"
        );
        assert_eq!(mcp_list, ui);
        assert!(!mcp.live_match_in_progress);
        assert!(blockers_of(&game, true).live_match_in_progress);
    }

    /// Given an injured pair of starters, and a game with only unread urgent mail
    /// When the MCP advance gate is asked, and the MCP skip is run on the injured game
    /// Then the gate refuses the injured game naming `injured_xi` and lets the mail-only game
    ///      through, and the skip returns Blocked with `injured_xi` before it advances a day.
    #[cfg(feature = "mcp")]
    #[test]
    fn the_mcp_advance_and_skip_stop_on_a_real_blocker_and_not_on_mail() {
        use crate::mcp_server::tools_impl::time::{advance_gate, skip_to_match_day_in};
        use mcp_results::time::SkipToMatchDay;
        let injured = || {
            let mut game = make_game(11);
            for player_id in ["p2", "p5"] {
                let player = game.players.iter_mut().find(|p| p.id == player_id).unwrap();
                player.injury = Some(Injury {
                    name: "Hamstring".to_string(),
                    days_remaining: 7,
                });
            }
            game
        };
        let mut mail_only = make_game(11);
        mail_only.messages = vec![make_message("urgent-1", MessagePriority::Urgent, false)];

        let refused = advance_gate(&injured()).expect_err("an injured XI stops time");
        let state = StateManager::new();
        let before = injured();
        let date = before.clock.current_date;
        state.set_game(before);
        let skipped = skip_to_match_day_in(&state).expect("the skip answers");

        assert!(refused.contains("injured_xi"), "{refused}");
        assert!(advance_gate(&mail_only).is_ok());
        match skipped {
            SkipToMatchDay::Blocked {
                days_advanced,
                blockers,
            } => {
                assert_eq!(days_advanced, 0);
                assert!(
                    blockers.iter().any(|b| b.id == "injured_xi"),
                    "{blockers:?}"
                );
            }
            other => panic!("expected the skip to be blocked, got {other:?}"),
        }
        assert_eq!(state.get_game(|g| g.clock.current_date), Some(date));
    }

    /// Given a clean game whose next match is thirty days away
    /// When the MCP skip runs
    /// Then it stops after the first day that brings a real blocker, with that blocker listed,
    ///      having advanced exactly that many days: the screen's skip loop stops there too.
    #[cfg(feature = "mcp")]
    #[test]
    fn the_mcp_skip_stops_after_the_day_that_brings_a_blocker() {
        use crate::application::time_blockers::stopping_blockers;
        use crate::mcp_server::tools_impl::time::skip_to_match_day_in;
        use mcp_results::time::SkipToMatchDay;
        let mut game = make_game_with_matchday();
        let start = game.clock.current_date;
        let far = (start + chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        game.league.as_mut().unwrap().fixtures[0].date = far;
        assert!(
            stopping_blockers(&game).is_empty(),
            "the game should start clean"
        );
        let state = StateManager::new();
        state.set_game(game);

        let outcome = skip_to_match_day_in(&state).expect("the skip answers");

        let SkipToMatchDay::Blocked {
            days_advanced,
            blockers,
        } = outcome
        else {
            panic!("a blocker should have stopped the skip, got {outcome:?}");
        };
        assert!((1..30).contains(&days_advanced), "{days_advanced}");
        assert!(!blockers.is_empty());
        assert!(
            blockers.iter().all(|b| b.severity != "info"),
            "{blockers:?}"
        );
        let moved = state.get_game(|g| g.clock.current_date - start).unwrap();
        assert_eq!(moved.num_days(), i64::from(days_advanced));
    }

    /// Given a manager on a warning whose board stands at 5, and his next match thirty days away
    /// When the MCP skip runs
    /// Then the board sacks him on the first day and the skip ends there, fired after one day,
    ///      the manager out of work
    #[cfg(feature = "mcp")]
    #[test]
    fn the_mcp_skip_reports_a_sacking_on_the_way() {
        use crate::mcp_server::tools_impl::time::skip_to_match_day_in;
        use mcp_results::time::SkipToMatchDay;
        let mut game = make_game_with_matchday();
        let far = (game.clock.current_date + chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        game.league.as_mut().unwrap().fixtures[0].date = far;
        game.manager.satisfaction = 5;
        game.manager.warning_stage = 1;
        let state = StateManager::new();
        state.set_game(game);

        let outcome = skip_to_match_day_in(&state).expect("the skip answers");

        assert_eq!(outcome, SkipToMatchDay::Fired { days_advanced: 1 });
        let employed = state.get_game(|g| g.manager.team_id.clone()).unwrap();
        assert_eq!(employed, None);
    }

    /// A game whose user's club has a cup tie `cup_in_days` from today, in `game.competitions` only
    /// (the `game.league` mirror never holds a cup), and its next league match twenty days away.
    #[cfg(feature = "mcp")]
    fn state_with_a_cup_tie_in(cup_in_days: i64) -> (StateManager, chrono::DateTime<chrono::Utc>) {
        let mut game = make_game_with_matchday();
        let start = game.clock.current_date;
        let date_in = |days: i64| {
            (start + chrono::Duration::days(days))
                .format("%Y-%m-%d")
                .to_string()
        };
        let mut league = game.league.clone().expect("the game has a league");
        league.fixtures[0].date = date_in(20);
        let mut cup = league.clone();
        cup.id = "cup-1".to_string();
        cup.fixtures[0].id = "cup-tie".to_string();
        cup.fixtures[0].competition_id = "cup-1".to_string();
        cup.fixtures[0].competition = FixtureCompetition::Cup;
        cup.fixtures[0].date = date_in(cup_in_days);
        game.league = Some(league.clone());
        game.competitions = vec![league, cup];
        let state = StateManager::new();
        state.set_game(game);
        (state, start)
    }

    /// The game's date and the cup tie's status.
    #[cfg(feature = "mcp")]
    fn clock_and_cup_tie(state: &StateManager) -> (chrono::DateTime<chrono::Utc>, FixtureStatus) {
        state
            .get_game(|g| {
                (
                    g.clock.current_date,
                    g.competitions[1].fixtures[0].status.clone(),
                )
            })
            .expect("a game")
    }

    /// Given a cup tie for the user's club today, and his next league match twenty days away
    /// When the MCP skip runs
    /// Then it stops at once on the match day, as the screen's skip does, and the cup tie is still
    ///      to be played (#1193: it aimed at the league match in the mirror and simulated the cup
    ///      tie on the way)
    #[cfg(feature = "mcp")]
    #[test]
    fn the_mcp_skip_stops_on_a_cup_tie_instead_of_playing_it() {
        use crate::mcp_server::tools_impl::time::skip_to_match_day_in;
        use mcp_results::time::SkipToMatchDay;
        let (state, start) = state_with_a_cup_tie_in(0);

        let outcome = skip_to_match_day_in(&state).expect("the skip answers");

        assert_eq!(outcome, SkipToMatchDay::MatchDayToday {});
        assert_eq!(clock_and_cup_tie(&state), (start, FixtureStatus::Scheduled));
    }

    /// Given the game's date is the user's match day, and then a date with no match of his
    /// When the skip's arrival is read
    /// Then it reached the match day on the first and not on the second: a skip whose 60th day is
    ///      its match day arrived on it (15076519 told the agent it had not reached a match)
    #[test]
    fn a_skip_ending_on_the_users_match_day_arrived_on_it_whichever_day_it_is() {
        let mut game = make_game_with_matchday();
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        game.league.as_mut().unwrap().fixtures[0].date = today;
        game.competitions = vec![game.league.clone().unwrap()];
        assert!(super::arrived_on_the_match_day(&game));

        game.clock.advance_days(1);
        assert!(!super::arrived_on_the_match_day(&game));
    }

    /// Given a cup tie for the user's club tomorrow, and his next league match twenty days away
    /// When the MCP skip runs
    /// Then it skips one day and stops on the cup tie's day with the tie still to be played (#1193:
    ///      it aimed at the league match in the mirror and played the cup tie on the way)
    #[cfg(feature = "mcp")]
    #[test]
    fn the_mcp_skip_stops_on_the_day_of_a_cup_tie_before_the_league_match() {
        use crate::mcp_server::tools_impl::time::skip_to_match_day_in;
        use mcp_results::time::SkipToMatchDay;
        let (state, start) = state_with_a_cup_tie_in(1);
        let tomorrow = start + chrono::Duration::days(1);

        let outcome = skip_to_match_day_in(&state).expect("the skip answers");

        assert_eq!(
            outcome,
            SkipToMatchDay::Skipped {
                days_advanced: 1,
                target_date: tomorrow.format("%Y-%m-%d").to_string(),
            }
        );
        assert_eq!(
            clock_and_cup_tie(&state),
            (tomorrow, FixtureStatus::Scheduled)
        );
    }

    /// Given career B is active (token 2) and a screen still holds career A's token (1)
    /// When each way of advancing time arrives with A's token
    /// Then each is refused with the career-changed key and career B's clock, messages and stats
    ///      are untouched; with B's own token (or Active) the same call advances.
    #[test]
    fn an_advance_meant_for_a_replaced_career_writes_nothing_to_the_new_one() {
        use ofm_core::state::Career;
        let state = StateManager::new();
        let mut career_b = make_game_with_matchday();
        career_b.career_token = 2;
        let started = career_b.clock.current_date;
        state.set_game(career_b);
        state.set_stats_state(StatsState::default());
        let stale = Career::Token(1);
        let refused = |result: Result<(), String>| {
            assert_eq!(result, Err("be.error.careerChanged".to_string()));
        };

        refused(super::advance_time_internal(&state, stale).map(|_| ()));
        refused(super::advance_time_with_mode_internal(&state, stale, "delegate").map(|_| ()));
        refused(super::advance_time_with_mode_internal(&state, stale, "live").map(|_| ()));
        refused(super::skip_to_match_day_internal(&state, stale).map(|_| ()));
        refused(super::advance_one_day_internal(&state, stale).map(|_| ()));
        refused(super::advance_to_next_event_internal(&state, stale).map(|_| ()));

        assert_eq!(
            state.get_game(|game| game.clock.current_date),
            Some(started)
        );
        let stats = state.get_stats_state(|current| current.clone()).unwrap();
        assert!(stats.player_matches.is_empty() && stats.team_matches.is_empty());
        assert!(state.take_live_match().is_none());
        let advanced = super::advance_time_internal(&state, Career::Token(2)).unwrap();
        assert!(advanced.clock.current_date > started);
    }

    #[test]
    fn healthy_squad_with_no_urgent_messages_has_no_blockers() {
        let game = make_game(11);

        let blockers = compute_blocking_actions(&game);

        assert!(blockers.is_empty());
    }

    #[test]
    fn injured_starters_trigger_injury_and_incomplete_xi_blockers() {
        let mut game = make_game(11);
        for player_id in ["p2", "p5"] {
            let player = game
                .players
                .iter_mut()
                .find(|player| player.id == player_id)
                .unwrap();
            player.injury = Some(Injury {
                name: "Hamstring".to_string(),
                days_remaining: 7,
            });
        }

        let blockers = compute_blocking_actions(&game);

        let injured = blocker_by_id(&blockers, "injured_xi").unwrap();
        assert_eq!(
            injured.get("severity").and_then(Value::as_str),
            Some("warn")
        );
        assert_eq!(injured.get("tab").and_then(Value::as_str), Some("Squad"));
        assert_eq!(
            injured.get("text_key").and_then(Value::as_str),
            Some("notifications.blockers.injuredXi")
        );
        assert_eq!(
            injured
                .get("text_params")
                .and_then(|params| params.get("count"))
                .and_then(Value::as_str),
            Some("2")
        );
        assert_eq!(
            injured
                .get("text_params")
                .and_then(|params| params.get("players"))
                .and_then(Value::as_str),
            Some("Player 2, Player 5")
        );

        let incomplete = blocker_by_id(&blockers, "incomplete_xi").unwrap();
        assert_eq!(
            incomplete.get("severity").and_then(Value::as_str),
            Some("warn")
        );
        assert_eq!(incomplete.get("tab").and_then(Value::as_str), Some("Squad"));
        assert_eq!(
            incomplete.get("text_key").and_then(Value::as_str),
            Some("notifications.blockers.incompleteXi")
        );
        assert_eq!(
            incomplete
                .get("text_params")
                .and_then(|params| params.get("count"))
                .and_then(Value::as_str),
            Some("9")
        );
    }

    #[test]
    fn incomplete_xi_is_not_reported_when_roster_has_fewer_than_eleven_players() {
        let mut game = make_game(10);
        let player = game
            .players
            .iter_mut()
            .find(|player| player.id == "p3")
            .unwrap();
        player.injury = Some(Injury {
            name: "Knee".to_string(),
            days_remaining: 14,
        });

        let blockers = compute_blocking_actions(&game);

        assert!(blocker_by_id(&blockers, "injured_xi").is_some());
        assert!(blocker_by_id(&blockers, "incomplete_xi").is_none());
        assert!(blocker_by_id(&blockers, "squad_size_crisis").is_some());
    }

    #[test]
    fn unsafe_planned_contract_exits_trigger_squad_crisis_blocker() {
        let mut game = make_game(11);
        mark_player_let_expire(&mut game, "p11");

        let blockers = compute_blocking_actions(&game);

        let planned_exit = blocker_by_id(&blockers, "planned_contract_exit_crisis").unwrap();
        assert_eq!(
            planned_exit.get("severity").and_then(Value::as_str),
            Some("warn")
        );
        assert_eq!(
            planned_exit.get("tab").and_then(Value::as_str),
            Some("Squad")
        );
        assert_eq!(
            planned_exit.get("text_key").and_then(Value::as_str),
            Some("notifications.blockers.plannedContractExitCrisis")
        );
        assert_eq!(
            planned_exit
                .get("text_params")
                .and_then(|params| params.get("healthyPlayers"))
                .and_then(Value::as_str),
            Some("10")
        );
        assert_eq!(
            planned_exit
                .get("text_params")
                .and_then(|params| params.get("players"))
                .and_then(Value::as_str),
            Some("Player 11")
        );
    }

    #[test]
    fn safe_planned_contract_exits_do_not_trigger_squad_crisis_blocker() {
        let mut game = make_game(12);
        mark_player_let_expire(&mut game, "p12");

        let blockers = compute_blocking_actions(&game);

        assert!(blocker_by_id(&blockers, "planned_contract_exit_crisis").is_none());
    }

    #[test]
    fn incomplete_xi_is_not_reported_when_a_partial_saved_lineup_can_be_filled_by_healthy_players()
    {
        let mut game = make_game(11);
        game.teams[0].starting_xi_ids = vec![
            "p1".to_string(),
            "p2".to_string(),
            "p3".to_string(),
            "p4".to_string(),
            "p5".to_string(),
            "p6".to_string(),
            "p7".to_string(),
            "p8".to_string(),
        ];

        let blockers = compute_blocking_actions(&game);

        assert!(blocker_by_id(&blockers, "injured_xi").is_none());
        assert!(blocker_by_id(&blockers, "incomplete_xi").is_none());
    }

    #[test]
    fn only_unread_urgent_messages_produce_message_blockers() {
        let mut game = make_game(11);
        game.messages = vec![
            make_message("urgent-1", MessagePriority::Urgent, false),
            make_message("urgent-2", MessagePriority::Urgent, false),
            make_message("urgent-read", MessagePriority::Urgent, true),
            make_message("high", MessagePriority::High, false),
        ];

        let blockers = compute_blocking_actions(&game);

        assert_eq!(blockers.len(), 1);
        let urgent = blocker_by_id(&blockers, "urgent_messages").unwrap();
        assert_eq!(urgent.get("severity").and_then(Value::as_str), Some("info"));
        assert_eq!(urgent.get("tab").and_then(Value::as_str), Some("Inbox"));
        assert_eq!(
            urgent.get("text_key").and_then(Value::as_str),
            Some("notifications.blockers.urgentMessages")
        );
        assert_eq!(
            urgent
                .get("text_params")
                .and_then(|params| params.get("count"))
                .and_then(Value::as_str),
            Some("2")
        );
    }

    #[test]
    fn key_player_contract_risk_triggers_squad_blocker() {
        let mut game = make_game(11);

        let first_key_player = game
            .players
            .iter_mut()
            .find(|player| player.id == "p10")
            .unwrap();
        first_key_player.stage_contract_end(Some("2025-08-01".to_string()));
        first_key_player.stage_wage(35_000);
        first_key_player.attributes.pace = 92;
        first_key_player.attributes.shooting = 94;
        first_key_player.attributes.dribbling = 90;

        let second_key_player = game
            .players
            .iter_mut()
            .find(|player| player.id == "p11")
            .unwrap();
        second_key_player.stage_contract_end(Some("2025-09-01".to_string()));
        second_key_player.stage_wage(25_000);
        second_key_player.attributes.pace = 90;
        second_key_player.attributes.shooting = 91;
        second_key_player.attributes.dribbling = 89;

        let blockers = compute_blocking_actions(&game);

        let contract_blocker = blocker_by_id(&blockers, "key_contract_risk").unwrap();
        assert_eq!(
            contract_blocker.get("severity").and_then(Value::as_str),
            Some("warn")
        );
        assert_eq!(
            contract_blocker.get("tab").and_then(Value::as_str),
            Some("Squad")
        );

        assert_eq!(
            contract_blocker.get("text_key").and_then(Value::as_str),
            Some("notifications.blockers.keyContractRisk")
        );
        assert_eq!(
            contract_blocker
                .get("text_params")
                .and_then(|params| params.get("players"))
                .and_then(Value::as_str),
            Some("Player 10, Player 11")
        );
    }

    #[test]
    fn large_at_risk_wage_share_triggers_finance_blocker() {
        let mut game = make_game(11);
        game.teams[0].wage_budget = 50_000;

        let first_risk = game
            .players
            .iter_mut()
            .find(|player| player.id == "p10")
            .unwrap();
        first_risk.stage_contract_end(Some("2025-08-01".to_string()));
        first_risk.stage_wage(35_000);

        let second_risk = game
            .players
            .iter_mut()
            .find(|player| player.id == "p11")
            .unwrap();
        second_risk.stage_contract_end(Some("2025-09-01".to_string()));
        second_risk.stage_wage(25_000);

        let blockers = compute_blocking_actions(&game);

        let finance_blocker = blocker_by_id(&blockers, "contract_wage_risk").unwrap();
        assert_eq!(
            finance_blocker.get("severity").and_then(Value::as_str),
            Some("warn")
        );
        assert_eq!(
            finance_blocker.get("tab").and_then(Value::as_str),
            Some("Finances")
        );

        assert_eq!(
            finance_blocker.get("text_key").and_then(Value::as_str),
            Some("notifications.blockers.contractWageRisk")
        );
        assert_eq!(
            finance_blocker
                .get("text_params")
                .and_then(|params| params.get("amount"))
                .and_then(Value::as_str),
            Some("60000")
        );
    }

    #[test]
    fn let_expire_intent_suppresses_contract_pressure_blockers() {
        let mut game = make_game(12);
        game.teams[0].wage_budget = 50_000;

        let first_key_player = game
            .players
            .iter_mut()
            .find(|player| player.id == "p10")
            .unwrap();
        first_key_player.stage_contract_end(Some("2025-08-01".to_string()));
        first_key_player.stage_wage(35_000);
        first_key_player.attributes.pace = 92;
        first_key_player.attributes.shooting = 94;
        first_key_player.attributes.dribbling = 90;
        first_key_player.morale_core.renewal_state = Some(ContractRenewalState {
            status: RenewalSessionStatus::Blocked,
            manager_blocked_until: None,
            last_attempt_date: Some("2025-06-15".to_string()),
            last_assistant_attempt_date: None,
            last_outcome: None,
            conversation_round: 0,
            exit_intent: Some(ContractExitIntent::LetExpire {
                set_on: "2025-06-15".to_string(),
                reason: Some("test".to_string()),
            }),
        });

        let second_key_player = game
            .players
            .iter_mut()
            .find(|player| player.id == "p11")
            .unwrap();
        second_key_player.stage_contract_end(Some("2025-09-01".to_string()));
        second_key_player.stage_wage(25_000);
        second_key_player.attributes.pace = 90;
        second_key_player.attributes.shooting = 91;
        second_key_player.attributes.dribbling = 89;
        second_key_player.morale_core.renewal_state = Some(ContractRenewalState {
            status: RenewalSessionStatus::Blocked,
            manager_blocked_until: None,
            last_attempt_date: Some("2025-06-15".to_string()),
            last_assistant_attempt_date: None,
            last_outcome: None,
            conversation_round: 0,
            exit_intent: Some(ContractExitIntent::LetExpire {
                set_on: "2025-06-15".to_string(),
                reason: Some("test".to_string()),
            }),
        });

        let blockers = compute_blocking_actions(&game);

        assert!(blocker_by_id(&blockers, "key_contract_risk").is_none());
        assert!(blocker_by_id(&blockers, "contract_wage_risk").is_none());
        assert!(blocker_by_id(&blockers, "planned_contract_exit_crisis").is_some());
    }

    fn make_round_summary_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());

        let teams = vec![
            Team::new(
                "team1".to_string(),
                "Test FC".to_string(),
                "TST".to_string(),
                "England".to_string(),
                "Testville".to_string(),
                "Test Ground".to_string(),
                20_000,
            ),
            Team::new(
                "team2".to_string(),
                "Rival FC".to_string(),
                "RIV".to_string(),
                "England".to_string(),
                "Rivaltown".to_string(),
                "Rival Ground".to_string(),
                20_000,
            ),
            Team::new(
                "team3".to_string(),
                "Third FC".to_string(),
                "THI".to_string(),
                "England".to_string(),
                "Thirdtown".to_string(),
                "Third Ground".to_string(),
                20_000,
            ),
            Team::new(
                "team4".to_string(),
                "Fourth FC".to_string(),
                "FOU".to_string(),
                "England".to_string(),
                "Fourthtown".to_string(),
                "Fourth Ground".to_string(),
                20_000,
            ),
        ];

        let mut players = Vec::new();
        for (team_id, prefix) in [
            ("team1", "a"),
            ("team2", "b"),
            ("team3", "c"),
            ("team4", "d"),
        ] {
            players.push(make_player(
                &format!("{}-gk", prefix),
                &format!("{} GK", prefix),
                team_id,
                Position::Goalkeeper,
            ));
            for idx in 0..4 {
                players.push(make_player(
                    &format!("{}-def{}", prefix, idx),
                    &format!("{} Def{}", prefix, idx),
                    team_id,
                    Position::Defender,
                ));
            }
            for idx in 0..4 {
                players.push(make_player(
                    &format!("{}-mid{}", prefix, idx),
                    &format!("{} Mid{}", prefix, idx),
                    team_id,
                    Position::Midfielder,
                ));
            }
            for idx in 0..2 {
                players.push(make_player(
                    &format!("{}-fwd{}", prefix, idx),
                    &format!("{} Fwd{}", prefix, idx),
                    team_id,
                    Position::Forward,
                ));
            }
        }

        let league = domain::league::League {
            id: "league1".to_string(),
            name: "Test League".to_string(),
            season: 1,
            fixtures: vec![
                domain::league::Fixture {
                    postponement: None,
                    id: "fix1".to_string(),
                    competition_id: "league1".to_string(),
                    matchday: 1,
                    date: "2025-06-15".to_string(),
                    home_team_id: "team1".to_string(),
                    away_team_id: "team2".to_string(),
                    competition: FixtureCompetition::League,
                    status: FixtureStatus::Scheduled,
                    result: None,
                },
                Fixture {
                    postponement: None,
                    id: "fix2".to_string(),
                    competition_id: "league1".to_string(),
                    matchday: 1,
                    date: "2025-06-15".to_string(),
                    home_team_id: "team3".to_string(),
                    away_team_id: "team4".to_string(),
                    competition: FixtureCompetition::League,
                    status: domain::league::FixtureStatus::Scheduled,
                    result: None,
                },
            ],
            standings: vec![
                domain::league::StandingEntry::new("team1".to_string()),
                domain::league::StandingEntry::new("team2".to_string()),
                domain::league::StandingEntry::new("team3".to_string()),
                domain::league::StandingEntry::new("team4".to_string()),
            ],
            transfer_log: vec![],
            transfer_rumours: vec![],
            ..domain::league::League::default()
        };

        let mut game = Game::new(clock, manager, teams, players, vec![], vec![]);
        game.league = Some(league);
        game
    }

    #[test]
    fn advance_time_with_mode_live_returns_partial_round_summary() {
        let state = StateManager::new();
        state.set_game(make_round_summary_game());

        let response =
            advance_time_with_mode_internal(&state, ofm_core::state::Career::Active, "live")
                .expect("live advance response");

        assert_eq!(response.action, "live_match");
        assert_eq!(response.competition_id.as_deref(), Some("league1"));
        assert_eq!(response.fixture_id.as_deref(), Some("fix1"));
        let round_summary = response.round_summary.expect("round summary");
        assert!(!round_summary.is_complete);
        assert_eq!(round_summary.pending_fixture_count, 1);
        assert_eq!(round_summary.completed_results.len(), 1);
    }

    #[test]
    fn advance_time_with_mode_delegate_returns_completed_round_summary() {
        let state = StateManager::new();
        state.set_game(make_round_summary_game());

        let response =
            advance_time_with_mode_internal(&state, ofm_core::state::Career::Active, "delegate")
                .expect("delegate advance response");

        assert_eq!(response.action, "advanced");
        let round_summary = response.round_summary.expect("round summary");
        assert!(round_summary.is_complete);
        assert_eq!(round_summary.pending_fixture_count, 0);
        assert_eq!(round_summary.completed_results.len(), 2);
    }

    #[test]
    fn advance_one_day_stops_on_match_day_without_advancing() {
        let state = StateManager::new();
        // Must use a competition-based fixture because user_has_scheduled_match_on
        // checks game.competitions, not the legacy game.league mirror.
        state.set_game(make_game_with_competition_matchday());

        let result = super::advance_one_day_internal(&state, ofm_core::state::Career::Active)
            .expect("advance_one_day result");

        assert_eq!(
            result.get("action").and_then(Value::as_str),
            Some("match_day")
        );
        // Clock must not have advanced — the match should be played interactively.
        let game: ofm_core::game::Game =
            serde_json::from_value(result["game"].clone()).expect("game deserializable");
        assert_eq!(
            game.clock.current_date.date_naive().to_string(),
            "2025-06-15",
            "clock must not advance on a match day stop"
        );
    }

    #[test]
    fn advance_one_day_stops_on_blocker_without_advancing() {
        let state = StateManager::new();
        let mut game = make_game(11);
        // Inject two injured starters to trigger the injured_xi blocker.
        for player_id in ["p2", "p5"] {
            let player = game.players.iter_mut().find(|p| p.id == player_id).unwrap();
            player.injury = Some(Injury {
                name: "Hamstring".to_string(),
                days_remaining: 7,
            });
        }
        state.set_game(game);

        let result = super::advance_one_day_internal(&state, ofm_core::state::Career::Active)
            .expect("advance_one_day result");

        assert_eq!(
            result.get("action").and_then(Value::as_str),
            Some("blocked")
        );
        let blockers = result.get("blockers").and_then(Value::as_array).unwrap();
        assert!(!blockers.is_empty(), "blockers array must be populated");
    }

    #[test]
    fn advance_one_day_advances_quiet_day_and_returns_date() {
        let state = StateManager::new();
        // No match today, no blockers.
        state.set_game(make_game(11));

        let result = super::advance_one_day_internal(&state, ofm_core::state::Career::Active)
            .expect("advance_one_day result");

        assert_eq!(
            result.get("action").and_then(Value::as_str),
            Some("advanced")
        );
        assert_eq!(
            result.get("date").and_then(Value::as_str),
            Some("2025-06-15"),
            "returned date must be the day that was processed"
        );
        // Game clock should now be 2025-06-16.
        let game: ofm_core::game::Game =
            serde_json::from_value(result["game"].clone()).expect("game deserializable");
        assert_eq!(
            game.clock.current_date.date_naive().to_string(),
            "2025-06-16"
        );
    }

    #[test]
    fn advance_one_day_while_unemployed_advances_instead_of_refiring() {
        let state = StateManager::new();
        let mut game = make_game(11);
        // Manager sacked on an earlier day; the digest keeps calling
        // advance_one_day while job hunting, and each quiet day must advance
        // rather than re-report the old dismissal as a fresh "fired" stop.
        game.manager.fire("2025-06-10");
        state.set_game(game);

        let result = super::advance_one_day_internal(&state, ofm_core::state::Career::Active)
            .expect("advance_one_day result");

        assert_eq!(
            result.get("action").and_then(Value::as_str),
            Some("advanced")
        );
        let game: ofm_core::game::Game =
            serde_json::from_value(result["game"].clone()).expect("game deserializable");
        assert_eq!(
            game.clock.current_date.date_naive().to_string(),
            "2025-06-16"
        );
    }

    // -----------------------------------------------------------------------
    // A dial saved before kick-off reaches the match waiting to be played
    // -----------------------------------------------------------------------

    /// The user's side as the live session holds it (the engine's own copy of the team, which
    /// is what the match is played with), and the match's phase.
    fn user_side_and_phase(state: &StateManager) -> (engine::MatchPhase, engine::TeamData) {
        let snapshot = state
            .with_live_match(|session| session.snapshot())
            .expect("a live match session exists after a live advance");
        let side = [snapshot.home_team, snapshot.away_team]
            .into_iter()
            .find(|team| team.id == "team1")
            .expect("the user's club is one of the two sides");
        (snapshot.phase, side)
    }

    /// The user's side while the match still waits at kick-off.
    fn user_side_before_kick_off(state: &StateManager) -> engine::TeamData {
        let (phase, side) = user_side_and_phase(state);
        assert_eq!(phase, engine::MatchPhase::PreKickOff);
        side
    }

    /// The round-summary game with the user's fixture turned round, so team1 plays away.
    fn make_round_summary_game_away() -> Game {
        let mut game = make_round_summary_game();
        for league in game.league.iter_mut().chain(game.competitions.iter_mut()) {
            for fixture in league.fixtures.iter_mut().filter(|f| f.id == "fix1") {
                std::mem::swap(&mut fixture.home_team_id, &mut fixture.away_team_id);
            }
        }
        game
    }

    /// What the pre-match screen does when the user raises the defensive line.
    fn set_defensive_line_high(state: &StateManager) {
        crate::commands::squad::set_tactics_phase_internal(
            state,
            ofm_core::state::Career::Active,
            crate::commands::squad::TacticsPhasePatch {
                defensive_line: Some("High".to_string()),
                ..Default::default()
            },
        )
        .expect("the dial is stored on the user's team");
    }

    /// Given an active career whose user team has a live match waiting at kick-off (built by
    /// the live advance, as on match day),
    /// When the user raises the defensive line to High on the pre-match screen,
    /// Then the match about to be played uses a high defensive line.
    #[test]
    fn a_dial_changed_before_kick_off_reaches_the_match() {
        let state = StateManager::new();
        state.set_game(make_round_summary_game());
        advance_time_with_mode_internal(&state, ofm_core::state::Career::Active, "live")
            .expect("live advance response");
        assert_ne!(
            user_side_before_kick_off(&state).tactics.defensive_line,
            engine::DefensiveLine::High,
            "the fixture must start from a different line, or the test proves nothing"
        );

        set_defensive_line_high(&state);

        let stored_line = state
            .get_game(|game| game.teams[0].tactics_phase.defensive_line.clone())
            .expect("active game");
        assert_eq!(stored_line, domain::team::DefensiveLine::High);
        assert_eq!(
            user_side_before_kick_off(&state).tactics.defensive_line,
            engine::DefensiveLine::High,
            "the live session must play with the defensive line set before kick-off"
        );
    }

    /// Given an active career whose user team plays away, with the match waiting at kick-off,
    /// When the user raises the defensive line on the pre-match screen,
    /// Then the away side of the match plays with the high line.
    #[test]
    fn a_dial_changed_before_kick_off_reaches_an_away_match() {
        let state = StateManager::new();
        state.set_game(make_round_summary_game_away());
        advance_time_with_mode_internal(&state, ofm_core::state::Career::Active, "live")
            .expect("live advance response");
        let away_id = state
            .with_live_match(|session| session.snapshot().away_team.id)
            .expect("a live match session exists");
        assert_eq!(
            away_id, "team1",
            "the fixture must put the user's club away"
        );
        assert_ne!(
            user_side_before_kick_off(&state).tactics.defensive_line,
            engine::DefensiveLine::High
        );

        set_defensive_line_high(&state);

        assert_eq!(
            user_side_before_kick_off(&state).tactics.defensive_line,
            engine::DefensiveLine::High
        );
    }

    /// Given an active career whose user team already has a high defensive line on the eve of
    /// match day,
    /// When the live advance builds the match session,
    /// Then the session carries it: so a failure in the tests above is the missing hand-over to
    /// a session already built, not the domain-to-engine mapping of the dial.
    #[test]
    fn a_dial_set_before_the_session_is_built_reaches_the_match() {
        let state = StateManager::new();
        state.set_game(make_round_summary_game());
        set_defensive_line_high(&state);

        advance_time_with_mode_internal(&state, ofm_core::state::Career::Active, "live")
            .expect("live advance response");

        assert_eq!(
            user_side_before_kick_off(&state).tactics.defensive_line,
            engine::DefensiveLine::High
        );
    }

    /// Given a user team whose live match has already kicked off,
    /// When the user saves a high defensive line on the team,
    /// Then the match in play keeps its line: once it has started, only the live commands
    /// change it.
    #[test]
    fn a_dial_saved_after_kick_off_leaves_the_match_alone() {
        let state = StateManager::new();
        state.set_game(make_round_summary_game());
        advance_time_with_mode_internal(&state, ofm_core::state::Career::Active, "live")
            .expect("live advance response");
        crate::application::live_match::step_live_match(&state, ofm_core::state::Career::Active, 1)
            .expect("the match kicks off");
        let (phase, side) = user_side_and_phase(&state);
        assert_ne!(
            phase,
            engine::MatchPhase::PreKickOff,
            "the match must have started"
        );
        let line_at_kick_off = side.tactics.defensive_line;
        assert_ne!(line_at_kick_off, engine::DefensiveLine::High);

        set_defensive_line_high(&state);

        let (_, side) = user_side_and_phase(&state);
        assert_eq!(side.tactics.defensive_line, line_at_kick_off);
    }

    // -----------------------------------------------------------------------
    // A role saved before kick-off reaches the match waiting to be played
    // -----------------------------------------------------------------------

    fn user_goalkeeper_role_before_kick_off(state: &StateManager) -> engine::PlayerRole {
        user_side_before_kick_off(state)
            .players
            .into_iter()
            .find(|player| player.id == "a-gk")
            .expect("the user's goalkeeper starts")
            .role
    }

    fn set_goalkeeper_role(state: &StateManager, role: Option<&str>) {
        crate::commands::squad::set_player_role_internal(
            state,
            ofm_core::state::Career::Active,
            "a-gk".to_string(),
            role.map(str::to_string),
        )
        .expect("a goalkeeper may take a keeper's role, or none");
    }

    /// Given an active career whose user team has a live match waiting at kick-off,
    /// When the user makes the goalkeeper a Sweeper Keeper on the pre-match screen, through the
    /// function the `set_player_role` command calls,
    /// Then the match about to be played has him as a Sweeper Keeper.
    #[test]
    fn a_role_changed_before_kick_off_reaches_the_match() {
        let state = StateManager::new();
        state.set_game(make_round_summary_game());
        advance_time_with_mode_internal(&state, ofm_core::state::Career::Active, "live")
            .expect("live advance response");
        assert_ne!(
            user_goalkeeper_role_before_kick_off(&state),
            engine::PlayerRole::SweeperKeeper,
            "the fixture must start from a different role, or the test proves nothing"
        );

        set_goalkeeper_role(&state, Some("SweeperKeeper"));

        let stored_role = state
            .get_game(|game| game.teams[0].player_roles.get("a-gk").cloned())
            .expect("active game");
        assert_eq!(stored_role, Some(domain::team::PlayerRole::SweeperKeeper));
        assert_eq!(
            user_goalkeeper_role_before_kick_off(&state),
            engine::PlayerRole::SweeperKeeper,
            "the live session must play with the role set before kick-off"
        );
    }

    /// Given a match waiting at kick-off whose goalkeeper was made a Sweeper Keeper on the
    /// pre-match screen,
    /// When the user clears his role,
    /// Then he goes into the match on the Standard role.
    #[test]
    fn a_role_cleared_before_kick_off_leaves_the_match_on_standard() {
        let state = StateManager::new();
        state.set_game(make_round_summary_game());
        advance_time_with_mode_internal(&state, ofm_core::state::Career::Active, "live")
            .expect("live advance response");
        set_goalkeeper_role(&state, Some("SweeperKeeper"));
        assert_eq!(
            user_goalkeeper_role_before_kick_off(&state),
            engine::PlayerRole::SweeperKeeper
        );

        set_goalkeeper_role(&state, None);

        assert_eq!(
            user_goalkeeper_role_before_kick_off(&state),
            engine::PlayerRole::Standard
        );
    }

    /// A live match waiting at kick-off in which a spare forward, benched in the saved team, has
    /// been swapped into the left-back slot on the pre-match screen. Returns his id.
    fn forward_swapped_into_left_back(state: &StateManager) -> String {
        let mut game = make_round_summary_game();
        game.players.push(make_player(
            "a-fwd-spare",
            "a Fwd Spare",
            "team1",
            Position::Forward,
        ));
        state.set_game(game);
        advance_time_with_mode_internal(state, ofm_core::state::Career::Active, "live")
            .expect("live advance response");
        let (forward_on_bench, left_back) = state
            .with_live_match(|session| {
                let snapshot = session.snapshot();
                let bench_forward = snapshot
                    .home_bench
                    .iter()
                    .find(|player| player.position == engine::Position::Forward)
                    .map(|player| player.id.clone())
                    .expect("a forward waits on the bench");
                // 4-4-2: the goalkeeper, then the left-back.
                (bench_forward, snapshot.home_team.players[1].id.clone())
            })
            .expect("a live match session exists");
        crate::application::live_match::apply_match_command(
            state,
            ofm_core::state::Career::Active,
            engine::MatchCommand::PreMatchSwap {
                side: engine::Side::Home,
                player_off_id: left_back,
                player_on_id: forward_on_bench.clone(),
            },
        )
        .expect("the swap is made before kick-off");
        forward_on_bench
    }

    fn role_in_the_waiting_match(state: &StateManager, player_id: &str) -> engine::PlayerRole {
        user_side_before_kick_off(state)
            .players
            .iter()
            .find(|player| player.id == player_id)
            .expect("he starts")
            .role
    }

    /// Given a match waiting at kick-off where a spare forward, benched in the saved team, has
    /// been swapped into the left-back slot on the pre-match screen (his live picker offers him
    /// the full-back roles),
    /// When the user saves him as an Attacking Full-Back,
    /// Then the save succeeds and he plays the left-back slot as one: the role is judged by the
    /// slot he fills in the match, not by the saved team's bench.
    #[test]
    fn a_role_his_new_slot_admits_is_saved_after_a_swap() {
        let state = StateManager::new();
        let swapped_in = forward_swapped_into_left_back(&state);

        crate::commands::squad::set_player_role_internal(
            &state,
            ofm_core::state::Career::Active,
            swapped_in.clone(),
            Some("AttackingFB".to_string()),
        )
        .expect("the left-back slot admits an Attacking Full-Back");

        assert_eq!(
            role_in_the_waiting_match(&state, &swapped_in),
            engine::PlayerRole::AttackingFB
        );
    }

    /// Given the same swapped-in forward,
    /// When the user saves him as a Poacher, which a left-back slot does not admit,
    /// Then the save is refused: the picker for the slot he fills would not have offered it,
    /// and he plays on the Standard role.
    #[test]
    fn a_role_his_new_slot_refuses_is_refused_after_a_swap() {
        let state = StateManager::new();
        let swapped_in = forward_swapped_into_left_back(&state);

        let refused = crate::commands::squad::set_player_role_internal(
            &state,
            ofm_core::state::Career::Active,
            swapped_in.clone(),
            Some("Poacher".to_string()),
        );

        assert_eq!(
            refused.err().as_deref(),
            Some("be.error.roleNotValidForPosition")
        );
        assert_eq!(
            role_in_the_waiting_match(&state, &swapped_in),
            engine::PlayerRole::Standard
        );
    }

    /// Given the same swapped-in forward, in a match that belongs to another career than the
    /// game now open (the game was swapped for another while the match waited),
    /// When the user saves him as an Attacking Full-Back,
    /// Then the role is judged by the saved team, where he is a benched forward, and refused:
    /// another career's match says nothing about this game's slots.
    #[test]
    fn a_role_is_judged_by_the_saved_team_when_the_waiting_match_is_another_careers() {
        let state = StateManager::new();
        let swapped_in = forward_swapped_into_left_back(&state);
        state
            .update_game(|game| game.career_token += 1)
            .expect("a game is open");

        let refused = crate::commands::squad::set_player_role_internal(
            &state,
            ofm_core::state::Career::Active,
            swapped_in,
            Some("AttackingFB".to_string()),
        );

        assert_eq!(
            refused.err().as_deref(),
            Some("be.error.roleNotValidForPosition")
        );
    }

    // -----------------------------------------------------------------------
    // A saved role the team drops, and the gate a role save and a swap pass through
    // -----------------------------------------------------------------------

    /// A live match waiting at kick-off whose team holds an Attacking Full-Back for a benched
    /// forward whose natural position, made centre-back in the game, refuses it (as a
    /// stale save can carry), and that the match has been told about. Returns his id.
    fn benched_centre_back_holding_a_role_his_position_refuses(state: &StateManager) -> String {
        // The spare forward is how `forward_swapped_into_left_back` gets a player onto the bench.
        let mut game = make_round_summary_game();
        game.players.push(make_player(
            "a-fwd-spare",
            "a Fwd Spare",
            "team1",
            Position::Forward,
        ));
        state.set_game(game);
        advance_time_with_mode_internal(state, ofm_core::state::Career::Active, "live")
            .expect("live advance response");
        let benched_forward = state
            .with_live_match(|session| {
                session
                    .snapshot()
                    .home_bench
                    .iter()
                    .find(|player| player.position == engine::Position::Forward)
                    .map(|player| player.id.clone())
            })
            .flatten()
            .expect("a forward waits on the bench");
        let game = state
            .update_game(|game| {
                game.players
                    .iter_mut()
                    .find(|player| player.id == benched_forward)
                    .expect("he is on the user's team")
                    .natural_position = Position::CenterBack;
                game.teams
                    .iter_mut()
                    .find(|team| team.id == "team1")
                    .expect("the user's team")
                    .player_roles
                    .insert(
                        benched_forward.clone(),
                        domain::team::PlayerRole::AttackingFB,
                    );
                game.clone()
            })
            .expect("a game is open");
        crate::application::live_match::follow_saved_role_before_kick_off(
            state,
            &game,
            &benched_forward,
        )
        .expect("the match is told");
        benched_forward
    }

    fn swap_into_left_back(state: &StateManager, player_on_id: &str) {
        let left_back = state
            .with_live_match(|session| session.snapshot().home_team.players[1].id.clone())
            .expect("a live match session exists");
        crate::application::live_match::apply_match_command(
            state,
            ofm_core::state::Career::Active,
            engine::MatchCommand::PreMatchSwap {
                side: engine::Side::Home,
                player_off_id: left_back,
                player_on_id: player_on_id.to_string(),
            },
        )
        .expect("the swap is made before kick-off");
    }

    /// Given a team that holds a role for a benched centre-back his position refuses,
    /// When `change` rewrites the team's roles (it reconciles them and drops that one) and the
    /// manager then swaps him into the left-back slot,
    /// Then he plays the slot on Standard: the match was told the role is gone, not left holding it.
    fn assert_a_dropped_role_is_not_restored_by_a_swap(change: impl FnOnce(&StateManager)) {
        let state = StateManager::new();
        let spare = benched_centre_back_holding_a_role_his_position_refuses(&state);
        change(&state);

        swap_into_left_back(&state, &spare);

        assert_eq!(
            role_in_the_waiting_match(&state, &spare),
            engine::PlayerRole::Standard
        );
    }

    /// The eleven the waiting match starts, which is a valid eleven to save.
    fn current_eleven(state: &StateManager) -> Vec<String> {
        state
            .with_live_match(|session| {
                session
                    .snapshot()
                    .home_team
                    .players
                    .iter()
                    .map(|player| player.id.clone())
                    .collect()
            })
            .expect("a live match session exists")
    }

    /// Given the scenario above, changed by saving the same starting eleven.
    #[test]
    fn a_role_the_starting_eleven_save_drops_is_not_restored_by_a_swap() {
        assert_a_dropped_role_is_not_restored_by_a_swap(|state| {
            let eleven = current_eleven(state);
            crate::commands::squad::set_starting_xi_internal(
                state,
                ofm_core::state::Career::Active,
                eleven,
            )
            .expect("the eleven is saved");
        });
    }

    /// Given the scenario above, changed by saving a formation.
    #[test]
    fn a_role_the_formation_save_drops_is_not_restored_by_a_swap() {
        assert_a_dropped_role_is_not_restored_by_a_swap(|state| {
            crate::commands::squad::set_formation_internal(
                state,
                ofm_core::state::Career::Active,
                "4-4-2",
            )
            .expect("the formation is saved");
        });
    }

    /// Given the scenario above, changed by applying a tactic preset.
    #[test]
    fn a_role_the_tactic_preset_drops_is_not_restored_by_a_swap() {
        assert_a_dropped_role_is_not_restored_by_a_swap(|state| {
            crate::commands::squad::apply_tactic_preset_internal(
                state,
                ofm_core::state::Career::Active,
                "4-4-2",
                "Balanced",
            )
            .expect("the preset is applied");
        });
    }

    /// Given the scenario above, changed by applying a saved custom tactic.
    #[test]
    fn a_role_the_custom_tactic_drops_is_not_restored_by_a_swap() {
        assert_a_dropped_role_is_not_restored_by_a_swap(|state| {
            let tactic = domain::custom_tactic::CustomTactic {
                id: "mine".to_string(),
                name: "Mine".to_string(),
                description: String::new(),
                formation: "4-4-2".to_string(),
                play_style: "Balanced".to_string(),
                source_preset_name: None,
                contents: None,
            };
            crate::commands::custom_tactics::save_current_tactic_internal(
                state,
                &tactic,
                ofm_core::state::Career::Active,
            )
            .expect("the tactic is saved");
            crate::commands::custom_tactics::apply_custom_tactic_internal(
                state,
                "mine",
                ofm_core::state::Career::Active,
            )
            .expect("the tactic is applied");
        });
    }

    /// Runs `call` on another thread while this one holds the lifecycle gate, and reports whether
    /// it finished before and after the gate was released.
    fn finishes_only_after_the_gate_is_released<F>(
        state: std::sync::Arc<StateManager>,
        call: F,
    ) -> (bool, bool)
    where
        F: FnOnce(&StateManager) + Send + 'static,
    {
        let operation = crate::application::live_session::operation();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            call(&state);
            done_tx.send(()).unwrap();
        });
        let finished_while_held = done_rx
            .recv_timeout(std::time::Duration::from_millis(300))
            .is_ok();
        drop(operation);
        let finished_after_release = done_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .is_ok();
        handle.join().unwrap();
        (finished_while_held, finished_after_release)
    }

    /// Given a lifecycle operation in flight,
    /// When a role is saved for a player of the waiting match,
    /// Then the save queues behind the operation, and completes once it is released: the read of
    /// his slot, the save and the hand-off to the match are one step against a kick-off or a swap.
    #[test]
    fn a_role_save_waits_for_a_lifecycle_operation_in_flight() {
        let state = std::sync::Arc::new(StateManager::new());
        let swapped_in = forward_swapped_into_left_back(&state);

        let (while_held, after_release) =
            finishes_only_after_the_gate_is_released(state, move |state| {
                crate::commands::squad::set_player_role_internal(
                    state,
                    ofm_core::state::Career::Active,
                    swapped_in,
                    Some("AttackingFB".to_string()),
                )
                .expect("the role is saved");
            });

        assert!(!while_held, "the save ran inside a held operation");
        assert!(after_release, "the save never completed once released");
    }

    /// Given a lifecycle operation in flight,
    /// When a pre-match swap is made,
    /// Then the swap queues behind the operation and completes once it is released.
    #[test]
    fn a_pre_match_swap_waits_for_a_lifecycle_operation_in_flight() {
        let state = std::sync::Arc::new(StateManager::new());
        let spare = benched_centre_back_holding_a_role_his_position_refuses(&state);

        let (while_held, after_release) =
            finishes_only_after_the_gate_is_released(state, move |state| {
                swap_into_left_back(state, &spare);
            });

        assert!(!while_held, "the swap ran inside a held operation");
        assert!(after_release, "the swap never completed once released");
    }

    /// Runs `call` against a game at rest (after `setup`) while a lifecycle operation is held.
    /// It must not have finished, nor changed what `observe` reads, while the gate is held; once
    /// the gate is released it completes and `observe` reads something different.
    fn assert_a_bulk_role_write_waits_for_the_gate<F>(
        setup: fn(&StateManager),
        call: F,
        observe: fn(&StateManager) -> String,
    ) where
        F: FnOnce(&StateManager) + Send + 'static,
    {
        let state = std::sync::Arc::new(StateManager::new());
        state.set_game(make_round_summary_game());
        setup(&state);
        let before = observe(&state);
        let worker_state = state.clone();
        let operation = crate::application::live_session::operation();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            call(&worker_state);
            done_tx.send(()).unwrap();
        });

        let finished_while_held = done_rx
            .recv_timeout(std::time::Duration::from_millis(300))
            .is_ok();
        let seen_while_held = observe(&state);
        drop(operation);
        let finished_after_release = done_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .is_ok();
        handle.join().unwrap();

        assert!(
            !finished_while_held,
            "the write ran inside a held operation"
        );
        assert_eq!(seen_while_held, before, "the write landed before the gate");
        assert!(
            finished_after_release,
            "the write never completed once released"
        );
        assert_ne!(observe(&state), before, "the write changed nothing");
    }

    fn no_setup(_: &StateManager) {}

    fn formation_of_the_user_team(state: &StateManager) -> String {
        state
            .get_game(|game| {
                game.teams
                    .iter()
                    .find(|team| team.id == "team1")
                    .map(|team| format!("{}/{:?}", team.formation, team.play_style))
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    fn starting_eleven_of_the_user_team(state: &StateManager) -> String {
        state
            .get_game(|game| {
                game.teams
                    .iter()
                    .find(|team| team.id == "team1")
                    .map(|team| team.starting_xi_ids.join(","))
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Given a lifecycle operation in flight, when the formation is saved,
    /// then the save queues behind it: commit and hand-off to the waiting match are one step
    /// against a role save.
    #[test]
    fn a_formation_save_waits_for_a_lifecycle_operation_in_flight() {
        assert_a_bulk_role_write_waits_for_the_gate(
            no_setup,
            |state| {
                crate::commands::squad::set_formation_internal(
                    state,
                    ofm_core::state::Career::Active,
                    "3-5-2",
                )
                .expect("the formation is saved");
            },
            formation_of_the_user_team,
        );
    }

    /// The same for a tactic preset.
    #[test]
    fn a_tactic_preset_waits_for_a_lifecycle_operation_in_flight() {
        assert_a_bulk_role_write_waits_for_the_gate(
            no_setup,
            |state| {
                crate::commands::squad::apply_tactic_preset_internal(
                    state,
                    ofm_core::state::Career::Active,
                    "3-5-2",
                    "Defensive",
                )
                .expect("the preset is applied");
            },
            formation_of_the_user_team,
        );
    }

    /// The same for a starting eleven.
    #[test]
    fn a_starting_eleven_save_waits_for_a_lifecycle_operation_in_flight() {
        // The club's eleven healthy seniors, in an order no fixture starts from.
        assert_a_bulk_role_write_waits_for_the_gate(
            no_setup,
            |state| {
                let eleven = [
                    "a-gk", "a-fwd1", "a-fwd0", "a-mid3", "a-mid2", "a-mid1", "a-mid0", "a-def3",
                    "a-def2", "a-def1", "a-def0",
                ]
                .map(String::from)
                .to_vec();
                crate::commands::squad::set_starting_xi_internal(
                    state,
                    ofm_core::state::Career::Active,
                    eleven,
                )
                .expect("the eleven is saved");
            },
            starting_eleven_of_the_user_team,
        );
    }

    /// The same for applying a saved custom tactic.
    #[test]
    fn a_custom_tactic_applied_waits_for_a_lifecycle_operation_in_flight() {
        // A tactic saved with a 3-5-2 team, then the team put back to 4-4-2: applying it changes
        // the formation again.
        fn save_a_three_five_two_tactic(state: &StateManager) {
            let career = ofm_core::state::Career::Active;
            crate::commands::squad::set_formation_internal(state, career, "3-5-2")
                .expect("the formation is saved");
            let tactic = domain::custom_tactic::CustomTactic {
                id: "mine".to_string(),
                name: "Mine".to_string(),
                description: String::new(),
                formation: "3-5-2".to_string(),
                play_style: "Balanced".to_string(),
                source_preset_name: None,
                contents: None,
            };
            crate::commands::custom_tactics::save_current_tactic_internal(state, &tactic, career)
                .expect("the tactic is saved");
            crate::commands::squad::set_formation_internal(state, career, "4-4-2")
                .expect("the formation is put back");
        }
        assert_a_bulk_role_write_waits_for_the_gate(
            save_a_three_five_two_tactic,
            |state| {
                crate::commands::custom_tactics::apply_custom_tactic_internal(
                    state,
                    "mine",
                    ofm_core::state::Career::Active,
                )
                .expect("the tactic is applied");
            },
            formation_of_the_user_team,
        );
    }

    fn waiting_match_state() -> std::sync::Arc<StateManager> {
        let state = std::sync::Arc::new(StateManager::new());
        state.set_game(make_round_summary_game());
        advance_time_with_mode_internal(&state, ofm_core::state::Career::Active, "live")
            .expect("live advance response");
        state
    }

    /// Given a live match waiting at kick-off and a lifecycle operation in flight,
    /// When the match is stepped (the kick-off),
    /// Then the step queues behind the operation and the match kicks off once it is released: a
    /// role saved while the gate was held is in the match when it starts.
    #[test]
    fn the_kick_off_waits_for_a_lifecycle_operation_in_flight() {
        let state = waiting_match_state();
        let watcher = state.clone();

        let (while_held, after_release) =
            finishes_only_after_the_gate_is_released(state, |state| {
                crate::application::live_match::step_live_match(
                    state,
                    ofm_core::state::Career::Active,
                    1,
                )
                .expect("the match kicks off");
            });

        assert!(!while_held, "the kick-off ran inside a held operation");
        assert!(after_release, "the kick-off never completed once released");
        let (phase, _) = user_side_and_phase(&watcher);
        assert_ne!(
            phase,
            engine::MatchPhase::PreKickOff,
            "the match kicked off"
        );
    }

    /// Given a live match waiting at kick-off and a lifecycle operation in flight,
    /// When the formation is changed on the pre-match screen,
    /// Then the change queues behind the operation and completes once it is released.
    #[test]
    fn a_pre_match_formation_change_waits_for_a_lifecycle_operation_in_flight() {
        let state = waiting_match_state();

        let (while_held, after_release) =
            finishes_only_after_the_gate_is_released(state, |state| {
                crate::application::live_match::apply_match_command(
                    state,
                    ofm_core::state::Career::Active,
                    engine::MatchCommand::ChangeFormation {
                        side: engine::Side::Home,
                        formation: "3-5-2".to_string(),
                    },
                )
                .expect("the shape is changed before kick-off");
            });

        assert!(!while_held, "the change ran inside a held operation");
        assert!(after_release, "the change never completed once released");
    }

    /// Given a match already under way and a lifecycle operation in flight,
    /// When the match is stepped,
    /// Then the step does not wait: only the kick-off needs the gate, and a step in play is
    /// neither a role save's nor a swap's concern.
    #[test]
    fn a_step_in_play_does_not_wait_for_a_lifecycle_operation() {
        let state = waiting_match_state();
        crate::application::live_match::step_live_match(&state, ofm_core::state::Career::Active, 1)
            .expect("the match kicks off");

        // The helper reads the completion once while the gate is held, so a call that finishes
        // then has nothing left to report after the release: only the first figure counts here.
        let (while_held, _) = finishes_only_after_the_gate_is_released(state, |state| {
            crate::application::live_match::step_live_match(
                state,
                ofm_core::state::Career::Active,
                1,
            )
            .expect("the match plays a minute");
        });

        assert!(while_held, "a step in play was held up by the operation");
    }
}
