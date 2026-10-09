//! MCP tool implementations: info

use mcp_results::info::{
    Attribute, DetailedFinancialSnapshot, FinanceFigures, FinancialOverview, FixtureRow, Fixtures,
    GameState, GameStatus, GameSummary, LeagueSnapshot, MatchPreview, NewsItem, NextMatch,
    PlayerAppearance, PlayerDetail, PlayerMatchHistory, PlayerProfile, PlayerStats, RecentNews,
    SeasonContext, StandingRow, Standings, TeamFinances, TeamLeagueFigures, TeamMatchHistory,
    TeamProfile, TeamResult, TeamStats,
};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{
    age_from_dob, format_position, require_game, require_league, serde_label, user_team,
};
use domain::league::{FixtureStatus, League, StandingEntry};
use std::sync::Arc;

fn goal_difference(standing: &StandingEntry) -> i64 {
    i64::from(standing.goals_for) - i64::from(standing.goals_against)
}

/// The table as the agent sees it: points, then goals scored.
fn ranked(league: &League) -> Vec<StandingEntry> {
    let mut standings = league.standings.clone();
    standings.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then_with(|| b.goals_for.cmp(&a.goals_for))
    });
    standings
}

fn result_letter(our_goals: u8, their_goals: u8) -> &'static str {
    match our_goals.cmp(&their_goals) {
        std::cmp::Ordering::Greater => "W",
        std::cmp::Ordering::Less => "L",
        std::cmp::Ordering::Equal => "D",
    }
}

/// `team_id`'s completed fixtures, as (goals for, goals against), oldest first.
fn completed_results<'a>(
    league: &'a League,
    team_id: &'a str,
) -> impl Iterator<Item = (u8, u8)> + 'a {
    league
        .fixtures
        .iter()
        .filter(|f| f.status == FixtureStatus::Completed)
        .filter(move |f| f.home_team_id == team_id || f.away_team_id == team_id)
        .filter_map(move |f| {
            let result = f.result.as_ref()?;
            Some(if f.home_team_id == team_id {
                (result.home_goals, result.away_goals)
            } else {
                (result.away_goals, result.home_goals)
            })
        })
}

fn team_name_of(game: &ofm_core::game::Game, team_id: &str) -> String {
    game.teams
        .iter()
        .find(|t| t.id == team_id)
        .map(|t| t.name.clone())
        .unwrap_or_default()
}

fn player_name_of(game: &ofm_core::game::Game, player_id: String) -> String {
    game.players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or(player_id)
}

fn finance_figures(snapshot: &ofm_core::finances::TeamFinanceSnapshot) -> FinanceFigures {
    FinanceFigures {
        weekly_wage_spend: snapshot.weekly_wage_spend,
        weekly_wage_budget: snapshot.weekly_wage_budget,
        weekly_recurring_income: snapshot.weekly_recurring_income,
        weekly_sponsor_income: snapshot.weekly_sponsor_income,
        projected_weekly_net: snapshot.projected_weekly_net,
        wage_budget_usage_percent: snapshot.wage_budget_usage_percent,
        cash_runway_weeks: snapshot.cash_runway_weeks,
        in_debt: snapshot.currently_in_debt,
        over_budget: snapshot.currently_over_budget,
        wage_budget_status: serde_label(&snapshot.wage_budget_status),
        runway_status: serde_label(&snapshot.runway_status),
        overall_status: serde_label(&snapshot.overall_status),
    }
}

// ─── info_game_state ────────────────────────────────────────────────────────

/// Return the full game state as JSON. This gives agents access to the raw
/// structured data rather than formatted text. It is available in competition mode
/// by maintainer decision. Only game_new, game_select_team, game_export_world,
/// game_exit and game_load_save are restricted by that mode.
pub fn info_game_state(ctx: Arc<McpContext>) -> Result<GameState, String> {
    let game = require_game(&ctx.state_manager)?;
    let game = serde_json::to_value(&game)
        .map_err(|e| format!("Failed to serialize game state: {}", e))?;
    Ok(GameState { game })
}

// ─── info_game_summary ──────────────────────────────────────────────────────

