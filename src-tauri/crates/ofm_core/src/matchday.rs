//! One path through a day on which the user's club plays.
//!
//! A live-match day is not `process_day`: the user's own fixture is played separately, and the rest
//! of the day happens around it. That sequence used to live only in the Tauri application layer, in
//! two near-copies — and [`crate::turn::finish_live_match_day_with_capture`] is only its *tail*.
//! Driving the tail alone leaves the user's fixture `Scheduled`, so the sweep plays it through the
//! batch engine instead of the live one, which is a path no real caller takes.
//!
//! So it lives here, where the season harness and the application layer can both call it rather
//! than each keeping their own version. #608 is what that costs: two copies of a day's steps
//! drifted, and the divergence was invisible for months because the user's own table stayed right.

use crate::game::Game;
use crate::live_match_manager::{self, MatchMode};
use domain::league::StandingEntry;
use domain::stats::StatsState;
use engine::report::MatchReport;

/// What a played matchday hands back to the caller.
pub struct UserMatchdayOutcome {
    /// The user's own match, for callers that persist or display it.
    pub report: MatchReport,
    /// The user's league table and round number as they stood *before* the day, for the round
    /// digest. Captured at session creation, because by the time the day is finished the round has
    /// been played and every delta would read as zero.
    pub league_round_context: Option<(u32, Vec<StandingEntry>)>,
}

/// The user's league round as it stands right now: the matchday of its fixture due today, and its
/// table before that round is played.
///
/// Deliberately the user's league and not the competition being played — on a cup day the digest
/// still describes the league round, and a knockout cup has no table to take a baseline from.
///
/// `user_league` and not `user_competition`, which falls back to any competition the club is in: for
/// a club that plays cup football only that returned the *cup*, and the digest then took a cup round
/// number and a knockout's empty standings as its league baseline. The right answer there is no
/// digest at all, which is what `None` gives.
///
/// The legacy mirror is a fallback only when there are no competitions at all — a save written
/// before `competitions` existed, where the mirror *is* the league. Reaching for it whenever no
/// league is found would hand a cup-only club whatever the mirror happened to hold.
pub fn user_league_round_context(game: &Game) -> Option<(u32, Vec<StandingEntry>)> {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let league = game.user_league().or_else(|| {
        game.competitions
            .is_empty()
            .then_some(game.league.as_ref())
            .flatten()
    })?;
    let matchday = league
        .fixtures
        .iter()
        .find(|fixture| fixture.date == today)
        .map(|fixture| fixture.matchday)?;

    Some((matchday, league.standings.clone()))
}

/// Whether a fixture may go to extra time, i.e. whether it is a knockout tie.
///
/// Indexes the competition currently in the legacy slot, which is where both the day-start code and
/// the live session put the fixture's own competition. A league match that goes to penalties is
/// what happens when a caller assumes `true` — see #601.
pub fn fixture_allows_extra_time(game: &Game, fixture_index: usize) -> bool {
    game.league
        .as_ref()
        .and_then(|league| {
            league
                .fixtures
                .get(fixture_index)
                .map(|fixture| league.is_knockout_fixture(&fixture.id))
        })
        .unwrap_or(false)
}

/// Play the user's fixture to completion and finish the day around it.
///
/// Covers the whole sequence for a match that resolves in one call: swap the competition into the
/// legacy slot, create the session, simulate the rest of the round, apply the user's result, restore
/// the competition, and run the day's tail. This is what the delegate/instant path does and what a
/// season harness needs.
///
/// `competition_index` is `None` when there is no competition to name — a save written before
/// `competitions` existed, where the user's league is reachable only through the legacy `game.league`
/// mirror. `Some(index)` must name a competition that exists; one that does not is refused rather
/// than reinterpreted.
///
/// It deliberately does **not** serve `Live`/`Spectator`. Those suspend between session creation and
/// the match being played while the UI drives the minutes, so their sequence belongs to the session
/// lifecycle and cannot be one call.
///
/// The fixture is played with [`MatchMode::Instant`] and no user side. That matters to anything
/// calling this as its "live path": it exercises the day's apply-and-finish sequence on the live
/// engine — benches, both managers, a full report — but not a steppable session, because nobody
/// makes an in-match decision. A harness mode named for this should say so, or it claims coverage of
/// the `Live` lifecycle it does not have.
pub fn play_user_matchday_with_capture<F>(
    game: &mut Game,
    competition_index: Option<usize>,
    fixture_index: usize,
    on_capture: &mut F,
) -> Result<UserMatchdayOutcome, String>
where
    F: FnMut(StatsState),
{
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    // The mirror is swapped in before the session is built, because the kick-off gate and
    // `create_live_match` both read the fixture out of it. So a failure to kick off has to put it
    // back: an empty squad nobody can fill is a handled `Err`, `update_game` mutates the live game
    // in place, and an early return would leave the player's game pointing at another competition
    // entirely — with no match played to explain why their fixture list changed. Players the gate
    // brought in before the refusal stay: the club needed them either way.
    let mirror_before_the_swap = game.league.clone();
    if let Some(competition_index) = competition_index {
        // `None` means "the mirror already holds the fixture" — a save written before `competitions`
        // existed. `Some` names a competition, and naming one that is not there is a caller error:
        // the old signature took a bare index and encoded "none" as one past the end, so a genuinely
        // wrong index was indistinguishable from the legacy case and played the day out of whatever
        // the mirror happened to hold. A stranger's fixture, reported `Ok`.
        let Some(competition) = game.competitions.get(competition_index).cloned() else {
            return Err(live_match_manager::LIVE_MATCH_FIXTURE_NOT_FOUND_ERROR.to_string());
        };
        game.league = Some(competition);
    }

    let allows_extra_time = fixture_allows_extra_time(game, fixture_index);
    let mut session = match live_match_manager::kick_off_live_match(
        game,
        fixture_index,
        MatchMode::Instant,
        allows_extra_time,
    ) {
        Ok(session) => session,
        Err(error) => {
            game.league = mirror_before_the_swap;
            return Err(error);
        }
    };
    session.user_side = None;
    let league_round_context = session.league_round_context.clone();
    session.run_to_completion();

    let home_team_id = session.home_team_id.clone();
    let away_team_id = session.away_team_id.clone();
    let report = session.match_state.into_report();

    crate::turn::simulate_other_matches_with_capture(game, &today, Some(fixture_index), on_capture);

    crate::turn::apply_match_report_with_capture(
        game,
        fixture_index,
        &home_team_id,
        &away_team_id,
        &report,
        on_capture,
    );

    if let Some(competition_index) = competition_index
        && let Some(updated_competition) = game.league.take()
    {
        game.competitions[competition_index] = updated_competition;
        game.sync_legacy_league();
    }

    crate::turn::finish_live_match_day_with_capture(game, on_capture);

    Ok(UserMatchdayOutcome {
        report,
        league_round_context,
    })
}

