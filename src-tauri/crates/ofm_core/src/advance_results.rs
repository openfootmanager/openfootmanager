//! Collects the match results produced by a single Continue / Skip advance so
//! the UI can show an FM-style day-by-day recap. Scoped to the matches the
//! player cares about: their own competitions plus national-team fixtures.

use crate::game::Game;
use domain::league::FixtureStatus;
use serde::{Deserialize, Serialize};

/// One finished match surfaced in the post-advance recap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdvanceMatchResult {
    pub date: String,
    /// Club competition name; empty for international fixtures.
    pub competition: String,
    pub international: bool,
    pub home_team: String,
    pub away_team: String,
    pub home_goals: u8,
    pub away_goals: u8,
    /// Shootout score for a knockout decided on penalties; `None` otherwise.
    pub home_penalties: Option<u8>,
    pub away_penalties: Option<u8>,
    /// True when the user's club featured (for highlighting).
    pub involves_user: bool,
}

fn team_name(game: &Game, team_id: &str) -> String {
    game.teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| team_id.to_string())
}

fn national_team_name(game: &Game, team_id: &str) -> String {
    game.national_teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| team_id.to_string())
}

/// Collect every match finished on or after `since_date` in the user's
/// competitions and in national-team fixtures, ordered by date. `since_date` is
/// the clock date captured *before* the advance, so only matches played during
/// this advance are included (earlier results are dated before it; matches
/// scheduled for the new "today" are still pending, not Completed).
pub fn collect_advance_results(game: &Game, since_date: &str) -> Vec<AdvanceMatchResult> {
    let user_team_id = game.manager.team_id.as_deref();
    let mut results = Vec::new();

    for competition in &game.competitions {
        let user_in_competition = user_team_id.is_some_and(|team_id| {
            competition
                .standings
                .iter()
                .any(|entry| entry.team_id == team_id)
                || competition.participant_ids.iter().any(|id| id == team_id)
        });
        let is_wc = crate::world_cup::is_world_cup_competition(competition);
        if !user_in_competition && !is_wc {
            continue;
        }
        for fixture in &competition.fixtures {
            if fixture.status != FixtureStatus::Completed || fixture.date.as_str() < since_date {
                continue;
            }
            let Some(result) = &fixture.result else {
                continue;
            };
            let (home_team, away_team) = if is_wc {
                (
                    national_team_name(game, &fixture.home_team_id),
                    national_team_name(game, &fixture.away_team_id),
                )
            } else {
                (
                    team_name(game, &fixture.home_team_id),
                    team_name(game, &fixture.away_team_id),
                )
            };
            results.push(AdvanceMatchResult {
                date: fixture.date.clone(),
                competition: competition.name.clone(),
                international: is_wc,
                home_team,
                away_team,
                home_goals: result.home_goals,
                away_goals: result.away_goals,
                home_penalties: result.home_penalties,
                away_penalties: result.away_penalties,
                involves_user: !is_wc && user_team_id.is_some_and(|team_id| {
                    fixture.home_team_id == team_id || fixture.away_team_id == team_id
                }),
            });
        }
    }

    for national_team in &game.national_teams {
        for fixture in &national_team.fixtures {
            if fixture.status != FixtureStatus::Completed || fixture.date.as_str() < since_date {
                continue;
            }
            let Some(result) = &fixture.result else {
                continue;
            };
            results.push(AdvanceMatchResult {
                date: fixture.date.clone(),
                competition: String::new(),
                international: true,
                home_team: national_team_name(game, &fixture.home_team_id),
                away_team: national_team_name(game, &fixture.away_team_id),
                home_goals: result.home_goals,
                away_goals: result.away_goals,
                home_penalties: result.home_penalties,
                away_penalties: result.away_penalties,
                involves_user: false,
            });
        }
    }

    results.sort_by(|left, right| left.date.cmp(&right.date));
    results
}

#[cfg(test)]
mod tests {
    use super::collect_advance_results;
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::league::{
        CompetitionScope, CompetitionType, Fixture, FixtureCompetition, FixtureStatus, League,
        MatchResult, StandingEntry,
    };
    use domain::manager::Manager;
    use domain::national_team::NationalTeam;
    use domain::team::Team;

    fn team(id: &str) -> Team {
        Team::new(
            id.to_string(),
            format!("{id} FC"),
            id.to_uppercase(),
            "England".to_string(),
            "Town".to_string(),
            "Ground".to_string(),
            20_000,
        )
    }

    fn completed(id: &str, date: &str, home: &str, away: &str, hg: u8, ag: u8) -> Fixture {
        Fixture {
            id: id.to_string(),
            competition_id: "comp".to_string(),
            matchday: 1,
            date: date.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Completed,
            result: Some(MatchResult {
                home_goals: hg,
                away_goals: ag,
                home_scorers: Vec::new(),
                away_scorers: Vec::new(),
                report: None,
                home_penalties: None,
                away_penalties: None,
            }),
        }
    }

