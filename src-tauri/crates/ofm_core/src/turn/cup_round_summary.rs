use crate::game::Game;
use crate::turn::round_summary::RoundResultSummary;
use domain::league::{CompetitionFormat, Fixture, FixtureStatus, League};
use serde::{Deserialize, Serialize};

/// One cup tie that was played: the shared result row, how a level score was settled, and who went through.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CupTieResult {
    #[serde(flatten)]
    pub result: RoundResultSummary,
    pub home_penalties: Option<u8>,
    pub away_penalties: Option<u8>,
    pub advancing_team_id: Option<String>,
    pub advancing_team_name: Option<String>,
}

/// What one of the user's cups played on one date. Like the league summary it is scoped to the date,
/// so a tie moved to another day is in the summary of the day it is played.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CupRoundSummary {
    pub competition_id: String,
    pub competition_name: String,
    /// The knockout round the matches belong to; `None` for a group-stage day.
    pub round_name: Option<String>,
    pub is_complete: bool,
    pub pending_fixture_count: u32,
    pub results: Vec<CupTieResult>,
}

fn cups_the_user_plays_in<'a>(
    game: &'a Game,
    team_id: &'a str,
) -> impl Iterator<Item = &'a League> {
    game.competitions.iter().filter(move |competition| {
        competition.rules.format != CompetitionFormat::LeagueTable
            && competition.participant_ids.iter().any(|id| id == team_id)
    })
}

fn tie_result(game: &Game, competition: &League, fixture: &Fixture) -> Option<CupTieResult> {
    let result = fixture.result.as_ref()?;
    if fixture.status != FixtureStatus::Completed {
        return None;
    }
    let advancing_team_id = competition
        .is_knockout_fixture(&fixture.id)
        .then(|| fixture.advancing_team_id().map(str::to_owned))
        .flatten();
    Some(CupTieResult {
        result: RoundResultSummary {
            fixture_id: fixture.id.clone(),
            home_team_id: fixture.home_team_id.clone(),
            home_team_name: game.team_name_or_id(&fixture.home_team_id),
            away_team_id: fixture.away_team_id.clone(),
            away_team_name: game.team_name_or_id(&fixture.away_team_id),
            home_goals: result.home_goals,
            away_goals: result.away_goals,
        },
        home_penalties: result.home_penalties,
        away_penalties: result.away_penalties,
        advancing_team_name: advancing_team_id
            .as_deref()
            .map(|id| game.team_name_or_id(id)),
        advancing_team_id,
    })
}

fn round_name_on(competition: &League, fixtures: &[&Fixture]) -> Option<String> {
    competition
        .knockout_rounds
        .iter()
        .find(|round| fixtures.iter().any(|f| round.fixture_ids.contains(&f.id)))
        .map(|round| round.name.clone())
}

