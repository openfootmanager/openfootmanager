mod match_report;
pub use match_report::match_report_article;

use crate::season_awards::SeasonAwards;
use domain::news::*;
use rand::{Rng, RngExt};
use serde::Serialize;
use std::collections::HashMap;

/// Helper to build a HashMap<String, String> from key-value pairs.
fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn result_lines(results: &[(String, u8, String, u8)]) -> Vec<String> {
    results
        .iter()
        .map(|(home, hg, away, ag)| format!("  {} {} - {} {}", home, hg, ag, away))
        .collect()
}

#[derive(Serialize)]
struct RoundupResultParam<'a> {
    home: &'a str,
    #[serde(rename = "homeGoals")]
    home_goals: u8,
    away: &'a str,
    #[serde(rename = "awayGoals")]
    away_goals: u8,
}

fn roundup_results_data(results: &[(String, u8, String, u8)]) -> String {
    let entries: Vec<RoundupResultParam<'_>> = results
        .iter()
        .map(|(home, home_goals, away, away_goals)| RoundupResultParam {
            home,
            home_goals: *home_goals,
            away,
            away_goals: *away_goals,
        })
        .collect();

    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
}

fn biggest_winner_name(results: &[(String, u8, String, u8)]) -> String {
    results
        .iter()
        .filter(|(_, hg, _, ag)| hg != ag)
        .max_by_key(|(_, hg, _, ag)| (*hg as i8 - *ag as i8).unsigned_abs())
        .map(
            |(home, hg, away, ag)| {
                if hg > ag { home.clone() } else { away.clone() }
            },
        )
        .unwrap_or_default()
}

fn goal_difference_text(goal_difference: i16) -> String {
    if goal_difference >= 0 {
        format!("+{}", goal_difference)
    } else {
        goal_difference.to_string()
    }
}

fn standings_lines(top_teams: &[(String, u32, i16)]) -> Vec<String> {
    top_teams
        .iter()
        .enumerate()
        .map(|(idx, (name, points, goal_difference))| {
            format!(
                "  {}. {} — {} pts (GD: {})",
                idx + 1,
                name,
                points,
                goal_difference_text(*goal_difference)
            )
        })
        .collect()
}

#[derive(Serialize)]
struct StandingsLineParam<'a> {
    rank: usize,
    team: &'a str,
    points: u32,
    #[serde(rename = "goalDifference")]
    goal_difference: String,
}

fn standings_data(top_teams: &[(String, u32, i16)]) -> String {
    let entries: Vec<StandingsLineParam<'_>> = top_teams
        .iter()
        .enumerate()
        .map(
            |(idx, (name, points, goal_difference))| StandingsLineParam {
                rank: idx + 1,
                team: name,
                points: *points,
                goal_difference: goal_difference_text(*goal_difference),
            },
        )
        .collect();

    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
}

fn preseason_unbeaten_line(unbeaten_teams: &[String]) -> String {
    match unbeaten_teams {
        [] => String::new(),
        [team] => format!("\n\n{} remain unbeaten in preseason.", team),
        [first, second, ..] => {
            format!("\n\n{} and {} remain unbeaten in preseason.", first, second)
        }
    }
}

fn preseason_unbeaten_data(unbeaten_teams: &[String]) -> String {
    serde_json::to_string(unbeaten_teams).unwrap_or_else(|_| "[]".to_string())
}

/// Generate a league roundup article summarising all matchday results.
pub fn league_roundup_article(
    matchday: u32,
    results: &[(String, u8, String, u8)], // (home_name, home_goals, away_name, away_goals)
    date: &str,
) -> NewsArticle {
    let mut rng = rand::rng();
    let results_text = result_lines(results);
    let results_data = roundup_results_data(results);
    let biggest_winner = biggest_winner_name(results);

    let mut body = format!(
        "Matchday {} is in the books. Here are the full results:\n",
        matchday
    );
    for line in &results_text {
        body.push_str(&format!("\n{}", line));
    }

    let total_goals: u8 = results.iter().map(|(_, hg, _, ag)| hg + ag).sum();
    body.push_str(&format!(
        "\n\n{} goals scored across {} matches. ",
        total_goals,
        results.len()
    ));

    if !biggest_winner.is_empty() {
        body.push_str(&format!(
            "{} recorded the biggest win of the day.",
            biggest_winner
        ));
    }

    let headlines = [
        format!(
            "Matchday {} Round-Up: {} Goals in Action-Packed Day",
            matchday, total_goals
        ),
        format!("Premier Division Matchday {}: All the Results", matchday),
        format!("Goals Galore in Matchday {} Action", matchday),
    ];

    let source_keys = [
        "be.source.leagueWire",
        "be.source.footballHerald",
        "be.source.sportsGazette",
    ];
    let sources = ["League Wire", "The Football Herald", "Sports Gazette"];
    let src_idx = rng.random_range(0..sources.len());
    let headline_idx = rng.random_range(0..headlines.len());

    NewsArticle::new(
        format!("roundup_md{}", matchday),
        headlines[headline_idx].clone(),
        body,
        sources[src_idx].to_string(),
        date.to_string(),
        NewsCategory::LeagueRoundup,
    )
    .with_i18n(
        &format!("be.news.roundup.headline{}", headline_idx),
        "be.news.roundup.body",
        source_keys[src_idx],
        params(&[
            ("matchday", &matchday.to_string()),
            ("totalGoals", &total_goals.to_string()),
            ("matchCount", &results.len().to_string()),
            ("results", &results_text.join("\n")),
            ("resultsData", &results_data),
            ("biggestWinner", &biggest_winner),
        ]),
    )
}

