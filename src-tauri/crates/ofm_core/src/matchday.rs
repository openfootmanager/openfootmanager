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
/// Deliberately the user's competition and not the one being played — on a cup day the digest still
/// describes the league round, and a knockout cup has no table to take a baseline from. Falls back
/// to the legacy mirror for a save written before `competitions` existed, where it is the only copy.
pub fn user_league_round_context(game: &Game) -> Option<(u32, Vec<StandingEntry>)> {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let league = game.user_competition().or(game.league.as_ref())?;
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
/// It deliberately does **not** serve `Live`/`Spectator`. Those suspend between session creation and
/// the match being played while the UI drives the minutes, so their sequence belongs to the session
/// lifecycle and cannot be one call.
pub fn play_user_matchday_with_capture<F>(
    game: &mut Game,
    competition_index: usize,
    fixture_index: usize,
    on_capture: &mut F,
) -> Result<UserMatchdayOutcome, String>
where
    F: FnMut(StatsState),
{
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    if let Some(competition) = game.competitions.get(competition_index).cloned() {
        game.league = Some(competition);
    }

    let allows_extra_time = fixture_allows_extra_time(game, fixture_index);
    let mut session = live_match_manager::create_live_match(
        game,
        fixture_index,
        MatchMode::Instant,
        allows_extra_time,
    )?;
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

    if competition_index < game.competitions.len()
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
