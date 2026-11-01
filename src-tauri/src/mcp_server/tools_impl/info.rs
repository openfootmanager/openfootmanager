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
use domain::league::{FixtureStatus, League};
use domain::player::PlayerAttributes;
use ofm_core::game::Game;
use ofm_core::player_knowledge::{player_knowledge, PlayerKnowledge};
use ofm_core::slices::scouted_report::query_scouted_report;
use ofm_core::state::StateManager;
use std::sync::Arc;

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

/// Return the full game state as JSON, for agents that want the raw structured data.
/// It is not available in competition mode (`McpMode::disabled_tools`): the dump holds every
/// club's players and finances, and competition agents follow the same knowledge rule as players.
/// A projection of it through `ofm_core::player_knowledge` for sandbox mode is the next change.
pub fn info_game_state(ctx: Arc<McpContext>) -> Result<GameState, String> {
    full_game_state(&ctx.state_manager)
}

/// The same dump for the end-to-end harness, which compares live state with the saved database and
/// checks that a refusal changed nothing. Registered only with `--dev-full-state`, never in
/// competition mode (`McpConfig::hidden_tools`, `parse_mcp_config_from_iter`).
pub fn dev_full_game_state(ctx: Arc<McpContext>) -> Result<GameState, String> {
    full_game_state(&ctx.state_manager)
}

