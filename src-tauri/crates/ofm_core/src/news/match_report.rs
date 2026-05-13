use super::params;
use domain::league::FixtureCompetition;
use domain::news::*;
use rand::RngExt;
use serde::Serialize;

fn scoreline_text(home_name: &str, away_name: &str, home_goals: u8, away_goals: u8) -> String {
    format!("{} {}-{} {}", home_name, home_goals, away_goals, away_name)
}

#[derive(Serialize)]
struct MatchReportScorerParam<'a> {
    player: &'a str,
    minute: u32,
    team: &'a str,
}

fn scorer_parts(
    home_name: &str,
    away_name: &str,
    home_scorers: &[(String, u32)],
    away_scorers: &[(String, u32)],
) -> Vec<String> {
    let mut parts = Vec::new();
    for (name, minute) in home_scorers {
        parts.push(format!("{} ({}', {})", name, minute, home_name));
    }
    for (name, minute) in away_scorers {
        parts.push(format!("{} ({}', {})", name, minute, away_name));
    }
    parts
}

fn scorer_player_ids(
    home_scorers: &[(String, u32)],
    away_scorers: &[(String, u32)],
) -> Vec<String> {
    home_scorers
        .iter()
        .chain(away_scorers.iter())
        .map(|(name, _)| name.clone())
        .collect()
}

fn scorers_section(parts: &[String]) -> String {
    if parts.is_empty() {
        String::new()
    } else {
        format!("\n\nGoals: {}", parts.join(", "))
    }
}

