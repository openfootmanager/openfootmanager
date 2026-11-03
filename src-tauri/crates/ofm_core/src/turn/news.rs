use crate::game::Game;
use crate::messages;
use crate::news;
use chrono::{Datelike, Duration, NaiveDate};
use domain::league::{Fixture, FixtureStatus, League, StandingEntry, TransferRumour};
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};

const MAX_WEEKLY_TRANSFER_RUMOURS: usize = 2;
const TRANSFER_RUMOUR_RETENTION_DAYS: i64 = 28;
const MAX_DAILY_WORLD_NEWS_ARTICLES: usize = 5;

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
) -> Vec<news::ReportScorer> {
    report
        .goals
        .iter()
        .filter(|goal| goal.side == side)
        .map(|goal| news::ReportScorer {
            player_id: goal.scorer_id.clone(),
            name: player_match_name_or_id(game, &goal.scorer_id),
            minute: u32::from(goal.minute),
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

/// The division's leading scorer: the most goals among the players of its clubs, ties to the
/// name. The digest speaks for one division, so a striker elsewhere in the world is not its
/// top scorer however many goals they have.
fn division_top_scorer<'a>(game: &'a Game, league: &League) -> Option<&'a domain::player::Player> {
    game.players
        .iter()
        .filter(|player| player.stats.goals > 0)
        .filter(|player| {
            player.team_id.as_deref().is_some_and(|team_id| {
                league
                    .standings
                    .iter()
                    .any(|entry| entry.team_id == team_id)
            })
        })
        .max_by(|a, b| {
            a.stats
                .goals
                .cmp(&b.stats.goals)
                .then_with(|| a.match_name.cmp(&b.match_name))
        })
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

/// Selects interesting players from non-user AI teams who could plausibly be the subject
/// of transfer speculation (high market value, expiring contract, or low morale).
fn rumour_candidates(game: &Game) -> Vec<(String, String, String, String)> {
    // (player_id, player_name, team_id, team_name)
    let user_team_id = game.manager.team_id.as_deref().unwrap_or("");

    let current_date = game.clock.current_date.date_naive();

    // `team_name` scans every team, and this runs once per surviving candidate.
    // One borrowed index turns a players × teams sweep into two linear passes.
    let team_names: std::collections::HashMap<&str, &str> = game
        .teams
        .iter()
        .map(|team| (team.id.as_str(), team.name.as_str()))
        .collect();

    game.players
        .iter()
        .filter(|p| {
            let Some(tid) = p.team_id.as_deref() else {
                return false;
            };
            if tid == user_team_id {
                return false;
            }
            if p.injury.is_some() {
                return false;
            }
            let high_value = p.market_value >= 800_000;
            let short_contract = p
                .contract_end()
                .and_then(|end| chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d").ok())
                .map(|end| {
                    let days = (end - current_date).num_days();
                    (1..=365).contains(&days)
                })
                .unwrap_or(false);
            let low_morale = p.morale <= 45;
            high_value || short_contract || low_morale
        })
        .filter_map(|p| {
            let tid = p.team_id.as_deref()?;
            // Matches the old `team_name` fallback: unknown club → empty name.
            let team_name = team_names.get(tid).copied().unwrap_or_default();
            Some((
                p.id.clone(),
                p.match_name.clone(),
                tid.to_string(),
                team_name.to_string(),
            ))
        })
        .collect()
}

fn prune_stale_transfer_rumours(league: &mut League, current_date: NaiveDate) {
    let earliest_kept_date = current_date - Duration::days(TRANSFER_RUMOUR_RETENTION_DAYS - 1);

    league.transfer_rumours.retain(|rumour| {
        chrono::DateTime::parse_from_rfc3339(&rumour.date)
            .map(|created_at| created_at.date_naive() >= earliest_kept_date)
            .unwrap_or(true)
    });
}

fn weekly_rumour_articles(
    game: &mut Game,
    suffix: &str,
    date: &str,
) -> Vec<domain::news::NewsArticle> {
    let mut rng = game.rng_today(&format!("news/weekly-rumours/{suffix}"));
    let current_date = game.clock.current_date.date_naive();
    let existing_article_ids: HashSet<String> =
        game.news.iter().map(|article| article.id.clone()).collect();
    let Some(league) = game.league.as_mut() else {
        return vec![];
    };
    prune_stale_transfer_rumours(league, current_date);
    let candidates = rumour_candidates(game);
    if candidates.is_empty() {
        return vec![];
    }
    let Some(league) = game.league.as_mut() else {
        return vec![];
    };

    // Pick at most 2 distinct players
    let count = candidates.len().min(MAX_WEEKLY_TRANSFER_RUMOURS);
    let mut chosen_indices: Vec<usize> = (0..candidates.len()).collect();
    chosen_indices.shuffle(&mut rng);
    chosen_indices.truncate(count);

    let mut articles = Vec::new();
    for idx in chosen_indices {
        let (player_id, player_name, team_id, team_name) = &candidates[idx];
        let article_id = format!("rumour_{}_{}", player_id, suffix);
        if existing_article_ids.contains(&article_id)
            || league
                .transfer_rumours
                .iter()
                .any(|rumour| rumour.id == article_id)
        {
            continue;
        }

        league.transfer_rumours.push(TransferRumour {
            id: article_id.clone(),
            date: date.to_string(),
            player_id: player_id.clone(),
            player_name: player_name.clone(),
            team_id: team_id.clone(),
            team_name: team_name.clone(),
        });
        articles.push(news::transfer_rumour_gossip_article(
            &article_id,
            player_id,
            player_name,
            team_id,
            team_name,
            date,
        ));
    }

    articles
}

fn completed_preseason_fixtures_for_window(
    league: &League,
    current_date: NaiveDate,
    window_days: i64,
) -> Vec<&Fixture> {
    let window_start = current_date - Duration::days(window_days.saturating_sub(1));

    league
        .fixtures
        .iter()
        .filter(|fixture| {
            fixture.status == FixtureStatus::Completed
                && matches!(
                    fixture.competition,
                    domain::league::FixtureCompetition::Friendly
                        | domain::league::FixtureCompetition::PreseasonTournament
                )
                && NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d")
                    .map(|fixture_date| {
                        fixture_date >= window_start && fixture_date <= current_date
                    })
                    .unwrap_or(false)
        })
        .collect()
}

fn world_news_category_priority(category: &domain::news::NewsCategory) -> i64 {
    match category {
        domain::news::NewsCategory::Editorial => 500_000,
        domain::news::NewsCategory::TransferRoundup => 400_000,
        domain::news::NewsCategory::ManagerialChange => 350_000,
        domain::news::NewsCategory::InjuryNews => 300_000,
        domain::news::NewsCategory::TransferRumour => 200_000,
        domain::news::NewsCategory::LeagueRoundup => 150_000,
        domain::news::NewsCategory::StandingsUpdate => 125_000,
        domain::news::NewsCategory::SeasonPreview => 100_000,
        domain::news::NewsCategory::MatchReport => 0,
    }
}

fn world_news_priority(game: &Game, article: &domain::news::NewsArticle) -> i64 {
    let team_reputation = article
        .team_ids
        .iter()
        .filter_map(|team_id| game.teams.iter().find(|team| team.id == *team_id))
        .map(|team| i64::from(team.reputation))
        .max()
        .unwrap_or(0);
    let player_heat = article
        .player_ids
        .iter()
        .filter_map(|player_id| game.players.iter().find(|player| player.id == *player_id))
        .map(|player| (player.market_value / 10_000) as i64)
        .max()
        .unwrap_or(0);
    let user_team_id = game.manager.team_id.as_deref();
    let rivalry_multiplier = if user_team_id.is_some_and(|team_id| {
        article
            .team_ids
            .iter()
            .any(|article_team_id| article_team_id == team_id)
    }) {
        2
    } else {
        1
    };

    (world_news_category_priority(&article.category) + (team_reputation * 100) + player_heat)
        * rivalry_multiplier
}

/// How many more non-match articles `date` can take under the daily cap.
fn available_world_news_slots(game: &Game, date: &str) -> usize {
    let existing_non_match_count = game
        .news
        .iter()
        .filter(|article| {
            article.date == date && article.category != domain::news::NewsCategory::MatchReport
        })
        .count();
    MAX_DAILY_WORLD_NEWS_ARTICLES.saturating_sub(existing_non_match_count)
}

fn apply_daily_world_news_cap(
    game: &Game,
    date: &str,
    candidates: Vec<domain::news::NewsArticle>,
) -> Vec<domain::news::NewsArticle> {
    let available_slots = available_world_news_slots(game, date);
    keep_highest_priority(game, candidates, available_slots)
}

/// The `available_slots` highest-priority candidates, in their original order.
fn keep_highest_priority(
    game: &Game,
    candidates: Vec<domain::news::NewsArticle>,
    available_slots: usize,
) -> Vec<domain::news::NewsArticle> {
    if candidates.len() <= available_slots {
        return candidates;
    }

    let mut ranked: Vec<(usize, i64)> = candidates
        .iter()
        .enumerate()
        .map(|(index, article)| (index, world_news_priority(game, article)))
        .collect();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));

    let kept_indexes: HashSet<usize> = ranked
        .into_iter()
        .take(available_slots)
        .map(|(index, _)| index)
        .collect();

    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, article)| kept_indexes.contains(&index).then_some(article))
        .collect()
}

