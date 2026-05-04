use crate::game::Game;
use crate::schedule::{append_fixtures, generate_league, generate_preseason_friendlies};
use crate::season_awards::compute_season_awards;
use chrono::Duration;
use domain::league::{FixtureStatus, League};
use domain::message::*;
use domain::player::PlayerSeasonStats;
use domain::team::{FinancialTransaction, FinancialTransactionKind, TeamSeasonRecord};

pub fn expected_fixture_count(team_count: usize) -> Option<usize> {
    if team_count >= 2 && team_count % 2 == 0 {
        Some(team_count * (team_count - 1))
    } else {
        None
    }
}

pub fn has_full_schedule(league: &League) -> bool {
    match expected_fixture_count(league.standings.len()) {
        Some(expected_fixture_count) => {
            league
                .fixtures
                .iter()
                .filter(|fixture| fixture.counts_for_league_standings())
                .count()
                == expected_fixture_count
        }
        None => false,
    }
}

/// Returns true if at least one competitive fixture has been completed or any
/// standing entry records a played match. Used as a guard to prevent premature
/// end-of-season processing for a season that has not yet kicked off.
pub fn season_has_started(league: &League) -> bool {
    league
        .fixtures
        .iter()
        .any(|f| f.counts_for_league_standings() && f.status == FixtureStatus::Completed)
        || league.standings.iter().any(|e| e.played > 0)
}

pub fn is_league_complete(league: &League) -> bool {
    season_has_started(league)
        && has_full_schedule(league)
        && league
            .fixtures
            .iter()
            .filter(|fixture| fixture.counts_for_league_standings())
            .all(|fixture| fixture.status == FixtureStatus::Completed)
}

/// Check if the season is complete (all fixtures played).
pub fn is_season_complete(game: &Game) -> bool {
    game.league.as_ref().is_some_and(is_league_complete)
}

const PRIZE_MONEY_BY_POSITION: [i64; 10] = [
    5_000_000, 3_000_000, 1_500_000, 750_000, 400_000, 300_000, 250_000, 200_000, 175_000, 150_000,
];

fn position_suffix(position: u32) -> &'static str {
    match position {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}

fn prize_money_for_position(position: u32) -> i64 {
    if position == 0 {
        return 0;
    }

    PRIZE_MONEY_BY_POSITION
        .get(position.saturating_sub(1) as usize)
        .copied()
        .unwrap_or(150_000)
}

