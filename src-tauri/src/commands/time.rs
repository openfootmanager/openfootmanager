use log::info;
use std::sync::Arc;
use tauri::State;

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
pub async fn advance_time_with_mode(
    state: State<'_, Arc<StateManager>>,
    mode: String,
    career_token: u64,
) -> Result<AdvanceTimeWithModeResponse, String> {
    let state = state.inner().clone();
    run_off_thread(move || {
        advance_time_with_mode_internal(&state, Career::Token(career_token), &mode)
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
        .map_err(|err| format!("be.error.taskJoinFailed: {err}"))?
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
pub async fn skip_to_match_day(
    state: State<'_, Arc<StateManager>>,
    career_token: u64,
) -> Result<serde_json::Value, String> {
    let state = state.inner().clone();
    run_off_thread(move || skip_to_match_day_internal(&state, Career::Token(career_token))).await
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
    Ok(serde_json::json!({
        "action": "arrived",
        "game": game,
        "days_skipped": days_skipped,
        "results": results
    }))
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
pub async fn advance_one_day(
    state: State<'_, Arc<StateManager>>,
    career_token: u64,
) -> Result<serde_json::Value, String> {
    let state = state.inner().clone();
    run_off_thread(move || advance_one_day_internal(&state, Career::Token(career_token))).await
}

fn count_high_priority_messages(game: &Game) -> usize {
    game.messages
        .iter()
        .filter(|message| message.priority == domain::message::MessagePriority::High)
        .count()
}

/// Whether the day just processed produced something the user should pause on:
/// a transfer deadline, the window opening, or a fresh high-priority inbox item.
/// (Match days and blockers are handled separately in the loop.)
fn continue_reached_attention_event(
    transfer_status: &domain::season::TransferWindowStatus,
    opens_on: Option<&str>,
    today: &str,
    high_priority_before: usize,
    high_priority_after: usize,
) -> bool {
    *transfer_status == domain::season::TransferWindowStatus::DeadlineDay
        || opens_on == Some(today)
        || high_priority_after > high_priority_before
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

            let high_priority_before = count_high_priority_messages(game);

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
                high_priority_before,
                count_high_priority_messages(game),
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
            0,
            0,
        ));
        // The window's opening day stops.
        assert!(continue_reached_attention_event(
            &TransferWindowStatus::Open,
            Some("2026-07-01"),
            "2026-07-01",
            0,
            0,
        ));
        // A freshly arrived high-priority message stops.
        assert!(continue_reached_attention_event(
            &TransferWindowStatus::Closed,
            None,
            "2026-07-05",
            1,
            2,
        ));
        // A quiet day mid-window with no new mail keeps rolling.
        assert!(!continue_reached_attention_event(
            &TransferWindowStatus::Open,
            Some("2026-07-01"),
            "2026-07-10",
            3,
            3,
        ));
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

        let refused =
            tauri::async_runtime::block_on(super::advance_one_day(app.state(), app.token + 1));

        assert_eq!(refused, Err("be.error.careerChanged".to_string()));
        assert_eq!(date(&app), before);
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
}