/// Generate a standings update article after a matchday.
pub fn standings_update_article(
    matchday: u32,
    top_teams: &[(String, u32, i16)], // (team_name, points, goal_diff)
    date: &str,
) -> NewsArticle {
    let mut rng = rand::rng();

    let leader = top_teams
        .first()
        .map(|(n, _, _)| n.as_str())
        .unwrap_or("Unknown");
    let mut body = format!(
        "After Matchday {}, {} sit at the top of the Premier Division table.\n\nStandings:",
        matchday, leader
    );

    let standings_text = standings_lines(top_teams);
    let standings_data = standings_data(top_teams);

    for line in &standings_text {
        body.push_str(&format!("\n{}", line));
    }

    let headlines = [
        format!("{} Lead the Way After Matchday {}", leader, matchday),
        format!("Premier Division Table: {} on Top", leader),
        format!("Standings Update — Matchday {}", matchday),
    ];

    let source_keys = [
        "be.source.leagueWire",
        "be.source.footballHerald",
        "be.source.leagueChronicle",
    ];
    let sources = ["League Wire", "The Football Herald", "League Chronicle"];
    let src_idx = rng.random_range(0..sources.len());
    let headline_idx = rng.random_range(0..headlines.len());

    NewsArticle::new(
        format!("standings_md{}", matchday),
        headlines[headline_idx].clone(),
        body,
        sources[src_idx].to_string(),
        date.to_string(),
        NewsCategory::StandingsUpdate,
    )
    .with_i18n(
        &format!("be.news.standings.headline{}", headline_idx),
        "be.news.standings.body",
        source_keys[src_idx],
        params(&[
            ("matchday", &matchday.to_string()),
            ("leader", leader),
            ("standings", &standings_text.join("\n")),
            ("standingsData", &standings_data),
        ]),
    )
}

fn preview_contenders<'a>(team_names: &'a [String], rng: &mut impl Rng) -> (&'a str, &'a str) {
    let favourite = &team_names[rng.random_range(0..team_names.len())];

    if team_names.len() == 1 {
        return (favourite.as_str(), favourite.as_str());
    }

    let dark_horse = loop {
        let pick = &team_names[rng.random_range(0..team_names.len())];
        if pick != favourite {
            break pick;
        }
    };

    (favourite.as_str(), dark_horse.as_str())
}

/// Generate a season preview article at the start of the season.
pub fn season_preview_article(team_names: &[String], date: &str) -> NewsArticle {
    let mut rng = rand::rng();

    let (favourite, dark_horse) = preview_contenders(team_names, &mut rng);

    let body = format!(
        "The Premier Division is set to kick off with {} teams vying for the title.\n\n\
        Pre-season predictions have {} as the early favourites, but {} could be the dark horse \
        to watch this campaign.\n\n\
        With new managers taking the reins at some clubs, this season promises to be one of the \
        most competitive in recent memory. Every point will matter as the race for the title \
        heats up.\n\n\
        Teams: {}",
        team_names.len(),
        favourite,
        dark_horse,
        team_names.join(", ")
    );

    let headlines = [
        format!(
            "Season Preview: {} Teams Battle for Glory",
            team_names.len()
        ),
        "Premier Division Season Set to Begin".to_string(),
        format!("Can {} Claim the Title? Season Preview", favourite),
    ];

    let headline_idx = rng.random_range(0..headlines.len());

    NewsArticle::new(
        "season_preview".to_string(),
        headlines[headline_idx].clone(),
        body,
        "The Football Herald".to_string(),
        date.to_string(),
        NewsCategory::SeasonPreview,
    )
    .with_i18n(
        &format!("be.news.seasonPreview.headline{}", headline_idx),
        "be.news.seasonPreview.body",
        "be.source.footballHerald",
        params(&[
            ("teamCount", &team_names.len().to_string()),
            ("favourite", favourite),
            ("darkHorse", dark_horse),
            ("teamList", &team_names.join(", ")),
        ]),
    )
}

pub fn managerial_appointment_article(
    manager_id: &str,
    manager_name: &str,
    team_id: &str,
    team_name: &str,
    date: &str,
) -> NewsArticle {
    NewsArticle::new(
        format!("managerial_appointment_{}_{}", team_id, date),
        format!("{} appoint {}", team_name, manager_name),
        format!(
            "{} have appointed {} as their new manager after moving quickly to fill the vacancy at the club.",
            team_name, manager_name
        ),
        "League Wire".to_string(),
        date.to_string(),
        NewsCategory::ManagerialChange,
    )
    .with_teams(vec![team_id.to_string()])
    .with_i18n(
        "be.news.managerialAppointment.headline",
        "be.news.managerialAppointment.body",
        "be.source.leagueWire",
        params(&[("team", team_name), ("manager", manager_name), ("managerId", manager_id)]),
    )
}

fn format_transfer_fee(fee: u64) -> String {
    if fee >= 1_000_000 {
        format!("€{:.1}M", fee as f64 / 1_000_000.0)
    } else if fee >= 1_000 {
        format!("€{}K", fee / 1_000)
    } else {
        format!("€{}", fee)
    }
}