/// Process end-of-season: record history, compute awards, reset stats, generate next season.
/// Returns a summary struct for the frontend to display.
pub fn process_end_of_season(game: &mut Game) -> EndOfSeasonSummary {
    let league = match &game.league {
        Some(l) => l,
        None => return EndOfSeasonSummary::default(),
    };

    let season = league.season;
    let league_name = league.name.clone();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    // Messages should be dated on the last match day, not on the clock date
    // (which may already be one day ahead due to process_day advancing the clock).
    let last_fixture_date = league
        .fixtures
        .iter()
        .filter(|f| f.counts_for_league_standings() && f.status == FixtureStatus::Completed)
        .map(|f| f.date.as_str())
        .max()
        .unwrap_or(today.as_str())
        .to_string();

    // 1. Compute final standings
    let final_standings = league.sorted_standings();

    // 2. Compute awards before resetting stats
    let awards = compute_season_awards(game);

    // 3. Build summary
    let user_team_id = game.manager.team_id.clone().unwrap_or_default();
    let user_position = final_standings
        .iter()
        .position(|s| s.team_id == user_team_id)
        .map(|i| i + 1)
        .unwrap_or(0) as u32;
    let user_standing = final_standings
        .iter()
        .find(|s| s.team_id == user_team_id)
        .cloned();

    let champion_id = final_standings
        .first()
        .map(|s| s.team_id.clone())
        .unwrap_or_default();
    let champion_name = game
        .teams
        .iter()
        .find(|t| t.id == champion_id)
        .map(|t| t.name.clone())
        .unwrap_or_default();

    let summary = EndOfSeasonSummary {
        season,
        league_name: league_name.clone(),
        champion_id: champion_id.clone(),
        champion_name,
        user_position,
        user_points: user_standing.as_ref().map(|s| s.points).unwrap_or(0),
        user_won: user_standing.as_ref().map(|s| s.won).unwrap_or(0),
        user_drawn: user_standing.as_ref().map(|s| s.drawn).unwrap_or(0),
        user_lost: user_standing.as_ref().map(|s| s.lost).unwrap_or(0),
        user_goals_for: user_standing.as_ref().map(|s| s.goals_for).unwrap_or(0),
        user_goals_against: user_standing.as_ref().map(|s| s.goals_against).unwrap_or(0),
        golden_boot_player: awards
            .golden_boot
            .first()
            .map(|e| e.player_name.clone())
            .unwrap_or_default(),
        golden_boot_goals: awards
            .golden_boot
            .first()
            .map(|e| e.value as u32)
            .unwrap_or(0),
        poty_player: awards
            .player_of_year
            .first()
            .map(|e| e.player_name.clone())
            .unwrap_or_default(),
        poty_rating: awards
            .player_of_year
            .first()
            .map(|e| e.value)
            .unwrap_or(0.0),
        total_teams: final_standings.len() as u32,
    };

    // 4. Record team season history
    for (idx, standing) in final_standings.iter().enumerate() {
        if let Some(team) = game.teams.iter_mut().find(|t| t.id == standing.team_id) {
            let position = (idx + 1) as u32;
            let prize_money = prize_money_for_position(position);

            team.history.push(TeamSeasonRecord {
                season,
                league_position: position,
                played: standing.played,
                won: standing.won,
                drawn: standing.drawn,
                lost: standing.lost,
                goals_for: standing.goals_for,
                goals_against: standing.goals_against,
            });
            // Reset form
            team.form.clear();

            if prize_money > 0 {
                team.finance += prize_money;
                team.season_income += prize_money;
                team.financial_ledger.push(FinancialTransaction {
                    date: last_fixture_date.clone(),
                    description: format!(
                        "Season {} prize money for {}{} place",
                        season,
                        position,
                        position_suffix(position)
                    ),
                    amount: prize_money,
                    kind: FinancialTransactionKind::PrizeMoney,
                });
            }
        }
    }

    crate::reputation::update_team_reputation(game, &final_standings);

    // 5. Record player career entries and reset stats
    for player in game.players.iter_mut() {
        if player.stats.appearances > 0 {
            let team_name = player
                .team_id
                .as_ref()
                .and_then(|tid| game.teams.iter().find(|t| &t.id == tid))
                .map(|t| t.name.clone())
                .unwrap_or_else(|| "Free Agent".to_string());
            let team_id = player.team_id.clone().unwrap_or_default();

            player.career.push(domain::player::CareerEntry {
                season,
                team_id,
                team_name,
                appearances: player.stats.appearances,
                goals: player.stats.goals,
                assists: player.stats.assists,
            });
        }
        // Reset stats for next season
        player.stats = PlayerSeasonStats::default();
    }

    // 6. Update manager career stats
    if let Some(standing) = &user_standing {
        let total_matches = standing.won + standing.drawn + standing.lost;
        game.manager.career_stats.matches_managed += total_matches;
        game.manager.career_stats.wins += standing.won;
        game.manager.career_stats.draws += standing.drawn;
        game.manager.career_stats.losses += standing.lost;
        if user_position == 1 {
            game.manager.career_stats.trophies += 1;
        }
        let best = game.manager.career_stats.best_finish;
        if best.is_none() || best.unwrap() > user_position {
            game.manager.career_stats.best_finish = Some(user_position);
        }
        // Update or create career history entry for current team
        let team_name = game
            .teams
            .iter()
            .find(|t| t.id == user_team_id)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        let today_str = game.clock.current_date.format("%Y-%m-%d").to_string();
        // Check if there's an existing open entry for this team
        let existing = game
            .manager
            .career_history
            .iter_mut()
            .find(|e| e.team_id == user_team_id && e.end_date.is_none());
        if let Some(entry) = existing {
            entry.matches += total_matches;
            entry.wins += standing.won;
            entry.draws += standing.drawn;
            entry.losses += standing.lost;
            let prev_best = entry.best_league_position;
            if prev_best.is_none() || prev_best.unwrap() > user_position {
                entry.best_league_position = Some(user_position);
            }
        } else {
            game.manager
                .career_history
                .push(domain::manager::ManagerCareerEntry {
                    team_id: user_team_id.clone(),
                    team_name,
                    start_date: today_str,
                    end_date: None,
                    matches: total_matches,
                    wins: standing.won,
                    draws: standing.drawn,
                    losses: standing.lost,
                    best_league_position: Some(user_position),
                });
        }
    }

    // 6b. Evaluate board objectives and adjust satisfaction
    let obj_delta = crate::board_objectives::evaluate_objectives(game);
    let new_sat = (game.manager.satisfaction as i16 + obj_delta as i16).clamp(0, 100) as u8;
    game.manager.satisfaction = new_sat;
    // Clear objectives for next season (will be regenerated on first process_day)
    game.board_objectives.clear();

    // 6c. Clear old news articles from the previous season
    game.news.clear();

    // 6d. Publish the season awards ceremony article (skipped when no marquee winners)
    if let Some(article) = crate::news::season_awards_article(&awards, season, &last_fixture_date) {
        game.news.push(article);
    }

    // 7. Generate next season schedule
    let next_season = season + 1;
    let team_ids: Vec<String> = game.teams.iter().map(|t| t.id.clone()).collect();
    // Start date: roughly a year after current start, or a few weeks from now
    let next_start = game.clock.current_date + Duration::days(28); // 4 weeks break
    let mut new_league = generate_league(&league_name, next_season, &team_ids, next_start);
    if !user_team_id.is_empty() {
        let opponents: Vec<String> = team_ids
            .iter()
            .filter(|team_id| team_id.as_str() != user_team_id)
            .cloned()
            .collect();
        let friendlies = generate_preseason_friendlies(&user_team_id, &opponents, next_start, 3);
        append_fixtures(&mut new_league, friendlies);
    }
    game.league = Some(new_league);

    let preview_date = game.clock.current_date.to_rfc3339();
    let team_names: Vec<String> = game.teams.iter().map(|team| team.name.clone()).collect();
    game.news.push(crate::news::season_preview_article(
        &team_names,
        &preview_date,
    ));

    // 8. Send end-of-season messages
    let pos_suffix = position_suffix(user_position);

    let user_team_name = game
        .teams
        .iter()
        .find(|t| t.id == user_team_id)
        .map(|t| t.name.clone())
        .unwrap_or_default();

    let board_msg = if user_position == 1 {
        format!(
            "Congratulations! {} are league champions! What an incredible achievement.\n\n\
            The board is absolutely delighted with your performance. You finished on {} points.\n\n\
            We look forward to defending the title next season.",
            user_team_name, summary.user_points
        )
    } else if user_position <= 4 {
        format!(
            "A solid season for {}. You finished in {}{} place with {} points.\n\n\
            The board is satisfied with the campaign. Let's push for the title next season.",
            user_team_name, user_position, pos_suffix, summary.user_points
        )
    } else if user_position <= summary.total_teams / 2 {
        format!(
            "{} finished the season in {}{} place with {} points.\n\n\
            A mid-table finish. The board expects improvement next season. \
            We need to be more competitive.",
            user_team_name, user_position, pos_suffix, summary.user_points
        )
    } else {
        format!(
            "A disappointing season for {}. Finishing {}{} with only {} points is below expectations.\n\n\
            The board is concerned. Significant improvement will be needed next season, \
            or your position may come under review.",
            user_team_name, user_position, pos_suffix, summary.user_points
        )
    };

    let existing_ids: std::collections::HashSet<String> =
        game.messages.iter().map(|m| m.id.clone()).collect();

    let payout_msg_id = format!("season_payout_{}", season);
    let user_prize_money = prize_money_for_position(user_position);
    if user_prize_money > 0 && !existing_ids.contains(&payout_msg_id) {
        let payout_message = InboxMessage::new(
            payout_msg_id,
            format!("Season {} Prize Money Awarded", season),
            format!(
                "The board has confirmed a prize payout of €{} for your {}{}-place league finish. The amount has been added to the club balance.",
                user_prize_money,
                user_position,
                pos_suffix
            ),
            "Board of Directors".to_string(),
            last_fixture_date.clone(),
        )
        .with_category(MessageCategory::Finance)
        .with_priority(MessagePriority::High)
        .with_sender_role("Chairman")
        .with_i18n("be.msg.seasonPayout.subject", "be.msg.seasonPayout.body", {
            let mut params = std::collections::HashMap::new();
            params.insert("season".to_string(), season.to_string());
            params.insert("amount".to_string(), user_prize_money.to_string());
            params.insert("position".to_string(), user_position.to_string());
            params
        })
        .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman");
        game.messages.push(payout_message);
    }

    let msg_id = format!("season_end_{}", season);
    if !existing_ids.contains(&msg_id) {
        let (body_key, mut i18n_params) = if user_position == 1 {
            let mut p = std::collections::HashMap::new();
            p.insert("team".to_string(), user_team_name.clone());
            p.insert("points".to_string(), summary.user_points.to_string());
            ("be.msg.seasonReview.body.champion", p)
        } else if user_position <= 4 {
            let mut p = std::collections::HashMap::new();
            p.insert("team".to_string(), user_team_name.clone());
            p.insert("position".to_string(), user_position.to_string());
            p.insert("suffix".to_string(), pos_suffix.to_string());
            p.insert("points".to_string(), summary.user_points.to_string());
            ("be.msg.seasonReview.body.topFour", p)
        } else if user_position <= summary.total_teams / 2 {
            let mut p = std::collections::HashMap::new();
            p.insert("team".to_string(), user_team_name.clone());
            p.insert("position".to_string(), user_position.to_string());
            p.insert("suffix".to_string(), pos_suffix.to_string());
            p.insert("points".to_string(), summary.user_points.to_string());
            ("be.msg.seasonReview.body.midTable", p)
        } else {
            let mut p = std::collections::HashMap::new();
            p.insert("team".to_string(), user_team_name.clone());
            p.insert("position".to_string(), user_position.to_string());
            p.insert("suffix".to_string(), pos_suffix.to_string());
            p.insert("points".to_string(), summary.user_points.to_string());
            ("be.msg.seasonReview.body.lowerHalf", p)
        };
        i18n_params.insert("season".to_string(), season.to_string());

        let msg = InboxMessage::new(
            msg_id,
            format!("Season {} Review", season),
            board_msg,
            "Board of Directors".to_string(),
            last_fixture_date.clone(),
        )
        .with_category(MessageCategory::BoardDirective)
        .with_priority(MessagePriority::High)
        .with_sender_role("Chairman")
        .with_i18n("be.msg.seasonReview.subject", body_key, i18n_params)
        .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman");
        game.messages.push(msg);
    }

    let sched_msg_id = format!("new_season_{}", next_season);
    if !existing_ids.contains(&sched_msg_id) {
        let mut sched_params = std::collections::HashMap::new();
        sched_params.insert("season".to_string(), next_season.to_string());
        let sched_msg = InboxMessage::new(
            sched_msg_id,
            format!("Season {} — New Schedule Released", next_season),
            format!(
                "The schedule for Season {} has been released! The new campaign kicks off in 4 weeks.\n\n\
                Use this break to assess your squad, make any necessary changes, and prepare for the challenges ahead.\n\n\
                Good luck!",
                next_season
            ),
            "League Office".to_string(),
            last_fixture_date,
        )
        .with_category(MessageCategory::LeagueInfo)
        .with_priority(MessagePriority::Normal)
        .with_sender_role("Competition Secretary")
        .with_i18n(
            "be.msg.newSeasonSchedule.subject",
            "be.msg.newSeasonSchedule.body",
            sched_params,
        )
        .with_sender_i18n("be.sender.leagueOffice", "be.role.competitionSecretary");
        game.messages.push(sched_msg);
    }

    crate::season_context::refresh_game_context(game);

    summary
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct EndOfSeasonSummary {
    pub season: u32,
    pub league_name: String,
    pub champion_id: String,
    pub champion_name: String,
    pub user_position: u32,
    pub user_points: u32,
    pub user_won: u32,
    pub user_drawn: u32,
    pub user_lost: u32,
    pub user_goals_for: u32,
    pub user_goals_against: u32,
    pub golden_boot_player: String,
    pub golden_boot_goals: u32,
    pub poty_player: String,
    pub poty_rating: f64,
    pub total_teams: u32,
}