fn scorer_params_json(
    home_name: &str,
    away_name: &str,
    home_scorers: &[(String, u32)],
    away_scorers: &[(String, u32)],
) -> String {
    let scorers: Vec<MatchReportScorerParam<'_>> = home_scorers
        .iter()
        .map(|(player, minute)| MatchReportScorerParam {
            player,
            minute: *minute,
            team: home_name,
        })
        .chain(
            away_scorers
                .iter()
                .map(|(player, minute)| MatchReportScorerParam {
                    player,
                    minute: *minute,
                    team: away_name,
                }),
        )
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

/// Generate a match report news article for a completed fixture.
pub fn match_report_article(
    fixture_id: &str,
    home_name: &str,
    away_name: &str,
    home_goals: u8,
    away_goals: u8,
    home_team_id: &str,
    away_team_id: &str,
    competition: FixtureCompetition,
    matchday: u32,
    home_scorers: &[(String, u32)], // (player_name, minute)
    away_scorers: &[(String, u32)],
    date: &str,
) -> NewsArticle {
    let mut rng = rand::rng();
    let is_league_fixture = matches!(competition, FixtureCompetition::League);

    let scoreline = scoreline_text(home_name, away_name, home_goals, away_goals);
    let scorer_parts = scorer_parts(home_name, away_name, home_scorers, away_scorers);
    let scorers_section = scorers_section(&scorer_parts);
    let scorers_data = scorer_params_json(home_name, away_name, home_scorers, away_scorers);

    let source_keys = [
        "be.source.sportsGazette",
        "be.source.footballHerald",
        "be.source.matchDayPress",
        "be.source.leagueChronicle",
    ];
    let sources = [
        "Sports Gazette",
        "The Football Herald",
        "Match Day Press",
        "League Chronicle",
    ];
    let src_idx = rng.random_range(0..sources.len());
    let source = sources[src_idx];
    let source_key = source_keys[src_idx];

    let player_ids = scorer_player_ids(home_scorers, away_scorers);

    if !is_league_fixture {
        let (context, title_key, body_key) = match competition {
            FixtureCompetition::Friendly => (
                "friendly",
                "be.news.matchReport.reportFriendly.title",
                "be.news.matchReport.reportFriendly.body",
            ),
            FixtureCompetition::PreseasonTournament => (
                "preseason tournament",
                "be.news.matchReport.reportPreseason.title",
                "be.news.matchReport.reportPreseason.body",
            ),
            FixtureCompetition::League => unreachable!("league fixtures use the localized branch"),
        };

        return NewsArticle::new(
            format!("report_{}", fixture_id),
            format!(
                "{} {} - {} {}: {} report",
                home_name,
                home_goals,
                away_goals,
                away_name,
                context
            ),
            format!(
                "In {} action, {}. Both sides used the fixture to build sharpness before the competitive campaign.{}",
                context, scoreline, scorers_section
            ),
            source.to_string(),
            date.to_string(),
            NewsCategory::MatchReport,
        )
        .with_teams(vec![home_team_id.to_string(), away_team_id.to_string()])
        .with_players(player_ids)
        .with_i18n(
            title_key,
            body_key,
            source_key,
            params(&[
                ("home", home_name),
                ("away", away_name),
                ("homeGoals", &home_goals.to_string()),
                ("awayGoals", &away_goals.to_string()),
                ("context", context),
                ("scorersSection", &scorers_section),
                ("scorersData", &scorers_data),
            ]),
        )
        .with_score(NewsMatchScore {
            home_team_id: home_team_id.to_string(),
            away_team_id: away_team_id.to_string(),
            home_goals,
            away_goals,
        });
    }

    let commentary = [
        format!(
            "In Matchday {} action, the match ended {}. The result could have implications on the league standings as the season progresses.{}",
            matchday, scoreline, scorers_section
        ),
        format!(
            "The match ended {} in a Matchday {} clash at {}. Both sides gave their all in an engaging contest.{}",
            scoreline,
            matchday,
            format!("{}'s ground", home_name),
            scorers_section
        ),
        format!(
            "Matchday {} delivered another exciting encounter as {} took on {}. The final score was {}. The fans were treated to a competitive fixture.{}",
            matchday, home_name, away_name, scoreline, scorers_section
        ),
    ];

    let idx = rng.random_range(0..commentary.len());

    let headline = if home_goals > away_goals {
        let headlines = [
            format!(
                "{} {} - {} {}: Hosts Triumph",
                home_name, home_goals, away_goals, away_name
            ),
            format!(
                "{} Edge Past {} in Matchday {}",
                home_name, away_name, matchday
            ),
            format!("Clinical {} See Off {}", home_name, away_name),
        ];
        headlines[rng.random_range(0..headlines.len())].clone()
    } else if away_goals > home_goals {
        let headlines = [
            format!(
                "{} {} - {} {}: Visitors Strike",
                home_name, home_goals, away_goals, away_name
            ),
            format!("{} Stun {} on the Road", away_name, home_name),
            format!("Away Day Delight for {}", away_name),
        ];
        headlines[rng.random_range(0..headlines.len())].clone()
    } else {
        let headlines = [
            format!(
                "{} {} - {} {}: Honours Even",
                home_name, home_goals, away_goals, away_name
            ),
            format!("{} and {} Share the Spoils", home_name, away_name),
            format!("Stalemate at {}'s Ground", home_name),
        ];
        headlines[rng.random_range(0..headlines.len())].clone()
    };

    // Determine outcome for i18n key
    let outcome = outcome_key(home_goals, away_goals);
    let headline_variant = rng.random_range(0..3u8);
    let body_key = if scorer_parts.is_empty() {
        format!("be.news.matchReport.body{}.noScorers", idx)
    } else {
        format!("be.news.matchReport.body{}", idx)
    };

    NewsArticle::new(
        format!("report_{}", fixture_id),
        headline,
        commentary[idx].clone(),
        source.to_string(),
        date.to_string(),
        NewsCategory::MatchReport,
    )
    .with_teams(vec![home_team_id.to_string(), away_team_id.to_string()])
    .with_players(player_ids)
    .with_score(NewsMatchScore {
        home_team_id: home_team_id.to_string(),
        away_team_id: away_team_id.to_string(),
        home_goals,
        away_goals,
    })
    .with_i18n(
        &format!(
            "be.news.matchReport.headline.{}.{}",
            outcome, headline_variant
        ),
        &body_key,
        source_key,
        {
            let mut p = params(&[
                ("home", home_name),
                ("away", away_name),
                ("homeGoals", &home_goals.to_string()),
                ("awayGoals", &away_goals.to_string()),
                ("matchday", &matchday.to_string()),
                ("scorers", &scorer_parts.join(", ")),
            ]);
            // For winner-specific headlines
            if home_goals > away_goals {
                p.insert("winner".to_string(), home_name.to_string());
                p.insert("loser".to_string(), away_name.to_string());
            } else if away_goals > home_goals {
                p.insert("winner".to_string(), away_name.to_string());
                p.insert("loser".to_string(), home_name.to_string());
            }
            p
        },
    )
}

#[cfg(test)]
mod tests {
    use super::match_report_article;
    use domain::league::FixtureCompetition;
    use domain::news::NewsCategory;

    fn assert_valid_source_pair(source: &str, source_key: &str) {
        let valid = [
            ("Sports Gazette", "be.source.sportsGazette"),
            ("The Football Herald", "be.source.footballHerald"),
            ("Match Day Press", "be.source.matchDayPress"),
            ("League Chronicle", "be.source.leagueChronicle"),
        ];

        assert!(
            valid
                .iter()
                .any(|pair| pair.0 == source && pair.1 == source_key)
        );
    }

    #[test]
    fn home_win_article_includes_match_metadata_and_scorers() {
        let article = match_report_article(
            "fix1",
            "Alpha FC",
            "Beta FC",
            2,
            1,
            "team1",
            "team2",
            FixtureCompetition::League,
            5,
            &[("Alice".to_string(), 10)],
            &[("Bob".to_string(), 75)],
            "2025-06-15",
        );

        assert_eq!(article.id, "report_fix1");
        assert_eq!(article.category, NewsCategory::MatchReport);
        assert_eq!(
            article.team_ids,
            vec!["team1".to_string(), "team2".to_string()]
        );
        assert_eq!(
            article.player_ids,
            vec!["Alice".to_string(), "Bob".to_string()]
        );
        let score = article.match_score.as_ref().unwrap();
        assert_eq!(score.home_team_id, "team1");
        assert_eq!(score.away_team_id, "team2");
        assert_eq!(score.home_goals, 2);
        assert_eq!(score.away_goals, 1);

        assert!(article.body.contains("Alpha FC 2-1 Beta FC"));
        assert!(
            article
                .body
                .contains("Goals: Alice (10', Alpha FC), Bob (75', Beta FC)")
        );
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
        assert_valid_source_pair(&article.source, article.source_key.as_deref().unwrap());
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
        assert_eq!(
            article.i18n_params.get("scorers"),
            Some(&"Alice (10', Alpha FC), Bob (75', Beta FC)".to_string())
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
        let article = match_report_article(
            "fix2",
            "Alpha FC",
            "Beta FC",
            1,
            3,
            "team1",
            "team2",
            FixtureCompetition::League,
            6,
            &[("Alice".to_string(), 12)],
            &[("Bob".to_string(), 40), ("Ben".to_string(), 88)],
            "2025-06-22",
        );

        assert!(article.body.contains("Alpha FC 1-3 Beta FC"));
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
            vec!["Alice".to_string(), "Bob".to_string(), "Ben".to_string()]
        );
    }

    #[test]
    fn draw_article_omits_winner_params_and_goal_section_when_scoreless() {
        let article = match_report_article(
            "fix3",
            "Alpha FC",
            "Beta FC",
            0,
            0,
            "team1",
            "team2",
            FixtureCompetition::League,
            7,
            &[],
            &[],
            "2025-06-29",
        );

        assert!(article.body.contains("Alpha FC 0-0 Beta FC"));
        assert!(!article.body.contains("Goals:"));
        assert!(
            article
                .headline_key
                .as_deref()
                .unwrap()
                .starts_with("be.news.matchReport.headline.draw.")
        );
        assert_eq!(article.i18n_params.get("winner"), None);
        assert_eq!(article.i18n_params.get("loser"), None);
        assert_eq!(article.i18n_params.get("scorers"), Some(&String::new()));
        assert!(article.player_ids.is_empty());
    }

    #[test]
    fn friendly_article_uses_non_league_preseason_copy() {
        let article = match_report_article(
            "fix4",
            "Alpha FC",
            "Beta FC",
            2,
            2,
            "team1",
            "team2",
            FixtureCompetition::Friendly,
            0,
            &[],
            &[],
            "2025-07-20",
        );

        assert_eq!(article.category, NewsCategory::MatchReport);
        assert!(article.body.to_lowercase().contains("friendly action"));
        assert!(article.headline.to_lowercase().contains("friendly report"));
        assert_eq!(
            article.headline_key.as_deref(),
            Some("be.news.matchReport.reportFriendly.title")
        );
        assert_eq!(
            article.body_key.as_deref(),
            Some("be.news.matchReport.reportFriendly.body")
        );
        assert!(article.source_key.is_some());
        assert_eq!(
            article.i18n_params.get("scorersSection"),
            Some(&String::new())
        );
        assert_eq!(
            article.i18n_params.get("scorersData"),
            Some(&"[]".to_string())
        );
    }
}
