use crate::clock::GameClock;
use crate::game::{BoardObjective, Game, ScoutingAssignment, YouthScoutingAssignment};
use domain::league::{Fixture, FixtureStatus};
use domain::manager::Manager;
use domain::season::SeasonContext;
use domain::team::Team;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct SessionStateQuery {}

/// Standing row with the team name already resolved so the frontend needs no lookup table.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StandingRow {
    pub team_id: String,
    pub team_name: String,
    pub played: u32,
    pub won: u32,
    pub drawn: u32,
    pub lost: u32,
    pub goals_for: u32,
    pub goals_against: u32,
    pub points: u32,
}

/// Slim view of the manager's primary competition — just standings + the team's
/// next 3 and last 2 fixtures. No full fixture list, no scorer history.
#[derive(Debug, Serialize)]
pub struct UserCompetitionSummary {
    pub competition_id: String,
    pub competition_name: String,
    pub name_key: Option<String>,
    pub standings: Vec<StandingRow>,
    pub upcoming_fixtures: Vec<Fixture>,
    pub recent_fixtures: Vec<Fixture>,
}

/// KB-sized session payload — replaces the full `Game` for hot commands in Phase 2.
///
/// Contains only what the Home screen and header need after any command returns:
/// clock, manager identity, the user's single team, season context, board objectives,
/// scouting queues, unread counts, and a slim competition summary (standings + 5
/// fixtures at most). Everything else (squads, news bodies, all transfer history,
/// national teams, world history) stays in the on-demand slice commands.
#[derive(Debug, Serialize)]
pub struct SessionState {
    pub clock: GameClock,
    pub manager: Manager,
    /// The manager's current club. `None` when between jobs.
    pub team: Option<Team>,
    pub season_context: SeasonContext,
    pub board_objectives: Vec<BoardObjective>,
    pub scouting_assignments: Vec<ScoutingAssignment>,
    pub youth_scouting_assignments: Vec<YouthScoutingAssignment>,
    pub active_competition_ids: Vec<String>,
    pub unread_news_count: usize,
    pub unread_messages_count: usize,
    /// Slim view of the manager's primary competition, or `None` when not yet set.
    pub user_competition: Option<UserCompetitionSummary>,
}

/// Project a `Game` into a `SessionState`.
///
/// This is the canonical projection; every hot command (advance, mutations) will
/// call this once Phase 2 flips their return types. It is deliberately read-only
/// and allocation-minimal: it clones only the handful of entities the Home screen
/// needs, not the full team/player/news/messages arrays.
pub fn project_session(game: &Game) -> SessionState {
    let team = game
        .manager
        .team_id
        .as_deref()
        .and_then(|team_id| game.teams.iter().find(|t| t.id == team_id))
        .cloned();

    let user_competition = build_user_competition(game);

    SessionState {
        clock: game.clock.clone(),
        manager: game.manager.clone(),
        team,
        season_context: game.season_context.clone(),
        board_objectives: game.board_objectives.clone(),
        scouting_assignments: game.scouting_assignments.clone(),
        youth_scouting_assignments: game.youth_scouting_assignments.clone(),
        active_competition_ids: game.active_competition_ids.clone(),
        unread_news_count: {
            // Don't count future-dated articles (e.g. a World Cup kickoff dated
            // at kickoff) — they aren't shown in the feed until their day.
            let today = game.clock.current_date.format("%Y-%m-%d").to_string();
            game.news
                .iter()
                .filter(|a| !a.read && crate::slices::news::article_is_visible(&a.date, &today))
                .count()
        },
        unread_messages_count: game.messages.iter().filter(|m| !m.read).count(),
        user_competition,
    }
}

