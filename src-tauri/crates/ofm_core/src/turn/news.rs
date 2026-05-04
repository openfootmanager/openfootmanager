use crate::game::Game;
use crate::messages;
use crate::news;
use chrono::Datelike;
use domain::league::{Fixture, FixtureStatus, League, StandingEntry};

fn completed_fixtures_for_day<'a>(league: &'a League, today: &str) -> Vec<&'a Fixture> {
    league
        .fixtures
        .iter()
        .filter(|fixture| {
            fixture.date == today
                && fixture.status == FixtureStatus::Completed
                && fixture.counts_for_league_standings()
        })
        .collect()
}

fn team_name_or(game: &Game, team_id: &str, fallback: &str) -> String {
    game.teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| fallback.to_string())
}

fn team_name(game: &Game, team_id: &str) -> String {
    team_name_or(game, team_id, "")
}

fn player_match_name_or_id(game: &Game, player_id: &str) -> String {
    game.players
        .iter()
        .find(|player| player.id == player_id)
        .map(|player| player.match_name.clone())
        .unwrap_or_else(|| player_id.to_string())
}

fn scorers_for_side(
    game: &Game,
    report: &engine::MatchReport,
    side: engine::Side,
) -> Vec<(String, u32)> {
    report
        .goals
        .iter()
        .filter(|goal| goal.side == side)
        .map(|goal| {
            (
                player_match_name_or_id(game, &goal.scorer_id),
                goal.minute as u32,
            )
        })
        .collect()
}

fn matchday_results(game: &Game, fixtures: &[&Fixture]) -> Vec<(String, u8, String, u8)> {
    fixtures
        .iter()
        .map(|fixture| {
            let (home_goals, away_goals) = fixture
                .result
                .as_ref()
                .map(|result| (result.home_goals, result.away_goals))
                .unwrap_or((0, 0));
            (
                team_name(game, &fixture.home_team_id),
                home_goals,
                team_name(game, &fixture.away_team_id),
                away_goals,
            )
        })
        .collect()
}

fn standings_rows(game: &Game, league: &League) -> Vec<(String, u32, i16)> {
    let mut standings: Vec<(String, u32, i16)> = league
        .standings
        .iter()
        .map(|entry| {
            (
                team_name(game, &entry.team_id),
                entry.points,
                entry.goal_difference() as i16,
            )
        })
        .collect();
    standings.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
    standings
}

fn pre_match_target_date(today: &str) -> Option<String> {
    let today_date = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d").ok()?;
    Some(
        (today_date + chrono::Duration::days(3))
            .format("%Y-%m-%d")
            .to_string(),
    )
}

fn scheduled_user_fixtures_for_date<'a>(
    league: &'a League,
    user_team_id: &str,
    target_date: &str,
) -> Vec<&'a Fixture> {
    league
        .fixtures
        .iter()
        .filter(|fixture| {
            fixture.date == target_date
                && fixture.status == FixtureStatus::Scheduled
                && fixture.counts_for_league_standings()
                && (fixture.home_team_id == user_team_id || fixture.away_team_id == user_team_id)
        })
        .collect()
}

fn opponent_for_fixture<'a>(fixture: &'a Fixture, user_team_id: &str) -> (&'a str, bool) {
    if fixture.home_team_id == user_team_id {
        (&fixture.away_team_id, true)
    } else {
        (&fixture.home_team_id, false)
    }
}

fn weekly_digest_suffix(game: &Game) -> String {
    let iso_week = game.clock.current_date.iso_week();
    format!("{}_w{:02}", iso_week.year(), iso_week.week())
}

fn season_has_started(league: &League) -> bool {
    crate::end_of_season::season_has_started(league)
}

fn title_race_is_newsworthy(leader: &StandingEntry, challenger: &StandingEntry) -> bool {
    leader.played >= 5
        && challenger.played >= 5
        && leader.points > 0
        && leader.points.saturating_sub(challenger.points) <= 3
}

