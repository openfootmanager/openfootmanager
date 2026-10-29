use super::params;
use domain::league::FixtureCompetition;
use domain::news::*;
use rand::RngExt;
use serde::Serialize;

/// One goal as the report tells it: who scored, under the name the reader knows, and when.
pub struct ReportScorer {
    pub player_id: String,
    pub name: String,
    pub minute: u32,
}

/// One side of a finished match.
pub struct ReportSide<'a> {
    pub team_id: &'a str,
    pub name: &'a str,
    pub goals: u8,
    pub scorers: &'a [ReportScorer],
}

/// The competition a match was played in, as the report names it.
///
/// A generated competition's `name` is an English template ("Germany Cup"); the frontend shows
/// the translated `name_key` with the country and season, as every other screen does. The key,
/// the country and the season travel with the name so the report can be translated the same way.
pub struct ReportCompetition<'a> {
    pub kind: FixtureCompetition,
    pub name: &'a str,
    pub name_key: Option<&'a str>,
    pub country_id: Option<&'a str>,
    pub season: u32,
}

/// Everything a match report is written from.
pub struct MatchReportFacts<'a> {
    pub fixture_id: &'a str,
    pub competition: ReportCompetition<'a>,
    pub matchday: u32,
    pub date: &'a str,
    pub home: ReportSide<'a>,
    pub away: ReportSide<'a>,
}

#[derive(Serialize)]
struct MatchReportScorerParam<'a> {
    player: &'a str,
    minute: u32,
    team: &'a str,
}

fn scorers_in_order<'a>(
    facts: &'a MatchReportFacts<'_>,
) -> impl Iterator<Item = (&'a ReportScorer, &'a str)> {
    let home = facts.home.scorers.iter().map(|s| (s, facts.home.name));
    let away = facts.away.scorers.iter().map(|s| (s, facts.away.name));
    home.chain(away)
}

/// The scorers as player ids, each once: a hat-trick is one player to link to, not three.
fn scorer_player_ids(facts: &MatchReportFacts<'_>) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for (scorer, _) in scorers_in_order(facts) {
        if !ids.contains(&scorer.player_id) {
            ids.push(scorer.player_id.clone());
        }
    }
    ids
}

fn scorer_params_json(facts: &MatchReportFacts<'_>) -> String {
    let scorers: Vec<MatchReportScorerParam<'_>> = scorers_in_order(facts)
        .map(|(scorer, team)| MatchReportScorerParam {
            player: &scorer.name,
            minute: scorer.minute,
            team,
        })
        .collect();

    serde_json::to_string(&scorers).unwrap_or_else(|_| "[]".to_string())
}

fn outcome_key(home_goals: u8, away_goals: u8) -> &'static str {
    if home_goals > away_goals {
        "homeWin"
    } else if away_goals > home_goals {
        "awayWin"
    } else {
        "draw"
    }
}

/// The title and body keys of a report on a match outside the league round.
///
/// A cup tie or a continental or international match is competitive football, so it gets copy
/// that names its competition; only friendlies are "building sharpness before the campaign".
fn non_league_copy(competition: &FixtureCompetition) -> (&'static str, &'static str) {
    match competition {
        FixtureCompetition::Friendly | FixtureCompetition::FriendlyCup => (
            "be.news.matchReport.reportFriendly.title",
            "be.news.matchReport.reportFriendly.body",
        ),
        FixtureCompetition::PreseasonTournament => (
            "be.news.matchReport.reportPreseason.title",
            "be.news.matchReport.reportPreseason.body",
        ),
        FixtureCompetition::Cup
        | FixtureCompetition::ContinentalClub
        | FixtureCompetition::InternationalClub
        | FixtureCompetition::InternationalNation
        | FixtureCompetition::League => (
            "be.news.matchReport.reportCompetition.title",
            "be.news.matchReport.reportCompetition.body",
        ),
    }
}

