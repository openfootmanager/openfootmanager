use log::info;

use crate::commands::round_summary::{build_round_summary_dto, RoundSummaryDto};
use ofm_core::game::Game;
use ofm_core::live_match_manager::{self, MatchMode};
use ofm_core::state::StateManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishLiveMatchResponse {
    pub game: Game,
    pub round_summary: Option<RoundSummaryDto>,
}

pub fn finish_live_match(state: &StateManager) -> Result<FinishLiveMatchResponse, String> {
    info!("[cmd] finish_live_match");
    let session = state.take_live_match().ok_or("be.error.noActiveLiveMatch")?;

    let fixture_index = session.fixture_index;
    let round_matchday = session.round_matchday;
    let round_previous_standings = session.round_previous_standings.clone();
    let home_team_id = session.home_team_id.clone();
    let away_team_id = session.away_team_id.clone();

    let report = session.match_state.into_report();
    info!(
        "[cmd] finish_live_match: fixture_index={}, home_team_id={}, away_team_id={}, events= {}",
        fixture_index,
        home_team_id,
        away_team_id,
        report.events.len()
    );

    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession")?;

    let mut captures = Vec::new();
    ofm_core::turn::apply_match_report_with_capture(
        &mut game,
        fixture_index,
        &home_team_id,
        &away_team_id,
        &report,
        &mut |capture| captures.push(capture),
    );
    for capture in captures {
        state.append_stats_state(capture);
    }

    let round_summary = build_round_summary_dto(&game, round_matchday, &round_previous_standings);

    ofm_core::turn::finish_live_match_day(&mut game);

    state.set_game(game.clone());
    Ok(FinishLiveMatchResponse {
        game,
        round_summary,
    })
}

pub fn start_live_match(
    state: &StateManager,
    fixture_index: usize,
    mode: &str,
    allows_extra_time: bool,
) -> Result<engine::MatchSnapshot, String> {
    info!(
        "[cmd] start_live_match: fixture={}, mode={}, extra_time={}",
        fixture_index, mode, allows_extra_time
    );
    let game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession")?;

    let match_mode = match mode {
        "spectator" => MatchMode::Spectator,
        "instant" => MatchMode::Instant,
        _ => MatchMode::Live,
    };

    let session =
        live_match_manager::create_live_match(&game, fixture_index, match_mode, allows_extra_time)?;
    let snapshot = session.snapshot();
    info!(
        "[cmd] start_live_match: created fixture={}, phase={:?}, home_team={}, away_team={}, home_players={}, away_players={}",
        fixture_index,
        snapshot.phase,
        snapshot.home_team.name,
        snapshot.away_team.name,
        snapshot.home_team.players.len(),
        snapshot.away_team.players.len()
    );
    state.set_live_match(session);
    Ok(snapshot)
}

pub fn step_live_match(
    state: &StateManager,
    minutes: u16,
) -> Result<Vec<engine::MinuteResult>, String> {
    log::debug!("[cmd] step_live_match: minutes={}", minutes);
    let results = state
        .with_live_match(|session| {
            if minutes <= 1 {
                vec![session.step()]
            } else {
                session.step_many(minutes)
            }
        })
        .ok_or_else(|| "be.error.noActiveLiveMatch".to_string())?;

    if let Some(last) = results.last() {
        info!(
            "[cmd] step_live_match: minutes={}, result_count={}, last_minute={}, phase={:?}, finished={}",
            minutes,
            results.len(),
            last.minute,
            last.phase,
            last.is_finished
        );
    }

    Ok(results)
}

pub fn apply_match_command(
    state: &StateManager,
    command: engine::MatchCommand,
) -> Result<engine::MatchSnapshot, String> {
    info!("[cmd] apply_match_command: {:?}", command);
    let snapshot = state
        .with_live_match(|session| {
            session.apply_command(command)?;
            Ok::<engine::MatchSnapshot, String>(session.snapshot())
        })
        .ok_or_else(|| "be.error.noActiveLiveMatch".to_string())??;

    info!(
        "[cmd] apply_match_command: snapshot phase={:?}, minute={}, home_players={}, away_players={}",
        snapshot.phase,
        snapshot.current_minute,
        snapshot.home_team.players.len(),
        snapshot.away_team.players.len()
    );

    Ok(snapshot)
}

pub fn get_match_snapshot(state: &StateManager) -> Result<engine::MatchSnapshot, String> {
    log::debug!("[cmd] get_match_snapshot");
    let snapshot = state
        .with_live_match(|session| session.snapshot())
        .ok_or_else(|| "be.error.noActiveLiveMatch".to_string())?;

    info!(
        "[cmd] get_match_snapshot: phase={:?}, minute={}, home_team={}, away_team={}",
        snapshot.phase, snapshot.current_minute, snapshot.home_team.name, snapshot.away_team.name
    );

    Ok(snapshot)
}
