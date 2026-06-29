use crate::game::Game;
use domain::league::{Fixture, FixtureCompetition, FixtureStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleQuery {
    pub competition_id: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct FixtureResult {
    pub home_goals: u8,
    pub away_goals: u8,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct FixtureSummary {
    pub id: String,
    pub matchday: u32,
    pub date: String,
    pub home_team_id: String,
    pub home_team_name: String,
    pub away_team_id: String,
    pub away_team_name: String,
    /// Serialises to "League", "Cup", "PreseasonTournament", etc.
    pub competition: String,
    pub competition_id: String,
    /// "Scheduled" | "InProgress" | "Completed"
    pub status: String,
    pub result: Option<FixtureResult>,
}

/// A group of fixtures sharing the same matchday (or date for cup rounds).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct MatchdayGroup {
    /// Stable key used as React list key and scroll anchor target.
    pub key: String,
    /// ISO date of the first fixture in this group — used for calendar dots.
    pub date: String,
    /// Matchday number (0 for non-league cup/international rounds).
    pub matchday: u32,
    /// Competition type string ("League", "Cup", …).
    pub competition: String,
    /// True for the group that contains the user's earliest upcoming match.
    pub is_next_user_match: bool,
    pub fixtures: Vec<FixtureSummary>,
}

#[derive(Debug, Serialize)]
pub struct ScheduleSlice {
    pub competition_id: String,
    pub competition_name: String,
    /// Game's current date (YYYY-MM-DD), used by the calendar to open on the
    /// right month and to split past/upcoming groups.
    pub today: String,
    pub past_groups: Vec<MatchdayGroup>,
    pub upcoming_groups: Vec<MatchdayGroup>,
    /// Date of the user's next scheduled match, if any.
    pub next_user_match_date: Option<String>,
}

pub fn query_schedule(game: &Game, query: &ScheduleQuery) -> Option<ScheduleSlice> {
    let competition = game
        .competitions
        .iter()
        .find(|c| c.id == query.competition_id)?;

    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let user_team_id = game.manager.team_id.as_deref().unwrap_or("");

    let team_name: BTreeMap<&str, &str> = game
        .teams
        .iter()
        .map(|t| (t.id.as_str(), t.name.as_str()))
        .collect();

    // Build ordered groups: key → Vec<FixtureSummary>.
    // BTreeMap preserves insertion order when keys sort correctly.
    // Key format mirrors the frontend's getFixtureGroupKey.
    let mut groups: BTreeMap<String, Vec<FixtureSummary>> = BTreeMap::new();

    let mut sorted_fixtures: Vec<&Fixture> = competition.fixtures.iter().collect();
    sorted_fixtures.sort_by(|a, b| a.date.cmp(&b.date).then(a.matchday.cmp(&b.matchday)));

    for fixture in sorted_fixtures {
        let key = group_key(fixture);
        let summary = project_fixture(fixture, &team_name);
        groups.entry(key).or_default().push(summary);
    }

    // Find the key of the group containing the user's earliest upcoming match.
    let next_user_match_key: Option<String> = competition
        .fixtures
        .iter()
        .filter(|f| {
            f.status == FixtureStatus::Scheduled
                && f.date.as_str() >= today.as_str()
                && (f.home_team_id == user_team_id || f.away_team_id == user_team_id)
        })
        .min_by_key(|f| &f.date)
        .map(group_key);

    let next_user_match_date = next_user_match_key.as_deref().and_then(|key| {
        groups
            .get(key)
            .and_then(|fs| fs.first().map(|f| f.date.clone()))
    });

    let mut past_groups: Vec<MatchdayGroup> = Vec::new();
    let mut upcoming_groups: Vec<MatchdayGroup> = Vec::new();

    for (key, fixtures) in groups {
        let date = fixtures.first().map(|f| f.date.clone()).unwrap_or_default();
        let matchday = fixtures.first().map(|f| f.matchday).unwrap_or(0);
        let competition_str = fixtures
            .first()
            .map(|f| f.competition.clone())
            .unwrap_or_default();
        let is_next_user_match = next_user_match_key.as_deref() == Some(key.as_str());

        // A group is "upcoming" when it has any scheduled fixture on or after today.
        let is_upcoming = fixtures
            .iter()
            .any(|f| f.status == "Scheduled" && f.date.as_str() >= today.as_str());

        let group = MatchdayGroup {
            key,
            date,
            matchday,
            competition: competition_str,
            is_next_user_match,
            fixtures,
        };

        if is_upcoming {
            upcoming_groups.push(group);
        } else {
            past_groups.push(group);
        }
    }

    // Sort by date, not by BTreeMap key (which is lexicographic and breaks at matchday ≥ 10).
    upcoming_groups.sort_by(|a, b| a.date.cmp(&b.date));
    past_groups.sort_by(|a, b| b.date.cmp(&a.date));

    Some(ScheduleSlice {
        competition_id: competition.id.clone(),
        competition_name: competition.name.clone(),
        today,
        past_groups,
        upcoming_groups,
        next_user_match_date,
    })
}

fn group_key(fixture: &Fixture) -> String {
    if fixture.competition == FixtureCompetition::League {
        format!("{}-league-{}", fixture.competition_id, fixture.matchday)
    } else {
        format!(
            "{}-{}-{}",
            fixture.competition_id,
            competition_type_str(&fixture.competition),
            fixture.date
        )
    }
}

fn competition_type_str(c: &FixtureCompetition) -> &'static str {
    match c {
        FixtureCompetition::League => "League",
        FixtureCompetition::Cup => "Cup",
        FixtureCompetition::ContinentalClub => "ContinentalClub",
        FixtureCompetition::InternationalClub => "InternationalClub",
        FixtureCompetition::InternationalNation => "InternationalNation",
        FixtureCompetition::Friendly => "Friendly",
        FixtureCompetition::FriendlyCup => "FriendlyCup",
        FixtureCompetition::PreseasonTournament => "PreseasonTournament",
    }
}

fn project_fixture(fixture: &Fixture, team_name: &BTreeMap<&str, &str>) -> FixtureSummary {
    let home_name = team_name
        .get(fixture.home_team_id.as_str())
        .copied()
        .unwrap_or(&fixture.home_team_id)
        .to_string();
    let away_name = team_name
        .get(fixture.away_team_id.as_str())
        .copied()
        .unwrap_or(&fixture.away_team_id)
        .to_string();

    let status = match fixture.status {
        FixtureStatus::Scheduled => "Scheduled",
        FixtureStatus::InProgress => "InProgress",
        FixtureStatus::Completed => "Completed",
    };

    let result = fixture.result.as_ref().map(|r| FixtureResult {
        home_goals: r.home_goals,
        away_goals: r.away_goals,
    });

    FixtureSummary {
        id: fixture.id.clone(),
        matchday: fixture.matchday,
        date: fixture.date.clone(),
        home_team_id: fixture.home_team_id.clone(),
        home_team_name: home_name,
        away_team_id: fixture.away_team_id.clone(),
        away_team_name: away_name,
        competition: competition_type_str(&fixture.competition).to_string(),
        competition_id: fixture.competition_id.clone(),
        status: status.to_string(),
        result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, MatchResult};

    fn make_fixture(
        id: &str,
        matchday: u32,
        date: &str,
        home: &str,
        away: &str,
        status: FixtureStatus,
    ) -> Fixture {
        Fixture {
            id: id.to_string(),
            competition_id: "league-1".to_string(),
            matchday,
            date: date.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            competition: FixtureCompetition::League,
            status,
            result: None,
        }
    }

    fn make_completed_fixture(
        id: &str,
        matchday: u32,
        date: &str,
        home: &str,
        away: &str,
        hg: u8,
        ag: u8,
    ) -> Fixture {
        Fixture {
            id: id.to_string(),
            competition_id: "league-1".to_string(),
            matchday,
            date: date.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Completed,
            result: Some(MatchResult {
                home_goals: hg,
                away_goals: ag,
                home_scorers: vec![],
                away_scorers: vec![],
                report: None,
                home_penalties: None,
                away_penalties: None,
            }),
        }
    }

    fn make_game(fixtures: Vec<Fixture>, today: &str, user_team: &str) -> Game {
        use chrono::DateTime;
        use domain::manager::Manager;
        use domain::team::Team;

        let competition = League {
            id: "league-1".to_string(),
            name: "Test League".to_string(),
            fixtures,
            participant_ids: vec!["user-team".to_string(), "other-team".to_string()],
            ..Default::default()
        };

        let start: DateTime<chrono::Utc> =
            format!("{today}T00:00:00Z").parse().expect("valid date");
        let clock = crate::clock::GameClock::new(start);

        let mut manager = Manager::new(
            "mgr-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "EN".to_string(),
        );
        manager.team_id = Some(user_team.to_string());

        let team_a = Team::new(
            "user-team".to_string(),
            "User FC".to_string(),
            "UFC".to_string(),
            "EN".to_string(),
            "London".to_string(),
            "Stadium A".to_string(),
            30_000,
        );
        let team_b = Team::new(
            "other-team".to_string(),
            "Other FC".to_string(),
            "OFC".to_string(),
            "EN".to_string(),
            "Manchester".to_string(),
            "Stadium B".to_string(),
            25_000,
        );

        let mut game = Game::new(clock, manager, vec![team_a, team_b], vec![], vec![], vec![]);
        game.competitions = vec![competition];
        game.sync_legacy_league();
        game
    }

    #[test]
    fn returns_none_for_unknown_competition() {
        let game = make_game(vec![], "2026-08-15", "user-team");
        let query = ScheduleQuery {
            competition_id: "no-such-league".to_string(),
        };
        assert!(query_schedule(&game, &query).is_none());
    }

    #[test]
    fn splits_past_and_upcoming_correctly() {
        let fixtures = vec![
            make_completed_fixture("f1", 1, "2026-08-10", "user-team", "other-team", 2, 1),
            make_fixture(
                "f2",
                2,
                "2026-08-17",
                "other-team",
                "user-team",
                FixtureStatus::Scheduled,
            ),
            make_fixture(
                "f3",
                3,
                "2026-08-24",
                "user-team",
                "other-team",
                FixtureStatus::Scheduled,
            ),
        ];
        let game = make_game(fixtures, "2026-08-15", "user-team");
        let slice = query_schedule(
            &game,
            &ScheduleQuery {
                competition_id: "league-1".to_string(),
            },
        )
        .unwrap();

        assert_eq!(slice.past_groups.len(), 1);
        assert_eq!(slice.upcoming_groups.len(), 2);
        // Past is most recent first.
        assert_eq!(slice.past_groups[0].fixtures[0].id, "f1");
        // Upcoming is chronological — earliest first.
        assert_eq!(slice.upcoming_groups[0].fixtures[0].id, "f2");
    }

    #[test]
    fn marks_next_user_match_group() {
        let fixtures = vec![
            make_completed_fixture("f1", 1, "2026-08-10", "user-team", "other-team", 2, 1),
            make_fixture(
                "f2",
                2,
                "2026-08-17",
                "other-team",
                "user-team",
                FixtureStatus::Scheduled,
            ),
            make_fixture(
                "f3",
                3,
                "2026-08-24",
                "user-team",
                "other-team",
                FixtureStatus::Scheduled,
            ),
        ];
        let game = make_game(fixtures, "2026-08-15", "user-team");
        let slice = query_schedule(
            &game,
            &ScheduleQuery {
                competition_id: "league-1".to_string(),
            },
        )
        .unwrap();

        // Matchday 2 contains user team first upcoming match.
        let next = slice.upcoming_groups.iter().find(|g| g.is_next_user_match);
        assert!(next.is_some());
        assert_eq!(next.unwrap().matchday, 2);
        assert_eq!(slice.next_user_match_date.as_deref(), Some("2026-08-17"));
    }

    #[test]
    fn no_next_match_when_all_completed() {
        let fixtures = vec![make_completed_fixture(
            "f1",
            1,
            "2026-08-10",
            "user-team",
            "other-team",
            1,
            0,
        )];
        let game = make_game(fixtures, "2026-08-15", "user-team");
        let slice = query_schedule(
            &game,
            &ScheduleQuery {
                competition_id: "league-1".to_string(),
            },
        )
        .unwrap();
        assert!(slice.next_user_match_date.is_none());
        assert!(slice.upcoming_groups.iter().all(|g| !g.is_next_user_match));
    }

    #[test]
    fn upcoming_groups_sorted_correctly_with_double_digit_matchdays() {
        // BTreeMap key sort puts "league-1-league-10" before "league-1-league-2" lexicographically.
        // Groups must be ordered by date, not by key.
        let mut fixtures = Vec::new();
        for md in 1u32..=12 {
            let date = format!("2026-08-{:02}", md + 10);
            fixtures.push(make_fixture(
                &format!("f{}", md),
                md,
                &date,
                "user-team",
                "other-team",
                FixtureStatus::Scheduled,
            ));
        }
        let game = make_game(fixtures, "2026-08-01", "user-team");
        let slice = query_schedule(
            &game,
            &ScheduleQuery {
                competition_id: "league-1".to_string(),
            },
        )
        .unwrap();

        // Upcoming groups must be in ascending date order (matchday 1 first, 12 last).
        let dates: Vec<&str> = slice
            .upcoming_groups
            .iter()
            .map(|g| g.date.as_str())
            .collect();
        let mut sorted = dates.clone();
        sorted.sort();
        assert_eq!(
            dates, sorted,
            "upcoming_groups must be sorted by date, not lexicographic key"
        );
        // Verify matchday 1 (earliest date) is first, not matchday 10.
        assert_eq!(slice.upcoming_groups[0].matchday, 1);
        assert_eq!(slice.upcoming_groups[9].matchday, 10);
    }

    #[test]
    fn past_groups_sorted_most_recent_first_with_double_digit_matchdays() {
        let mut fixtures = Vec::new();
        for md in 1u32..=12 {
            let date = format!("2026-07-{:02}", md + 1);
            fixtures.push(make_completed_fixture(
                &format!("f{}", md),
                md,
                &date,
                "user-team",
                "other-team",
                1,
                0,
            ));
        }
        let game = make_game(fixtures, "2026-08-15", "user-team");
        let slice = query_schedule(
            &game,
            &ScheduleQuery {
                competition_id: "league-1".to_string(),
            },
        )
        .unwrap();

        // Past groups must be in descending date order (matchday 12 first, 1 last).
        let dates: Vec<&str> = slice.past_groups.iter().map(|g| g.date.as_str()).collect();
        let mut sorted_desc = dates.clone();
        sorted_desc.sort_by(|a, b| b.cmp(a));
        assert_eq!(
            dates, sorted_desc,
            "past_groups must be sorted by date descending"
        );
        assert_eq!(slice.past_groups[0].matchday, 12);
        assert_eq!(slice.past_groups[11].matchday, 1);
    }

    #[test]
    fn team_names_are_resolved() {
        let fixtures = vec![make_fixture(
            "f1",
            1,
            "2026-08-17",
            "user-team",
            "other-team",
            FixtureStatus::Scheduled,
        )];
        let game = make_game(fixtures, "2026-08-15", "user-team");
        let slice = query_schedule(
            &game,
            &ScheduleQuery {
                competition_id: "league-1".to_string(),
            },
        )
        .unwrap();

        let fixture = &slice.upcoming_groups[0].fixtures[0];
        assert_eq!(fixture.home_team_name, "User FC");
        assert_eq!(fixture.away_team_name, "Other FC");
    }
}