pub fn info_game_summary(ctx: Arc<McpContext>) -> Result<GameSummary, String> {
    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;
    let team_id = team.id.as_str();

    let league = game.league.as_ref().map(|league| {
        let standings = ranked(league);
        let standing = standings.iter().find(|s| s.team_id == team_id);
        // Newest five results, shown oldest first.
        let mut recent: Vec<&str> = completed_results(league, team_id)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(5)
            .map(|(ours, theirs)| result_letter(ours, theirs))
            .collect();
        recent.reverse();
        LeagueSnapshot {
            position: standings
                .iter()
                .position(|s| s.team_id == team_id)
                .map_or(0, |i| i + 1),
            points: standing.map_or(0, |s| s.points),
            goal_difference: standing.map_or(0, goal_difference),
            form: recent.join("-"),
        }
    });

    let squad: Vec<&domain::player::Player> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .collect();
    let average = |value: fn(&domain::player::Player) -> u8| {
        if squad.is_empty() {
            0.0
        } else {
            squad.iter().map(|p| f64::from(value(p))).sum::<f64>() / squad.len() as f64
        }
    };

    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let next_match = game.league.as_ref().and_then(|league| {
        league
            .fixtures
            .iter()
            .filter(|f| {
                f.date >= today
                    && f.status == FixtureStatus::Scheduled
                    && (f.home_team_id == team_id || f.away_team_id == team_id)
            })
            .min_by_key(|f| f.date.clone())
            .map(|f| {
                let at_home = f.home_team_id == team_id;
                let opponent_id = if at_home {
                    &f.away_team_id
                } else {
                    &f.home_team_id
                };
                NextMatch {
                    opponent: team_name_of(&game, opponent_id),
                    at_home,
                    date: f.date.clone(),
                }
            })
    });

    let window = &game.season_context.transfer_window;
    let transfer_window = match window.status {
        domain::season::TransferWindowStatus::Open => {
            format!(
                "Open ({} days remaining)",
                window.days_remaining.unwrap_or(0)
            )
        }
        domain::season::TransferWindowStatus::Closed => "Closed".to_string(),
        _ => "Unknown".to_string(),
    };

    Ok(GameSummary {
        date: game.clock.current_date.format("%d %B %Y").to_string(),
        manager_first_name: game.manager.first_name.clone(),
        manager_last_name: game.manager.last_name.clone(),
        team_name: team.name.clone(),
        season_phase: format!("{:?}", game.season_context.phase),
        transfer_window,
        league,
        balance: team.finance,
        wage_budget: team.wage_budget,
        transfer_budget: team.transfer_budget,
        avg_condition: average(|p| p.condition),
        avg_ovr: average(|p| p.ovr),
        injured: squad.iter().filter(|p| p.injury.is_some()).count(),
        squad_size: squad.len(),
        next_match,
        unread_messages: game.messages.iter().filter(|m| !m.read).count(),
    })
}

// ─── info_standings ─────────────────────────────────────────────────────────

pub fn info_standings(ctx: Arc<McpContext>) -> Result<Standings, String> {
    let game = require_game(&ctx.state_manager)?;
    let league = require_league(&game)?;

    let team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or("be.error.noTeamAssigned")?;

    let rows = ranked(league)
        .iter()
        .enumerate()
        .map(|(i, s)| StandingRow {
            position: i + 1,
            team: game
                .teams
                .iter()
                .find(|t| t.id == s.team_id)
                .map_or_else(|| s.team_id.clone(), |t| t.name.clone()),
            is_your_team: s.team_id == team_id,
            played: s.played,
            won: s.won,
            drawn: s.drawn,
            lost: s.lost,
            goal_difference: goal_difference(s),
            points: s.points,
        })
        .collect();

    Ok(Standings {
        competition: league.name.clone(),
        season: league.season,
        rows,
    })
}

// ─── game_is_finished ──────────────────────────────────────────────────────

pub fn game_is_finished(ctx: Arc<McpContext>) -> Result<GameStatus, String> {
    let game = require_game(&ctx.state_manager)?;

    // Game is "finished" if manager has no team (was fired)
    if game.manager.team_id.is_none() {
        return Ok(GameStatus::ManagerFired {});
    }

    // Or if the season is complete and all fixtures are played
    let Some(league) = &game.league else {
        return Ok(GameStatus::NoLeagueYet {});
    };
    let remaining_fixtures = league
        .fixtures
        .iter()
        .filter(|f| f.status != FixtureStatus::Completed)
        .count();
    if remaining_fixtures == 0 && !league.fixtures.is_empty() {
        Ok(GameStatus::AllFixturesCompleted {})
    } else {
        Ok(GameStatus::InProgress { remaining_fixtures })
    }
}

// ─── info_fixtures ─────────────────────────────────────────────────────────