#[cfg(test)]
mod tests {
    use super::user_league_round_context;
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::league::{
        CompetitionType, Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry,
    };
    use domain::manager::Manager;

    const TODAY: &str = "2036-05-01";

    fn game_managing(team_id: &str) -> Game {
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire(team_id.to_string());
        let clock = GameClock::new(Utc.with_ymd_and_hms(2036, 5, 1, 12, 0, 0).unwrap());
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    fn competition(
        id: &str,
        kind: CompetitionType,
        participants: &[&str],
        matchday: u32,
    ) -> League {
        let mut competition = League::new(
            id.to_string(),
            id.to_string(),
            2036,
            &participants
                .iter()
                .map(|id| (*id).to_string())
                .collect::<Vec<_>>(),
        );
        competition.kind = kind;
        competition.standings = participants
            .iter()
            .map(|id| StandingEntry::new((*id).to_string()))
            .collect();
        competition.fixtures = vec![Fixture {
            id: format!("{id}-f1"),
            competition_id: id.to_string(),
            matchday,
            date: TODAY.to_string(),
            home_team_id: participants[0].to_string(),
            away_team_id: participants[1].to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
        }];
        competition
    }

    /// **Given** the user's club plays in a league, **when** the round context is asked for,
    /// **then** it is that league's round and table.
    #[test]
    fn the_baseline_is_the_users_own_league() {
        let mut game = game_managing("eng-00");
        game.competitions = vec![
            competition("cup", CompetitionType::Cup, &["eng-00", "eng-01"], 7),
            competition("eng-d1", CompetitionType::League, &["eng-00", "eng-01"], 32),
        ];

        let (matchday, standings) =
            user_league_round_context(&game).expect("the user is in a league");

        assert_eq!(matchday, 32, "the league's round, not the cup's");
        assert_eq!(standings.len(), 2);
    }

    /// **Given** the user's club is in a cup and no league at all, **when** the round context is
    /// asked for, **then** there is none.
    ///
    /// `user_competition` falls back to any competition the club is in, so this used to answer with
    /// the *cup*: a cup round number presented as a league matchday, and a knockout's standings —
    /// empty, or the seeded positions of whoever was drawn — as the table the digest measures
    /// movement against. Every delta in that digest is meaningless. No league, no digest.
    #[test]
    fn a_club_that_plays_only_cup_football_has_no_league_round() {
        let mut game = game_managing("eng-00");
        game.competitions = vec![
            competition("cup", CompetitionType::Cup, &["eng-00", "eng-01"], 7),
            // A league exists in the world, but the user's club is not in it.
            competition("eng-d1", CompetitionType::League, &["eng-02", "eng-03"], 32),
        ];

        assert!(
            user_league_round_context(&game).is_none(),
            "a cup round is not a league round, and a knockout has no table to be a baseline"
        );
    }

    /// **Given** a save written before `competitions` existed — the league is in the legacy mirror
    /// only — **when** the round context is asked for, **then** the mirror supplies it.
    #[test]
    fn a_pre_competitions_save_takes_its_baseline_from_the_mirror() {
        let mut game = game_managing("eng-00");
        game.league = Some(competition(
            "legacy",
            CompetitionType::League,
            &["eng-00", "eng-01"],
            14,
        ));

        let (matchday, standings) =
            user_league_round_context(&game).expect("the mirror is the only copy this save has");

        assert_eq!(matchday, 14);
        assert_eq!(standings.len(), 2);
    }

    /// **Given** competitions exist but none is the user's league, **when** the round context is
    /// asked for, **then** the legacy mirror is *not* consulted.
    ///
    /// The mirror is a working buffer as well as a legacy home — `turn` swaps competitions through
    /// it — so falling back to it whenever no league is found would hand a cup-only club a baseline
    /// from whatever was last staged there.
    #[test]
    fn the_mirror_is_not_consulted_when_competitions_exist() {
        let mut game = game_managing("eng-00");
        game.competitions = vec![competition(
            "cup",
            CompetitionType::Cup,
            &["eng-00", "eng-01"],
            7,
        )];
        // As the day-start code would have left it.
        game.league = Some(competition(
            "someone-elses-league",
            CompetitionType::League,
            &["eng-02", "eng-03"],
            9,
        ));

        assert!(
            user_league_round_context(&game).is_none(),
            "the mirror is a staging slot here, not this club's league"
        );
    }
}