fn build_user_competition(game: &Game) -> Option<UserCompetitionSummary> {
    let team_id = game.manager.team_id.as_deref()?;

    let comp = game
        .competitions
        .iter()
        .find(|c| c.participant_ids.iter().any(|id| id == team_id))?;

    let team_name_map: BTreeMap<&str, &str> = game
        .teams
        .iter()
        .map(|t| (t.id.as_str(), t.name.as_str()))
        .collect();

    let standings: Vec<StandingRow> = comp
        .standings
        .iter()
        .map(|s| {
            let team_name = team_name_map
                .get(s.team_id.as_str())
                .map(|n| n.to_string())
                .unwrap_or_default();
            StandingRow {
                team_id: s.team_id.clone(),
                team_name,
                played: s.played,
                won: s.won,
                drawn: s.drawn,
                lost: s.lost,
                goals_for: s.goals_for,
                goals_against: s.goals_against,
                points: s.points,
            }
        })
        .collect();

    // Upcoming: next 3 scheduled fixtures for the user's team, sorted by date.
    let mut upcoming: Vec<&Fixture> = comp
        .fixtures
        .iter()
        .filter(|f| {
            f.status == FixtureStatus::Scheduled
                && (f.home_team_id == team_id || f.away_team_id == team_id)
        })
        .collect();
    upcoming.sort_by(|a, b| a.date.cmp(&b.date));
    let upcoming_fixtures: Vec<Fixture> = upcoming.into_iter().take(3).cloned().collect();

    // Recent: last 2 completed fixtures for the user's team, most-recent-first.
    let mut recent: Vec<&Fixture> = comp
        .fixtures
        .iter()
        .filter(|f| {
            f.status == FixtureStatus::Completed
                && (f.home_team_id == team_id || f.away_team_id == team_id)
        })
        .collect();
    recent.sort_by(|a, b| b.date.cmp(&a.date));
    let recent_fixtures: Vec<Fixture> = recent.into_iter().take(2).cloned().collect();

    Some(UserCompetitionSummary {
        competition_id: comp.id.clone(),
        competition_name: comp.name.clone(),
        name_key: comp.name_key.clone(),
        standings,
        upcoming_fixtures,
        recent_fixtures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::Utc;
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League, StandingEntry};
    use domain::manager::Manager;
    use domain::message::InboxMessage;
    use domain::news::{NewsArticle, NewsCategory};
    use domain::team::Team;

    fn make_clock() -> GameClock {
        GameClock {
            current_date: Utc::now(),
            start_date: Utc::now(),
        }
    }

    fn make_manager(team_id: Option<&str>) -> Manager {
        let mut m = Manager::new(
            "mgr1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "EN".to_string(),
        );
        m.team_id = team_id.map(|s| s.to_string());
        m
    }

    fn make_team(id: &str, name: &str) -> Team {
        Team::new(
            id.to_string(),
            name.to_string(),
            name[..3].to_string(),
            "EN".to_string(),
            "London".to_string(),
            "Stadium".to_string(),
            40_000,
        )
    }

    fn make_fixture(
        id: &str,
        home: &str,
        away: &str,
        date: &str,
        status: FixtureStatus,
    ) -> Fixture {
        Fixture {
            id: id.to_string(),
            competition_id: "league1".to_string(),
            matchday: 1,
            date: date.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            competition: FixtureCompetition::default(),
            status,
            result: None,
        }
    }

    fn make_game_with_team() -> Game {
        let team_a = make_team("t1", "Arsenal");
        let team_b = make_team("t2", "Chelsea");

        let league = League {
            id: "league1".to_string(),
            name: "Premier League".to_string(),
            participant_ids: vec!["t1".to_string(), "t2".to_string()],
            standings: vec![
                StandingEntry {
                    team_id: "t1".to_string(),
                    played: 2,
                    won: 1,
                    drawn: 1,
                    lost: 0,
                    goals_for: 3,
                    goals_against: 1,
                    points: 4,
                },
                StandingEntry {
                    team_id: "t2".to_string(),
                    played: 2,
                    won: 0,
                    drawn: 1,
                    lost: 1,
                    goals_for: 1,
                    goals_against: 3,
                    points: 1,
                },
            ],
            fixtures: vec![
                make_fixture("f0", "t2", "t1", "2025-08-10", FixtureStatus::Completed),
                make_fixture("f1", "t1", "t2", "2025-08-24", FixtureStatus::Scheduled),
                make_fixture("f2", "t1", "t2", "2025-09-07", FixtureStatus::Scheduled),
                make_fixture("f3", "t2", "t1", "2025-09-21", FixtureStatus::Scheduled),
                make_fixture("f4", "t2", "t1", "2025-10-05", FixtureStatus::Scheduled),
            ],
            ..Default::default()
        };

        let mut game = Game::new(
            make_clock(),
            make_manager(Some("t1")),
            vec![team_a, team_b],
            vec![],
            vec![],
            vec![],
        );
        game.competitions = vec![league];
        game
    }

    fn news_article(id: &str, date: &str, read: bool) -> NewsArticle {
        let mut article = NewsArticle::new(
            id.to_string(),
            "Headline".to_string(),
            "Body".to_string(),
            "Source".to_string(),
            date.to_string(),
            NewsCategory::Editorial,
        );
        article.read = read;
        article
    }

    #[test]
    fn session_unread_news_excludes_future_dated_articles() {
        use chrono::TimeZone;
        let mut game = make_game_with_team();
        game.clock.current_date = Utc.with_ymd_and_hms(2026, 2, 15, 12, 0, 0).unwrap();
        game.news = vec![
            news_article("past-unread", "2026-02-10", false),
            news_article("today-unread", "2026-02-15T08:00:00+00:00", false),
            news_article("future-unread", "2026-06-03", false),
            news_article("past-read", "2026-02-01", true),
        ];

        let session = project_session(&game);

        // Past + same-day unread count; the future-dated article (e.g. a World
        // Cup kickoff) does not inflate the badge before it happens.
        assert_eq!(session.unread_news_count, 2);
    }

    #[test]
    fn session_includes_manager_team() {
        let game = make_game_with_team();
        let session = project_session(&game);

        assert_eq!(session.manager.team_id, Some("t1".to_string()));
        assert!(session.team.is_some());
        assert_eq!(session.team.as_ref().unwrap().id, "t1");
    }

    #[test]
    fn session_competition_standings_resolved() {
        let game = make_game_with_team();
        let session = project_session(&game);

        let comp = session.user_competition.as_ref().unwrap();
        assert_eq!(comp.competition_id, "league1");
        assert_eq!(comp.standings.len(), 2);
        assert_eq!(comp.standings[0].team_name, "Arsenal");
        assert_eq!(comp.standings[1].team_name, "Chelsea");
        assert_eq!(comp.standings[0].points, 4);
    }

    #[test]
    fn session_upcoming_capped_at_3_and_sorted() {
        let game = make_game_with_team();
        let session = project_session(&game);

        let comp = session.user_competition.as_ref().unwrap();
        assert_eq!(comp.upcoming_fixtures.len(), 3, "capped at 3 upcoming");
        assert!(comp.upcoming_fixtures[0].date <= comp.upcoming_fixtures[1].date);
        assert!(comp.upcoming_fixtures[1].date <= comp.upcoming_fixtures[2].date);
    }

    #[test]
    fn session_recent_capped_at_2_and_most_recent_first() {
        let mut game = make_game_with_team();
        // Add a second completed fixture with a later date.
        game.competitions[0].fixtures.push(make_fixture(
            "f_past2",
            "t1",
            "t2",
            "2025-08-17",
            FixtureStatus::Completed,
        ));

        let session = project_session(&game);
        let comp = session.user_competition.as_ref().unwrap();

        assert!(comp.recent_fixtures.len() <= 2);
        if comp.recent_fixtures.len() == 2 {
            assert!(comp.recent_fixtures[0].date >= comp.recent_fixtures[1].date);
        }
    }

    #[test]
    fn session_unread_counts() {
        let mut game = make_game_with_team();

        let mut unread_msg = InboxMessage::new(
            "m1".to_string(),
            String::new(),
            String::new(),
            String::new(),
            "2025-01-01".to_string(),
        );
        unread_msg.read = false;
        let mut read_msg = InboxMessage::new(
            "m2".to_string(),
            String::new(),
            String::new(),
            String::new(),
            "2025-01-01".to_string(),
        );
        read_msg.read = true;
        game.messages = vec![unread_msg, read_msg];

        let mut unread_article = NewsArticle::new(
            "n1".to_string(),
            String::new(),
            String::new(),
            String::new(),
            "2025-01-01".to_string(),
            NewsCategory::MatchReport,
        );
        unread_article.read = false;
        let mut read_article = NewsArticle::new(
            "n2".to_string(),
            String::new(),
            String::new(),
            String::new(),
            "2025-01-01".to_string(),
            NewsCategory::MatchReport,
        );
        read_article.read = true;
        game.news = vec![unread_article, read_article];

        let session = project_session(&game);
        assert_eq!(session.unread_messages_count, 1);
        assert_eq!(session.unread_news_count, 1);
    }

    #[test]
    fn session_no_team_when_between_jobs() {
        let mut game = make_game_with_team();
        game.manager.team_id = None;
        let session = project_session(&game);

        assert!(session.team.is_none());
        assert!(session.user_competition.is_none());
    }
}