fn completed_transfers_for_window(
    game: &Game,
    current_date: NaiveDate,
    window_days: i64,
) -> Vec<(String, String, String, String, String, String, u64)> {
    let Some(league) = &game.league else {
        return Vec::new();
    };

    let window_start = current_date - Duration::days(window_days.saturating_sub(1));
    let mut transfers: Vec<_> = league
        .transfer_log
        .iter()
        .filter_map(|transfer| {
            let transfer_date = NaiveDate::parse_from_str(&transfer.date, "%Y-%m-%d").ok()?;
            if transfer_date < window_start || transfer_date > current_date {
                return None;
            }

            Some((
                player_match_name_or_id(game, &transfer.player_id),
                team_name(game, &transfer.from_team_id),
                team_name(game, &transfer.to_team_id),
                transfer.player_id.clone(),
                transfer.from_team_id.clone(),
                transfer.to_team_id.clone(),
                transfer.fee,
                transfer.date.clone(),
            ))
        })
        .collect();

    transfers.sort_by(|left, right| right.6.cmp(&left.6).then(right.7.cmp(&left.7)));
    transfers.truncate(3);
    transfers
        .into_iter()
        .map(
            |(player, from_team, to_team, player_id, from_team_id, to_team_id, fee, _)| {
                (
                    player,
                    from_team,
                    to_team,
                    player_id,
                    from_team_id,
                    to_team_id,
                    fee,
                )
            },
        )
        .collect()
}

fn weekly_transfer_roundup_article(
    game: &Game,
    suffix: &str,
    week_start: &str,
    date: &str,
) -> Option<domain::news::NewsArticle> {
    let article_id = format!("weekly_transfer_roundup_{}", suffix);
    if game.news.iter().any(|article| article.id == article_id) {
        return None;
    }

    let transfers = completed_transfers_for_window(game, game.clock.current_date.date_naive(), 7);
    if transfers.is_empty() {
        return None;
    }

    Some(news::transfer_roundup_article(
        &article_id,
        week_start,
        &transfers,
        date,
    ))
}

fn preseason_unbeaten_teams(game: &Game) -> Vec<String> {
    let Some(league) = &game.league else {
        return Vec::new();
    };

    let mut records: HashMap<String, (u32, u32)> = HashMap::new();

    for fixture in &league.fixtures {
        if fixture.status != FixtureStatus::Completed
            || !matches!(
                fixture.competition,
                domain::league::FixtureCompetition::Friendly
                    | domain::league::FixtureCompetition::PreseasonTournament
            )
        {
            continue;
        }

        let Some(result) = &fixture.result else {
            continue;
        };

        let home_record = records
            .entry(fixture.home_team_id.clone())
            .or_insert((0, 0));
        home_record.0 += 1;
        if result.home_goals < result.away_goals {
            home_record.1 += 1;
        }

        let away_record = records
            .entry(fixture.away_team_id.clone())
            .or_insert((0, 0));
        away_record.0 += 1;
        if result.away_goals < result.home_goals {
            away_record.1 += 1;
        }
    }

    let mut unbeaten: Vec<(String, u32)> = records
        .into_iter()
        .filter(|(_, (played, losses))| *played >= 2 && *losses == 0)
        .map(|(team_id, (played, _))| (team_name(game, &team_id), played))
        .collect();
    unbeaten.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));

    unbeaten.into_iter().map(|(team, _)| team).collect()
}

