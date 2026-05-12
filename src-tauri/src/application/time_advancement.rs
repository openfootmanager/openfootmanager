use log::info;
use serde::{Deserialize, Serialize};

use crate::commands::round_summary::{build_round_summary_dto, RoundSummaryDto};
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
}

fn round_context_for_today(
    game: &Game,
    today: &str,
) -> Option<(u32, Vec<domain::league::StandingEntry>)> {
    let league = game.league.as_ref()?;
    let matchday = league
        .fixtures
        .iter()
        .find(|fixture| fixture.date == today)
        .map(|fixture| fixture.matchday)?;

    Some((matchday, league.standings.clone()))
}

fn scheduled_user_fixture_index(game: &Game, today: &str) -> Option<usize> {
    let user_team_id = game.manager.team_id.as_ref()?;
    let league = game.league.as_ref()?;

    league
        .fixtures
        .iter()
        .enumerate()
        .find_map(|(index, fixture)| {
            if fixture.date == today
                && fixture.status == domain::league::FixtureStatus::Scheduled
                && (fixture.home_team_id == *user_team_id || fixture.away_team_id == *user_team_id)
            {
                Some(index)
            } else {
                None
            }
        })
}

pub fn advance_time_with_mode(
    state: &StateManager,
    mode: &str,
) -> Result<AdvanceTimeWithModeResponse, String> {
    info!("[cmd] advance_time_with_mode: mode={}", mode);
    let mut game = state
        .get_game(|current_game| current_game.clone())
        .ok_or("be.error.noActiveGameSession")?;

    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let round_context = round_context_for_today(&game, &today);
    let user_fixture_idx = scheduled_user_fixture_index(&game, &today);

    info!(
        "[cmd] advance_time_with_mode: date={}, user_team_id={:?}, user_fixture_idx={:?}",
        today, game.manager.team_id, user_fixture_idx
    );

    match (mode, user_fixture_idx) {
        ("live" | "spectator", Some(index)) => {
            let match_mode = if mode == "live" {
                MatchMode::Live
            } else {
                MatchMode::Spectator
            };
            let session = live_match_manager::create_live_match(&game, index, match_mode, false)?;
            let snapshot = session.snapshot();
            info!(
                "[cmd] advance_time_with_mode: live_match fixture_idx={}, phase={:?}, home_team={}, away_team={}",
                index,
                snapshot.phase,
                snapshot.home_team.name,
                snapshot.away_team.name
            );
            state.set_live_match(session);

            let mut captures = Vec::new();
            ofm_core::turn::simulate_other_matches_with_capture(
                &mut game,
                &today,
                Some(index),
                &mut |capture| captures.push(capture),
            );
            for capture in captures {
                state.append_stats_state(capture);
            }
            let round_summary =
                round_context
                    .as_ref()
                    .and_then(|(matchday, previous_standings)| {
                        build_round_summary_dto(&game, *matchday, previous_standings)
                    });
            state.set_game(game);

            Ok(AdvanceTimeWithModeResponse {
                action: "live_match".to_string(),
                game: None,
                snapshot: Some(snapshot),
                fixture_index: Some(index),
                mode: Some(mode.to_string()),
                round_summary,
            })
        }
        ("delegate", Some(index)) => {
            info!(
                "[cmd] advance_time_with_mode: delegate fixture_idx={}, date={}",
                index, today
            );
            let mut session =
                live_match_manager::create_live_match(&game, index, MatchMode::Instant, false)?;
            session.user_side = None;
            session.run_to_completion();

            let home_team_id = session.home_team_id.clone();
            let away_team_id = session.away_team_id.clone();
            let report = session.match_state.into_report();

            let mut captures = Vec::new();
            ofm_core::turn::simulate_other_matches_with_capture(
                &mut game,
                &today,
                Some(index),
                &mut |capture| captures.push(capture),
            );

            ofm_core::turn::apply_match_report_with_capture(
                &mut game,
                index,
                &home_team_id,
                &away_team_id,
                &report,
                &mut |capture| captures.push(capture),
            );

            for capture in captures {
                state.append_stats_state(capture);
            }

            let round_summary =
                round_context
                    .as_ref()
                    .and_then(|(matchday, previous_standings)| {
                        build_round_summary_dto(&game, *matchday, previous_standings)
                    });

            ofm_core::turn::finish_live_match_day(&mut game);
            state.set_game(game.clone());

            Ok(AdvanceTimeWithModeResponse {
                action: "advanced".to_string(),
                game: Some(game),
                snapshot: None,
                fixture_index: None,
                mode: None,
                round_summary,
            })
        }
        _ => {
            info!(
                "[cmd] advance_time_with_mode: normal_advance date={}, mode={}",
                today, mode
            );
            let mut captures = Vec::new();
            ofm_core::turn::process_day_with_capture(&mut game, &mut |capture| {
                captures.push(capture);
            });
            for capture in captures {
                state.append_stats_state(capture);
            }
            let round_summary =
                round_context
                    .as_ref()
                    .and_then(|(matchday, previous_standings)| {
                        build_round_summary_dto(&game, *matchday, previous_standings)
                    });
            state.set_game(game.clone());

            Ok(AdvanceTimeWithModeResponse {
                action: "advanced".to_string(),
                game: Some(game),
                snapshot: None,
                fixture_index: None,
                mode: None,
                round_summary,
            })
        }
    }
}