fn has_equivalent_storyline(game: &Game, candidate: &domain::news::NewsArticle) -> bool {
    game.news.iter().any(|article| {
        article.category == candidate.category
            && article.headline_key == candidate.headline_key
            && article.body_key == candidate.body_key
            && article.source_key == candidate.source_key
            && article.team_ids == candidate.team_ids
            && article.player_ids == candidate.player_ids
            && article.i18n_params == candidate.i18n_params
    })
}

fn unbeaten_run_length(form: &[String]) -> u32 {
    let mut streak = 0;

    for result in form.iter().rev() {
        if result == "L" {
            break;
        }

        if result == "W" || result == "D" {
            streak += 1;
        }
    }

    streak
}

fn top_scorer_summary(game: &Game) -> Option<(String, u32)> {
    game.players
        .iter()
        .filter(|player| player.stats.goals > 0)
        .max_by(|a, b| {
            a.stats
                .goals
                .cmp(&b.stats.goals)
                .then_with(|| a.match_name.cmp(&b.match_name))
        })
        .map(|player| (player.match_name.clone(), player.stats.goals))
}

fn weekly_storyline_articles(
    game: &Game,
    suffix: &str,
    date: &str,
) -> Vec<domain::news::NewsArticle> {
    let mut articles = Vec::new();
    let league = match &game.league {
        Some(league) => league,
        None => return articles,
    };

    let sorted_standings = league.sorted_standings();
    if sorted_standings.len() >= 2 {
        let leader = &sorted_standings[0];
        let challenger = &sorted_standings[1];

        if title_race_is_newsworthy(leader, challenger) {
            let leader_name = team_name(game, &leader.team_id);
            let challenger_name = team_name(game, &challenger.team_id);
            let gap = leader.points.saturating_sub(challenger.points);
            let article = news::title_race_storyline_article(
                &format!("storyline_title_race_{}", suffix),
                &leader.team_id,
                &leader_name,
                &challenger.team_id,
                &challenger_name,
                gap,
                date,
            );

            if !has_equivalent_storyline(game, &article) {
                articles.push(article);
            }
        }
    }

    if let Some(team) = game
        .teams
        .iter()
        .map(|team| (team, unbeaten_run_length(&team.form)))
        .filter(|(_, streak)| *streak >= 5)
        .max_by_key(|(_, streak)| *streak)
        .map(|(team, streak)| (team.id.clone(), team.name.clone(), streak))
    {
        let article = news::unbeaten_streak_storyline_article(
            &format!("storyline_unbeaten_streak_{}", suffix),
            &team.0,
            &team.1,
            team.2,
            date,
        );

        if !has_equivalent_storyline(game, &article) {
            articles.push(article);
        }
    }

    articles
}

pub(super) fn generate_weekly_digest_news(game: &mut Game, today: &str) {
    if game.clock.current_date.weekday().num_days_from_monday() != 0 {
        return;
    }

    let league = match &game.league {
        Some(league) => league,
        None => return,
    };

    if !season_has_started(league) {
        return;
    }

    let suffix = weekly_digest_suffix(game);
    let digest_id = format!("weekly_digest_{}", suffix);
    if game.news.iter().any(|article| article.id == digest_id) {
        return;
    }

    let date = game.clock.current_date.to_rfc3339();
    let sorted_standings = league.sorted_standings();
    let leader = sorted_standings
        .first()
        .map(|entry| team_name(game, &entry.team_id))
        .unwrap_or_else(|| "Unknown".to_string());
    let storylines = weekly_storyline_articles(game, &suffix, &date);
    let (top_scorer, top_scorer_goals) =
        top_scorer_summary(game).unwrap_or_else(|| (String::new(), 0));

    game.news.push(news::weekly_digest_article(
        &digest_id,
        today,
        &leader,
        &top_scorer,
        top_scorer_goals,
        storylines.len(),
        &date,
    ));
    game.news.extend(storylines);
}