pub fn transfer_roundup_article(
    id: &str,
    week_start: &str,
    transfers: &[(String, String, String, String, String, String, u64)],
    date: &str,
) -> NewsArticle {
    let deals_data = serde_json::to_string(
        &transfers
            .iter()
            .map(|(player, from_team, to_team, _, _, _, fee)| {
                serde_json::json!({
                    "player": player,
                    "fromTeam": from_team,
                    "toTeam": to_team,
                    "fee": format_transfer_fee(*fee),
                })
            })
            .collect::<Vec<_>>(),
    )
    .unwrap_or_default();
    let deals = transfers
        .iter()
        .map(|(player, from_team, to_team, _, _, _, fee)| {
            format!(
                "  {}: {} -> {} ({})",
                player,
                from_team,
                to_team,
                format_transfer_fee(*fee)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let headline = format!("Transfer Roundup — Week of {}", week_start);
    let body = format!(
        "The transfer market stayed busy this week. {} completed deal(s) stood out across the division.\n\n{}",
        transfers.len(),
        deals
    );

    let mut team_ids = Vec::new();
    let mut player_ids = Vec::new();
    for (_, _, _, player_id, from_team_id, to_team_id, _) in transfers {
        if !player_ids.contains(player_id) {
            player_ids.push(player_id.clone());
        }
        if !team_ids.contains(from_team_id) {
            team_ids.push(from_team_id.clone());
        }
        if !team_ids.contains(to_team_id) {
            team_ids.push(to_team_id.clone());
        }
    }

    NewsArticle::new(
        id.to_string(),
        headline,
        body,
        "Transfer Intelligence".to_string(),
        date.to_string(),
        NewsCategory::TransferRoundup,
    )
    .with_teams(team_ids)
    .with_players(player_ids)
    .with_i18n(
        "be.news.transferRoundup.headline",
        "be.news.transferRoundup.body",
        "be.source.transferIntelligence",
        params(&[
            ("weekStart", week_start),
            ("transferCount", &transfers.len().to_string()),
            ("deals", &deals),
            ("dealsData", &deals_data),
        ]),
    )
}

/// Generate the end-of-season awards ceremony news article.
///
/// Returns `None` when neither marquee award (Golden Boot, Player of the Year) has a winner —
/// nothing to celebrate, so no article.
pub fn season_awards_article(
    awards: &SeasonAwards,
    season: u32,
    date: &str,
) -> Option<NewsArticle> {
    let golden_boot = awards.golden_boot.first();
    let poty = awards.player_of_year.first();
    if golden_boot.is_none() && poty.is_none() {
        return None;
    }

    let mut i18n_params = HashMap::new();
    i18n_params.insert("season".to_string(), season.to_string());
    if let Some(gb) = golden_boot {
        i18n_params.insert("goldenBootWinner".to_string(), gb.player_name.clone());
        i18n_params.insert("goldenBootTeam".to_string(), gb.team_name.clone());
        i18n_params.insert("goldenBootGoals".to_string(), (gb.value as u32).to_string());
    }
    if let Some(p) = poty {
        i18n_params.insert("potyWinner".to_string(), p.player_name.clone());
        i18n_params.insert("potyTeam".to_string(), p.team_name.clone());
        i18n_params.insert("potyRating".to_string(), format!("{:.1}", p.value));
    }

    let (body, body_key) = match (golden_boot, poty) {
        (Some(gb), Some(p)) => (
            format!(
                "Season {} concluded with {} ({}) lifting the Golden Boot with {} goals. \
                 Player of the Year went to {} ({}) with an average rating of {:.1}.",
                season,
                gb.player_name,
                gb.team_name,
                gb.value as u32,
                p.player_name,
                p.team_name,
                p.value,
            ),
            "be.news.seasonAwards.bodyBoth",
        ),
        (Some(gb), None) => (
            format!(
                "Season {} closed with {} ({}) crowned top scorer with {} goals.",
                season, gb.player_name, gb.team_name, gb.value as u32,
            ),
            "be.news.seasonAwards.bodyGoldenBootOnly",
        ),
        (None, Some(p)) => (
            format!(
                "Season {} ended with {} ({}) named Player of the Year, posting an average rating of {:.1}.",
                season, p.player_name, p.team_name, p.value,
            ),
            "be.news.seasonAwards.bodyPotyOnly",
        ),
        (None, None) => unreachable!("guarded above"),
    };

    let mut player_ids = Vec::new();
    let mut team_ids = Vec::new();
    for entry in [golden_boot, poty].into_iter().flatten() {
        if !entry.player_id.is_empty() && !player_ids.contains(&entry.player_id) {
            player_ids.push(entry.player_id.clone());
        }
        if !entry.team_id.is_empty() && !team_ids.contains(&entry.team_id) {
            team_ids.push(entry.team_id.clone());
        }
    }

    Some(
        NewsArticle::new(
            format!("season_awards_{}", season),
            format!("Season {} Awards", season),
            body,
            "The Football Herald".to_string(),
            date.to_string(),
            NewsCategory::Editorial,
        )
        .with_teams(team_ids)
        .with_players(player_ids)
        .with_i18n(
            "be.news.seasonAwards.headline",
            body_key,
            "be.source.footballHerald",
            i18n_params,
        ),
    )
}

pub fn major_transfer_article(
    id: &str,
    player_id: &str,
    player_name: &str,
    from_team_id: &str,
    from_team_name: &str,
    to_team_id: &str,
    to_team_name: &str,
    fee: u64,
    date: &str,
) -> NewsArticle {
    let fee_display = if fee >= 1_000_000 {
        format!("€{:.1}M", fee as f64 / 1_000_000.0)
    } else if fee >= 1_000 {
        format!("€{}K", fee / 1_000)
    } else {
        format!("€{}", fee)
    };

    NewsArticle::new(
        id.to_string(),
        format!("{} Completes Move to {}", player_name, to_team_name),
        format!(
            "{} have completed the signing of {} from {} for {}.",
            to_team_name, player_name, from_team_name, fee_display
        ),
        "League Chronicle".to_string(),
        date.to_string(),
        NewsCategory::TransferRumour,
    )
    .with_teams(vec![from_team_id.to_string(), to_team_id.to_string()])
    .with_players(vec![player_id.to_string()])
    .with_i18n(
        "be.news.majorTransfer.headline",
        "be.news.majorTransfer.body",
        "be.source.leagueChronicle",
        params(&[
            ("player", player_name),
            ("fromTeam", from_team_name),
            ("toTeam", to_team_name),
            ("fee", &fee_display),
        ]),
    )
}

pub fn weekly_digest_article(
    id: &str,
    week_start: &str,
    leader: &str,
    top_scorer: &str,
    top_scorer_goals: u32,
    storyline_count: usize,
    date: &str,
) -> NewsArticle {
    let headline = format!("Weekly Digest — Week of {}", week_start);
    let (body, body_key) = if top_scorer.is_empty() {
        (
            format!(
                "The latest weekly digest is here. {} lead the table, and {} storyline(s) are shaping the division this week.",
                leader, storyline_count
            ),
            "be.news.weeklyDigest.bodyNoTopScorer",
        )
    } else {
        (
            format!(
                "The latest weekly digest is here. {} lead the table, while {} heads the scoring charts with {} goal(s). {} storyline(s) are shaping the division this week.",
                leader, top_scorer, top_scorer_goals, storyline_count
            ),
            "be.news.weeklyDigest.bodyWithTopScorer",
        )
    };

    NewsArticle::new(
        id.to_string(),
        headline,
        body,
        "League Chronicle".to_string(),
        date.to_string(),
        NewsCategory::Editorial,
    )
    .with_i18n(
        "be.news.weeklyDigest.headline",
        body_key,
        "be.source.leagueChronicle",
        params(&[
            ("weekStart", week_start),
            ("leader", leader),
            ("topScorer", top_scorer),
            ("topScorerGoals", &top_scorer_goals.to_string()),
            ("storylineCount", &storyline_count.to_string()),
        ]),
    )
}

pub fn preseason_digest_article(
    id: &str,
    week_start: &str,
    results: &[(String, u8, String, u8)],
    unbeaten_teams: &[String],
    date: &str,
) -> NewsArticle {
    let results_text = result_lines(results);
    let results_data = roundup_results_data(results);
    let total_goals: u32 = results
        .iter()
        .map(|(_, home_goals, _, away_goals)| u32::from(*home_goals) + u32::from(*away_goals))
        .sum();
    let unbeaten_line = preseason_unbeaten_line(unbeaten_teams);
    let unbeaten_teams_data = preseason_unbeaten_data(unbeaten_teams);

    let (mut body, body_key) = if results.is_empty() {
        (
            "The latest preseason digest is here. Training camps, selection decisions, and transfer business continue across the division as clubs prepare for opening day.".to_string(),
            "be.news.preseasonDigest.bodyNoResults",
        )
    } else {
        (
            format!(
                "The latest preseason digest is here. {} friendly result(s) were played across the division this week, producing {} goal(s).\n\nResults:\n{}",
                results.len(),
                total_goals,
                results_text.join("\n")
            ),
            "be.news.preseasonDigest.bodyWithResults",
        )
    };

    if !unbeaten_line.is_empty() {
        body.push_str(&unbeaten_line);
    }

    NewsArticle::new(
        id.to_string(),
        format!("Preseason Digest — Week of {}", week_start),
        body,
        "League Chronicle".to_string(),
        date.to_string(),
        NewsCategory::Editorial,
    )
    .with_i18n(
        "be.news.preseasonDigest.headline",
        body_key,
        "be.source.leagueChronicle",
        params(&[
            ("weekStart", week_start),
            ("resultCount", &results.len().to_string()),
            ("totalGoals", &total_goals.to_string()),
            ("results", &results_text.join("\n")),
            ("resultsData", &results_data),
            ("unbeatenLine", &unbeaten_line),
            ("unbeatenTeamsData", &unbeaten_teams_data),
        ]),
    )
}

pub fn title_race_storyline_article(
    id: &str,
    leader_team_id: &str,
    leader: &str,
    challenger_team_id: &str,
    challenger: &str,
    gap: u32,
    date: &str,
) -> NewsArticle {
    NewsArticle::new(
        id.to_string(),
        format!(
            "Title Race Tightens — {} Lead {} by {} Point(s)",
            leader, challenger, gap
        ),
        format!(
            "{} remain in front, but {} are only {} point(s) behind as the title race takes shape.",
            leader, challenger, gap
        ),
        "League Chronicle".to_string(),
        date.to_string(),
        NewsCategory::Editorial,
    )
    .with_teams(vec![
        leader_team_id.to_string(),
        challenger_team_id.to_string(),
    ])
    .with_i18n(
        "be.news.storyline.titleRace.headline",
        "be.news.storyline.titleRace.body",
        "be.source.leagueChronicle",
        params(&[
            ("leader", leader),
            ("challenger", challenger),
            ("gap", &gap.to_string()),
        ]),
    )
}

pub fn unbeaten_streak_storyline_article(
    id: &str,
    team_id: &str,
    team: &str,
    run_length: u32,
    date: &str,
) -> NewsArticle {
    NewsArticle::new(
        id.to_string(),
        format!("{} Extend Unbeaten Run to {}", team, run_length),
        format!(
            "{} have gone {} match(es) without defeat and are building real momentum.",
            team, run_length
        ),
        "League Chronicle".to_string(),
        date.to_string(),
        NewsCategory::Editorial,
    )
    .with_teams(vec![team_id.to_string()])
    .with_i18n(
        "be.news.storyline.unbeatenStreak.headline",
        "be.news.storyline.unbeatenStreak.body",
        "be.source.leagueChronicle",
        params(&[("team", team), ("runLength", &run_length.to_string())]),
    )
}

/// Generate a speculative transfer rumour article linking a player to other clubs.
///
/// Unlike `major_transfer_article` (which reports a completed move), this function
/// produces gossip-style speculation. The article is attributed to a tabloid-leaning
/// source and uses hedged language ("according to sources", "understood to be", etc.).
pub fn transfer_rumour_gossip_article(
    id: &str,
    player_id: &str,
    player_name: &str,
    from_team_id: &str,
    from_team_name: &str,
    date: &str,
) -> NewsArticle {
    let mut rng = rand::rng();

    let headlines = [
        format!("{} Attracting Interest from Several Clubs", player_name),
        format!("Clubs Circle as {}'s Future Remains Uncertain", player_name),
        format!(
            "{} Linked with Move Away from {}",
            player_name, from_team_name
        ),
    ];
    let headline_idx = rng.random_range(0..headlines.len());
    let headline = headlines[headline_idx].clone();

    let bodies = [
        format!(
            "Sources close to the situation suggest that {} could be on the move, with multiple \
            clubs reportedly monitoring the {} player. {} have not commented publicly, but \
            the speculation is unlikely to die down soon.",
            player_name, from_team_name, from_team_name
        ),
        format!(
            "{} is understood to have attracted attention from clubs across the division. \
            The {} star has been one of the standout performers this season, and it is \
            believed that offers could materialise before the window closes.",
            player_name, from_team_name
        ),
        format!(
            "Transfer whispers are growing louder around {}, with the {} player \
            said to be weighing their options. Agent talks are rumoured to have taken place, \
            though nothing has been confirmed.",
            player_name, from_team_name
        ),
    ];
    let body_idx = rng.random_range(0..bodies.len());
    let body = bodies[body_idx].clone();

    let source_keys = [
        "be.source.transferIntelligence",
        "be.source.sportsGazette",
        "be.source.footballHerald",
    ];
    let sources = [
        "Transfer Intelligence",
        "Sports Gazette",
        "The Football Herald",
    ];
    let src_idx = rng.random_range(0..sources.len());

    NewsArticle::new(
        id.to_string(),
        headline,
        body,
        sources[src_idx].to_string(),
        date.to_string(),
        NewsCategory::TransferRumour,
    )
    .with_teams(vec![from_team_id.to_string()])
    .with_players(vec![player_id.to_string()])
    .with_i18n(
        &format!("be.news.transferRumour.headline{}", headline_idx),
        &format!("be.news.transferRumour.body{}", body_idx),
        source_keys[src_idx],
        params(&[("player", player_name), ("team", from_team_name)]),
    )
}

/// Generate a news article reporting that a notable player has been injured.
pub fn injury_news_article(
    id: &str,
    player_id: &str,
    player_name: &str,
    team_id: &str,
    team_name: &str,
    days_out: u32,
    date: &str,
) -> NewsArticle {
    let mut rng = rand::rng();

    let is_short = days_out <= 7;
    let weeks = (days_out + 6) / 7;
    // Body keys: locale-specific phrasing for days (short) vs weeks (long).
    // headline2 (contains duration) is only picked for long injuries so the locale
    // template can use {{weeksOut}} without needing a conditional.
    let duration_suffix = if is_short { "Days" } else { "Weeks" };
    let duration_display = if is_short {
        format!("{} day(s)", days_out)
    } else {
        format!("~{} week(s)", weeks)
    };

    let headline_count = if is_short { 2 } else { 3 };
    let headline_idx = rng.random_range(0..headline_count);

    let headlines = [
        format!("{} Injury Blow for {}", player_name, team_name),
        format!("{} Set for Spell on the Sidelines", player_name),
        format!(
            "{} — {} Sweating on Key Player",
            duration_display, team_name
        ),
    ];

    let body_idx = rng.random_range(0..2_usize);
    let bodies = [
        format!(
            "{} have confirmed that {} has picked up an injury and is expected to be \
            sidelined for {}. The club will assess the situation before providing \
            a further update.",
            team_name, player_name, duration_display
        ),
        format!(
            "{} suffered a setback in training, with the {} player ruled out for \
            {}. The absence will be a significant blow as the season reaches a \
            critical stage.",
            player_name, team_name, duration_display
        ),
    ];

    let source_keys = [
        "be.source.leagueWire",
        "be.source.footballHerald",
        "be.source.matchDayPress",
    ];
    let sources = ["League Wire", "The Football Herald", "Match Day Press"];
    let src_idx = rng.random_range(0..sources.len());

    NewsArticle::new(
        id.to_string(),
        headlines[headline_idx].clone(),
        bodies[body_idx].clone(),
        sources[src_idx].to_string(),
        date.to_string(),
        NewsCategory::InjuryNews,
    )
    .with_teams(vec![team_id.to_string()])
    .with_players(vec![player_id.to_string()])
    .with_i18n(
        &format!("be.news.injuryNews.headline{}", headline_idx),
        &format!("be.news.injuryNews.body{}{}", body_idx, duration_suffix),
        source_keys[src_idx],
        params(&[
            ("player", player_name),
            ("team", team_name),
            ("daysOut", &days_out.to_string()),
            ("weeksOut", &weeks.to_string()),
        ]),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        league_roundup_article, season_awards_article, season_preview_article,
        standings_update_article,
    };
    use crate::season_awards::{AwardEntry, SeasonAwards};
    use domain::news::NewsCategory;

    fn empty_awards() -> SeasonAwards {
        SeasonAwards {
            golden_boot: vec![],
            assist_king: vec![],
            player_of_year: vec![],
            clean_sheet_king: vec![],
            most_appearances: vec![],
            young_player: vec![],
        }
    }

    fn award_entry(
        player_id: &str,
        player_name: &str,
        team_id: &str,
        team_name: &str,
        value: f64,
    ) -> AwardEntry {
        AwardEntry {
            player_id: player_id.to_string(),
            player_name: player_name.to_string(),
            team_id: team_id.to_string(),
            team_name: team_name.to_string(),
            value,
        }
    }

    fn assert_valid_roundup_source_pair(source: &str, source_key: &str) {
        let valid = [
            ("League Wire", "be.source.leagueWire"),
            ("The Football Herald", "be.source.footballHerald"),
            ("Sports Gazette", "be.source.sportsGazette"),
        ];

        assert!(
            valid
                .iter()
                .any(|pair| pair.0 == source && pair.1 == source_key)
        );
    }

    fn assert_valid_standings_source_pair(source: &str, source_key: &str) {
        let valid = [
            ("League Wire", "be.source.leagueWire"),
            ("The Football Herald", "be.source.footballHerald"),
            ("League Chronicle", "be.source.leagueChronicle"),
        ];

        assert!(
            valid
                .iter()
                .any(|pair| pair.0 == source && pair.1 == source_key)
        );
    }

    #[test]
    fn league_roundup_article_includes_results_totals_and_biggest_winner() {
        let results = vec![
            ("Alpha FC".to_string(), 3, "Beta FC".to_string(), 0),
            ("Gamma FC".to_string(), 1, "Delta FC".to_string(), 1),
        ];

        let article = league_roundup_article(4, &results, "2025-08-12");

        assert_eq!(article.id, "roundup_md4");
        assert_eq!(article.category, NewsCategory::LeagueRoundup);
        assert!(article.body.contains("Matchday 4 is in the books."));
        assert!(article.body.contains("Alpha FC 3 - 0 Beta FC"));
        assert!(article.body.contains("Gamma FC 1 - 1 Delta FC"));
        assert!(article.body.contains("5 goals scored across 2 matches."));
        assert!(
            article
                .body
                .contains("Alpha FC recorded the biggest win of the day.")
        );
        assert!(
            [
                "be.news.roundup.headline0",
                "be.news.roundup.headline1",
                "be.news.roundup.headline2"
            ]
            .contains(&article.headline_key.as_deref().unwrap())
        );
        assert_eq!(article.body_key.as_deref(), Some("be.news.roundup.body"));
        assert_valid_roundup_source_pair(&article.source, article.source_key.as_deref().unwrap());
        assert_eq!(article.i18n_params.get("matchday"), Some(&"4".to_string()));
        assert_eq!(
            article.i18n_params.get("totalGoals"),
            Some(&"5".to_string())
        );
        assert_eq!(
            article.i18n_params.get("matchCount"),
            Some(&"2".to_string())
        );
        assert_eq!(
            article.i18n_params.get("results"),
            Some(&"  Alpha FC 3 - 0 Beta FC\n  Gamma FC 1 - 1 Delta FC".to_string())
        );
        let results_data = article.i18n_params.get("resultsData").unwrap();
        assert!(results_data.contains("\"home\":\"Alpha FC\""));
        assert!(results_data.contains("\"homeGoals\":3"));
        assert!(results_data.contains("\"away\":\"Beta FC\""));
        assert!(results_data.contains("\"awayGoals\":0"));
        assert_eq!(
            article.i18n_params.get("biggestWinner"),
            Some(&"Alpha FC".to_string())
        );
    }

    #[test]
    fn league_roundup_article_leaves_biggest_winner_empty_when_all_matches_are_draws() {
        let results = vec![
            ("Alpha FC".to_string(), 1, "Beta FC".to_string(), 1),
            ("Gamma FC".to_string(), 0, "Delta FC".to_string(), 0),
        ];

        let article = league_roundup_article(5, &results, "2025-08-19");

        assert!(!article.body.contains("recorded the biggest win of the day"));
        assert_eq!(
            article.i18n_params.get("biggestWinner"),
            Some(&String::new())
        );
    }

    #[test]
    fn standings_update_article_formats_leader_and_goal_differences() {
        let standings = vec![
            ("Alpha FC".to_string(), 12, 5),
            ("Beta FC".to_string(), 10, 0),
            ("Gamma FC".to_string(), 9, -3),
        ];

        let article = standings_update_article(4, &standings, "2025-08-12");

        assert_eq!(article.id, "standings_md4");
        assert_eq!(article.category, NewsCategory::StandingsUpdate);
        assert!(
            article
                .body
                .contains("After Matchday 4, Alpha FC sit at the top")
        );
        assert!(article.body.contains("1. Alpha FC — 12 pts (GD: +5)"));
        assert!(article.body.contains("2. Beta FC — 10 pts (GD: +0)"));
        assert!(article.body.contains("3. Gamma FC — 9 pts (GD: -3)"));
        assert!(
            [
                "be.news.standings.headline0",
                "be.news.standings.headline1",
                "be.news.standings.headline2"
            ]
            .contains(&article.headline_key.as_deref().unwrap())
        );
        assert_eq!(article.body_key.as_deref(), Some("be.news.standings.body"));
        assert_valid_standings_source_pair(&article.source, article.source_key.as_deref().unwrap());
        assert_eq!(article.i18n_params.get("matchday"), Some(&"4".to_string()));
        assert_eq!(
            article.i18n_params.get("leader"),
            Some(&"Alpha FC".to_string())
        );
        assert_eq!(
            article.i18n_params.get("standings"),
            Some(&"  1. Alpha FC — 12 pts (GD: +5)\n  2. Beta FC — 10 pts (GD: +0)\n  3. Gamma FC — 9 pts (GD: -3)".to_string())
        );
        assert_eq!(
            article.i18n_params.get("standingsData"),
            Some(&"[{\"rank\":1,\"team\":\"Alpha FC\",\"points\":12,\"goalDifference\":\"+5\"},{\"rank\":2,\"team\":\"Beta FC\",\"points\":10,\"goalDifference\":\"+0\"},{\"rank\":3,\"team\":\"Gamma FC\",\"points\":9,\"goalDifference\":\"-3\"}]".to_string())
        );
    }

    #[test]
    fn standings_update_article_handles_empty_table_with_unknown_leader() {
        let article = standings_update_article(1, &[], "2025-08-01");

        assert!(
            article
                .body
                .contains("After Matchday 1, Unknown sit at the top")
        );
        assert_eq!(
            article.i18n_params.get("leader"),
            Some(&"Unknown".to_string())
        );
        assert_eq!(article.i18n_params.get("standings"), Some(&String::new()));
        assert_eq!(
            article.i18n_params.get("standingsData"),
            Some(&"[]".to_string())
        );
    }

    #[test]
    fn season_preview_article_includes_team_list_and_distinct_contenders() {
        let teams = vec![
            "Alpha FC".to_string(),
            "Beta FC".to_string(),
            "Gamma FC".to_string(),
        ];

        let article = season_preview_article(&teams, "2025-08-01");

        assert_eq!(article.id, "season_preview");
        assert_eq!(article.category, NewsCategory::SeasonPreview);
        assert_eq!(article.source, "The Football Herald");
        assert_eq!(
            article.source_key.as_deref(),
            Some("be.source.footballHerald")
        );
        assert!(
            [
                "be.news.seasonPreview.headline0",
                "be.news.seasonPreview.headline1",
                "be.news.seasonPreview.headline2"
            ]
            .contains(&article.headline_key.as_deref().unwrap())
        );
        assert_eq!(
            article.body_key.as_deref(),
            Some("be.news.seasonPreview.body")
        );
        assert!(article.body.contains("3 teams vying for the title"));
        assert!(article.body.contains("Teams: Alpha FC, Beta FC, Gamma FC"));
        assert_eq!(article.i18n_params.get("teamCount"), Some(&"3".to_string()));
        assert_eq!(
            article.i18n_params.get("teamList"),
            Some(&"Alpha FC, Beta FC, Gamma FC".to_string())
        );

        let favourite = article.i18n_params.get("favourite").unwrap();
        let dark_horse = article.i18n_params.get("darkHorse").unwrap();
        assert!(teams.contains(favourite));
        assert!(teams.contains(dark_horse));
        assert_ne!(favourite, dark_horse);
    }

    #[test]
    fn season_preview_article_handles_single_team_without_looping() {
        let teams = vec!["Solo FC".to_string()];

        let article = season_preview_article(&teams, "2025-08-01");

        assert!(article.body.contains("1 teams vying for the title"));
        assert!(article.body.contains("Teams: Solo FC"));
        assert_eq!(article.i18n_params.get("teamCount"), Some(&"1".to_string()));
        assert_eq!(
            article.i18n_params.get("favourite"),
            Some(&"Solo FC".to_string())
        );
        assert_eq!(
            article.i18n_params.get("darkHorse"),
            Some(&"Solo FC".to_string())
        );
    }

    // ---------------------------------------------------------------------
    // season_awards_article — celebrates marquee winners on the final day
    // ---------------------------------------------------------------------

    #[test]
    fn season_awards_article_returns_none_when_no_marquee_winners() {
        let awards = empty_awards();
        assert!(season_awards_article(&awards, 1, "2026-05-20").is_none());
    }

    #[test]
    fn season_awards_article_returns_none_when_only_minor_awards_present() {
        // Without a Golden Boot or Player of the Year, there's nothing headline-worthy.
        let mut awards = empty_awards();
        awards.assist_king = vec![award_entry("p1", "Maker", "t1", "Test FC", 12.0)];
        awards.most_appearances = vec![award_entry("p1", "Maker", "t1", "Test FC", 36.0)];
        assert!(season_awards_article(&awards, 1, "2026-05-20").is_none());
    }

    #[test]
    fn season_awards_article_celebrates_golden_boot_winner() {
        let mut awards = empty_awards();
        awards.golden_boot = vec![award_entry("p1", "Star Striker", "team1", "Test FC", 22.0)];

        let article = season_awards_article(&awards, 3, "2026-05-20")
            .expect("expected an awards article when Golden Boot has a winner");

        assert_eq!(article.id, "season_awards_3");
        assert_eq!(article.category, NewsCategory::Editorial);
        assert_eq!(article.date, "2026-05-20");
        assert!(
            article.body.contains("Star Striker"),
            "body should reference the Golden Boot winner's name"
        );
        assert!(
            article.body.contains("Test FC"),
            "body should reference the Golden Boot winner's club"
        );
        assert!(
            article.body.contains("22"),
            "body should reference the goal tally"
        );
        assert!(article.player_ids.contains(&"p1".to_string()));
        assert!(article.team_ids.contains(&"team1".to_string()));
    }

    #[test]
    fn season_awards_article_celebrates_player_of_the_year_winner() {
        let mut awards = empty_awards();
        awards.player_of_year = vec![award_entry("p2", "Magnifique", "team2", "Rival FC", 8.4)];

        let article = season_awards_article(&awards, 2, "2026-05-20")
            .expect("expected an awards article when POTY has a winner");

        assert!(article.body.contains("Magnifique"));
        assert!(article.body.contains("Rival FC"));
        assert!(article.player_ids.contains(&"p2".to_string()));
        assert!(article.team_ids.contains(&"team2".to_string()));
    }

    #[test]
    fn season_awards_article_celebrates_both_winners_when_both_exist() {
        let mut awards = empty_awards();
        awards.golden_boot = vec![award_entry("p1", "Striker", "team1", "Test FC", 18.0)];
        awards.player_of_year = vec![award_entry("p2", "Maestro", "team2", "Rival FC", 7.9)];

        let article = season_awards_article(&awards, 5, "2026-05-20").unwrap();

        assert!(article.body.contains("Striker"));
        assert!(article.body.contains("Maestro"));
        assert!(article.player_ids.contains(&"p1".to_string()));
        assert!(article.player_ids.contains(&"p2".to_string()));
        assert!(article.team_ids.contains(&"team1".to_string()));
        assert!(article.team_ids.contains(&"team2".to_string()));
    }

    #[test]
    fn season_awards_article_dedupes_team_ids_when_winners_share_a_club() {
        let mut awards = empty_awards();
        awards.golden_boot = vec![award_entry("p1", "Striker", "team1", "Test FC", 18.0)];
        awards.player_of_year = vec![award_entry("p2", "Maestro", "team1", "Test FC", 7.9)];

        let article = season_awards_article(&awards, 1, "2026-05-20").unwrap();

        assert_eq!(article.team_ids.len(), 1);
        assert_eq!(article.team_ids[0], "team1");
    }

    #[test]
    fn season_awards_article_uses_i18n_keys_for_localization() {
        let mut awards = empty_awards();
        awards.golden_boot = vec![award_entry("p1", "Striker", "team1", "Test FC", 18.0)];
        awards.player_of_year = vec![award_entry("p2", "Maestro", "team2", "Rival FC", 7.9)];

        let article = season_awards_article(&awards, 4, "2026-05-20").unwrap();

        assert!(
            article.headline_key.is_some(),
            "headline_key must be set so the headline can be translated"
        );
        assert!(
            article.body_key.is_some(),
            "body_key must be set so the body can be translated"
        );
        assert!(
            article.source_key.is_some(),
            "source_key must be set so the byline can be translated"
        );
        assert_eq!(
            article.i18n_params.get("season"),
            Some(&"4".to_string()),
            "season number must be in i18n params for the localized template"
        );
    }

    #[test]
    fn season_awards_article_body_key_differs_per_award_combination() {
        let gb_winner = award_entry("p1", "Striker", "team1", "Test FC", 18.0);
        let poty_winner = award_entry("p2", "Maestro", "team2", "Rival FC", 7.9);

        let mut both = empty_awards();
        both.golden_boot = vec![gb_winner.clone()];
        both.player_of_year = vec![poty_winner.clone()];

        let mut gb_only = empty_awards();
        gb_only.golden_boot = vec![gb_winner];

        let mut poty_only = empty_awards();
        poty_only.player_of_year = vec![poty_winner];

        let key_both = season_awards_article(&both, 1, "d")
            .unwrap()
            .body_key
            .unwrap();
        let key_gb = season_awards_article(&gb_only, 1, "d")
            .unwrap()
            .body_key
            .unwrap();
        let key_poty = season_awards_article(&poty_only, 1, "d")
            .unwrap()
            .body_key
            .unwrap();

        assert_ne!(key_both, key_gb);
        assert_ne!(key_both, key_poty);
        assert_ne!(key_gb, key_poty);
    }

    #[test]
    fn transfer_rumour_gossip_article_sets_expected_fields() {
        use super::transfer_rumour_gossip_article;
        let article = transfer_rumour_gossip_article(
            "rumour_player1_2026-08-01",
            "player-1",
            "Adam Smith",
            "team-1",
            "Alpha FC",
            "2026-08-01",
        );

        assert_eq!(article.id, "rumour_player1_2026-08-01");
        assert_eq!(article.category, NewsCategory::TransferRumour);
        assert_eq!(article.team_ids, vec!["team-1".to_string()]);
        assert_eq!(article.player_ids, vec!["player-1".to_string()]);
        assert!(
            article
                .headline_key
                .as_deref()
                .unwrap()
                .starts_with("be.news.transferRumour.headline")
        );
        assert!(
            article
                .body_key
                .as_deref()
                .unwrap()
                .starts_with("be.news.transferRumour.body")
        );
        let valid_sources = [
            ("Transfer Intelligence", "be.source.transferIntelligence"),
            ("Sports Gazette", "be.source.sportsGazette"),
            ("The Football Herald", "be.source.footballHerald"),
        ];
        assert!(valid_sources.iter().any(
            |(src, key)| article.source == *src && article.source_key.as_deref() == Some(*key)
        ));
        assert_eq!(
            article.i18n_params.get("player").map(|s| s.as_str()),
            Some("Adam Smith")
        );
        assert_eq!(
            article.i18n_params.get("team").map(|s| s.as_str()),
            Some("Alpha FC")
        );
    }

    #[test]
    fn transfer_roundup_article_uses_dedicated_category_and_entities() {
        use super::transfer_roundup_article;
        let article = transfer_roundup_article(
            "weekly_transfer_roundup_2026_w31",
            "2026-08-03",
            &[
                (
                    "Adam Smith".to_string(),
                    "Alpha FC".to_string(),
                    "Beta FC".to_string(),
                    "player-1".to_string(),
                    "team-1".to_string(),
                    "team-2".to_string(),
                    1_800_000,
                ),
                (
                    "Bruno Costa".to_string(),
                    "Gamma FC".to_string(),
                    "Delta FC".to_string(),
                    "player-2".to_string(),
                    "team-3".to_string(),
                    "team-4".to_string(),
                    850_000,
                ),
            ],
            "2026-08-03",
        );

        assert_eq!(article.category, NewsCategory::TransferRoundup);
        assert!(article.headline.contains("Transfer Roundup"));
        assert!(
            article
                .body
                .contains("Adam Smith: Alpha FC -> Beta FC (€1.8M)")
        );
        assert!(
            article
                .body
                .contains("Bruno Costa: Gamma FC -> Delta FC (€850K)")
        );
        assert_eq!(
            article.headline_key.as_deref(),
            Some("be.news.transferRoundup.headline")
        );
        assert_eq!(
            article.body_key.as_deref(),
            Some("be.news.transferRoundup.body")
        );
        assert_eq!(
            article.source_key.as_deref(),
            Some("be.source.transferIntelligence")
        );
        assert_eq!(
            article.i18n_params.get("weekStart"),
            Some(&"2026-08-03".to_string())
        );
        assert_eq!(
            article.i18n_params.get("transferCount"),
            Some(&"2".to_string())
        );
        assert!(
            article
                .i18n_params
                .get("deals")
                .is_some_and(|deals| deals.contains("Adam Smith: Alpha FC -> Beta FC (€1.8M)"))
        );
        assert!(
            article
                .i18n_params
                .get("dealsData")
                .is_some_and(|deals| deals.contains("\"fromTeam\":\"Alpha FC\""))
        );
        assert_eq!(
            article.player_ids,
            vec!["player-1".to_string(), "player-2".to_string()]
        );
        assert_eq!(
            article.team_ids,
            vec![
                "team-1".to_string(),
                "team-2".to_string(),
                "team-3".to_string(),
                "team-4".to_string()
            ]
        );
    }

    #[test]
    fn injury_news_article_sets_expected_fields() {
        use super::injury_news_article;
        let article = injury_news_article(
            "injury_player2_2026-08-10",
            "player-2",
            "Bruno Costa",
            "team-2",
            "Beta FC",
            14,
            "2026-08-10",
        );

        assert_eq!(article.id, "injury_player2_2026-08-10");
        assert_eq!(article.category, NewsCategory::InjuryNews);
        assert_eq!(article.team_ids, vec!["team-2".to_string()]);
        assert_eq!(article.player_ids, vec!["player-2".to_string()]);
        assert!(
            article
                .headline_key
                .as_deref()
                .unwrap()
                .starts_with("be.news.injuryNews.headline")
        );
        assert!(
            article
                .body_key
                .as_deref()
                .unwrap()
                .starts_with("be.news.injuryNews.body")
        );
        assert_eq!(
            article.i18n_params.get("player").map(|s| s.as_str()),
            Some("Bruno Costa")
        );
        assert_eq!(
            article.i18n_params.get("team").map(|s| s.as_str()),
            Some("Beta FC")
        );
        // 14 days ≈ 2 weeks → long injury selects Weeks body variant
        assert!(article.body_key.as_deref().unwrap().ends_with("Weeks"));
        assert_eq!(
            article.i18n_params.get("weeksOut").map(|s| s.as_str()),
            Some("2")
        );
    }

    #[test]
    fn injury_news_article_short_injury_uses_days() {
        use super::injury_news_article;
        let article = injury_news_article(
            "injury_test_short",
            "player-3",
            "Carlos Diaz",
            "team-3",
            "Gamma FC",
            5,
            "2026-09-01",
        );
        // Short injury selects Days body variant and never picks headline2
        assert!(article.body_key.as_deref().unwrap().ends_with("Days"));
        assert_eq!(
            article.i18n_params.get("daysOut").map(|s| s.as_str()),
            Some("5")
        );
        assert_ne!(
            article.headline_key.as_deref().unwrap(),
            "be.news.injuryNews.headline2"
        );
    }
}