pub fn info_fixtures(ctx: Arc<McpContext>) -> Result<Fixtures, String> {
    let game = require_game(&ctx.state_manager)?;
    let league = require_league(&game)?;
    let team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or("be.error.noTeamAssigned")?;
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    let mut upcoming = Vec::new();
    let mut past = Vec::new();
    for f in &league.fixtures {
        if f.home_team_id != team_id && f.away_team_id != team_id {
            continue;
        }
        let completed = f.status == FixtureStatus::Completed;
        let row = FixtureRow {
            date: f.date.clone(),
            score: f
                .result
                .as_ref()
                .filter(|_| completed)
                .map(|r| format!("{} - {}", r.home_goals, r.away_goals)),
            matchup: format!(
                "{} vs {}",
                team_name_of(&game, &f.home_team_id),
                team_name_of(&game, &f.away_team_id)
            ),
            matchday: f.matchday,
        };
        if completed {
            past.push(row);
        } else if f.date >= today {
            upcoming.push(row);
        }
    }

    Ok(Fixtures {
        upcoming,
        recent: past.into_iter().rev().take(5).collect(),
    })
}

// ─── info_player_profile ────────────────────────────────────────────────────

pub fn info_player_profile(
    ctx: Arc<McpContext>,
    player_id: String,
) -> Result<PlayerProfile, String> {
    let game = require_game(&ctx.state_manager)?;
    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or_else(|| format!("Player {} not found", player_id))?;

    let team = player
        .team_id
        .as_deref()
        .and_then(|tid| game.teams.iter().find(|t| t.id == tid))
        .map_or_else(|| "Free Agent".to_string(), |t| t.name.clone());

    let detail = if game.manager.team_id.as_deref() == player.team_id.as_deref() {
        let a = &player.attributes;
        let attribute = |name: &str, value: u8| Attribute {
            name: name.to_string(),
            value,
        };
        PlayerDetail::Own {
            ovr: player.ovr,
            condition: player.condition,
            morale: player.morale,
            fitness: player.fitness,
            wage: player.wage(),
            contract_end: player.contract_end().map(str::to_string),
            injury: player
                .injury
                .as_ref()
                .map(|injury| serde_json::to_string(injury).unwrap_or_default()),
            attributes: vec![
                attribute("Pace", a.pace),
                attribute("Shooting", a.shooting),
                attribute("Passing", a.passing),
                attribute("Dribbling", a.dribbling),
                attribute("Defending", a.defending),
                attribute("Tackling", a.tackling),
                attribute("Strength", a.strength),
                attribute("Stamina", a.stamina),
                attribute("Agility", a.agility),
                attribute("Vision", a.vision),
                attribute("Decisions", a.decisions),
                attribute("Composure", a.composure),
                attribute("Positioning", a.positioning),
                attribute("Aggression", a.aggression),
                attribute("Teamwork", a.teamwork),
                attribute("Leadership", a.leadership),
                attribute("Handling", a.handling),
                attribute("Reflexes", a.reflexes),
                attribute("Aerial", a.aerial),
            ],
        }
    } else {
        // Limited detail for other teams' players (competition mode)
        PlayerDetail::Other {
            ovr: player.ovr,
            form: player.condition,
        }
    };

    Ok(PlayerProfile {
        id: player.id.clone(),
        match_name: player.match_name.clone(),
        full_name: player.full_name.clone(),
        position: format_position(&player.position).to_string(),
        age: age_from_dob(&player.date_of_birth, &game),
        nationality: player.nationality.clone(),
        team,
        detail,
    })
}

// ─── info_finances ──────────────────────────────────────────────────────────

pub fn info_finances(ctx: Arc<McpContext>) -> Result<FinancialOverview, String> {
    let response =
        crate::commands::finances::get_finance_snapshot_internal(&ctx.state_manager, None)?;
    Ok(FinancialOverview {
        figures: finance_figures(&response.snapshot),
    })
}

// ─── info_season_context ────────────────────────────────────────────────────

pub fn info_season_context(ctx: Arc<McpContext>) -> Result<SeasonContext, String> {
    let game = require_game(&ctx.state_manager)?;

    Ok(SeasonContext {
        phase: format!("{:?}", game.season_context.phase),
        transfer_window_open: game.season_context.transfer_window.status
            == domain::season::TransferWindowStatus::Open,
        season: game.league.as_ref().map_or(0, |l| l.season),
    })
}