/// Generate a match report news article for the completed fixture.
pub(super) fn generate_match_news(
    game: &mut Game,
    fixture_index: usize,
    home_team_id: &str,
    away_team_id: &str,
    report: &engine::MatchReport,
) {
    let fixture = &game.league.as_ref().unwrap().fixtures[fixture_index];
    let article_id = format!("report_{}", fixture.id);
    if game.news.iter().any(|n| n.id == article_id) {
        return;
    }

    let home_name = team_name_or(game, home_team_id, "Home");
    let away_name = team_name_or(game, away_team_id, "Away");
    let home_scorers = scorers_for_side(game, report, engine::Side::Home);
    let away_scorers = scorers_for_side(game, report, engine::Side::Away);

    let article = news::match_report_article(
        &fixture.id,
        &home_name,
        &away_name,
        report.home_goals,
        report.away_goals,
        home_team_id,
        away_team_id,
        fixture.matchday,
        &home_scorers,
        &away_scorers,
        &game.clock.current_date.to_rfc3339(),
    );
    game.news.push(article);
}

/// After all matches in a matchday are simulated, generate roundup + standings news.
pub fn generate_matchday_news(game: &mut Game, today: &str) {
    let league = match &game.league {
        Some(l) => l,
        None => return,
    };

    let todays_fixtures = completed_fixtures_for_day(league, today);

    if todays_fixtures.is_empty() {
        return;
    }

    let matchday = todays_fixtures[0].matchday;
    let date_str = game.clock.current_date.to_rfc3339();

    // Don't duplicate
    let roundup_id = format!("roundup_md{}", matchday);
    if game.news.iter().any(|n| n.id == roundup_id) {
        return;
    }

    let results = matchday_results(game, &todays_fixtures);

    let roundup = news::league_roundup_article(matchday, &results, &date_str);
    game.news.push(roundup);

    let standings = standings_rows(game, league);

    let standings_article = news::standings_update_article(matchday, &standings, &date_str);
    game.news.push(standings_article);
}