/// Generate a match report news article for a completed fixture.
pub fn match_report_article(facts: &MatchReportFacts<'_>) -> NewsArticle {
    let home = &facts.home;
    let away = &facts.away;
    let mut rng = crate::seed::rng_from_key(&format!(
        "news/match-report/{}/{}/{}/{}",
        home.team_id, away.team_id, facts.matchday, facts.date
    ));
    let is_league_fixture = matches!(facts.competition.kind, FixtureCompetition::League);

    let scorers_data = scorer_params_json(facts);

    let source_keys = [
        "be.source.sportsGazette",
        "be.source.footballHerald",
        "be.source.matchDayPress",
        "be.source.leagueChronicle",
    ];
    let src_idx = rng.random_range(0..source_keys.len());
    let source_key = source_keys[src_idx];

    let player_ids = scorer_player_ids(facts);
    let score = NewsMatchScore {
        home_team_id: home.team_id.to_string(),
        away_team_id: away.team_id.to_string(),
        home_goals: home.goals,
        away_goals: away.goals,
    };
    let home_goals = home.goals.to_string();
    let away_goals = away.goals.to_string();

    if !is_league_fixture {
        let (title_key, body_key) = non_league_copy(&facts.competition.kind);
        let mut report_params = params(&[
            ("home", home.name),
            ("away", away.name),
            ("homeGoals", &home_goals),
            ("awayGoals", &away_goals),
            ("competition", facts.competition.name),
            ("competitionSeason", &facts.competition.season.to_string()),
            ("scorers", ""),
            ("scorersSection", ""),
            ("scorersData", &scorers_data),
        ]);
        if let Some(name_key) = facts.competition.name_key {
            report_params.insert("competitionKey".to_string(), name_key.to_string());
        }
        if let Some(country_id) = facts.competition.country_id {
            report_params.insert("competitionCountry".to_string(), country_id.to_string());
        }

        return NewsArticle::new(
            format!("report_{}", facts.fixture_id),
            String::new(),
            String::new(),
            String::new(),
            facts.date.to_string(),
            NewsCategory::MatchReport,
        )
        .with_teams(vec![home.team_id.to_string(), away.team_id.to_string()])
        .with_players(player_ids)
        .with_i18n(title_key, body_key, source_key, report_params)
        .with_score(score);
    }

    let idx = rng.random_range(0..3);

    // Determine outcome for i18n key
    let outcome = outcome_key(home.goals, away.goals);
    let headline_variant = rng.random_range(0..3u8);
    let has_scorers = !home.scorers.is_empty() || !away.scorers.is_empty();
    let body_key = if has_scorers {
        format!("be.news.matchReport.body{}", idx)
    } else {
        format!("be.news.matchReport.body{}.noScorers", idx)
    };

    NewsArticle::new(
        format!("report_{}", facts.fixture_id),
        String::new(),
        String::new(),
        String::new(),
        facts.date.to_string(),
        NewsCategory::MatchReport,
    )
    .with_teams(vec![home.team_id.to_string(), away.team_id.to_string()])
    .with_players(player_ids)
    .with_score(score)
    .with_i18n(
        &format!(
            "be.news.matchReport.headline.{}.{}",
            outcome, headline_variant
        ),
        &body_key,
        source_key,
        {
            let mut p = params(&[
                ("home", home.name),
                ("away", away.name),
                ("homeGoals", &home_goals),
                ("awayGoals", &away_goals),
                ("matchday", &facts.matchday.to_string()),
                ("scorers", ""),
                ("scorersData", &scorers_data),
            ]);
            // For winner-specific headlines
            if home.goals > away.goals {
                p.insert("winner".to_string(), home.name.to_string());
                p.insert("loser".to_string(), away.name.to_string());
            } else if away.goals > home.goals {
                p.insert("winner".to_string(), away.name.to_string());
                p.insert("loser".to_string(), home.name.to_string());
            }
            p
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{
        MatchReportFacts, ReportCompetition, ReportScorer, ReportSide, match_report_article,
    };
    use domain::league::FixtureCompetition;
    use domain::news::NewsCategory;

    fn scorer(player_id: &str, name: &str, minute: u32) -> ReportScorer {
        ReportScorer {
            player_id: player_id.to_string(),
            name: name.to_string(),
            minute,
        }
    }

    fn facts<'a>(
        fixture_id: &'a str,
        competition: FixtureCompetition,
        goals: (u8, u8),
        scorers: (&'a [ReportScorer], &'a [ReportScorer]),
        matchday: u32,
        date: &'a str,
    ) -> MatchReportFacts<'a> {
        MatchReportFacts {
            fixture_id,
            competition: ReportCompetition {
                kind: competition,
                name: "Premier Division",
                name_key: None,
                country_id: None,
                season: 1,
            },
            matchday,
            date,
            home: ReportSide {
                team_id: "team1",
                name: "Alpha FC",
                goals: goals.0,
                scorers: scorers.0,
            },
            away: ReportSide {
                team_id: "team2",
                name: "Beta FC",
                goals: goals.1,
                scorers: scorers.1,
            },
        }
    }

    #[test]
    fn home_win_article_includes_match_metadata_and_scorers() {
        let home = [scorer("p-alice", "Alice", 10)];
        let away = [scorer("p-bob", "Bob", 75)];
        let article = match_report_article(&facts(
            "fix1",
            FixtureCompetition::League,
            (2, 1),
            (&home, &away),
            5,
            "2025-06-15",
        ));

        assert_eq!(article.id, "report_fix1");
        assert_eq!(article.category, NewsCategory::MatchReport);
        assert_eq!(
            article.team_ids,
            vec!["team1".to_string(), "team2".to_string()]
        );
        assert_eq!(
            article.player_ids,
            vec!["p-alice".to_string(), "p-bob".to_string()]
        );
        let score = article.match_score.as_ref().unwrap();
        assert_eq!(score.home_team_id, "team1");
        assert_eq!(score.away_team_id, "team2");
        assert_eq!(score.home_goals, 2);
        assert_eq!(score.away_goals, 1);
        assert_eq!(article.headline, "");
        assert_eq!(article.body, "");
        assert_eq!(article.source, "");
        assert!(
            article
                .headline_key
                .as_deref()
                .unwrap()
                .starts_with("be.news.matchReport.headline.homeWin.")
        );
        assert!(
            [
                "be.news.matchReport.body0",
                "be.news.matchReport.body1",
                "be.news.matchReport.body2"
            ]
            .contains(&article.body_key.as_deref().unwrap())
        );
        assert!(
            [
                "be.source.sportsGazette",
                "be.source.footballHerald",
                "be.source.matchDayPress",
                "be.source.leagueChronicle"
            ]
            .contains(&article.source_key.as_deref().unwrap())
        );
        assert_eq!(
            article.i18n_params.get("home"),
            Some(&"Alpha FC".to_string())
        );
        assert_eq!(
            article.i18n_params.get("away"),
            Some(&"Beta FC".to_string())
        );
        assert_eq!(article.i18n_params.get("homeGoals"), Some(&"2".to_string()));
        assert_eq!(article.i18n_params.get("awayGoals"), Some(&"1".to_string()));
        assert_eq!(article.i18n_params.get("matchday"), Some(&"5".to_string()));
        assert_eq!(article.i18n_params.get("scorers"), Some(&String::new()));
        assert_eq!(
            article.i18n_params.get("scorersData"),
            Some(
                &"[{\"player\":\"Alice\",\"minute\":10,\"team\":\"Alpha FC\"},{\"player\":\"Bob\",\"minute\":75,\"team\":\"Beta FC\"}]".to_string()
            )
        );
        assert_eq!(
            article.i18n_params.get("winner"),
            Some(&"Alpha FC".to_string())
        );
        assert_eq!(
            article.i18n_params.get("loser"),
            Some(&"Beta FC".to_string())
        );
    }

    #[test]
    fn away_win_article_sets_away_winner_params() {
        let home = [scorer("p-alice", "Alice", 12)];
        let away = [scorer("p-bob", "Bob", 40), scorer("p-ben", "Ben", 88)];
        let article = match_report_article(&facts(
            "fix2",
            FixtureCompetition::League,
            (1, 3),
            (&home, &away),
            6,
            "2025-06-22",
        ));

        assert_eq!(article.body, "");
        assert_eq!(
            article.i18n_params.get("winner"),
            Some(&"Beta FC".to_string())
        );
        assert_eq!(
            article.i18n_params.get("loser"),
            Some(&"Alpha FC".to_string())
        );
        assert!(
            article
                .headline_key
                .as_deref()
                .unwrap()
                .starts_with("be.news.matchReport.headline.awayWin.")
        );
        assert_eq!(
            article.player_ids,
            vec![
                "p-alice".to_string(),
                "p-bob".to_string(),
                "p-ben".to_string()
            ]
        );
    }

    #[test]
    fn draw_article_omits_winner_params_and_goal_section_when_scoreless() {
        let article = match_report_article(&facts(
            "fix3",
            FixtureCompetition::League,
            (0, 0),
            (&[], &[]),
            7,
            "2025-06-29",
        ));

        assert_eq!(article.body, "");
        assert!(
            article
                .headline_key
                .as_deref()
                .unwrap()
                .starts_with("be.news.matchReport.headline.draw.")
        );
        assert!(article.body_key.as_deref().unwrap().ends_with(".noScorers"));
        assert_eq!(article.i18n_params.get("winner"), None);
        assert_eq!(article.i18n_params.get("loser"), None);
        assert_eq!(article.i18n_params.get("scorers"), Some(&String::new()));
        assert!(article.player_ids.is_empty());
    }

    #[test]
    fn friendly_article_uses_non_league_preseason_copy() {
        let article = match_report_article(&facts(
            "fix4",
            FixtureCompetition::Friendly,
            (2, 2),
            (&[], &[]),
            0,
            "2025-07-20",
        ));

        assert_eq!(article.category, NewsCategory::MatchReport);
        assert_eq!(article.headline, "");
        assert_eq!(article.body, "");
        assert_eq!(article.source, "");
        assert_eq!(
            article.headline_key.as_deref(),
            Some("be.news.matchReport.reportFriendly.title")
        );
        assert_eq!(
            article.body_key.as_deref(),
            Some("be.news.matchReport.reportFriendly.body")
        );
        assert!(article.source_key.is_some());
        assert_eq!(article.i18n_params.get("scorers"), Some(&String::new()));
        assert_eq!(
            article.i18n_params.get("scorersSection"),
            Some(&String::new())
        );
        assert_eq!(
            article.i18n_params.get("scorersData"),
            Some(&"[]".to_string())
        );
    }

    /// Given a 1-0 league win where only the home side scored,
    /// when its report is written,
    /// then it uses a body with a goals line, not the scoreless one.
    #[test]
    fn a_one_sided_win_still_lists_its_scorer() {
        let home = [scorer("p-alice", "Alice", 55)];
        let article = match_report_article(&facts(
            "fix6",
            FixtureCompetition::League,
            (1, 0),
            (&home, &[]),
            8,
            "2025-07-28",
        ));

        let body_key = article.body_key.as_deref().unwrap();
        assert!(
            !body_key.ends_with(".noScorers"),
            "a goal was scored, so the report lists it: {body_key}"
        );
    }

    /// Given a preseason-tournament fixture,
    /// when its report is written,
    /// then it uses the preseason-tournament copy.
    #[test]
    fn a_preseason_tournament_match_uses_the_preseason_copy() {
        let article = match_report_article(&facts(
            "fix5",
            FixtureCompetition::PreseasonTournament,
            (1, 0),
            (&[], &[]),
            0,
            "2025-07-21",
        ));

        assert_eq!(
            article.headline_key.as_deref(),
            Some("be.news.matchReport.reportPreseason.title")
        );
        assert_eq!(
            article.body_key.as_deref(),
            Some("be.news.matchReport.reportPreseason.body")
        );
    }
}