fn generate_preseason_digest_news(game: &mut Game, today: &str) {
    let suffix = weekly_digest_suffix(game);
    let digest_id = format!("preseason_digest_{}", suffix);
    if game.news.iter().any(|article| article.id == digest_id) {
        return;
    }

    let current_date = game.clock.current_date.date_naive();
    let date = game.clock.current_date.to_rfc3339();
    let (results, unbeaten_teams) = {
        let Some(league) = &game.league else {
            return;
        };

        let fixtures = completed_preseason_fixtures_for_window(league, current_date, 7);
        (
            matchday_results(game, &fixtures),
            preseason_unbeaten_teams(game),
        )
    };

    let mut candidates = vec![news::preseason_digest_article(
        &digest_id,
        today,
        &results,
        &unbeaten_teams,
        &date,
    )];
    if let Some(roundup) = weekly_transfer_roundup_article(game, &suffix, today, &date) {
        candidates.push(roundup);
    }
    let rumours = weekly_rumour_articles(game, &suffix, &date);
    candidates.extend(rumours);
    game.news
        .extend(apply_daily_world_news_cap(game, &date, candidates));
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
        generate_preseason_digest_news(game, today);
        return;
    }

    let suffix = weekly_digest_suffix(game);
    let digest_id = format!("weekly_digest_{}", suffix);
    if game.news.iter().any(|article| article.id == digest_id) {
        return;
    }

    let date = game.clock.current_date.to_rfc3339();
    let leader_team_id = league
        .sorted_standings()
        .first()
        .map(|entry| entry.team_id.clone());
    let top_scorer = division_top_scorer(game, league).map(|player| {
        (
            player.id.clone(),
            player.match_name.clone(),
            player.stats.goals,
        )
    });
    let storylines = weekly_storyline_articles(game, &suffix, &date);
    // Rumours are drawn even on a full day: drawing them also retires stale ones.
    let rumours = weekly_rumour_articles(game, &suffix, &date);
    let mut coverage = Vec::new();
    if let Some(roundup) = weekly_transfer_roundup_article(game, &suffix, today, &date) {
        coverage.push(roundup);
    }
    coverage.extend(storylines);
    coverage.extend(rumours);

    // A division with no table has no digest ("Unknown leads the table" is not news), but the
    // week's other coverage does not depend on the table.
    let Some(leader_team_id) = leader_team_id else {
        let coverage = apply_daily_world_news_cap(game, &date, coverage);
        game.news.extend(coverage);
        return;
    };
    let leader = team_name(game, &leader_team_id);

    // The digest is published every week, outside the daily cap, as match reports are: on a busy
    // Monday the transfer, loan, appointment and injury writers can fill the day before it runs,
    // and the week's summary must not be what gives way. The rest of the week's coverage competes
    // for the slots that are left, and the digest counts only the storylines that won one.
    let available_slots = available_world_news_slots(game, &date);
    let coverage = keep_highest_priority(game, coverage, available_slots);
    // Counted after the cap, so the digest names only storylines that were published.
    let storyline_count = coverage
        .iter()
        .filter(|article| article.id.starts_with("storyline_"))
        .count();

    let digest = news::weekly_digest_article(
        &digest_id,
        today,
        &leader_team_id,
        &leader,
        top_scorer
            .as_ref()
            .map(|(player_id, name, goals)| news::DigestTopScorer {
                player_id,
                name,
                goals: *goals,
            })
            .as_ref(),
        storyline_count,
        &date,
    );
    game.news.push(digest);
    game.news.extend(coverage);
}

