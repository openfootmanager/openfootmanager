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
        if let Some(fixture_index) =
            competition
                .fixtures
                .iter()
                .enumerate()
                .find_map(|(index, fixture)| {
                    if fixture.date == today
                        && fixture.status == domain::league::FixtureStatus::Scheduled
                        && (fixture.home_team_id == *user_team_id
                            || fixture.away_team_id == *user_team_id)
                    {
                        Some(index)
                    } else {
                        None
                    }
                })
        {
            return Some((competition_index, fixture_index));
        }
    }
    // Fall back to the legacy `game.league` mirror, for saves written before
    // competitions existed. The index must name the mirror's own competition,
    // not competition zero: the caller replaces `game.league` with whatever it
    // finds there, so a hardcoded zero handed the user a stranger's fixture.
    // When the mirror is not one of the competitions, an index past the end
    // resolves to nothing, which leaves `game.league` as it is — the mirror
    // already holds the fixture.
    let league = game.league.as_ref()?;
    let mirror_index = game
        .competitions
        .iter()
        .position(|competition| competition.id == league.id)
        .unwrap_or(game.competitions.len());
    league
        .fixtures
        .iter()
        .enumerate()
        .find_map(|(index, fixture)| {
            if fixture.date == today
                && fixture.status == domain::league::FixtureStatus::Scheduled
                && (fixture.home_team_id == *user_team_id || fixture.away_team_id == *user_team_id)
            {
                Some((mirror_index, index))
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

#[cfg(test)]
mod tests {
    use super::scheduled_user_fixture_index;
    use chrono::{TimeZone, Utc};
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League};
    use domain::manager::Manager;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;

    fn scheduled(id: &str, date: &str, home: &str, away: &str) -> Fixture {
        Fixture {
            id: id.to_string(),
            matchday: 1,
            date: date.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            ..Default::default()
        }
    }

    fn league(id: &str, fixtures: Vec<Fixture>) -> League {
        League {
            id: id.to_string(),
            name: id.to_string(),
            season: 2036,
            fixtures,
            ..Default::default()
        }
    }

    fn game_with(competitions: Vec<League>, mirror: Option<League>) -> Game {
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("eng-00".to_string());
        let clock = GameClock::new(Utc.with_ymd_and_hms(2036, 5, 1, 12, 0, 0).unwrap());
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        game.competitions = competitions;
        game.league = mirror;
        game
    }

    /// The caller replaces `game.league` with whatever competition the returned
    /// index names, and builds the live match from it. The legacy fallback used
    /// to answer zero regardless, so a player whose fixture was only reachable
    /// through the mirror was handed the first competition in the list — in a
    /// generated world an Argentine league, for an English manager.
    #[test]
    fn the_mirror_fallback_names_the_mirrors_own_competition_not_the_first() {
        let foreign = league("ar-d1-apertura", vec![]);
        let ours = league(
            "eng-d2",
            vec![scheduled("f1", "2036-05-01", "eng-00", "eng-01")],
        );
        // Out of scope, so the scan above the fallback skips it and the mirror
        // is what answers.
        let mut game = game_with(vec![foreign, ours.clone()], Some(ours));
        game.active_competition_ids = vec!["ar-d1-apertura".to_string()];

        let (competition_index, fixture_index) =
            scheduled_user_fixture_index(&game, "2036-05-01").expect("the user plays today");

        assert_eq!(fixture_index, 0);
        assert_eq!(
            game.competitions[competition_index].id, "eng-d2",
            "the index must name the mirror's competition, not competitions[0]"
        );
    }

    /// A save written before competitions existed has only the mirror. An index
    /// past the end resolves to nothing, which leaves `game.league` alone —
    /// correct, because the mirror already holds the fixture.
    #[test]
    fn a_mirror_that_is_not_a_competition_resolves_to_nothing() {
        let ours = league(
            "legacy-league",
            vec![scheduled("f1", "2036-05-01", "eng-00", "eng-01")],
        );
        let game = game_with(Vec::new(), Some(ours));

        let (competition_index, _) =
            scheduled_user_fixture_index(&game, "2036-05-01").expect("the user plays today");

        assert!(
            game.competitions.get(competition_index).is_none(),
            "nothing to replace the mirror with, so the mirror stands"
        );
    }
}