// ─── info_news ──────────────────────────────────────────────────────────────

pub fn info_news(ctx: Arc<McpContext>) -> Result<RecentNews, String> {
    let game = require_game(&ctx.state_manager)?;
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    Ok(recent_news(&game.news, &today))
}

/// The ten most recent articles, hiding future-dated ones (e.g. a World Cup
/// kickoff dated at kickoff) so they don't show here every day before they
/// happen — the same rule the feed applies.
fn recent_news(news: &[domain::news::NewsArticle], today: &str) -> RecentNews {
    RecentNews {
        articles: news
            .iter()
            .filter(|n| ofm_core::slices::news::article_is_visible(&n.date, today))
            .take(10)
            .map(|n| NewsItem {
                headline: n.headline.clone(),
                date: n.date.clone(),
            })
            .collect(),
    }
}

// ─── info_match_preview ─────────────────────────────────────────────────────

pub fn info_match_preview(ctx: Arc<McpContext>) -> Result<MatchPreview, String> {
    let game = require_game(&ctx.state_manager)?;
    let league = require_league(&game)?;
    let team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or("be.error.noTeamAssigned")?;

    let next_fixture = league
        .fixtures
        .iter()
        .filter(|f| f.status != FixtureStatus::Completed)
        .filter(|f| f.home_team_id == team_id || f.away_team_id == team_id)
        .min_by_key(|f| &f.date);

    let Some(fixture) = next_fixture else {
        return Ok(MatchPreview::NoUpcomingFixtures {});
    };

    let at_home = fixture.home_team_id == team_id;
    let opponent_id = if at_home {
        &fixture.away_team_id
    } else {
        &fixture.home_team_id
    };

    let results: Vec<(u8, u8)> = completed_results(league, opponent_id).collect();
    let opponent_form = results
        .iter()
        .rev()
        .take(5)
        .map(|(ours, theirs)| result_letter(*ours, *theirs))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(MatchPreview::Upcoming {
        opponent: team_name_of(&game, opponent_id),
        at_home,
        date: fixture.date.clone(),
        matchday: fixture.matchday,
        opponent_position: ranked(league)
            .iter()
            .position(|st| st.team_id == *opponent_id)
            .map_or(0, |p| p + 1),
        opponent_form,
    })
}

// ─── info_player_stats ──────────────────────────────────────────────────────

pub fn info_player_stats(ctx: Arc<McpContext>, player_id: String) -> Result<PlayerStats, String> {
    let response =
        crate::commands::stats::get_player_stats_overview_internal(&ctx.state_manager, &player_id)?;

    let game = require_game(&ctx.state_manager)?;
    Ok(PlayerStats {
        player_name: player_name_of(&game, player_id),
        stats: serde_json::to_value(&response).map_err(|e| e.to_string())?,
    })
}

// ─── info_player_match_history ──────────────────────────────────────────────

pub fn info_player_match_history(
    ctx: Arc<McpContext>,
    player_id: String,
    limit: Option<usize>,
) -> Result<PlayerMatchHistory, String> {
    let response = crate::commands::stats::get_player_match_history_internal(
        &ctx.state_manager,
        &player_id,
        limit,
    )?;

    let game = require_game(&ctx.state_manager)?;
    Ok(PlayerMatchHistory {
        player_name: player_name_of(&game, player_id),
        matches: response
            .into_iter()
            .map(|entry| PlayerAppearance {
                date: entry.date,
                opponent: entry.opponent_name,
                minutes_played: entry.minutes_played,
                goals: entry.goals,
                assists: entry.assists,
            })
            .collect(),
    })
}

// ─── info_team_profile ──────────────────────────────────────────────────────