/// Generate a match report news article for the completed fixture.
pub(super) fn generate_match_news(
    game: &mut Game,
    fixture_index: usize,
    home_team_id: &str,
    away_team_id: &str,
    report: &engine::MatchReport,
) {
    let league = game.league.as_ref().unwrap();
    let fixture = &league.fixtures[fixture_index];
    let article_id = format!("report_{}", fixture.id);
    if game.news.iter().any(|n| n.id == article_id) {
        return;
    }

    let home_name = team_name_or(game, home_team_id, "Home");
    let away_name = team_name_or(game, away_team_id, "Away");
    let home_scorers = scorers_for_side(game, report, engine::Side::Home);
    let away_scorers = scorers_for_side(game, report, engine::Side::Away);
    let date = game.clock.current_date.to_rfc3339();

    let article = news::match_report_article(&news::MatchReportFacts {
        fixture_id: &fixture.id,
        competition: news::ReportCompetition {
            kind: fixture.competition.clone(),
            name: &league.name,
            name_key: league.name_key.as_deref(),
            country_id: league.country_id.as_deref(),
            season: league.season,
        },
        matchday: fixture.matchday,
        date: &date,
        home: news::ReportSide {
            team_id: home_team_id,
            name: &home_name,
            goals: report.home_goals,
            scorers: &home_scorers,
        },
        away: news::ReportSide {
            team_id: away_team_id,
            name: &away_name,
            goals: report.away_goals,
            scorers: &away_scorers,
        },
    });
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

    let matchday = todays_fixtures
        .iter()
        .map(|fixture| fixture.matchday)
        .min()
        .unwrap_or_default();
    // The clock's time of day on the day being summarised: the day is `today`, which is also what
    // the fixtures were filtered by, so the article id and the filter cannot disagree.
    let clock_stamp = game.clock.current_date.to_rfc3339();
    let date_str = format!("{today}{}", clock_stamp.get(10..).unwrap_or_default());

    // Don't duplicate. The competition id is part of the key because the live-match day now
    // sweeps every competition due today: without it, a foreign league reaching matchday 2 first
    // would silently suppress the user's own matchday-2 roundup and standings article, and news
    // is only cleared at rollover.
    let roundup_id = news::roundup_article_id(&league.id, today);
    if game.news.iter().any(|n| n.id == roundup_id) {
        return;
    }

    let results = matchday_results(game, &todays_fixtures);

    let roundup = news::league_roundup_article(&league.id, matchday, &results, &date_str);
    game.news.push(roundup);

    let standings = standings_rows(game, league);

    let standings_article =
        news::standings_update_article(&league.id, matchday, &standings, &date_str);
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

    let mut previews = Vec::new();
    if let Some(league) = &game.league {
        let upcoming = scheduled_user_fixtures_for_date(league, &user_team_id, &target_str);

        for fixture in upcoming {
            let (opponent_id, is_home) = opponent_for_fixture(fixture, &user_team_id);
            let opponent_name = team_name_or(game, opponent_id, "Unknown");

            // Check if we already sent this message
            let msg_id = format!("prematch_{}", fixture.id);
            if crate::inbox::already_emitted(game, &msg_id) {
                continue;
            }

            previews.push(messages::pre_match_message(
                &fixture.id,
                &opponent_name,
                opponent_id,
                is_home,
                fixture.matchday,
                &target_str,
                &game.clock.current_date.to_rfc3339(),
            ));
        }
    }
    crate::inbox::emit_all(game, previews);
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
        TransferRumour,
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
                home_penalties: None,
                away_penalties: None,
            }),
            ..Default::default()
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
            home_penalties: None,
            away_penalties: None,
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
            transfer_rumours: vec![],
            ..Default::default()
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

    fn add_completed_transfer(
        game: &mut Game,
        date: &str,
        player_id: &str,
        player_name: &str,
        from_team_id: &str,
        to_team_id: &str,
        fee: u64,
    ) {
        game.players
            .push(make_player(player_id, player_name, to_team_id));
        game.league
            .as_mut()
            .unwrap()
            .transfer_log
            .push(domain::league::CompletedTransfer {
                date: date.to_string(),
                from_team_id: from_team_id.to_string(),
                to_team_id: to_team_id.to_string(),
                player_id: player_id.to_string(),
                fee,
            });
    }

    #[test]
    fn generate_matchday_news_adds_roundup_and_standings_for_completed_fixtures_today() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);

        generate_matchday_news(&mut game, "2025-08-12");

        assert_eq!(game.news.len(), 2);

        let roundup = game
            .news
            .iter()
            .find(|article| article.id == "roundup_league1_2025-08-12")
            .unwrap();
        assert_eq!(roundup.category, NewsCategory::LeagueRoundup);
        assert_eq!(roundup.body, "");
        assert_eq!(roundup.body_key.as_deref(), Some("be.news.roundup.body"));
        assert_eq!(
            roundup.i18n_params.get("results"),
            Some(&"  Alpha FC 2 - 1 Beta FC".to_string())
        );
        assert_eq!(
            roundup.i18n_params.get("resultsData"),
            Some(
                &"[{\"home\":\"Alpha FC\",\"homeGoals\":2,\"away\":\"Beta FC\",\"awayGoals\":1}]"
                    .to_string()
            )
        );

        let standings = game
            .news
            .iter()
            .find(|article| article.id == "standings_league1_2025-08-12")
            .unwrap();
        assert_eq!(standings.category, NewsCategory::StandingsUpdate);
        assert_eq!(standings.body, "");
        assert_eq!(
            standings.body_key.as_deref(),
            Some("be.news.standings.body")
        );
        assert_eq!(
            standings.i18n_params.get("leader"),
            Some(&"Alpha FC".to_string())
        );
    }

    #[test]
    fn generate_matchday_news_does_nothing_when_today_has_no_completed_fixtures() {
        let mut game = make_game("2025-08-12", FixtureStatus::Scheduled);

        generate_matchday_news(&mut game, "2025-08-12");

        assert!(game.news.is_empty());
    }

    /// Given a round whose second match was played a day later, when the news is generated on each day,
    /// then each day gets its own roundup and standings article, listing only that day's match.
    #[test]
    fn news_for_a_split_round_posts_each_date_once() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);

        generate_matchday_news(&mut game, "2025-08-12");
        set_current_date(&mut game, 2025, 8, 13);
        generate_matchday_news(&mut game, "2025-08-13");
        generate_matchday_news(&mut game, "2025-08-13");

        let roundups: Vec<_> = game
            .news
            .iter()
            .filter(|article| article.category == NewsCategory::LeagueRoundup)
            .collect();
        assert_eq!(roundups.len(), 2, "one roundup for each date");
        assert_eq!(
            game.news.len(),
            4,
            "plus one standings article for each date"
        );
        let first = roundups
            .iter()
            .find(|a| a.id == "roundup_league1_2025-08-12")
            .unwrap();
        let second = roundups
            .iter()
            .find(|a| a.id == "roundup_league1_2025-08-13")
            .unwrap();
        assert_eq!(
            first.i18n_params.get("results"),
            Some(&"  Alpha FC 2 - 1 Beta FC".to_string())
        );
        assert_eq!(
            second.i18n_params.get("results"),
            Some(&"  Gamma FC 0 - 0 Beta FC".to_string())
        );
    }

    /// Given a late match of one round and an on-time match of the next played on one day, when the news
    /// is generated, then the roundup is labelled with the lowest matchday of the day.
    #[test]
    fn a_day_with_two_matchdays_is_labelled_by_the_lowest() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);
        let league = game.league.as_mut().unwrap();
        league.fixtures[1].date = "2025-08-12".to_string();
        league.fixtures[1].matchday = 3;

        generate_matchday_news(&mut game, "2025-08-12");

        let roundup = game
            .news
            .iter()
            .find(|article| article.id == "roundup_league1_2025-08-12")
            .expect("one roundup for the day");
        assert_eq!(roundup.i18n_params.get("matchday"), Some(&"3".to_string()));
    }

    /// Given a day whose lowest matchday changes between two passes of the news (a late match of an
    /// earlier round completes after the first pass), when the news is generated on both passes,
    /// then the day has exactly one roundup and one standings article.
    #[test]
    fn a_lower_matchday_completing_later_in_the_day_does_not_double_the_news() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);
        let league = game.league.as_mut().unwrap();
        let mut late = league.fixtures[1].clone();
        late.id = "fx3".to_string();
        late.date = "2025-08-12".to_string();
        late.matchday = 3;
        late.status = FixtureStatus::Scheduled;
        late.result = None;
        league.fixtures.push(late);

        generate_matchday_news(&mut game, "2025-08-12");
        let league = game.league.as_mut().unwrap();
        league.fixtures[2].status = FixtureStatus::Completed;
        league.fixtures[2].result = Some(MatchResult {
            home_goals: 1,
            away_goals: 1,
            ..Default::default()
        });
        generate_matchday_news(&mut game, "2025-08-12");

        let count = |category: NewsCategory| {
            game.news
                .iter()
                .filter(|article| article.category == category)
                .count()
        };
        assert_eq!(count(NewsCategory::LeagueRoundup), 1);
        assert_eq!(count(NewsCategory::StandingsUpdate), 1);
    }

    #[test]
    fn a_foreign_leagues_matchday_does_not_suppress_the_users_own() {
        // The live-match day sweeps every competition due today, so `generate_matchday_news` is
        // now called for leagues other than the user's. Keyed on the matchday alone, a foreign
        // league reaching matchday 4 first would make the user's own matchday-4 roundup and
        // standings article look like duplicates and silently drop them — and news is only
        // cleared at rollover, so they would never arrive.
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);
        let users_league = game.league.clone().expect("the helper builds a league");

        let mut foreign = users_league.clone();
        foreign.id = "league2".to_string();
        foreign.name = "Somewhere Else".to_string();

        // The sweep's order: some other competition's matchday 4, then the user's.
        game.league = Some(foreign);
        generate_matchday_news(&mut game, "2025-08-12");
        game.league = Some(users_league);
        generate_matchday_news(&mut game, "2025-08-12");

        for id in [
            "roundup_league2_2025-08-12",
            "standings_league2_2025-08-12",
            "roundup_league1_2025-08-12",
            "standings_league1_2025-08-12",
        ] {
            assert_eq!(
                game.news.iter().filter(|article| article.id == id).count(),
                1,
                "{id} should be written exactly once"
            );
        }
        assert_eq!(game.news.len(), 4, "both competitions get both articles");
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
                .filter(|article| article.id == "roundup_league1_2025-08-12")
                .count(),
            1
        );
        assert_eq!(
            game.news
                .iter()
                .filter(|article| article.id == "standings_league1_2025-08-12")
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
                    goal_source: engine::report::GoalSource::OpenPlay,
                    side: Side::Home,
                },
                GoalDetail {
                    minute: 74,
                    scorer_id: "ghost9".to_string(),
                    assist_id: None,
                    goal_source: engine::report::GoalSource::OpenPlay,
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
            vec!["p1".to_string(), "ghost9".to_string()],
            "player_ids are ids to link to; the names travel in scorersData"
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
        assert_eq!(article.headline, "");
        assert_eq!(article.body, "");
        assert_eq!(article.i18n_params.get("scorers"), Some(&String::new()));
        assert_eq!(
            article.i18n_params.get("scorersData"),
            Some(
                &"[{\"player\":\"Alice\",\"minute\":10,\"team\":\"Alpha FC\"},{\"player\":\"ghost9\",\"minute\":74,\"team\":\"Beta FC\"}]".to_string()
            )
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
    fn generate_match_news_for_friendly_uses_preseason_framing() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);
        game.league.as_mut().unwrap().fixtures[0].competition =
            domain::league::FixtureCompetition::Friendly;
        let report = make_report(vec![], 2, 1);

        generate_match_news(&mut game, 0, "team1", "team2", &report);

        let article = &game.news[0];
        assert_eq!(article.category, NewsCategory::MatchReport);
        assert_eq!(article.headline, "");
        assert_eq!(article.body, "");
        assert_eq!(
            article.headline_key.as_deref(),
            Some("be.news.matchReport.reportFriendly.title")
        );
        assert_eq!(
            article.body_key.as_deref(),
            Some("be.news.matchReport.reportFriendly.body")
        );
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

    fn goal(minute: u8, scorer_id: &str, side: Side) -> GoalDetail {
        GoalDetail {
            minute,
            scorer_id: scorer_id.to_string(),
            assist_id: None,
            goal_source: engine::report::GoalSource::OpenPlay,
            side,
        }
    }

    /// Given a home player who scores three times,
    /// when the match report is written,
    /// then the article links that player once, by id.
    #[test]
    fn a_hat_trick_scorer_is_linked_once_by_player_id() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);
        game.players = vec![make_player("p1", "Alice", "team1")];
        let report = make_report(
            vec![
                goal(10, "p1", Side::Home),
                goal(30, "p1", Side::Home),
                goal(80, "p1", Side::Home),
            ],
            3,
            0,
        );

        generate_match_news(&mut game, 0, "team1", "team2", &report);

        assert_eq!(game.news[0].player_ids, vec!["p1".to_string()]);
    }

    /// Given a cup tie (and each other competitive fixture outside the league round),
    /// when its report is written,
    /// then it is not framed as a friendly, and it names its competition.
    #[test]
    fn a_competitive_match_outside_the_league_is_not_reported_as_a_friendly() {
        for competition in [
            FixtureCompetition::Cup,
            FixtureCompetition::ContinentalClub,
            FixtureCompetition::InternationalClub,
            FixtureCompetition::InternationalNation,
        ] {
            let mut game = make_game("2025-08-12", FixtureStatus::Completed);
            let league = game.league.as_mut().unwrap();
            league.name = "Germany Cup".to_string();
            league.name_key = Some("tournaments.competitions.nationalCup".to_string());
            league.country_id = Some("DE".to_string());
            league.fixtures[0].competition = competition.clone();
            let report = make_report(vec![], 2, 1);

            generate_match_news(&mut game, 0, "team1", "team2", &report);

            let article = &game.news[0];
            assert_eq!(
                article.headline_key.as_deref(),
                Some("be.news.matchReport.reportCompetition.title"),
                "{competition:?}"
            );
            assert_eq!(
                article.body_key.as_deref(),
                Some("be.news.matchReport.reportCompetition.body"),
                "{competition:?}"
            );
            let param = |name: &str| article.i18n_params.get(name).map(String::as_str);
            assert_eq!(param("competition"), Some("Germany Cup"), "{competition:?}");
            assert_eq!(
                param("competitionKey"),
                Some("tournaments.competitions.nationalCup"),
                "the frontend translates the name from its key, as every other screen does"
            );
            assert_eq!(param("competitionCountry"), Some("DE"));
            assert_eq!(param("competitionSeason"), Some("1"));
        }
    }

    /// Given a friendly-cup fixture (a preseason invitational),
    /// when its report is written,
    /// then it keeps the friendly framing.
    #[test]
    fn a_friendly_cup_match_keeps_the_friendly_copy() {
        let mut game = make_game("2025-08-12", FixtureStatus::Completed);
        game.league.as_mut().unwrap().fixtures[0].competition = FixtureCompetition::FriendlyCup;
        let report = make_report(vec![], 1, 1);

        generate_match_news(&mut game, 0, "team1", "team2", &report);

        assert_eq!(
            game.news[0].headline_key.as_deref(),
            Some("be.news.matchReport.reportFriendly.title")
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
        assert!(message.subject.is_empty());
        assert!(message.body.is_empty());
        assert_eq!(
            message.subject_key.as_deref(),
            Some("be.msg.preMatch.subject")
        );
        assert!(matches!(
            message.body_key.as_deref(),
            Some("be.msg.preMatch.body0Home" | "be.msg.preMatch.body1Home")
        ));
        assert_eq!(message.context.fixture_id.as_deref(), Some("fx1"));
        assert_eq!(message.context.team_id.as_deref(), Some("team2"));
        assert_eq!(message.i18n_params.get("venue"), Some(&"H".to_string()));
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
    fn generate_weekly_digest_news_creates_preseason_digest_even_on_monday() {
        let mut game = make_game("2025-08-11", FixtureStatus::Scheduled);
        set_current_date(&mut game, 2025, 8, 11);
        reset_to_preseason(&mut game);

        generate_weekly_digest_news(&mut game, "2025-08-11");

        let digest = game
            .news
            .iter()
            .find(|article| article.id.starts_with("preseason_digest_"))
            .unwrap();
        assert_eq!(digest.category, NewsCategory::Editorial);
        assert_eq!(digest.headline, "");
        assert_eq!(digest.body, "");
        assert_eq!(digest.source, "");
        assert_eq!(
            digest.headline_key.as_deref(),
            Some("be.news.preseasonDigest.headline")
        );
        assert_eq!(
            digest.body_key.as_deref(),
            Some("be.news.preseasonDigest.bodyNoResults")
        );
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
    fn preseason_digest_summarizes_recent_friendly_results_without_league_table_copy() {
        let mut game = make_game("2025-08-11", FixtureStatus::Scheduled);
        set_current_date(&mut game, 2025, 8, 11);
        reset_to_preseason(&mut game);

        let league = game.league.as_mut().unwrap();
        league.fixtures[0].competition = domain::league::FixtureCompetition::Friendly;
        league.fixtures[0].status = FixtureStatus::Completed;
        league.fixtures[0].date = "2025-08-09".to_string();
        league.fixtures[0].result = Some(domain::league::MatchResult {
            home_goals: 2,
            away_goals: 1,
            home_scorers: vec![],
            away_scorers: vec![],
            report: None,
            home_penalties: None,
            away_penalties: None,
        });
        league.fixtures[1].competition = domain::league::FixtureCompetition::Friendly;
        league.fixtures[1].status = FixtureStatus::Completed;
        league.fixtures[1].date = "2025-08-10".to_string();
        league.fixtures[1].result = Some(domain::league::MatchResult {
            home_goals: 0,
            away_goals: 0,
            home_scorers: vec![],
            away_scorers: vec![],
            report: None,
            home_penalties: None,
            away_penalties: None,
        });

        generate_weekly_digest_news(&mut game, "2025-08-11");

        let digest = game
            .news
            .iter()
            .find(|article| article.id.starts_with("preseason_digest_"))
            .unwrap();
        assert_eq!(
            digest.headline_key.as_deref(),
            Some("be.news.preseasonDigest.headline")
        );
        assert_eq!(
            digest.body_key.as_deref(),
            Some("be.news.preseasonDigest.bodyWithResults")
        );
        assert_eq!(digest.body, "");
        assert!(
            digest
                .i18n_params
                .get("results")
                .is_some_and(|results| results.contains("Alpha FC 2 - 1 Beta FC"))
        );
        assert!(
            digest
                .i18n_params
                .get("results")
                .is_some_and(|results| results.contains("Gamma FC 0 - 0 Beta FC"))
        );
        assert_eq!(
            digest.i18n_params.get("resultCount"),
            Some(&"2".to_string())
        );
    }

    #[test]
    fn generate_weekly_digest_news_does_not_duplicate_same_preseason_week() {
        let mut game = make_game("2025-08-11", FixtureStatus::Scheduled);
        set_current_date(&mut game, 2025, 8, 11);
        reset_to_preseason(&mut game);

        generate_weekly_digest_news(&mut game, "2025-08-11");
        generate_weekly_digest_news(&mut game, "2025-08-11");

        assert_eq!(
            game.news
                .iter()
                .filter(|article| article.id.starts_with("preseason_digest_"))
                .count(),
            1
        );
    }

    #[test]
    fn preseason_digest_includes_weekly_transfer_roundup_for_recent_moves() {
        let mut game = make_game("2025-08-11", FixtureStatus::Scheduled);
        set_current_date(&mut game, 2025, 8, 11);
        reset_to_preseason(&mut game);
        add_completed_transfer(
            &mut game,
            "2025-08-09",
            "player-transfer-1",
            "Marcos",
            "team2",
            "team3",
            1_800_000,
        );
        add_completed_transfer(
            &mut game,
            "2025-08-10",
            "player-transfer-2",
            "Elliot",
            "team3",
            "team2",
            950_000,
        );

        generate_weekly_digest_news(&mut game, "2025-08-11");

        let roundup = game
            .news
            .iter()
            .find(|article| article.id.starts_with("weekly_transfer_roundup_"))
            .expect("expected a weekly transfer roundup article");
        assert_eq!(roundup.category, NewsCategory::TransferRoundup);
        assert_eq!(roundup.headline, "");
        assert_eq!(roundup.body, "");
        assert_eq!(roundup.source, "");
        assert_eq!(
            roundup.headline_key.as_deref(),
            Some("be.news.transferRoundup.headline")
        );
        assert_eq!(
            roundup.body_key.as_deref(),
            Some("be.news.transferRoundup.body")
        );
        assert_eq!(
            roundup.source_key.as_deref(),
            Some("be.source.transferIntelligence")
        );
        assert!(
            roundup
                .i18n_params
                .get("deals")
                .is_some_and(|deals| deals.contains("Marcos: Beta FC -> Gamma FC (€1.8M)"))
        );
        assert!(
            roundup
                .i18n_params
                .get("deals")
                .is_some_and(|deals| deals.contains("Elliot: Gamma FC -> Beta FC (€950K)"))
        );
        assert_eq!(roundup.player_ids.len(), 2);
        assert!(
            roundup
                .player_ids
                .contains(&"player-transfer-1".to_string())
        );
        assert!(
            roundup
                .player_ids
                .contains(&"player-transfer-2".to_string())
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
        assert!(!weekly_digest.i18n_params.contains_key("weekLabel"));

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
        assert!(
            game.news
                .iter()
                .all(|article| !article.id.starts_with("weekly_transfer_roundup_"))
        );
    }

    #[test]
    fn weekly_digest_includes_transfer_roundup_for_recent_completed_moves() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);
        add_completed_transfer(
            &mut game,
            "2025-08-08",
            "player-transfer-3",
            "Nico",
            "team2",
            "team3",
            2_400_000,
        );
        add_completed_transfer(
            &mut game,
            "2025-08-10",
            "player-transfer-4",
            "Rui",
            "team3",
            "team2",
            1_250_000,
        );

        generate_weekly_digest_news(&mut game, "2025-08-11");

        let roundup = game
            .news
            .iter()
            .find(|article| article.id.starts_with("weekly_transfer_roundup_"))
            .expect("expected a weekly transfer roundup article");
        assert_eq!(roundup.category, NewsCategory::TransferRoundup);
        assert_eq!(roundup.body, "");
        assert_eq!(
            roundup.body_key.as_deref(),
            Some("be.news.transferRoundup.body")
        );
        assert!(
            roundup
                .i18n_params
                .get("deals")
                .is_some_and(|deals| deals.contains("Nico: Beta FC -> Gamma FC (€2.4M)"))
        );
        assert!(
            roundup
                .i18n_params
                .get("deals")
                .is_some_and(|deals| deals.contains("Rui: Gamma FC -> Beta FC (€1.2M)"))
        );
    }

    #[test]
    fn weekly_digest_caps_world_news_and_keeps_higher_priority_rumours() {
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
        team_mut(&mut game, "team2").reputation = 780;
        team_mut(&mut game, "team3").reputation = 320;

        add_completed_transfer(
            &mut game,
            "2025-08-10",
            "player-transfer-6",
            "Rafa",
            "team2",
            "team3",
            1_400_000,
        );

        let mut marquee_player = make_player("ai-marquee", "Marquee", "team2");
        marquee_player.market_value = 1_500_000;
        game.players.push(marquee_player);

        let mut fringe_player = make_player("ai-fringe", "Fringe", "team3");
        fringe_player.market_value = 900_000;
        game.players.push(fringe_player);

        // One article already out today leaves four slots for the five coverage candidates (the
        // roundup, two storylines, two rumours), so the cap has to choose between the rumours.
        publish_earlier_news_today(&mut game, 1);
        generate_weekly_digest_news(&mut game, "2025-08-11");

        assert!(weekly_digest(&game).is_some());
        assert_eq!(
            game.news
                .iter()
                .filter(|article| !article.id.starts_with("weekly_digest_"))
                .count(),
            5,
            "the week's other coverage respects the hard cap; only the digest stands outside it"
        );
        assert!(
            game.news
                .iter()
                .any(|article| article.id.starts_with("rumour_ai-marquee_")),
            "The higher-priority rumour should survive the cap"
        );
        assert!(
            game.news
                .iter()
                .all(|article| !article.id.starts_with("rumour_ai-fringe_")),
            "The lower-priority rumour should be dropped when the cap is reached"
        );
    }

    /// Two storylines this week: Alpha one point clear of Beta, and on a five-match unbeaten run.
    fn set_up_a_week_with_two_storylines(game: &mut Game) {
        for (team_id, points, goals_for, goals_against) in [
            ("team1", 25, 18, 8),
            ("team2", 24, 16, 9),
            ("team3", 7, 6, 15),
        ] {
            let entry = standing_mut(game, team_id);
            entry.played = 10;
            entry.points = points;
            entry.goals_for = goals_for;
            entry.goals_against = goals_against;
        }
        team_mut(game, "team1").form = ["D", "W", "W", "W", "W"]
            .iter()
            .map(|result| result.to_string())
            .collect();
    }

    /// Fill `count` of the day's world-news slots with articles already published today.
    fn publish_earlier_news_today(game: &mut Game, count: usize) {
        let today = game.clock.current_date.to_rfc3339();
        for index in 0..count {
            game.news.push(domain::news::NewsArticle::new(
                format!("earlier_{index}"),
                String::new(),
                String::new(),
                String::new(),
                today.clone(),
                NewsCategory::TransferRumour,
            ));
        }
    }

    fn weekly_digest(game: &Game) -> Option<&domain::news::NewsArticle> {
        game.news
            .iter()
            .find(|article| article.id.starts_with("weekly_digest_"))
    }

    fn published_storylines(game: &Game) -> usize {
        game.news
            .iter()
            .filter(|article| article.id.starts_with("storyline_"))
            .count()
    }

    /// Given a week with two storylines and only one free news slot today,
    /// when the weekly digest is written,
    /// then it is published, and the number of storylines it announces is the number published.
    #[test]
    fn the_digest_counts_only_the_storylines_published_with_it() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);
        set_up_a_week_with_two_storylines(&mut game);
        publish_earlier_news_today(&mut game, 4);

        generate_weekly_digest_news(&mut game, "2025-08-11");

        let digest = weekly_digest(&game).expect("the digest is published outside the cap");
        assert_eq!(
            digest.i18n_params.get("storylineCount"),
            Some(&published_storylines(&game).to_string()),
            "the digest may only count storylines the reader can find"
        );
        assert_eq!(published_storylines(&game), 1);
        assert_eq!(
            digest.body_key.as_deref(),
            Some("be.news.weeklyDigest.bodyNoTopScorer"),
            "a published storyline is announced in the body"
        );
    }

    /// Given a busy Monday whose world-news slots were all taken by earlier news,
    /// when the weekly digest is written,
    /// then the digest is still published, with none of the week's other coverage, and it says
    /// nothing about storylines it could not publish.
    #[test]
    fn a_full_news_day_still_gets_its_digest_and_no_coverage() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);
        set_up_a_week_with_two_storylines(&mut game);
        publish_earlier_news_today(&mut game, 5);

        generate_weekly_digest_news(&mut game, "2025-08-11");

        let digest = weekly_digest(&game).expect("the digest is not subject to the daily cap");
        assert_eq!(published_storylines(&game), 0);
        assert_eq!(
            digest.body_key.as_deref(),
            Some("be.news.weeklyDigest.bodyNoTopScorerNoStorylines")
        );
        assert!(!digest.i18n_params.contains_key("storylineCount"));
    }

    /// Given a week with no storyline,
    /// when the weekly digest is written,
    /// then its body makes no claim about storylines.
    #[test]
    fn a_week_without_storylines_makes_no_storyline_claim() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);

        generate_weekly_digest_news(&mut game, "2025-08-11");

        let digest = weekly_digest(&game).expect("a digest still reports the leader");
        assert_eq!(published_storylines(&game), 0);
        assert_eq!(
            digest.body_key.as_deref(),
            Some("be.news.weeklyDigest.bodyNoTopScorerNoStorylines")
        );
        assert!(!digest.i18n_params.contains_key("storylineCount"));
    }

    /// Given a striker abroad with more goals than anyone in the division,
    /// when the weekly digest is written,
    /// then it names the division's top scorer, and links the leader and that scorer.
    #[test]
    fn the_digest_names_and_links_the_divisions_top_scorer_and_leader() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);
        let mut home_striker = make_player("p-home", "Home Striker", "team2");
        home_striker.stats.goals = 6;
        let mut abroad = make_player("p-abroad", "Abroad Striker", "team-elsewhere");
        abroad.stats.goals = 20;
        game.players = vec![home_striker, abroad];

        generate_weekly_digest_news(&mut game, "2025-08-11");

        let digest = weekly_digest(&game).unwrap();
        assert_eq!(
            digest.i18n_params.get("topScorer").map(String::as_str),
            Some("Home Striker")
        );
        assert_eq!(digest.team_ids, vec!["team1".to_string()]);
        assert_eq!(digest.player_ids, vec!["p-home".to_string()]);
    }

    /// Given a division whose table is empty,
    /// when the weekly digest would be written,
    /// then none is written rather than one led by "Unknown".
    #[test]
    fn a_division_without_a_table_gets_no_digest() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);
        game.league.as_mut().unwrap().standings.clear();

        generate_weekly_digest_news(&mut game, "2025-08-11");

        assert!(weekly_digest(&game).is_none());
    }

    /// Given a division with no table but a completed transfer this week,
    /// when the week's news is written,
    /// then the transfer roundup is still published: only the digest needs a table.
    #[test]
    fn a_division_without_a_table_still_gets_its_other_weekly_news() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);
        game.league.as_mut().unwrap().standings.clear();
        add_completed_transfer(
            &mut game,
            "2025-08-10",
            "player-transfer-1",
            "Rafa",
            "team2",
            "team3",
            1_400_000,
        );

        generate_weekly_digest_news(&mut game, "2025-08-11");

        assert!(weekly_digest(&game).is_none());
        assert!(
            game.news
                .iter()
                .any(|article| article.id.starts_with("weekly_transfer_roundup_"))
        );
    }

    #[test]
    fn generate_weekly_digest_news_does_not_duplicate_same_week() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);

        let mut notable_player = Player::new(
            "ai-repeatable".to_string(),
            "Repeatable".to_string(),
            "Repeatable Player".to_string(),
            "1997-06-01".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        notable_player.team_id = Some("team2".to_string());
        notable_player.market_value = 1_300_000;
        game.players.push(notable_player);

        add_completed_transfer(
            &mut game,
            "2025-08-10",
            "player-transfer-5",
            "Dario",
            "team2",
            "team3",
            1_600_000,
        );

        generate_weekly_digest_news(&mut game, "2025-08-11");
        generate_weekly_digest_news(&mut game, "2025-08-11");

        assert_eq!(
            game.news
                .iter()
                .filter(|article| article.id.starts_with("weekly_digest_"))
                .count(),
            1
        );
        assert_eq!(
            game.news
                .iter()
                .filter(|article| article.id.starts_with("weekly_transfer_roundup_"))
                .count(),
            1
        );
        assert_eq!(
            game.league
                .as_ref()
                .expect("league should exist")
                .transfer_rumours
                .iter()
                .filter(|rumour| rumour.player_id == "ai-repeatable")
                .count(),
            1
        );
    }

    #[test]
    fn weekly_digest_prunes_stale_transfer_rumours_when_creating_new_ones() {
        let mut game = make_game("2025-09-08", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 9, 8);

        let mut notable_player = Player::new(
            "ai-fresh".to_string(),
            "Fresh".to_string(),
            "Fresh Prospect".to_string(),
            "1999-04-10".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        notable_player.team_id = Some("team2".to_string());
        notable_player.market_value = 1_450_000;
        game.players.push(notable_player);

        game.league
            .as_mut()
            .expect("league should exist")
            .transfer_rumours
            .push(TransferRumour {
                id: "rumour_old_2025-W31".to_string(),
                date: "2025-07-31T12:00:00+00:00".to_string(),
                player_id: "stale-player".to_string(),
                player_name: "Stale Player".to_string(),
                team_id: "team3".to_string(),
                team_name: "Gamma FC".to_string(),
            });

        generate_weekly_digest_news(&mut game, "2025-09-08");

        let rumours = &game
            .league
            .as_ref()
            .expect("league should exist")
            .transfer_rumours;
        assert!(
            rumours
                .iter()
                .all(|rumour| rumour.player_id != "stale-player")
        );
        assert!(rumours.iter().any(|rumour| rumour.player_id == "ai-fresh"));
    }

    #[test]
    fn weekly_digest_prunes_stale_transfer_rumours_even_without_new_candidates() {
        let mut game = make_game("2025-09-08", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 9, 8);
        game.players
            .retain(|player| player.team_id.as_deref() == Some("team1"));

        game.league
            .as_mut()
            .expect("league should exist")
            .transfer_rumours
            .push(TransferRumour {
                id: "rumour_old_2025-W31".to_string(),
                date: "2025-07-31T12:00:00+00:00".to_string(),
                player_id: "stale-player".to_string(),
                player_name: "Stale Player".to_string(),
                team_id: "team3".to_string(),
                team_name: "Gamma FC".to_string(),
            });

        generate_weekly_digest_news(&mut game, "2025-09-08");

        assert!(
            game.league
                .as_ref()
                .expect("league should exist")
                .transfer_rumours
                .is_empty()
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

    #[test]
    fn weekly_digest_includes_transfer_rumours_for_notable_ai_players() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        // Monday 2025-08-11
        set_current_date(&mut game, 2025, 8, 11);

        // Add a high-value player on an AI team (not user's team1)
        let mut notable_player = Player::new(
            "ai-notable".to_string(),
            "Notable".to_string(),
            "Notable Player".to_string(),
            "1997-06-01".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        notable_player.team_id = Some("team2".to_string());
        notable_player.market_value = 1_500_000;
        game.players.push(notable_player);

        // Mark fixtures as completed so season is started
        if let Some(league) = &mut game.league {
            for f in &mut league.fixtures {
                f.status = FixtureStatus::Completed;
            }
        }

        generate_weekly_digest_news(&mut game, "2025-08-11");

        let rumours: Vec<_> = game
            .news
            .iter()
            .filter(|a| a.category == NewsCategory::TransferRumour)
            .collect();
        // We should have at least one rumour for the notable AI player
        assert!(!rumours.is_empty(), "expected transfer rumour articles");
        let rumour = rumours[0];
        assert!(rumour.id.starts_with("rumour_ai-notable_"));
        assert_eq!(rumour.player_ids, vec!["ai-notable".to_string()]);

        let stored_rumours = &game
            .league
            .as_ref()
            .expect("league should exist")
            .transfer_rumours;
        assert_eq!(stored_rumours.len(), 1);
        assert_eq!(stored_rumours[0].player_id, "ai-notable");
        assert_eq!(stored_rumours[0].team_id, "team2");
    }

    #[test]
    fn weekly_digest_does_not_generate_rumours_for_user_team_players() {
        let mut game = make_game("2025-08-11", FixtureStatus::Completed);
        set_current_date(&mut game, 2025, 8, 11);

        // High-value player on user's team
        let mut user_player = Player::new(
            "user-notable".to_string(),
            "UserStar".to_string(),
            "User Star".to_string(),
            "1997-06-01".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        user_player.team_id = Some("team1".to_string()); // team1 is the user's team
        user_player.market_value = 2_000_000;
        game.players.push(user_player);

        if let Some(league) = &mut game.league {
            for f in &mut league.fixtures {
                f.status = FixtureStatus::Completed;
            }
        }

        generate_weekly_digest_news(&mut game, "2025-08-11");

        // No rumours should be about user-notable (user's own player)
        let rumours_about_user_player: Vec<_> = game
            .news
            .iter()
            .filter(|a| {
                a.category == NewsCategory::TransferRumour
                    && a.player_ids.iter().any(|pid| pid == "user-notable")
            })
            .collect();
        assert!(
            rumours_about_user_player.is_empty(),
            "should not generate rumours about user's own players"
        );
    }

    #[test]
    fn preseason_digest_includes_transfer_rumours_for_notable_ai_players() {
        let mut game = make_game("2025-08-11", FixtureStatus::Scheduled);
        set_current_date(&mut game, 2025, 8, 11);
        reset_to_preseason(&mut game);

        let mut notable_player = Player::new(
            "ai-preseason-notable".to_string(),
            "Preseason".to_string(),
            "Preseason Notable".to_string(),
            "1997-06-01".to_string(),
            "England".to_string(),
            Position::Forward,
            default_attrs(),
        );
        notable_player.team_id = Some("team2".to_string());
        notable_player.market_value = 1_500_000;
        game.players.push(notable_player);

        generate_weekly_digest_news(&mut game, "2025-08-11");

        let rumours: Vec<_> = game
            .news
            .iter()
            .filter(|article| article.category == NewsCategory::TransferRumour)
            .collect();
        assert!(
            !rumours.is_empty(),
            "expected preseason transfer rumour articles"
        );
        assert!(
            rumours
                .iter()
                .any(|article| article.id.starts_with("rumour_ai-preseason-notable_"))
        );
    }
}
