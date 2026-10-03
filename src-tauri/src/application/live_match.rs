use log::info;

use crate::commands::round_summary::{build_round_summary_dto, RoundSummaryDto};
use domain::league::FixtureStatus;
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
    let mut session = state
        .take_live_match()
        .ok_or("be.error.noActiveLiveMatch")?;

    // The GUI only finishes after FullTime, but MCP's match_finish can be
    // called mid-match — persisting a partial score (or a half-taken
    // shootout) as the final result. Run the remainder instantly so the
    // report always describes a completed match.
    if !session.is_finished() {
        info!("[cmd] finish_live_match: match not finished, running to completion");
        session.run_to_completion();
    }

    let fixture_index = session.fixture_index;
    let fixture_id = session.fixture_id.clone();
    let competition_id = session.competition_id.clone();
    let league_round_context = session.league_round_context.clone();
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

    // Apply the result under the game lock (update_game) so a concurrent
    // GUI/MCP write is never clobbered by a stale whole-game clone. Stats
    // captures are appended after the lock is released.
    let mut captures = Vec::new();
    let (game, round_summary) = state
        .update_game(|game| -> Result<(Game, Option<RoundSummaryDto>), String> {
            // Check the authoritative competition before changing the legacy
            // mirror. A stale session must not replay a completed fixture or
            // leave the mirror pointing at a cup when finish is rejected.
            let competition_index = game
                .competitions
                .iter()
                .position(|c| c.id == competition_id);
            if competition_index.is_none() && !game.competitions.is_empty() {
                return Err("be.error.liveMatch.fixtureNotFound".to_string());
            }
            let league = competition_index
                .map(|idx| &game.competitions[idx])
                .or(game.league.as_ref());
            let fixture = league
                .and_then(|league| league.fixtures.get(fixture_index))
                .ok_or("be.error.liveMatch.fixtureNotFound")?;
            if fixture.id != fixture_id
                || fixture.home_team_id != home_team_id
                || fixture.away_team_id != away_team_id
            {
                return Err("be.error.liveMatch.fixtureNotFound".to_string());
            }
            if fixture.status != FixtureStatus::Scheduled {
                return Err("be.error.liveMatch.fixtureNotScheduled".to_string());
            }

            // `fixture_index` belongs to the session's competition (possibly
            // a cup), while game.league may mirror the domestic league. Swap
            // only after the fixture has passed validation.
            if let Some(idx) = competition_index {
                game.league = Some(game.competitions[idx].clone());
            }

            ofm_core::turn::apply_match_report_with_capture(
                game,
                fixture_index,
                &home_team_id,
                &away_team_id,
                &report,
                &mut |capture| captures.push(capture),
            );

            // apply_match_report_with_capture mutates the legacy `game.league`
            // mirror (fixture status, fixture.result, standings). The modern
            // `game.competitions` is the source of truth, and
            // finish_live_match_day's sync_legacy_league would otherwise
            // overwrite our changes with the stale competition copy.
            if let Some(league) = game.league.clone() {
                if let Some(idx) = game.competitions.iter().position(|c| c.id == league.id) {
                    game.competitions[idx] = league;
                }
            }
            // Restore the legacy mirror to the user's domestic league before
            // the rest of the day runs (legacy saves without competitions
            // keep game.league).
            if !game.competitions.is_empty() {
                game.sync_legacy_league();
            }

            // The digest describes the user's league round, and `build_round_summary` reads the
            // competition `sync_legacy_league` has just mirrored — so its context has to come
            // from that same competition. The session's `round_matchday` and
            // `round_previous_standings` describe whatever the user *played*, which on a cup day
            // is the cup: a knockout cup has no table at all, so its standings would be an empty
            // baseline for the league's deltas, and its matchday would select the wrong round.
            //
            // It also has to be the table as it stood *before* the round. The GUI simulates the
            // rest of the round right after creating the session, so reading it here instead
            // reports every club's delta as zero — the round has already happened. So the
            // session carries the league baseline from creation time, and today's context is only
            // the fallback for a session that has none.
            let summary_context = league_round_context
                .or_else(|| ofm_core::matchday::user_league_round_context(game));

            ofm_core::turn::finish_live_match_day_with_capture(game, &mut |capture| {
                captures.push(capture)
            });

            // After the sweep, not before: on a cup day with domestic fixtures also due, the
            // summary built beforehand described a round that had not been played yet and came
            // back `None`, so the digest read "unavailable" for a round the response was
            // carrying. The matchday and standings are captured above, so the advanced clock
            // does not affect it.
            let round_summary = summary_context.and_then(|(matchday, previous)| {
                build_round_summary_dto(game, matchday, &previous)
            });

            Ok((game.clone(), round_summary))
        })
        .ok_or("be.error.noActiveGameSession")??;
    for capture in captures {
        state.append_stats_state(capture);
    }

    Ok(FinishLiveMatchResponse {
        game,
        round_summary,
    })
}

#[cfg(any(feature = "mcp", test))]
pub fn start_live_match(
    state: &StateManager,
    fixture_index: usize,
    mode: &str,
    allows_extra_time: bool,
    home_team_id: Option<&str>,
    away_team_id: Option<&str>,
) -> Result<engine::MatchSnapshot, String> {
    if home_team_id.is_some() || away_team_id.is_some() {
        // Team IDs cannot identify a fixture when two competitions pair the
        // same clubs. Session restoration must supply stable identity.
        return Err("be.error.liveMatch.fixtureNotFound".to_string());
    }
    start_live_match_with_identity(state, fixture_index, mode, allows_extra_time, None, None)
}