    fn game_with_user_competition() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 11, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("home".to_string());
        let mut game = Game::new(
            clock,
            manager,
            vec![team("home"), team("away"), team("x"), team("y")],
            vec![],
            vec![],
            vec![],
        );
        game.competitions = vec![League {
            id: "comp".to_string(),
            name: "Test League".to_string(),
            season: 2026,
            participant_ids: vec![
                "home".to_string(),
                "away".to_string(),
                "x".to_string(),
                "y".to_string(),
            ],
            fixtures: vec![
                // Played before this advance — excluded by since_date.
                completed("old", "2026-08-08", "home", "away", 1, 0),
                // Played during this advance.
                completed("u", "2026-08-10", "home", "x", 2, 1),
                completed("o", "2026-08-10", "away", "y", 0, 0),
            ],
            standings: vec![
                StandingEntry::new("home".to_string()),
                StandingEntry::new("away".to_string()),
            ],
            ..League::default()
        }];
        game
    }

    #[test]
    fn collects_only_results_from_this_advance_and_flags_user_match() {
        let game = game_with_user_competition();

        let results = collect_advance_results(&game, "2026-08-10");

        assert_eq!(results.len(), 2, "older result must be excluded");
        let user_match = results
            .iter()
            .find(|r| r.involves_user)
            .expect("user's match should be present");
        assert_eq!(user_match.home_team, "home FC");
        assert_eq!((user_match.home_goals, user_match.away_goals), (2, 1));
        assert!(results.iter().all(|r| !r.international));
    }

    #[test]
    fn ignores_competitions_the_user_is_not_in() {
        let mut game = game_with_user_competition();
        game.manager.team_id = Some("unaffiliated".to_string());

        let results = collect_advance_results(&game, "2026-08-10");

        assert!(results.is_empty());
    }

    fn wc_fixture(id: &str, date: &str, home: &str, away: &str, hg: u8, ag: u8) -> Fixture {
        Fixture {
            id: id.to_string(),
            competition_id: "wc-2026".to_string(),
            matchday: 1,
            date: date.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            competition: FixtureCompetition::InternationalNation,
            status: FixtureStatus::Completed,
            result: Some(MatchResult {
                home_goals: hg,
                away_goals: ag,
                home_scorers: Vec::new(),
                away_scorers: Vec::new(),
                report: None,
                home_penalties: None,
                away_penalties: None,
            }),
        }
    }

    fn game_with_wc() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 15, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        game.national_teams = vec![
            NationalTeam::new("nt-bra".to_string(), "Brazil".to_string(), "BR".to_string(), None),
            NationalTeam::new("nt-ger".to_string(), "Germany".to_string(), "DE".to_string(), None),
        ];
        let mut wc = League::new("wc-2026".to_string(), "World Cup 2026".to_string(), 2026, &[]);
        wc.kind = CompetitionType::InternationalNation;
        wc.scope = CompetitionScope::International;
        wc.fixtures = vec![
            wc_fixture("g1", "2026-06-15", "nt-bra", "nt-ger", 2, 1),
            wc_fixture("g2", "2026-06-10", "nt-ger", "nt-bra", 1, 3),
        ];
        game.competitions = vec![wc];
        game
    }

    #[test]
    fn includes_wc_fixtures_as_international() {
        let game = game_with_wc();

        let results = collect_advance_results(&game, "2026-06-12");

        assert_eq!(results.len(), 1, "only fixture on or after since_date");
        let r = &results[0];
        assert!(r.international);
        assert_eq!(r.competition, "World Cup 2026");
        assert_eq!(r.home_team, "Brazil");
        assert_eq!(r.away_team, "Germany");
        assert_eq!((r.home_goals, r.away_goals), (2, 1));
        assert!(!r.involves_user);
    }

    #[test]
    fn wc_fixtures_excluded_before_since_date() {
        let game = game_with_wc();

        let results = collect_advance_results(&game, "2026-06-20");

        assert!(results.is_empty());
    }

    #[test]
    fn carries_penalty_shootout_scores() {
        let mut game = game_with_wc();
        let wc = &mut game.competitions[0];
        let mut knockout = wc_fixture("ko", "2026-07-10", "nt-bra", "nt-ger", 1, 1);
        if let Some(result) = knockout.result.as_mut() {
            result.home_penalties = Some(4);
            result.away_penalties = Some(2);
        }
        wc.fixtures = vec![knockout];

        let results = collect_advance_results(&game, "2026-07-01");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].home_penalties, Some(4));
        assert_eq!(results[0].away_penalties, Some(2));
    }
}