fn full_game_state(state: &StateManager) -> Result<GameState, String> {
    let game = require_game(state)?;
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
        let standings = league.sorted_standings();
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
            goal_difference: standing.map_or(0, |s| i64::from(s.goal_difference())),
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

    let rows = league
        .sorted_standings()
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
            goal_difference: i64::from(s.goal_difference()),
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
        let row = fixture_row(&game, f);
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

fn fixture_row(game: &ofm_core::game::Game, f: &domain::league::Fixture) -> FixtureRow {
    FixtureRow {
        date: f.date.clone(),
        score: f
            .result
            .as_ref()
            .filter(|_| f.status == FixtureStatus::Completed)
            .map(|r| format!("{} - {}", r.home_goals, r.away_goals)),
        matchup: format!(
            "{} vs {}",
            team_name_of(game, &f.home_team_id),
            team_name_of(game, &f.away_team_id)
        ),
        matchday: f.matchday,
        rescheduled_from: f
            .postponement
            .as_ref()
            .map(|postponement| postponement.original_date.clone()),
    }
}

#[cfg(test)]
mod fixture_row_tests {
    use super::*;
    use chrono::TimeZone;
    use domain::league::{Fixture, Postponement, RescheduleReason};

    fn game() -> ofm_core::game::Game {
        let clock = ofm_core::clock::GameClock::new(
            chrono::Utc.with_ymd_and_hms(2033, 3, 1, 12, 0, 0).unwrap(),
        );
        let manager = domain::manager::Manager::new(
            "mgr".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        ofm_core::game::Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    /// Given a moved fixture, when the MCP fixtures row is built, then it carries the original date.
    #[test]
    fn a_moved_fixture_row_names_its_original_date() {
        let moved = Fixture {
            postponement: Some(Postponement {
                original_date: "2033-03-01".into(),
                reason: RescheduleReason::CupTie,
            }),
            date: "2033-03-08".into(),
            ..Default::default()
        };
        assert_eq!(
            fixture_row(&game(), &moved).rescheduled_from.as_deref(),
            Some("2033-03-01")
        );
        assert_eq!(
            fixture_row(&game(), &Fixture::default()).rescheduled_from,
            None
        );
    }
}

// ─── info_player_profile ────────────────────────────────────────────────────

pub fn info_player_profile(
    ctx: Arc<McpContext>,
    player_id: String,
) -> Result<PlayerProfile, String> {
    let game = require_game(&ctx.state_manager)?;
    player_profile_in(&game, &player_id)
}

/// The 19 attributes in the order the profile lists them.
fn attribute_rows(a: &PlayerAttributes) -> Vec<Attribute> {
    let attribute = |name: &str, value: u8| Attribute {
        name: name.to_string(),
        value,
    };
    vec![
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
    ]
}

/// The attributes a scout's report holds for `player_id`: a snapshot, never the live player's.
fn scouted_attributes(game: &Game, player_id: &str) -> Vec<Attribute> {
    query_scouted_report(game, player_id)
        .map(|view| attribute_rows(&view.report.attributes))
        .unwrap_or_default()
}

fn player_profile_in(game: &Game, player_id: &str) -> Result<PlayerProfile, String> {
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

    let detail = match player_knowledge(game, player) {
        PlayerKnowledge::Own => PlayerDetail::Own {
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
            attributes: attribute_rows(&player.attributes),
        },
        PlayerKnowledge::Known => PlayerDetail::Known {
            ovr: player.ovr,
            attributes: attribute_rows(&player.attributes),
        },
        PlayerKnowledge::Scouted {
            scouted_on,
            current,
        } => PlayerDetail::Scouted {
            attributes: scouted_attributes(game, &player.id),
            scouted_on,
            current,
        },
        PlayerKnowledge::Unknown => PlayerDetail::Unknown,
    };

    Ok(PlayerProfile {
        id: player.id.clone(),
        match_name: player.match_name.clone(),
        full_name: player.full_name.clone(),
        position: format_position(&player.position).to_string(),
        age: age_from_dob(&player.date_of_birth, game),
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
        opponent_position: league
            .sorted_standings()
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
        let standings = league.sorted_standings();
        let position = standings.iter().position(|st| st.team_id == team_id)?;
        let s = &standings[position];
        Some(TeamLeagueFigures {
            position: position + 1,
            points: s.points,
            won: s.won,
            drawn: s.drawn,
            lost: s.lost,
            goal_difference: i64::from(s.goal_difference()),
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
    finance_snapshot_in(&ctx.state_manager, team_id.as_deref())
}

/// A club's money is its own business: only the manager's own club can be asked for, by leaving
/// `team_id` out or naming it. Another club's figures are not public (maintainer ruling, 28 Oct).
fn finance_snapshot_in(
    state: &StateManager,
    requested: Option<&str>,
) -> Result<DetailedFinancialSnapshot, String> {
    let own_club = state
        .get_game(|game| game.manager.team_id.clone())
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())?;
    if requested.is_some_and(|team| own_club.as_deref() != Some(team)) {
        return Err(
            "Other clubs' finances are not public: ask for your own club (leave team_id out)."
                .to_string(),
        );
    }
    let response = crate::commands::finances::get_finance_snapshot_internal(state, None)?;

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

#[cfg(test)]
mod player_profile_tests {
    use super::player_profile_in;
    use chrono::{TimeZone, Utc};
    use domain::league::League;
    use domain::manager::Manager;
    use domain::player::{Injury, Player, Position};
    use ofm_core::clock::GameClock;
    use ofm_core::game::{Game, ScoutedPlayer};
    use serde_json::Value;

    const MY_CLUB: &str = "mine";

    fn attributes(value: u8) -> domain::player::PlayerAttributes {
        domain::player::PlayerAttributes {
            pace: value,
            stamina: value,
            strength: value,
            agility: value,
            passing: value,
            shooting: value,
            tackling: value,
            dribbling: value,
            defending: value,
            positioning: value,
            vision: value,
            decisions: value,
            composure: value,
            aggression: value,
            teamwork: value,
            leadership: value,
            handling: value,
            reflexes: value,
            aerial: value,
        }
    }

    /// A 77-rated player (attributes 77) whose condition (31), morale (22), fitness (13) and wage
    /// differ from each other, so a value shown under the wrong name cannot pass as the right one.
    fn player(id: &str, team_id: Option<&str>) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            format!("{id} Full"),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            attributes(77),
        );
        player.team_id = team_id.map(str::to_string);
        player.ovr = 77;
        player.condition = 31;
        player.morale = 22;
        player.fitness = 13;
        player.stage_contract(None, Some("2028-06-30".to_string()), 4_200);
        player.injury = Some(Injury {
            name: "Hamstring".to_string(),
            days_remaining: 9,
        });
        player
    }

    /// My club and "rival" play in division one; "faraway" plays in division two.
    fn world(manager_club: Option<&str>, players: Vec<Player>) -> Game {
        let mut manager = Manager::new(
            "m1".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        );
        manager.team_id = manager_club.map(str::to_string);
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap());
        let mut game = Game::new(clock, manager, vec![], players, vec![], vec![]);
        let division = |id: &str, clubs: &[&str]| {
            let clubs: Vec<String> = clubs.iter().map(|club| club.to_string()).collect();
            League::new(id.to_string(), id.to_string(), 2026, &clubs)
        };
        game.competitions = vec![
            division("division-one", &[MY_CLUB, "rival"]),
            division("division-two", &["faraway", "other"]),
        ];
        game
    }

    fn detail_of(game: &Game, player_id: &str) -> Value {
        let profile = player_profile_in(game, player_id).expect("the player exists");
        serde_json::to_value(profile.detail).expect("serializes")
    }

    /// Every name the profile may not show for a player the club does not own.
    const OWN_ONLY: [&str; 6] = [
        "condition",
        "morale",
        "fitness",
        "wage",
        "contract_end",
        "injury",
    ];

    fn shows(detail: &Value, field: &str) -> bool {
        detail.get(field).is_some()
    }

    /// The profile lists all 19 attributes by name in the profile's order, each at `value`: a list
    /// that is empty, short, repeats a name, or carries someone else's numbers would not pass.
    fn shows_all_attributes_at(detail: &Value, value: u64) {
        const NAMES: [&str; 19] = [
            "Pace",
            "Shooting",
            "Passing",
            "Dribbling",
            "Defending",
            "Tackling",
            "Strength",
            "Stamina",
            "Agility",
            "Vision",
            "Decisions",
            "Composure",
            "Positioning",
            "Aggression",
            "Teamwork",
            "Leadership",
            "Handling",
            "Reflexes",
            "Aerial",
        ];
        let attributes = detail["attributes"].as_array().expect("an attributes list");
        assert_eq!(attributes.len(), 19, "{detail}");
        let names: Vec<&str> = attributes
            .iter()
            .map(|attribute| attribute["name"].as_str().expect("a name"))
            .collect();
        assert_eq!(names, NAMES, "{detail}");
        assert!(
            attributes
                .iter()
                .all(|attribute| attribute["value"] == value),
            "every attribute should be {value}: {detail}"
        );
    }

    /// Given a player at the manager's club
    /// When the profile is read
    /// Then everything shows: rating, condition, morale, fitness, injury and the attributes.
    #[test]
    fn the_clubs_own_player_shows_everything() {
        let game = world(Some(MY_CLUB), vec![player("p", Some(MY_CLUB))]);

        let detail = detail_of(&game, "p");

        assert_eq!(detail["visibility"], "own");
        for field in [
            "ovr",
            "condition",
            "morale",
            "fitness",
            "injury",
            "attributes",
        ] {
            assert!(shows(&detail, field), "{field} missing: {detail}");
        }
        assert_eq!(detail["condition"], 31);
        assert_eq!(detail["morale"], 22);
        assert_eq!(detail["fitness"], 13);
        assert!(
            detail["injury"]
                .as_str()
                .is_some_and(|injury| injury.contains("Hamstring")),
            "{detail}"
        );
        assert_eq!(detail["wage"], 4_200);
        assert_eq!(detail["contract_end"], "2028-06-30");
        shows_all_attributes_at(&detail, 77);
    }

    /// Given a player at a rival in the manager's own division
    /// When the profile is read
    /// Then the rating and attributes show, and condition, morale, fitness, wage and injury do not.
    #[test]
    fn a_rival_in_the_same_division_shows_rating_and_attributes_only() {
        let game = world(Some(MY_CLUB), vec![player("p", Some("rival"))]);

        let detail = detail_of(&game, "p");

        assert_eq!(detail["visibility"], "known");
        assert_eq!(detail["ovr"], 77);
        shows_all_attributes_at(&detail, 77);
        for field in OWN_ONLY {
            assert!(!shows(&detail, field), "{field} leaked: {detail}");
        }
    }

    /// Given a player in another division whom a scout reported on, as he was then
    /// When the profile is read
    /// Then the report's attributes and date show, not his live rating, condition or morale.
    #[test]
    fn a_scouted_player_shows_the_report_and_not_the_live_player() {
        let mut game = world(Some(MY_CLUB), vec![player("p", Some("faraway"))]);
        game.scouted_players.push(ScoutedPlayer {
            player_id: "p".to_string(),
            scouted_on: "2026-09-01".to_string(),
            attributes: attributes(55),
            potential: None,
            scout_name: None,
        });

        let detail = detail_of(&game, "p");

        assert_eq!(detail["visibility"], "scouted");
        assert_eq!(detail["scouted_on"], "2026-09-01");
        assert_eq!(detail["current"], true);
        shows_all_attributes_at(&detail, 55);
        assert!(!shows(&detail, "ovr"), "the live rating leaked: {detail}");
        for field in OWN_ONLY {
            assert!(!shows(&detail, field), "{field} leaked: {detail}");
        }
    }

    /// Given a player in another division whom nobody scouted
    /// When the profile is read
    /// Then only who he is shows: no rating, no attributes, nothing about his body or contract.
    #[test]
    fn an_unscouted_player_in_another_division_shows_identity_only() {
        let game = world(Some(MY_CLUB), vec![player("p", Some("faraway"))]);

        let profile = player_profile_in(&game, "p").expect("the player exists");
        let detail = serde_json::to_value(&profile.detail).unwrap();

        assert_eq!(profile.full_name, "p Full");
        assert_eq!(detail["visibility"], "unknown");
        for field in ["ovr", "attributes"].into_iter().chain(OWN_ONLY) {
            assert!(!shows(&detail, field), "{field} leaked: {detail}");
        }
    }

    /// Given a manager with no club, and a free agent nobody scouted
    /// When the free agent's profile is read
    /// Then he is unknown, not "the club's own" because both have no team.
    #[test]
    fn a_free_agent_is_not_the_players_of_a_manager_without_a_club() {
        let game = world(None, vec![player("p", None)]);

        let detail = detail_of(&game, "p");

        assert_eq!(detail["visibility"], "unknown");
        assert!(!shows(&detail, "condition"), "{detail}");
    }

    /// Given a player another club loaned to the manager's club
    /// When the profile is read
    /// Then he shows in full: he trains with the club every day.
    #[test]
    fn a_player_loaned_in_shows_everything() {
        let mut loanee = player("p", Some(MY_CLUB));
        loanee.active_loan = Some(domain::player::ActiveLoan {
            parent_team_id: "faraway".to_string(),
            loan_team_id: MY_CLUB.to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: "2027-06-30".to_string(),
            wage_contribution_pct: 100,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });
        let game = world(Some(MY_CLUB), vec![loanee]);

        assert_eq!(detail_of(&game, "p")["visibility"], "own");
    }
}

#[cfg(test)]
mod finance_snapshot_tests {
    use super::finance_snapshot_in;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::StateManager;

    fn team(id: &str, wage_budget: i64) -> Team {
        let mut team = Team::new(
            id.to_string(),
            format!("{id} FC"),
            "TMS".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            25_000,
        );
        team.wage_budget = wage_budget;
        team
    }

    /// The manager's club has a 120 000 wage budget, a rival 77 000, so the figures say whose they are.
    fn state(manager_club: Option<&str>) -> StateManager {
        let mut manager = Manager::new(
            "m1".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.team_id = manager_club.map(str::to_string);
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap());
        let game = Game::new(
            clock,
            manager,
            vec![team("mine", 120_000), team("rival", 77_000)],
            vec![],
            vec![],
            vec![],
        );
        let state = StateManager::new();
        state.set_game(game);
        state
    }

    /// Given a manager asking for no team, or for their own club by id
    /// When the finance snapshot is read
    /// Then it is their club's figures.
    #[test]
    fn the_managers_own_club_is_available_by_omission_or_by_id() {
        let state = state(Some("mine"));

        for requested in [None, Some("mine")] {
            let snapshot = finance_snapshot_in(&state, requested).expect("own club");
            assert_eq!(snapshot.figures.weekly_wage_budget, 120_000);
        }
    }

    /// Given a manager asking for another club's finances
    /// When the finance snapshot is read
    /// Then it is refused and no figure of that club leaves.
    #[test]
    fn another_clubs_finances_are_not_available() {
        let state = state(Some("mine"));

        let refused = finance_snapshot_in(&state, Some("rival"));

        assert!(refused.is_err(), "the rival's figures were returned");
    }

    /// Given a manager without a club
    /// When they ask for a club's finances by id
    /// Then it is refused: they have no club of their own to match it against.
    #[test]
    fn a_manager_without_a_club_cannot_read_any_clubs_finances() {
        let state = state(None);

        assert!(finance_snapshot_in(&state, Some("rival")).is_err());
    }
}