/// The cup ties played on `date` in the cups the user's club is in; empty on a day with none,
/// and when the user has no club.
pub fn build_cup_round_summaries(game: &Game, date: &str) -> Vec<CupRoundSummary> {
    let Some(team_id) = game.manager.team_id.as_deref() else {
        return Vec::new();
    };
    cups_the_user_plays_in(game, team_id)
        .filter_map(|competition| {
            let fixtures: Vec<&Fixture> = competition
                .fixtures
                .iter()
                .filter(|fixture| fixture.date == date && fixture.is_competitive())
                .collect();
            if fixtures.is_empty() {
                return None;
            }
            let results: Vec<CupTieResult> = fixtures
                .iter()
                .filter_map(|fixture| tie_result(game, competition, fixture))
                .collect();
            let pending = (fixtures.len() - results.len()) as u32;
            Some(CupRoundSummary {
                competition_id: competition.id.clone(),
                competition_name: competition.name.clone(),
                round_name: round_name_on(competition, &fixtures),
                is_complete: pending == 0,
                pending_fixture_count: pending,
                results,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::league::{FixtureCompetition, KnockoutRoundState, MatchResult};
    use domain::manager::Manager;

    fn fixture(id: &str, date: &str, home: &str, away: &str, played: Option<(u8, u8)>) -> Fixture {
        Fixture {
            id: id.into(),
            competition_id: "cup".into(),
            date: date.into(),
            home_team_id: home.into(),
            away_team_id: away.into(),
            competition: FixtureCompetition::Cup,
            status: if played.is_some() {
                FixtureStatus::Completed
            } else {
                FixtureStatus::Scheduled
            },
            result: played.map(|(home_goals, away_goals)| MatchResult {
                home_goals,
                away_goals,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn game_with_cup(fixtures: Vec<Fixture>, user_club: &str) -> Game {
        let mut manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        manager.hire(user_club.to_string());
        let clock = GameClock::new(Utc.with_ymd_and_hms(2033, 3, 8, 12, 0, 0).unwrap());
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        let mut cup = League::new(
            "cup".into(),
            "National Cup".into(),
            2033,
            &["a".into(), "b".into(), "c".into(), "d".into()],
        );
        cup.rules.format = CompetitionFormat::Knockout;
        cup.knockout_rounds = vec![KnockoutRoundState {
            id: "r1".into(),
            name: "Semifinal".into(),
            fixture_ids: fixtures.iter().map(|f| f.id.clone()).collect(),
            ..Default::default()
        }];
        cup.fixtures = fixtures;
        game.competitions = vec![cup];
        game
    }

    fn ids(summary: &CupRoundSummary) -> Vec<&str> {
        summary
            .results
            .iter()
            .map(|r| r.result.fixture_id.as_str())
            .collect()
    }

    /// Given a day on which the cup holds only a friendly-cup tie
    /// When the summary is built
    /// Then there is none: a friendly cup is not competitive, so it is no cup round.
    #[test]
    fn a_friendly_cup_tie_is_not_a_cup_round() {
        let mut tie = fixture("t1", "2033-03-08", "a", "b", Some((2, 1)));
        tie.competition = FixtureCompetition::FriendlyCup;
        let game = game_with_cup(vec![tie], "a");

        assert!(build_cup_round_summaries(&game, "2033-03-08").is_empty());
    }

    /// Given a cup day with the user's club in the cup, when the summary is built, then it lists every
    /// tie of the day, who went through, and the round.
    #[test]
    fn a_cup_day_lists_every_tie_and_who_went_through() {
        let game = game_with_cup(
            vec![
                fixture("t1", "2033-03-08", "a", "b", Some((2, 1))),
                fixture("t2", "2033-03-08", "c", "d", Some((0, 1))),
            ],
            "a",
        );
        let summaries = build_cup_round_summaries(&game, "2033-03-08");
        assert_eq!(summaries.len(), 1);
        let summary = &summaries[0];
        assert_eq!(summary.round_name.as_deref(), Some("Semifinal"));
        assert_eq!(ids(summary), vec!["t1", "t2"]);
        assert!(summary.is_complete);
        let advancing: Vec<_> = summary
            .results
            .iter()
            .map(|r| r.advancing_team_id.as_deref())
            .collect();
        assert_eq!(advancing, vec![Some("a"), Some("d")]);
    }

    /// Given a day with no cup fixture, when the summary is built, then there is none.
    #[test]
    fn a_league_only_day_has_no_cup_summary() {
        let game = game_with_cup(
            vec![fixture("t1", "2033-03-15", "a", "b", Some((2, 1)))],
            "a",
        );
        assert!(build_cup_round_summaries(&game, "2033-03-08").is_empty());
    }

    /// Given a cup the user's club is not in, when the summary is built, then it is left out.
    #[test]
    fn a_cup_the_user_is_not_in_has_no_summary() {
        let game = game_with_cup(
            vec![fixture("t1", "2033-03-08", "a", "b", Some((2, 1)))],
            "z",
        );
        assert!(build_cup_round_summaries(&game, "2033-03-08").is_empty());
    }

    /// Given a tie moved from the round's date to a later day, when the summaries of both days are
    /// built, then the moved tie is only in the later day's.
    #[test]
    fn a_moved_tie_is_in_the_summary_of_the_day_it_is_played() {
        let game = game_with_cup(
            vec![
                fixture("t1", "2033-03-08", "a", "b", Some((2, 1))),
                fixture("t2", "2033-03-11", "c", "d", Some((0, 1))),
            ],
            "a",
        );
        let first = &build_cup_round_summaries(&game, "2033-03-08")[0];
        let later = &build_cup_round_summaries(&game, "2033-03-11")[0];
        assert_eq!(ids(first), vec!["t1"]);
        assert_eq!(ids(later), vec!["t2"]);
    }

    /// Given a tie of the day that has not been played, when the summary is built, then it is pending.
    #[test]
    fn an_unplayed_tie_of_the_day_is_pending() {
        let game = game_with_cup(
            vec![
                fixture("t1", "2033-03-08", "a", "b", Some((2, 1))),
                fixture("t2", "2033-03-08", "c", "d", None),
            ],
            "a",
        );
        let summary = &build_cup_round_summaries(&game, "2033-03-08")[0];
        assert_eq!(
            (summary.is_complete, summary.pending_fixture_count),
            (false, 1)
        );
        assert_eq!(ids(summary), vec!["t1"]);
    }

    /// Given a level tie settled on penalties, when the summary is built, then the shootout and the
    /// team that went through are shown.
    #[test]
    fn a_tie_settled_on_penalties_names_the_winner_of_the_shootout() {
        let mut level = fixture("t1", "2033-03-08", "a", "b", Some((1, 1)));
        let result = level.result.as_mut().unwrap();
        result.home_penalties = Some(3);
        result.away_penalties = Some(5);
        let game = game_with_cup(vec![level], "a");
        let tie = &build_cup_round_summaries(&game, "2033-03-08")[0].results[0];
        assert_eq!((tie.home_penalties, tie.away_penalties), (Some(3), Some(5)));
        assert_eq!(tie.advancing_team_id.as_deref(), Some("b"));
    }

    /// Given a friendly in the cup's fixture list on the same day, when the summary is built, then it is not a tie.
    #[test]
    fn friendlies_are_not_cup_ties() {
        let mut friendly = fixture("fr", "2033-03-08", "a", "b", Some((1, 0)));
        friendly.competition = FixtureCompetition::Friendly;
        let game = game_with_cup(vec![friendly], "a");
        assert!(build_cup_round_summaries(&game, "2033-03-08").is_empty());
    }
}
