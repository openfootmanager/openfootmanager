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
    let session = state
        .take_live_match()
        .ok_or("be.error.noActiveLiveMatch")?;

    let fixture_index = session.fixture_index;
    let competition_id = session.competition_id.clone();
    let round_matchday = session.round_matchday;
    let round_previous_standings = session.round_previous_standings.clone();
    let home_team_id = session.home_team_id.clone();
    let away_team_id = session.away_team_id.clone();

    let report = session.match_state.into_report();
    info!(
        "[cmd] finish_live_match: fixture_index={}, competition_id={}, home_team_id={}, away_team_id={}, events= {}",
        fixture_index,
        competition_id,
        home_team_id,
        away_team_id,
        report.events.len()
    );

    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession")?;

    // `fixture_index` indexes the fixtures of the competition the session was
    // created for (possibly a cup). `game.league` was reset to the user's
    // domestic league by sync_legacy_league when the match day started, so
    // applying the report through it would write the result onto an unrelated
    // fixture of the wrong competition (or panic on an out-of-range index).
    // Swap the session's competition back in first.
    if let Some(idx) = game
        .competitions
        .iter()
        .position(|c| c.id == competition_id)
    {
        game.league = Some(game.competitions[idx].clone());
    } else if !game.competitions.is_empty() {
        // The session's competition no longer exists; applying by index to
        // whatever game.league holds would corrupt an unrelated fixture.
        return Err("be.error.liveMatch.fixtureNotFound".to_string());
    }
    // Legacy saves (no competitions) keep game.league, which the session was
    // created against.
    let fixture_count = game.league.as_ref().map_or(0, |l| l.fixtures.len());
    if fixture_index >= fixture_count {
        return Err("be.error.liveMatch.fixtureNotFound".to_string());
    }

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

    // apply_match_report_with_capture mutates the legacy `game.league` mirror
    // (fixture status, fixture.result, standings). The modern `game.competitions`
    // is the source of truth, and finish_live_match_day's sync_legacy_league
    // would otherwise overwrite our changes with the stale competition copy.
    if let Some(league) = game.league.clone() {
        if let Some(idx) = game.competitions.iter().position(|c| c.id == league.id) {
            game.competitions[idx] = league;
        }
    }
    // Restore the legacy mirror to the user's domestic league before the rest
    // of the day runs (legacy saves without competitions keep game.league).
    if !game.competitions.is_empty() {
        game.sync_legacy_league();
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
    home_team_id: Option<&str>,
    away_team_id: Option<&str>,
) -> Result<engine::MatchSnapshot, String> {
    info!(
        "[cmd] start_live_match: fixture={}, mode={}, extra_time={}, teams={:?}/{:?}",
        fixture_index, mode, allows_extra_time, home_team_id, away_team_id
    );
    let mut game = state
        .get_game(|g| g.clone())
        .ok_or("be.error.noActiveGameSession")?;

    let match_mode = match mode {
        "spectator" => MatchMode::Spectator,
        "instant" => MatchMode::Instant,
        _ => MatchMode::Live,
    };

    // Session restore after an app restart: `game.league` mirrors the user's
    // domestic league, but the fixture being restored may belong to another
    // competition (a cup). When the caller identifies the fixture by its
    // teams, resolve today's scheduled fixture across all competitions and
    // swap its competition into the local `game.league` so the session is
    // created against — and later applied to — the right competition.
    let mut fixture_index = fixture_index;
    if let (Some(home_id), Some(away_id)) = (home_team_id, away_team_id) {
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        let resolved = game
            .competitions
            .iter()
            .enumerate()
            .find_map(|(competition_index, competition)| {
                competition
                    .fixtures
                    .iter()
                    .position(|fixture| {
                        fixture.date == today
                            && fixture.status == domain::league::FixtureStatus::Scheduled
                            && fixture.home_team_id == home_id
                            && fixture.away_team_id == away_id
                    })
                    .map(|index| (competition_index, index))
            });
        if let Some((competition_index, index)) = resolved {
            game.league = Some(game.competitions[competition_index].clone());
            fixture_index = index;
        }
        // No match: fall back to the caller-supplied index into game.league.
    }

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
