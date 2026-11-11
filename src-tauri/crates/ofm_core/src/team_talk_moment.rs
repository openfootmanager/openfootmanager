//! When a team talk may be given, and how the match stood for it (#1194).
//!
//! The screen offers a talk at three moments: half-time, half-time of extra time, and once the
//! match is over (the post-match tab). Each is offered once, and the screen works the context
//! (winning, losing, drawing) out of the score. That was only the screen's habit: the command
//! behind it, which an MCP agent calls too, took any tone and any context at any time, so an agent
//! could talk daily with no match on, and alternate contexts to dodge the damping a repeated talk
//! gets. The moments, and the context, are the game's to say.

use engine::{MatchPhase, Side};

use crate::game::Game;

/// No live match is at half-time, and no match of the user's has just been played.
pub const NOT_AT_A_BREAK_ERROR: &str = "be.error.teamTalk.notAtABreak";
/// The team talk of this moment has been given.
pub const ALREADY_GIVEN_ERROR: &str = "be.error.teamTalk.alreadyGiven";

/// A moment a talk may be given in: its identity (one talk per key) and how the match stands for
/// the user's side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamTalkMoment {
    pub key: String,
    /// `winning`, `losing` or `drawing`: the vocabulary the talk's weights are written in.
    pub context: &'static str,
}

/// How the match stands for `side`, as a team talk reads it.
pub fn team_talk_context(side: Side, home_goals: u8, away_goals: u8) -> &'static str {
    let (ours, theirs) = match side {
        Side::Home => (home_goals, away_goals),
        Side::Away => (away_goals, home_goals),
    };
    match ours.cmp(&theirs) {
        std::cmp::Ordering::Greater => "winning",
        std::cmp::Ordering::Less => "losing",
        std::cmp::Ordering::Equal => "drawing",
    }
}

/// The moment a live match at `phase` offers, or the refusal: only the two half-times offer one.
pub fn interval_moment(
    fixture_id: &str,
    phase: MatchPhase,
    side: Side,
    home_goals: u8,
    away_goals: u8,
) -> Result<TeamTalkMoment, &'static str> {
    if !matches!(phase, MatchPhase::HalfTime | MatchPhase::ExtraTimeHalfTime) {
        return Err(NOT_AT_A_BREAK_ERROR);
    }
    Ok(TeamTalkMoment {
        key: format!("{fixture_id}/{phase:?}"),
        context: team_talk_context(side, home_goals, away_goals),
    })
}