pub(super) fn generate_pre_match_messages(game: &mut Game, today: &str) {
    let user_team_id = match &game.manager.team_id {
        Some(id) => id.clone(),
        None => return,
    };

    let target_str = match pre_match_target_date(today) {
        Some(date) => date,
        None => return,
    };

    if let Some(league) = &game.league {
        let upcoming = scheduled_user_fixtures_for_date(league, &user_team_id, &target_str);

        for fixture in upcoming {
            let (opponent_id, is_home) = opponent_for_fixture(fixture, &user_team_id);
            let opponent_name = team_name_or(game, opponent_id, "Unknown");

            // Check if we already sent this message
            let msg_id = format!("prematch_{}", fixture.id);
            let already_sent = game.messages.iter().any(|m| m.id == msg_id);
            if already_sent {
                continue;
            }

            let msg = messages::pre_match_message(
                &fixture.id,
                &opponent_name,
                opponent_id,
                is_home,
                fixture.matchday,
                &target_str,
                &game.clock.current_date.to_rfc3339(),
            );
            game.messages.push(msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        generate_match_news, generate_matchday_news, generate_pre_match_messages,
        generate_weekly_digest_news,
    };
    use crate::clock::GameClock;
    use crate::game::Game;
    use chrono::{TimeZone, Utc};
    use domain::league::{
        Fixture, FixtureCompetition, FixtureStatus, League, MatchResult, StandingEntry,
    };
    use domain::manager::Manager;
    use domain::message::{MessageCategory, MessagePriority};
    use domain::news::NewsCategory;
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;
    use engine::{GoalDetail, MatchReport, Side, TeamStats};
    use std::collections::HashMap;

    fn make_team(id: &str, name: &str) -> Team {
        Team::new(
            id.to_string(),
            name.to_string(),
            name.to_string(),
            "England".to_string(),
            "Test City".to_string(),
            format!("{} Ground", name),
            20_000,
        )
    }

    fn make_manager() -> Manager {
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());
        manager
    }

    fn make_fixture(
        id: &str,
        matchday: u32,
        date: &str,
        home_team_id: &str,
        away_team_id: &str,
        status: FixtureStatus,
        result: Option<(u8, u8)>,
    ) -> Fixture {
        Fixture {
            id: id.to_string(),
            matchday,
            date: date.to_string(),
            home_team_id: home_team_id.to_string(),
            away_team_id: away_team_id.to_string(),
            competition: FixtureCompetition::League,
            status,
            result: result.map(|(home_goals, away_goals)| MatchResult {
                home_goals,
                away_goals,
                home_scorers: vec![],
                away_scorers: vec![],
                report: None,
            }),
        }
    }

    fn default_attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 70,
            stamina: 70,
            strength: 65,
            agility: 68,
            passing: 66,
            shooting: 72,
            tackling: 40,
            dribbling: 69,
            defending: 38,
            positioning: 64,
            vision: 65,
            decisions: 67,
            composure: 66,
            aggression: 50,
            teamwork: 64,
            leadership: 52,
            handling: 20,
            reflexes: 20,
            aerial: 45,
        }
    }

    fn make_player(id: &str, name: &str, team_id: &str) -> Player {
        let mut player = Player::new(
            id.to_string(),
            name.to_string(),
            format!("Full {}", name),
            "1998-03-15".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        player.team_id = Some(team_id.to_string());
        player
    }

    fn make_report(goals: Vec<GoalDetail>, home_goals: u8, away_goals: u8) -> MatchReport {
        MatchReport {
            home_goals,
            away_goals,
            home_stats: TeamStats::default(),
            away_stats: TeamStats::default(),
            events: vec![],
            goals,
            player_stats: HashMap::new(),
            home_possession: 50.0,
            total_minutes: 90,
        }
    }

    fn make_game(today: &str, todays_fixture_status: FixtureStatus) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 8, 12, 12, 0, 0).unwrap());
        let manager = make_manager();
        let teams = vec![
            make_team("team1", "Alpha FC"),
            make_team("team2", "Beta FC"),
            make_team("team3", "Gamma FC"),
        ];

        let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

        let mut alpha = StandingEntry::new("team1".to_string());
        alpha.record_result(2, 1);
        let mut beta = StandingEntry::new("team2".to_string());
        beta.record_result(1, 2);
        let gamma = StandingEntry::new("team3".to_string());

        game.league = Some(League {
            id: "league1".to_string(),
            name: "Premier Division".to_string(),
            season: 1,
            fixtures: vec![
                make_fixture(
                    "fx1",
                    4,
                    today,
                    "team1",
                    "team2",
                    todays_fixture_status,
                    Some((2, 1)),
                ),
                make_fixture(
                    "fx2",
                    4,
                    "2025-08-13",
                    "team3",
                    "team2",
                    FixtureStatus::Completed,
                    Some((0, 0)),
                ),
            ],
            standings: vec![alpha, beta, gamma],
            transfer_log: vec![],
        });

        game
    }

    fn set_current_date(game: &mut Game, year: i32, month: u32, day: u32) {
        game.clock = GameClock::new(Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap());
    }

    fn standing_mut<'a>(game: &'a mut Game, team_id: &str) -> &'a mut StandingEntry {
        game.league
            .as_mut()
            .unwrap()
            .standings
            .iter_mut()
            .find(|entry| entry.team_id == team_id)
            .unwrap()
    }

    fn team_mut<'a>(game: &'a mut Game, team_id: &str) -> &'a mut Team {
        game.teams
            .iter_mut()
            .find(|team| team.id == team_id)
            .unwrap()
    }

    fn reset_to_preseason(game: &mut Game) {
        let league = game.league.as_mut().unwrap();
        for fixture in &mut league.fixtures {
            fixture.status = FixtureStatus::Scheduled;
        }
        league.standings = vec![
            StandingEntry::new("team1".to_string()),
            StandingEntry::new("team2".to_string()),
            StandingEntry::new("team3".to_string()),
        ];
    }

    #[test]
    fn generate_matchday_news_adds_roundup_and_standings_for_completed_fixtures_today() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);

        generate_matchday_news(&mut game, "2025-08-12");

        assert_eq!(game.news.len(), 2);

        let roundup = game
            .news
            .iter()
            .find(|article| article.id == "roundup_md4")
            .unwrap();
        assert_eq!(roundup.category, NewsCategory::LeagueRoundup);
        assert!(roundup.body.contains("Alpha FC 2 - 1 Beta FC"));
        assert!(!roundup.body.contains("Gamma FC"));

        let standings = game
            .news
            .iter()
            .find(|article| article.id == "standings_md4")
            .unwrap();
        assert_eq!(standings.category, NewsCategory::StandingsUpdate);
        assert!(standings.body.contains("Alpha FC sit at the top"));
    }

    #[test]
    fn generate_matchday_news_does_nothing_when_today_has_no_completed_fixtures() {
        let mut game = make_game("2025-08-12", FixtureStatus::Scheduled);

        generate_matchday_news(&mut game, "2025-08-12");

        assert!(game.news.is_empty());
    }

    #[test]
    fn generate_matchday_news_does_not_duplicate_articles_on_repeat_calls() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);

        generate_matchday_news(&mut game, "2025-08-12");
        generate_matchday_news(&mut game, "2025-08-12");

        assert_eq!(game.news.len(), 2);
        assert_eq!(
            game.news
                .iter()
                .filter(|article| article.id == "roundup_md4")
                .count(),
            1
        );
        assert_eq!(
            game.news
                .iter()
                .filter(|article| article.id == "standings_md4")
                .count(),
            1
        );
    }

    #[test]
    fn generate_match_news_resolves_known_names_and_falls_back_to_scorer_ids() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);
        game.players = vec![make_player("p1", "Alice", "team1")];

        let report = make_report(
            vec![
                GoalDetail {
                    minute: 10,
                    scorer_id: "p1".to_string(),
                    assist_id: None,
                    is_penalty: false,
                    side: Side::Home,
                },
                GoalDetail {
                    minute: 74,
                    scorer_id: "ghost9".to_string(),
                    assist_id: None,
                    is_penalty: false,
                    side: Side::Away,
                },
            ],
            1,
            1,
        );

        generate_match_news(&mut game, 0, "team1", "team2", &report);

        assert_eq!(game.news.len(), 1);

        let article = &game.news[0];
        assert_eq!(article.id, "report_fx1");
        assert_eq!(article.category, NewsCategory::MatchReport);
        assert_eq!(
            article.team_ids,
            vec!["team1".to_string(), "team2".to_string()]
        );
        assert_eq!(
            article.player_ids,
            vec!["Alice".to_string(), "ghost9".to_string()]
        );
        assert_eq!(
            article.match_score.as_ref().map(|score| (
                score.home_team_id.as_str(),
                score.away_team_id.as_str(),
                score.home_goals,
                score.away_goals,
            )),
            Some(("team1", "team2", 1, 1))
        );
        assert_eq!(
            article.i18n_params.get("scorers"),
            Some(&"Alice (10', Alpha FC), ghost9 (74', Beta FC)".to_string())
        );
    }

    #[test]
    fn generate_match_news_does_not_duplicate_existing_report_article() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);
        let report = make_report(vec![], 0, 0);

        generate_match_news(&mut game, 0, "team1", "team2", &report);
        generate_match_news(&mut game, 0, "team1", "team2", &report);

        assert_eq!(game.news.len(), 1);
        assert_eq!(
            game.news
                .iter()
                .filter(|article| article.id == "report_fx1")
                .count(),
            1
        );
    }

    #[test]
    fn generate_pre_match_messages_adds_preview_metadata_for_user_fixture_three_days_ahead() {
        let mut game = make_game("2025-08-15", FixtureStatus::Scheduled);

        generate_pre_match_messages(&mut game, "2025-08-12");

        assert_eq!(game.messages.len(), 1);

        let message = &game.messages[0];
        assert_eq!(message.id, "prematch_fx1");
        assert_eq!(message.category, MessageCategory::MatchPreview);
        assert_eq!(message.priority, MessagePriority::Normal);
        assert!(message.subject.contains("Beta FC"));
        assert!(message.subject.contains("(H)"));
        assert_eq!(message.context.fixture_id.as_deref(), Some("fx1"));
        assert_eq!(message.context.team_id.as_deref(), Some("team2"));
        assert_eq!(message.i18n_params.get("venue"), Some(&"home".to_string()));
        assert_eq!(
            message.i18n_params.get("opponent"),
            Some(&"Beta FC".to_string())
        );
        assert_eq!(
            message.i18n_params.get("matchDate"),
            Some(&"2025-08-15".to_string())
        );
        assert_eq!(message.i18n_params.get("matchday"), Some(&"4".to_string()));
    }

    #[test]
    fn generate_pre_match_messages_skips_fixtures_without_user_team() {
        let mut game = make_game("2025-08-15", FixtureStatus::Scheduled);
        let fixture = &mut game.league.as_mut().unwrap().fixtures[0];
        fixture.home_team_id = "team2".to_string();
        fixture.away_team_id = "team3".to_string();

        generate_pre_match_messages(&mut game, "2025-08-12");

        assert!(game.messages.is_empty());
    }

    #[test]
    fn generate_pre_match_messages_does_not_duplicate_same_fixture() {
        let mut game = make_game("2025-08-15", FixtureStatus::Scheduled);

        generate_pre_match_messages(&mut game, "2025-08-12");
        generate_pre_match_messages(&mut game, "2025-08-12");

        assert_eq!(game.messages.len(), 1);
        assert_eq!(
            game.messages
                .iter()
                .filter(|message| message.id == "prematch_fx1")
                .count(),
            1
        );
    }

    #[test]
    fn generate_weekly_digest_news_only_runs_on_monday_cadence() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);

        generate_weekly_digest_news(&mut game, "2025-08-12");

        assert!(
            game.news
                .iter()
                .all(|article| !article.id.starts_with("weekly_digest_"))
        );

        set_current_date(&mut game, 2025, 8, 11);
        generate_weekly_digest_news(&mut game, "2025-08-11");

        assert!(
            game.news
                .iter()
                .any(|article| article.id.starts_with("weekly_digest_"))
        );
    }

    #[test]
    fn generate_weekly_digest_news_skips_preseason_even_on_monday() {
        let mut game = make_game("2025-08-11", FixtureStatus::Scheduled);
        set_current_date(&mut game, 2025, 8, 11);
        reset_to_preseason(&mut game);

        generate_weekly_digest_news(&mut game, "2025-08-11");

        assert!(
            game.news
                .iter()
                .all(|article| !article.id.starts_with("weekly_digest_"))
        );
        assert!(
            game.news
                .iter()
                .all(|article| !article.id.starts_with("storyline_"))
        );
    }

    #[test]
    fn generate_weekly_digest_news_creates_storylines_from_standings_and_form() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);

        let alpha = standing_mut(&mut game, "team1");
        alpha.played = 10;
        alpha.points = 25;
        alpha.goals_for = 18;
        alpha.goals_against = 8;

        let beta = standing_mut(&mut game, "team2");
        beta.played = 10;
        beta.points = 24;
        beta.goals_for = 16;
        beta.goals_against = 9;

        let gamma = standing_mut(&mut game, "team3");
        gamma.played = 10;
        gamma.points = 7;
        gamma.goals_for = 6;
        gamma.goals_against = 15;

        team_mut(&mut game, "team1").form = vec![
            "D".to_string(),
            "W".to_string(),
            "W".to_string(),
            "W".to_string(),
            "W".to_string(),
        ];

        generate_weekly_digest_news(&mut game, "2025-08-11");

        let weekly_digest = game
            .news
            .iter()
            .find(|article| article.id.starts_with("weekly_digest_"))
            .unwrap();
        assert_eq!(weekly_digest.category, NewsCategory::Editorial);
        assert_eq!(
            weekly_digest.headline_key.as_deref(),
            Some("be.news.weeklyDigest.headline")
        );
        assert_eq!(
            weekly_digest.i18n_params.get("weekStart"),
            Some(&"2025-08-11".to_string())
        );
        assert!(weekly_digest.i18n_params.get("weekLabel").is_none());

        let title_race = game
            .news
            .iter()
            .find(|article| article.id.starts_with("storyline_title_race_"))
            .unwrap();
        assert_eq!(title_race.category, NewsCategory::Editorial);
        assert_eq!(
            title_race.headline_key.as_deref(),
            Some("be.news.storyline.titleRace.headline")
        );
        assert_eq!(
            title_race.body_key.as_deref(),
            Some("be.news.storyline.titleRace.body")
        );
        assert_eq!(
            title_race.i18n_params.get("leader"),
            Some(&"Alpha FC".to_string())
        );
        assert_eq!(
            title_race.i18n_params.get("challenger"),
            Some(&"Beta FC".to_string())
        );
        assert_eq!(title_race.i18n_params.get("gap"), Some(&"1".to_string()));

        let unbeaten = game
            .news
            .iter()
            .find(|article| article.id.starts_with("storyline_unbeaten_streak_"))
            .unwrap();
        assert_eq!(unbeaten.category, NewsCategory::Editorial);
        assert_eq!(
            unbeaten.headline_key.as_deref(),
            Some("be.news.storyline.unbeatenStreak.headline")
        );
        assert_eq!(
            unbeaten.body_key.as_deref(),
            Some("be.news.storyline.unbeatenStreak.body")
        );
        assert_eq!(
            unbeaten.i18n_params.get("team"),
            Some(&"Alpha FC".to_string())
        );
        assert_eq!(
            unbeaten.i18n_params.get("runLength"),
            Some(&"5".to_string())
        );
    }

    #[test]
    fn generate_weekly_digest_news_does_not_duplicate_same_week() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);

        generate_weekly_digest_news(&mut game, "2025-08-11");
        generate_weekly_digest_news(&mut game, "2025-08-11");

        assert_eq!(
            game.news
                .iter()
                .filter(|article| article.id.starts_with("weekly_digest_"))
                .count(),
            1
        );
    }

    #[test]
    fn generate_weekly_digest_news_does_not_repeat_identical_storylines_in_later_weeks() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);

        let alpha = standing_mut(&mut game, "team1");
        alpha.played = 10;
        alpha.points = 25;
        alpha.goals_for = 18;
        alpha.goals_against = 8;

        let beta = standing_mut(&mut game, "team2");
        beta.played = 10;
        beta.points = 24;
        beta.goals_for = 16;
        beta.goals_against = 9;

        let gamma = standing_mut(&mut game, "team3");
        gamma.played = 10;
        gamma.points = 7;
        gamma.goals_for = 6;
        gamma.goals_against = 15;

        team_mut(&mut game, "team1").form = vec![
            "D".to_string(),
            "W".to_string(),
            "W".to_string(),
            "W".to_string(),
            "W".to_string(),
        ];

        generate_weekly_digest_news(&mut game, "2025-08-11");

        set_current_date(&mut game, 2025, 8, 18);
        generate_weekly_digest_news(&mut game, "2025-08-18");

        assert_eq!(
            game.news
                .iter()
                .filter(|article| article.id.starts_with("weekly_digest_"))
                .count(),
            2
        );
        assert_eq!(
            game.news
                .iter()
                .filter(|article| article.id.starts_with("storyline_title_race_"))
                .count(),
            1
        );
        assert_eq!(
            game.news
                .iter()
                .filter(|article| article.id.starts_with("storyline_unbeaten_streak_"))
                .count(),
            1
        );
    }
}
