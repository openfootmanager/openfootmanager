use log::info;
use serde::{Deserialize, Serialize};

use crate::commands::round_summary::{build_round_summary_dto, RoundSummaryDto};
use ofm_core::advance_results::{collect_advance_results, AdvanceMatchResult};
use ofm_core::game::Game;
use ofm_core::live_match_manager::{self, MatchMode};
use ofm_core::state::StateManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvanceTimeWithModeResponse {
    pub action: String,
    pub game: Option<Game>,
    pub snapshot: Option<engine::MatchSnapshot>,
    pub fixture_index: Option<usize>,
    pub mode: Option<String>,
    pub round_summary: Option<RoundSummaryDto>,
    /// Matches finished during this advance (user's competitions + nationals).
    #[serde(default)]
    pub results: Vec<AdvanceMatchResult>,
}

fn round_context_for_today(
    game: &Game,
    today: &str,
) -> Option<(u32, Vec<domain::league::StandingEntry>)> {
    let league = game.primary_competition()?;
    let matchday = league
        .fixtures
        .iter()
        .find(|fixture| fixture.date == today)
        .map(|fixture| fixture.matchday)?;

    Some((matchday, league.standings.clone()))
}

fn scheduled_user_fixture_index(game: &Game, today: &str) -> Option<(usize, usize)> {
    let user_team_id = game.manager.team_id.as_ref()?;
    for (competition_index, competition) in game.competitions.iter().enumerate() {
        if !game.active_competition_ids.is_empty()
            && !game.active_competition_ids.contains(&competition.id)
        {
            continue;
        }
        if let Some(fixture_index) = competition.fixtures.iter().enumerate().find_map(|(index, fixture)| {
            if fixture.date == today
                && fixture.status == domain::league::FixtureStatus::Scheduled
                && (fixture.home_team_id == *user_team_id || fixture.away_team_id == *user_team_id)
            {
                Some(index)
            } else {
                None
            }
        }) {
            return Some((competition_index, fixture_index));
        }
    }
    let league = game.league.as_ref()?;
    league.fixtures.iter().enumerate().find_map(|(index, fixture)| {
        if fixture.date == today
            && fixture.status == domain::league::FixtureStatus::Scheduled
            && (fixture.home_team_id == *user_team_id || fixture.away_team_id == *user_team_id)
        {
            Some((0, index))
        } else {
            None
        }
    })
}

/// Knockout ties get extra time (and, if still level, a shootout) in the live
/// engine; league fixtures end after regulation. `index` is a fixture index
/// into `game.league`, which at the call sites holds the competition being
/// played today.
fn fixture_allows_extra_time(game: &Game, index: usize) -> bool {
    game.league
        .as_ref()
        .and_then(|league| {
            league
                .fixtures
                .get(index)
                .map(|fixture| league.is_knockout_fixture(&fixture.id))
        })
        .unwrap_or(false)
}

