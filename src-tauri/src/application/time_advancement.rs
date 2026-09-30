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
    #[serde(default)]
    pub competition_id: Option<String>,
    #[serde(default)]
    pub fixture_id: Option<String>,
    pub mode: Option<String>,
    pub round_summary: Option<RoundSummaryDto>,
    /// Matches finished during this advance (user's competitions + nationals).
    #[serde(default)]
    pub results: Vec<AdvanceMatchResult>,
}

/// Where the user's fixture due today is: which competition to swap into the legacy slot, and the
/// fixture's index within it.
///
/// The competition is an `Option` because for a save written before `competitions` existed there is
/// no competition to name — the fixture is reachable only through the legacy `game.league` mirror,
/// and the answer is "leave the mirror alone". That used to be encoded as an index one past the end
/// of `competitions`, which read as an out-of-range bug at every call site and stopped anyone
/// guarding against a genuinely bad index.
fn scheduled_user_fixture_index(game: &Game, today: &str) -> Option<(Option<usize>, usize)> {
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
            return Some((Some(competition_index), fixture_index));
        }
    }
    // Fall back to the legacy `game.league` mirror, for saves written before
    // competitions existed. When the mirror *is* one of the competitions, name
    // it — not competition zero: the caller replaces `game.league` with whatever
    // it finds there, so a hardcoded zero handed the user a stranger's fixture.
    // When it is not, there is nothing to name, and `None` says so: the mirror
    // already holds the fixture and must be left as it is.
    let league = game.league.as_ref()?;
    let mirror_index = game
        .competitions
        .iter()
        .position(|competition| competition.id == league.id);
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
            let round_context = ofm_core::matchday::user_league_round_context(game);
            let user_fixture = scheduled_user_fixture_index(game, &today);

            info!(
                "[cmd] advance_time_with_mode: date={}, user_team_id={:?}, user_fixture={:?}",
                today, game.manager.team_id, user_fixture
            );

            match (mode, user_fixture) {
                ("live" | "spectator", Some((competition_index, index))) => {
                    // Same hazard as the delegate path: the mirror is swapped before the session is
                    // built, and `create_live_match` can refuse — an empty squad is a handled `Err`.
                    // This closure runs inside `update_game`, which mutates the live game in place,
                    // so returning without putting the mirror back leaves the player looking at
                    // another competition with no match to explain it.
                    let mirror_before_the_swap = game.league.clone();
                    if let Some(index) = competition_index {
                        let Some(competition) = game.competitions.get(index).cloned() else {
                            return Err("be.error.liveMatch.fixtureNotFound".to_string());
                        };
                        game.league = Some(competition);
                    }
                    let match_mode = if mode == "live" {
                        MatchMode::Live
                    } else {
                        MatchMode::Spectator
                    };
                    let allows_extra_time = ofm_core::matchday::fixture_allows_extra_time(game, index);
                    let session = match live_match_manager::create_live_match(
                        game,
                        index,
                        match_mode,
                        allows_extra_time,
                    ) {
                        Ok(session) => session,
                        Err(error) => {
                            game.league = mirror_before_the_swap;
                            return Err(error);
                        }
                    };
                    let snapshot = session.snapshot();
                    let competition_id = session.competition_id.clone();
                    let fixture_id = session.fixture_id.clone();
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
                    if let Some(competition_index) = competition_index {
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
                        competition_id: Some(competition_id),
                        fixture_id: Some(fixture_id),
                        mode: Some(mode.to_string()),
                        round_summary,
                        results: Vec::new(),
                    })
                }
                ("delegate", Some((competition_index, index))) => {
                    info!(
                        "[cmd] advance_time_with_mode: delegate fixture_idx={}, date={}",
                        index, today
                    );
                    // The whole day, in one call, so the harness and this branch cannot drift.
                    let outcome = ofm_core::matchday::play_user_matchday_with_capture(
                        game,
                        competition_index,
                        index,
                        &mut |capture| captures.push(capture),
                    )?;

                    // The baseline comes from the session, captured before the round was played;
                    // reading the table now would report every delta as zero. Built after the day
                    // so it describes the round the response is carrying.
                    let round_summary = outcome
                        .league_round_context
                        .or(round_context)
                        .and_then(|(matchday, previous_standings)| {
                            build_round_summary_dto(game, matchday, &previous_standings)
                        });
                    let results = collect_advance_results(game, &today);

                    Ok(AdvanceTimeWithModeResponse {
                        action: "advanced".to_string(),
                        game: Some(game.clone()),
                        snapshot: None,
                        fixture_index: None,
                        competition_id: None,
                        fixture_id: None,
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
                        competition_id: None,
                        fixture_id: None,
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
    use super::{advance_time_with_mode, scheduled_user_fixture_index};
    use chrono::{TimeZone, Utc};
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League};
    use domain::manager::Manager;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::StateManager;

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
        let competition_index = competition_index.expect("the mirror is one of the competitions");
        assert_eq!(
            game.competitions[competition_index].id, "eng-d2",
            "the index must name the mirror's competition, not competitions[0]"
        );
    }

    /// A save written before competitions existed has only the mirror, so there is no competition
    /// to name and the answer is `None` — which leaves `game.league` alone, correct, because the
    /// mirror already holds the fixture.
    #[test]
    fn a_mirror_that_is_not_a_competition_names_no_competition() {
        let ours = league(
            "legacy-league",
            vec![scheduled("f1", "2036-05-01", "eng-00", "eng-01")],
        );
        let game = game_with(Vec::new(), Some(ours));

        let (competition_index, _) =
            scheduled_user_fixture_index(&game, "2036-05-01").expect("the user plays today");

        assert_eq!(
            competition_index, None,
            "nothing to replace the mirror with, so the mirror stands"
        );
    }

    /// **Given** the user's fixture sits in a competition whose clubs have nobody available,
    /// **when** a live match is started for it,
    /// **then** the call fails and `game.league` still holds what it held before.
    ///
    /// This branch swaps the competition into the legacy slot before building the session, because
    /// that is where `create_live_match` reads the fixture from — and it can refuse, a side with
    /// nobody available being a handled `Err` rather than a panic. The closure runs inside
    /// `update_game`, which hands out `&mut Game` and keeps every mutation whether the closure
    /// returns `Ok` or `Err`. So an early return without restoring leaves the player's own game
    /// pointing at another competition, with no match played to explain why.
    ///
    /// The delegate path has the same shape and its own test in `ofm_core`; this pins the one the
    /// reviewer's probe did not reach. Dropping the restore here left all twenty-five app-crate
    /// tests green.
    #[test]
    fn a_live_match_that_cannot_start_leaves_the_mirror_as_it_found_it() {
        let ours = league(
            "eng-d1",
            vec![scheduled("f1", "2036-05-01", "eng-00", "eng-01")],
        );
        // What the day-start code happened to leave staged in the legacy slot.
        let staged = league("staged-elsewhere", Vec::new());
        // No players anywhere in the world, so `create_live_match` refuses.
        let game = game_with(vec![ours], Some(staged));

        let state = StateManager::new();
        state.set_game(game);

        let outcome = advance_time_with_mode(&state, "live");

        assert!(
            outcome.is_err(),
            "a side with nobody available cannot play, and that is an Err rather than a panic"
        );
        let mirror = state
            .get_game(|game| game.league.as_ref().map(|league| league.id.clone()))
            .flatten();
        assert_eq!(
            mirror.as_deref(),
            Some("staged-elsewhere"),
            "the mirror is restored, so the game the player returns to is the one they left"
        );
    }
}