pub fn start_live_match_with_identity(
    state: &StateManager,
    fixture_index: usize,
    mode: &str,
    _allows_extra_time: bool,
    competition_id: Option<&str>,
    fixture_id: Option<&str>,
) -> Result<engine::MatchSnapshot, String> {
    info!(
        "[cmd] start_live_match: fixture={}, mode={}, competition={:?}, fixture_id={:?}",
        fixture_index, mode, competition_id, fixture_id
    );
    let match_mode = match mode {
        "spectator" => MatchMode::Spectator,
        "instant" => MatchMode::Instant,
        _ => MatchMode::Live,
    };

    // Everything runs under the game lock (update_game) so a concurrent
    // GUI/MCP write is never clobbered; captures and the live session are
    // stored after the lock is released.
    let mut captures = Vec::new();
    let (snapshot, session) = state
        .update_game(|game| -> Result<(engine::MatchSnapshot, live_match_manager::LiveMatchSession), String> {
            // After a restart, game.league may mirror the domestic league
            // while the selected fixture belongs to a cup. Resolve the exact
            // competition and fixture IDs before creating the session.
            let mut fixture_index = fixture_index;
            let mut swapped_league = false;
            match (competition_id, fixture_id) {
                (Some(competition_id), Some(fixture_id)) => {
                    let competition = game
                    .competitions
                    .iter()
                        .find(|competition| competition.id == competition_id)
                        .cloned();
                    if let Some(competition) = competition {
                        fixture_index = competition
                            .fixtures
                            .iter()
                            .position(|fixture| fixture.id == fixture_id)
                            .ok_or("be.error.liveMatch.fixtureNotFound")?;
                        if competition.fixtures[fixture_index].status != FixtureStatus::Scheduled {
                            return Err("be.error.liveMatch.fixtureNotScheduled".to_string());
                        }
                        game.league = Some(competition);
                        swapped_league = true;
                    } else if game.competitions.is_empty() {
                        let league = game.league.as_ref().ok_or("be.error.liveMatch.noLeague")?;
                        if league.id != competition_id {
                            return Err("be.error.liveMatch.fixtureNotFound".to_string());
                        }
                        fixture_index = league
                            .fixtures
                            .iter()
                            .position(|fixture| fixture.id == fixture_id)
                            .ok_or("be.error.liveMatch.fixtureNotFound")?;
                    } else {
                        return Err("be.error.liveMatch.fixtureNotFound".to_string());
                    }
                }
                (None, None) => {} // MCP selects the current mirror by index.
                _ => return Err("be.error.liveMatch.fixtureNotFound".to_string()),
            }

            let league = game
                .league
                .as_ref()
                .ok_or("be.error.liveMatch.noLeague")?;
            let fixture = league
                .fixtures
                .get(fixture_index)
                .ok_or("be.error.liveMatch.fixtureNotFound")?;
            if let Some(competition) = game.competitions.iter().find(|c| c.id == league.id) {
                let current_fixture = competition
                    .fixtures
                    .get(fixture_index)
                    .ok_or("be.error.liveMatch.fixtureNotFound")?;
                if current_fixture.id != fixture.id {
                    return Err("be.error.liveMatch.fixtureNotFound".to_string());
                }
                if current_fixture.status != FixtureStatus::Scheduled {
                    return Err("be.error.liveMatch.fixtureNotScheduled".to_string());
                }
            } else if !game.competitions.is_empty() {
                return Err("be.error.liveMatch.fixtureNotFound".to_string());
            }

            // A caller's flag cannot turn a league draw into a knockout result or suppress
            // a cup decider. Resolve identity first so the shared rule reads this fixture's
            // competition, including cup restores while the domestic league is mirrored.
            let allows_extra_time = ofm_core::matchday::fixture_allows_extra_time(game, fixture_index);
            let session = live_match_manager::kick_off_live_match(
                game,
                fixture_index,
                match_mode,
                allows_extra_time,
            )
            .inspect_err(|_| {
                if swapped_league {
                    game.sync_legacy_league();
                }
            })?;
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

            // Simulate the rest of today's fixtures in this competition, exactly
            // like the GUI match-day path does. Without this, an MCP
            // `match_start` → `match_finish` flow advanced the clock past
            // fixtures that were never played, stranding them Scheduled in the
            // past. simulate_other_matches only touches Scheduled fixtures, so
            // the session-restore path (where they are already Completed) is
            // naturally idempotent.
            let today = game.clock.current_date.format("%Y-%m-%d").to_string();
            let fixture_is_today = game
                .league
                .as_ref()
                .and_then(|league| league.fixtures.get(fixture_index))
                .map(|fixture| fixture.date == today)
                .unwrap_or(false);
            if fixture_is_today {
                ofm_core::turn::simulate_other_matches_with_capture(
                    game,
                    &today,
                    Some(fixture_index),
                    &mut |capture| captures.push(capture),
                );
                if let Some(league) = game.league.clone() {
                    if let Some(idx) = game.competitions.iter().position(|c| c.id == league.id) {
                        game.competitions[idx] = league;
                        game.sync_legacy_league();
                    }
                }
            } else if swapped_league && !game.competitions.is_empty() {
                // The swap now persists (update_game mutates in place); restore
                // the legacy mirror when no simulation ran to do it for us.
                game.sync_legacy_league();
            }

            Ok((snapshot, session))
        })
        .ok_or("be.error.noActiveGameSession")??;
    for capture in captures {
        state.append_stats_state(capture);
    }

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