/// The moment the user's match offers once it is over, if there is one: the user's club's most
/// recent fixture with a result, dated today or yesterday. Yesterday because finishing a live
/// match moves the clock on a day, and the post-match tab is read after that.
pub fn post_match_moment(game: &Game) -> Option<TeamTalkMoment> {
    let team_id = game.manager.team_id.as_deref()?;
    let day = |date: chrono::DateTime<chrono::Utc>| date.format("%Y-%m-%d").to_string();
    let today = day(game.clock.current_date);
    let yesterday = day(game.clock.current_date - chrono::Duration::days(1));
    let fixture = game
        .competitions
        .iter()
        .flat_map(|competition| competition.fixtures.iter())
        .filter(|fixture| {
            fixture.result.is_some()
                && (fixture.home_team_id == team_id || fixture.away_team_id == team_id)
                && [today.as_str(), yesterday.as_str()]
                    .contains(&fixture.date.get(..10).unwrap_or(""))
        })
        .max_by(|a, b| (&a.date, &a.id).cmp(&(&b.date, &b.id)))?;
    let result = fixture.result.as_ref()?;
    let side = if fixture.home_team_id == team_id {
        Side::Home
    } else {
        Side::Away
    };
    // A tie level after extra time is settled by the shootout, and the post-match tab judges it so
    // too: penalties only break a level score.
    let (home, away) = match (result.home_penalties, result.away_penalties) {
        (Some(home), Some(away)) if result.home_goals == result.away_goals => (home, away),
        _ => (result.home_goals, result.away_goals),
    };
    Some(TeamTalkMoment {
        key: format!("{}/PostMatch", fixture.id),
        context: team_talk_context(side, home, away),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::league::{Fixture, MatchResult};

    /// Given each way a match can stand for the side talked to
    /// When the context is worked out
    /// Then a lead is `winning`, a deficit `losing`, level `drawing`, whichever side that is
    #[test]
    fn the_context_follows_the_score_from_the_talked_to_sides_view() {
        assert_eq!(team_talk_context(Side::Home, 2, 1), "winning");
        assert_eq!(team_talk_context(Side::Away, 2, 1), "losing");
        assert_eq!(team_talk_context(Side::Home, 0, 3), "losing");
        assert_eq!(team_talk_context(Side::Away, 0, 3), "winning");
        assert_eq!(team_talk_context(Side::Home, 1, 1), "drawing");
        assert_eq!(team_talk_context(Side::Away, 0, 0), "drawing");
    }

    /// Given each phase a live match can be in
    /// When a talk is asked for
    /// Then only the two half-times offer a moment, one per interval of the fixture, and the
    ///      context is the score's
    #[test]
    fn only_the_two_half_times_of_a_live_match_offer_a_moment() {
        let half = interval_moment("f1", MatchPhase::HalfTime, Side::Home, 0, 1).unwrap();
        let extra = interval_moment("f1", MatchPhase::ExtraTimeHalfTime, Side::Home, 0, 1).unwrap();

        assert_eq!(half.context, "losing");
        assert_ne!(half.key, extra.key, "each interval is its own moment");
        for phase in [
            MatchPhase::PreKickOff,
            MatchPhase::FirstHalf,
            MatchPhase::SecondHalf,
            MatchPhase::FullTime,
            MatchPhase::ExtraTimeFirstHalf,
            MatchPhase::ExtraTimeSecondHalf,
            MatchPhase::ExtraTimeEnd,
            MatchPhase::PenaltyShootout,
            MatchPhase::Finished,
        ] {
            assert_eq!(
                interval_moment("f1", phase, Side::Home, 0, 0),
                Err(NOT_AT_A_BREAK_ERROR),
                "{phase:?}"
            );
        }
    }

    /// Given two fixtures
    /// When each is at half-time
    /// Then the moments differ, so a talk at one match's half-time never counts for the next
    #[test]
    fn two_matches_half_times_are_two_moments() {
        let first = interval_moment("f1", MatchPhase::HalfTime, Side::Home, 0, 0).unwrap();
        let second = interval_moment("f2", MatchPhase::HalfTime, Side::Home, 0, 0).unwrap();

        assert_ne!(first.key, second.key);
    }

    fn game_managing(team_id: &str, fixtures: Vec<Fixture>) -> Game {
        use crate::clock::GameClock;
        use chrono::{TimeZone, Utc};
        use domain::league::League;
        use domain::manager::Manager;

        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.team_id = Some(team_id.to_string());
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        game.competitions = vec![League {
            fixtures,
            ..League::default()
        }];
        game
    }

    fn played(id: &str, date: &str, home: &str, away: &str, score: (u8, u8)) -> Fixture {
        Fixture {
            id: id.to_string(),
            date: date.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            result: Some(MatchResult {
                home_goals: score.0,
                away_goals: score.1,
                ..MatchResult::default()
            }),
            ..Fixture::default()
        }
    }

    /// Given the user's club has played today, at home and winning, and then (in another game)
    ///       away and losing
    /// When the post-match moment is asked for
    /// Then it is that fixture's, with the context from the user's side of the score
    #[test]
    fn a_match_played_today_offers_a_post_match_moment_read_from_the_users_side() {
        let home_win = game_managing("me", vec![played("f1", "2026-05-20", "me", "them", (3, 1))]);
        let away_loss = game_managing("me", vec![played("f2", "2026-05-20", "them", "me", (2, 0))]);

        let won = post_match_moment(&home_win).unwrap();
        let lost = post_match_moment(&away_loss).unwrap();

        assert_eq!((won.context, lost.context), ("winning", "losing"));
        assert!(
            won.key.contains("f1") && lost.key.contains("f2"),
            "{won:?} {lost:?}"
        );
        assert_ne!(
            won.key,
            interval_moment("f1", MatchPhase::HalfTime, Side::Home, 0, 0)
                .unwrap()
                .key,
            "the post-match talk is not the half-time one"
        );
    }

    /// Given the clock a day after the user's match (finishing a live match moves it on a day)
    /// When the post-match moment is asked for
    /// Then the match is still offered its talk
    #[test]
    fn a_match_played_yesterday_still_offers_its_post_match_moment() {
        let game = game_managing("me", vec![played("f1", "2026-05-19", "me", "them", (1, 0))]);

        assert_eq!(
            post_match_moment(&game).map(|moment| moment.context),
            Some("winning")
        );
    }

    /// Given a match played two days ago, one dated today with no result, and one between other
    ///       clubs
    /// When the post-match moment is asked for
    /// Then there is none: only the user's match just played is offered a talk
    #[test]
    fn no_post_match_moment_without_a_match_of_the_users_just_played() {
        let game = game_managing(
            "me",
            vec![
                played("long_ago", "2026-05-18", "me", "them", (1, 0)),
                Fixture {
                    id: "unplayed".to_string(),
                    date: "2026-05-20".to_string(),
                    home_team_id: "me".to_string(),
                    away_team_id: "them".to_string(),
                    ..Fixture::default()
                },
                played("others", "2026-05-20", "x", "y", (1, 0)),
            ],
        );

        assert_eq!(post_match_moment(&game), None);
    }

    fn level_after_goals_won_on_penalties(
        id: &str,
        home: &str,
        away: &str,
        pens: (u8, u8),
    ) -> Fixture {
        Fixture {
            id: id.to_string(),
            date: "2026-05-20".to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            result: Some(MatchResult {
                home_goals: 1,
                away_goals: 1,
                home_penalties: Some(pens.0),
                away_penalties: Some(pens.1),
                ..MatchResult::default()
            }),
            ..Fixture::default()
        }
    }

    /// Given a knockout tie level at 1-1 and won 5-4 on penalties by the home side
    /// When the post-match moment is asked for the home side's manager, then the away side's
    /// Then it reads as won for the winner of the shootout and lost for the loser, as the
    ///      post-match tab judges it, not as a draw
    #[test]
    fn a_shootout_decides_the_post_match_context_of_a_level_tie() {
        let as_winner = game_managing(
            "me",
            vec![level_after_goals_won_on_penalties(
                "f1",
                "me",
                "them",
                (5, 4),
            )],
        );
        let as_loser = game_managing(
            "me",
            vec![level_after_goals_won_on_penalties(
                "f2",
                "them",
                "me",
                (5, 4),
            )],
        );

        assert_eq!(
            post_match_moment(&as_winner).map(|moment| moment.context),
            Some("winning")
        );
        assert_eq!(
            post_match_moment(&as_loser).map(|moment| moment.context),
            Some("losing")
        );
    }

    /// Given a tie level at 1-1 with no shootout, and one where the goals decide though a shootout
    ///       tally is on the result
    /// When the post-match moment is asked for
    /// Then the first is a draw, and the second follows the goals: penalties only break a level tie
    #[test]
    fn penalties_only_break_a_level_tie() {
        let level = game_managing("me", vec![played("f1", "2026-05-20", "me", "them", (1, 1))]);
        let mut decided_by_goals = game_managing(
            "me",
            vec![level_after_goals_won_on_penalties(
                "f2",
                "me",
                "them",
                (0, 3),
            )],
        );
        if let Some(result) = decided_by_goals.competitions[0].fixtures[0].result.as_mut() {
            result.home_goals = 2;
        }

        assert_eq!(
            post_match_moment(&level).map(|moment| moment.context),
            Some("drawing")
        );
        assert_eq!(
            post_match_moment(&decided_by_goals).map(|moment| moment.context),
            Some("winning")
        );
    }
}