pub fn advance_time_with_mode(
    state: &StateManager,
    mode: &str,
) -> Result<AdvanceTimeWithModeResponse, String> {
    info!("[cmd] advance_time_with_mode: mode={}", mode);

    // The whole day-start sequence runs under the game lock (update_game) so a
    // concurrent GUI/MCP mutation is never clobbered by writing back a stale
    // whole-game clone. Stats captures are appended and the live session is
    // stored only after the game lock is released, so the two state mutexes
    // are never held at once.
    let mut captures = Vec::new();
    let mut session_out: Option<live_match_manager::LiveMatchSession> = None;
    let response = state
        .update_game(|game| -> Result<AdvanceTimeWithModeResponse, String> {
            let today = game.clock.current_date.format("%Y-%m-%d").to_string();
            let round_context = round_context_for_today(game, &today);
            let user_fixture = scheduled_user_fixture_index(game, &today);

            info!(
                "[cmd] advance_time_with_mode: date={}, user_team_id={:?}, user_fixture={:?}",
                today, game.manager.team_id, user_fixture
            );

            match (mode, user_fixture) {
                ("live" | "spectator", Some((competition_index, index))) => {
                    if let Some(competition) = game.competitions.get(competition_index).cloned() {
                        game.league = Some(competition);
                    }
                    let match_mode = if mode == "live" {
                        MatchMode::Live
                    } else {
                        MatchMode::Spectator
                    };
                    let allows_extra_time = fixture_allows_extra_time(game, index);
                    let session = live_match_manager::create_live_match(
                        game,
                        index,
                        match_mode,
                        allows_extra_time,
                    )?;
                    let snapshot = session.snapshot();
                    info!(
                        "[cmd] advance_time_with_mode: live_match fixture_idx={}, phase={:?}, home_team={}, away_team={}",
                        index,
                        snapshot.phase,
                        snapshot.home_team.name,
                        snapshot.away_team.name
                    );
                    session_out = Some(session);

                    ofm_core::turn::simulate_other_matches_with_capture(
                        game,
                        &today,
                        Some(index),
                        &mut |capture| captures.push(capture),
                    );
                    if competition_index < game.competitions.len() {
                        if let Some(updated_competition) = game.league.take() {
                            game.competitions[competition_index] = updated_competition;
                            game.sync_legacy_league();
                        }
                    }
                    let round_summary =
                        round_context
                            .as_ref()
                            .and_then(|(matchday, previous_standings)| {
                                build_round_summary_dto(game, *matchday, previous_standings)
                            });

                    Ok(AdvanceTimeWithModeResponse {
                        action: "live_match".to_string(),
                        game: None,
                        snapshot: Some(snapshot),
                        fixture_index: Some(index),
                        mode: Some(mode.to_string()),
                        round_summary,
                        results: Vec::new(),
                    })
                }
                ("delegate", Some((competition_index, index))) => {
                    if let Some(competition) = game.competitions.get(competition_index).cloned() {
                        game.league = Some(competition);
                    }
                    info!(
                        "[cmd] advance_time_with_mode: delegate fixture_idx={}, date={}",
                        index, today
                    );
                    let allows_extra_time = fixture_allows_extra_time(game, index);
                    let mut session = live_match_manager::create_live_match(
                        game,
                        index,
                        MatchMode::Instant,
                        allows_extra_time,
                    )?;
                    session.user_side = None;
                    session.run_to_completion();

                    let home_team_id = session.home_team_id.clone();
                    let away_team_id = session.away_team_id.clone();
                    let report = session.match_state.into_report();

                    ofm_core::turn::simulate_other_matches_with_capture(
                        game,
                        &today,
                        Some(index),
                        &mut |capture| captures.push(capture),
                    );

                    ofm_core::turn::apply_match_report_with_capture(
                        game,
                        index,
                        &home_team_id,
                        &away_team_id,
                        &report,
                        &mut |capture| captures.push(capture),
                    );
                    if competition_index < game.competitions.len() {
                        if let Some(updated_competition) = game.league.take() {
                            game.competitions[competition_index] = updated_competition;
                            game.sync_legacy_league();
                        }
                    }

                    let round_summary =
                        round_context
                            .as_ref()
                            .and_then(|(matchday, previous_standings)| {
                                build_round_summary_dto(game, *matchday, previous_standings)
                            });

                    ofm_core::turn::finish_live_match_day(game);
                    let results = collect_advance_results(game, &today);

                    Ok(AdvanceTimeWithModeResponse {
                        action: "advanced".to_string(),
                        game: Some(game.clone()),
                        snapshot: None,
                        fixture_index: None,
                        mode: None,
                        round_summary,
                        results,
                    })
                }
                _ => {
                    info!(
                        "[cmd] advance_time_with_mode: normal_advance date={}, mode={}",
                        today, mode
                    );
                    ofm_core::turn::process_day_with_capture(game, &mut |capture| {
                        captures.push(capture);
                    });
                    let round_summary =
                        round_context
                            .as_ref()
                            .and_then(|(matchday, previous_standings)| {
                                build_round_summary_dto(game, *matchday, previous_standings)
                            });
                    let results = collect_advance_results(game, &today);

                    Ok(AdvanceTimeWithModeResponse {
                        action: "advanced".to_string(),
                        game: Some(game.clone()),
                        snapshot: None,
                        fixture_index: None,
                        mode: None,
                        round_summary,
                        results,
                    })
                }
            }
        })
        .ok_or("be.error.noActiveGameSession")??;

    for capture in captures {
        state.append_stats_state(capture);
    }
    if let Some(session) = session_out {
        state.set_live_match(session);
    }

    Ok(response)
}