pub fn info_team_profile(ctx: Arc<McpContext>, team_id: String) -> Result<TeamProfile, String> {
    let game = require_game(&ctx.state_manager)?;
    let team = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or_else(|| format!("Team {} not found", team_id))?;

    let squad_size = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id.as_str()))
        .count();

    let league = game.league.as_ref().and_then(|league| {
        let standings = ranked(league);
        let position = standings.iter().position(|st| st.team_id == team_id)?;
        let s = &standings[position];
        Some(TeamLeagueFigures {
            position: position + 1,
            points: s.points,
            won: s.won,
            drawn: s.drawn,
            lost: s.lost,
            goal_difference: goal_difference(s),
        })
    });

    let recent_form = game.league.as_ref().and_then(|league| {
        let results: Vec<(u8, u8)> = completed_results(league, &team_id).collect();
        let form = results
            .iter()
            .rev()
            .take(5)
            .map(|(ours, theirs)| result_letter(*ours, *theirs))
            .collect::<Vec<_>>()
            .join(" ");
        (!form.is_empty()).then_some(form)
    });

    let finances = (game.manager.team_id.as_deref() == Some(team_id.as_str()))
        .then(|| {
            crate::commands::finances::get_finance_snapshot_internal(
                &ctx.state_manager,
                Some(team_id.as_str()),
            )
            .ok()
        })
        .flatten()
        .map(|response| {
            let snap = &response.snapshot;
            TeamFinances {
                weekly_wage_spend: snap.weekly_wage_spend,
                weekly_wage_budget: snap.weekly_wage_budget,
                projected_weekly_net: snap.projected_weekly_net,
                in_debt: snap.currently_in_debt,
                overall_status: serde_label(&snap.overall_status),
            }
        });

    Ok(TeamProfile {
        id: team.id.clone(),
        name: team.name.clone(),
        formation: team.formation.clone(),
        play_style: serde_label(&team.play_style),
        squad_size,
        training_focus: serde_label(&team.training_focus),
        training_intensity: serde_label(&team.training_intensity),
        league,
        recent_form,
        finances,
    })
}

// ─── info_team_stats ────────────────────────────────────────────────────────

pub fn info_team_stats(ctx: Arc<McpContext>, team_id: String) -> Result<TeamStats, String> {
    let response =
        crate::commands::stats::get_team_stats_overview_internal(&ctx.state_manager, &team_id)?;

    let game = require_game(&ctx.state_manager)?;
    let team_name = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .map(|t| t.name.clone())
        .unwrap_or(team_id);

    Ok(TeamStats {
        team_name,
        stats: response
            .map(|stats| serde_json::to_value(&stats))
            .transpose()
            .map_err(|e| e.to_string())?,
    })
}

// ─── info_team_match_history ────────────────────────────────────────────────

pub fn info_team_match_history(
    ctx: Arc<McpContext>,
    team_id: String,
    limit: Option<usize>,
) -> Result<TeamMatchHistory, String> {
    let response = crate::commands::stats::get_team_match_history_internal(
        &ctx.state_manager,
        &team_id,
        limit,
    )?;

    let game = require_game(&ctx.state_manager)?;
    let team_name = game
        .teams
        .iter()
        .find(|t| t.id == team_id)
        .map(|t| t.name.clone())
        .unwrap_or(team_id);

    Ok(TeamMatchHistory {
        team_name,
        matches: response
            .into_iter()
            .map(|entry| TeamResult {
                date: entry.date,
                opponent: entry.opponent_name,
                goals_for: entry.goals_for,
                goals_against: entry.goals_against,
            })
            .collect(),
    })
}

// ─── info_finance_snapshot ──────────────────────────────────────────────────

pub fn info_finance_snapshot(
    ctx: Arc<McpContext>,
    team_id: Option<String>,
) -> Result<DetailedFinancialSnapshot, String> {
    let response = crate::commands::finances::get_finance_snapshot_internal(
        &ctx.state_manager,
        team_id.as_deref(),
    )?;

    Ok(DetailedFinancialSnapshot {
        figures: finance_figures(&response.snapshot),
    })
}

#[cfg(test)]
mod info_news_tests {
    use super::recent_news;
    use domain::news::{NewsArticle, NewsCategory};

    fn article(id: &str, date: &str) -> NewsArticle {
        NewsArticle::new(
            id.to_string(),
            format!("{id} headline"),
            "Body".to_string(),
            "Source".to_string(),
            date.to_string(),
            NewsCategory::Editorial,
        )
    }

    #[test]
    fn recent_news_hides_future_dated_articles() {
        let news = vec![
            article("today", "2026-02-15"),
            article("kickoff", "2026-06-03"),
        ];

        let rendered = recent_news(&news, "2026-02-15").to_string();

        assert!(rendered.contains("today headline"));
        assert!(
            !rendered.contains("kickoff headline"),
            "a future-dated article must not show in the MCP news tool"
        );
    }

    #[test]
    fn recent_news_shows_same_day_rfc3339_articles() {
        let news = vec![article("digest", "2026-02-15T08:00:00+00:00")];

        let rendered = recent_news(&news, "2026-02-15").to_string();

        assert!(rendered.contains("digest headline"));
    }
}
