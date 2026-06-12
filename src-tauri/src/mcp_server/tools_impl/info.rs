//! MCP tool implementations: info

use std::sync::Arc;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::{require_game, user_team, require_league, format_position, age_from_dob};
use crate::mcp_server::formatting::translate_error;

// ─── info_game_state ────────────────────────────────────────────────────────

/// Return the full game state as JSON. This gives agents access to the raw
/// structured data rather than formatted text. In Competition mode, this tool
/// is disabled because it exposes detailed info about all teams/players.
pub fn info_game_state(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    serde_json::to_string_pretty(&game)
        .map_err(|e| format!("Failed to serialize game state: {}", e))
}

// ─── info_game_summary ──────────────────────────────────────────────────────

pub fn info_game_summary(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    let date = game.clock.current_date.format("%d %B %Y").to_string();
    let team = user_team(&game)?;

    // League position
    let league_info = if let Some(league) = &game.league {
        let team_id = team.id.as_str();
        let mut standings = league.standings.clone();
        standings.sort_by(|a, b| b.points.cmp(&a.points).then_with(|| b.goals_for.cmp(&a.goals_for)));
        let position = standings
            .iter()
            .position(|s| s.team_id == team_id)
            .map(|i| i + 1)
            .unwrap_or(0);
        let standing = standings.iter().find(|s| s.team_id == team_id);
        let pts = standing.map(|s| s.points).unwrap_or(0);
        let gd = standing.map(|s| i64::from(s.goals_for) - i64::from(s.goals_against)).unwrap_or(0);

        // Recent form (from last 5 completed fixtures involving our team)
        let mut recent: Vec<String> = Vec::new();
        for fixture in league.fixtures.iter().rev() {
            if recent.len() >= 5 { break; }
            if fixture.status != domain::league::FixtureStatus::Completed { continue; }
            if fixture.home_team_id != team_id && fixture.away_team_id != team_id { continue; }
            if let Some(ref result) = fixture.result {
                let is_home = fixture.home_team_id == team_id;
                let our_goals = if is_home { result.home_goals } else { result.away_goals };
                let their_goals = if is_home { result.away_goals } else { result.home_goals };
                recent.push(if our_goals > their_goals {
                    "W".to_string()
                } else if our_goals < their_goals {
                    "L".to_string()
                } else {
                    "D".to_string()
                });
            }
        }
        recent.reverse();
        let form = recent.join("-");

        format!(
            "**League Position**: {}st | **Points**: {} | **GD**: {:+}\n**Form**: {}",
            position, pts, gd, form
        )
    } else {
        "**League**: No league yet (pre-season)".to_string()
    };

    // Finances
    let finance = team.finance;
    let wage_budget = team.wage_budget;
    let transfer_budget = team.transfer_budget;

    // Squad health
    let team_id = team.id.as_str();
    let squad_players: Vec<&domain::player::Player> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .collect();
    let avg_condition = if squad_players.is_empty() {
        0.0
    } else {
        squad_players.iter().map(|p| f64::from(p.condition)).sum::<f64>() / squad_players.len() as f64
    };
    let avg_ovr = if squad_players.is_empty() {
        0.0
    } else {
        squad_players.iter().map(|p| f64::from(p.ovr)).sum::<f64>() / squad_players.len() as f64
    };
    let injured = squad_players
        .iter()
        .filter(|p| p.injury.is_some())
        .count();

    // Next match
    let next_match = game.league.as_ref().and_then(|league| {
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        league.fixtures.iter()
            .filter(|f| {
                f.date >= today
                    && f.status == domain::league::FixtureStatus::Scheduled
                    && (f.home_team_id == team_id || f.away_team_id == team_id)
            })
            .min_by_key(|f| f.date.clone())
            .map(|f| {
                let opponent = if f.home_team_id == team_id {
                    format!("{} (H)", game.teams.iter().find(|t| t.id == f.away_team_id).map(|t| t.name.clone()).unwrap_or_default())
                } else {
                    format!("{} (A)", game.teams.iter().find(|t| t.id == f.home_team_id).map(|t| t.name.clone()).unwrap_or_default())
                };
                format!("vs {} — {}", opponent, f.date)
            })
    });

    // Unread messages
    let unread = game.messages.iter().filter(|m| !m.read).count();

    // Season context
    let phase = format!("{:?}", game.season_context.phase);
    let transfer_window = match &game.season_context.transfer_window.status {
        domain::season::TransferWindowStatus::Open => {
            format!("Open ({} days remaining)", game.season_context.transfer_window.days_remaining.unwrap_or(0))
        }
        domain::season::TransferWindowStatus::Closed => "Closed".to_string(),
        _ => "Unknown".to_string(),
    };

    Ok(format!(
        "## Game Summary — {date}\n\n\
         **Manager**: {mgr_first} {mgr_last} | **Team**: {team_name}\n\
         **Season Phase**: {phase} | **Transfer Window**: {tw}\n\n\
         ### Position & Form\n{league_info}\n\n\
         ### Finances\n\
         **Balance**: €{finance} | **Wage Budget**: €{wage_budget}/wk | **Transfer Budget**: €{transfer_budget}\n\n\
         ### Squad Health\n\
         **Avg Condition**: {avg_cond:.0}% | **Avg OVR**: {avg_ovr:.0} | **Injured**: {injured} | **Squad Size**: {squad_size}\n\n\
         ### Next Match\n{next}\n\n\
         ### Unread Messages: {unread}",
        date = date,
        mgr_first = game.manager.first_name,
        mgr_last = game.manager.last_name,
        team_name = team.name,
        phase = phase,
        tw = transfer_window,
        league_info = league_info,
        finance = finance,
        wage_budget = wage_budget,
        transfer_budget = transfer_budget,
        avg_cond = avg_condition,
        avg_ovr = avg_ovr,
        injured = injured,
        squad_size = squad_players.len(),
        next = next_match.unwrap_or_else(|| "No upcoming match".to_string()),
        unread = unread,
    ))
}

// ─── info_standings ─────────────────────────────────────────────────────────

// ─── info_standings ─────────────────────────────────────────────────────────

pub fn info_standings(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let league = require_league(&game)?;

    let team_id = game.manager.team_id.as_deref().ok_or("be.error.noTeamAssigned")?;

    let mut standings = league.standings.clone();
    standings.sort_by(|a, b| {
        b.points.cmp(&a.points)
            .then_with(|| b.goals_for.cmp(&a.goals_for))
    });

    let mut rows = String::new();
    for (i, s) in standings.iter().enumerate() {
        let team_name = game
            .teams
            .iter()
            .find(|t| t.id == s.team_id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| s.team_id.clone());
        let marker = if s.team_id == team_id { " ← YOU" } else { "" };
        let gd = i64::from(s.goals_for) - i64::from(s.goals_against);
        rows.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {:+} | {} |\n",
            i + 1,
            team_name,
            s.played,
            s.won,
            s.drawn,
            s.lost,
            gd,
            s.points,
        ));
        rows.push_str(marker);
        // Actually marker should be in the team name cell
    }

    // Redo with marker in team name
    let mut rows2 = String::new();
    for (i, s) in standings.iter().enumerate() {
        let team_name = game
            .teams
            .iter()
            .find(|t| t.id == s.team_id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| s.team_id.clone());
        let name_col = if s.team_id == team_id {
            format!("{} ←", team_name)
        } else {
            team_name
        };
        let gd = i64::from(s.goals_for) - i64::from(s.goals_against);
        rows2.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {:+} | {} |\n",
            i + 1,
            name_col,
            s.played,
            s.won,
            s.drawn,
            s.lost,
            gd,
            s.points,
        ));
    }

    Ok(format!(
        "## {} — Season {}\n\n| # | Team | P | W | D | L | GD | Pts |\n|---|------|---|---|---|---|-----|-----|\n{}",
        league.name,
        league.season,
        rows2,
    ))
}

// ─── game_is_finished ──────────────────────────────────────────────────────

// ─── game_is_finished ──────────────────────────────────────────────────────

pub fn game_is_finished(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    // Game is "finished" if manager has no team (was fired)
    if game.manager.team_id.is_none() {
        return Ok("## Game Status: Finished\n\n**Reason**: Manager was fired.".to_string());
    }

    // Or if the season is complete and all fixtures are played
    if let Some(league) = &game.league {
        let incomplete = league.fixtures.iter()
            .filter(|f| f.status != domain::league::FixtureStatus::Completed)
            .count();
        if incomplete == 0 && !league.fixtures.is_empty() {
            return Ok("## Game Status: Finished\n\n**Reason**: All fixtures completed.".to_string());
        }
        return Ok(format!("## Game Status: In Progress\n\n**Remaining fixtures**: {}", incomplete));
    }

    Ok("## Game Status: In Progress\n\nNo league active yet.".to_string())
}

// ─── info_fixtures ─────────────────────────────────────────────────────────

// ─── info_fixtures ─────────────────────────────────────────────────────────

pub fn info_fixtures(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let league = require_league(&game)?;

    let team_id = game.manager.team_id.as_deref().ok_or("be.error.noTeamAssigned")?;

    let mut upcoming = Vec::new();
    let mut past = Vec::new();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    for f in &league.fixtures {
        let involves_us = f.home_team_id == team_id || f.away_team_id == team_id;
        if !involves_us { continue; }

        let home_name = game.teams.iter().find(|t| t.id == f.home_team_id).map(|t| t.name.clone()).unwrap_or_default();
        let away_name = game.teams.iter().find(|t| t.id == f.away_team_id).map(|t| t.name.clone()).unwrap_or_default();

        let entry = if f.status == domain::league::FixtureStatus::Completed {
            if let Some(ref result) = f.result {
                format!("| {} | {} - {} | {} | MD{} |", f.date, result.home_goals, result.away_goals, format!("{} vs {}", home_name, away_name), f.matchday)
            } else {
                format!("| {} | - | {} | MD{} |", f.date, format!("{} vs {}", home_name, away_name), f.matchday)
            }
        } else {
            format!("| {} | - | {} | MD{} |", f.date, format!("{} vs {}", home_name, away_name), f.matchday)
        };

        if f.date >= today && f.status != domain::league::FixtureStatus::Completed {
            upcoming.push(entry);
        } else if f.status == domain::league::FixtureStatus::Completed {
            past.push(entry);
        }
    }

    let mut output = String::new();

    if !upcoming.is_empty() {
        output.push_str("### Upcoming Fixtures\n\n| Date | Score | Match | MD |\n|------|-------|-------|----|\n");
        for row in &upcoming {
            output.push_str(row);
            output.push('\n');
        }
    }

    if !past.is_empty() {
        // Show last 5 past matches
        let recent: Vec<_> = past.iter().rev().take(5).collect();
        output.push_str("\n### Recent Results (last 5)\n\n| Date | Score | Match | MD |\n|------|-------|-------|----|\n");
        for row in recent {
            output.push_str(row);
            output.push_str("\n");
        }
    }

    if upcoming.is_empty() && past.is_empty() {
        output.push_str("No fixtures found for your team.");
    }

    Ok(output)
}

// ─── time_advance ───────────────────────────────────────────────────────────

// ─── info_player_profile ────────────────────────────────────────────────────

pub fn info_player_profile(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player = game.players.iter()
        .find(|p| p.id == player_id)
        .ok_or_else(|| format!("Player {} not found", player_id))?;

    let team_name = player.team_id.as_deref()
        .and_then(|tid| game.teams.iter().find(|t| t.id == tid))
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Free Agent".to_string());

    let is_own = game.manager.team_id.as_deref() == player.team_id.as_deref();

    let mut output = format!(
        "## {} — {}\n\n\
         | Field | Value |\n|-------|-------|\n\
         | ID | {} |\n\
         | Full Name | {} |\n\
         | Position | {} |\n\
         | Age | {} |\n\
         | Nationality | {} |\n\
         | Team | {} |\n",
        player.match_name,
        format_position(&player.position),
        player.id,
        player.full_name,
        format_position(&player.position),
        age_from_dob(&player.date_of_birth, &game),
        player.nationality,
        team_name,
    );

    if is_own {
        // Full detail for own players
        output.push_str(&format!(
            "| OVR | {} |\n\
             | Condition | {}% |\n\
             | Morale | {}% |\n\
             | Fitness | {}% |\n\
             | Wage | {} |\n\
             | Contract End | {} |\n",
            player.ovr,
            player.condition,
            player.morale,
            player.fitness,
            player.wage,
            player.contract_end.as_deref().unwrap_or("-"),
        ));

        if player.injury.is_some() {
            output.push_str(&format!("| Injury | ⚠️ {:?} |\n", player.injury));
        }

        // Attributes
        output.push_str("\n### Attributes\n\n| Attr | Val | Attr | Val | Attr | Val |\n|------|-----|------|-----|------|-----|\n");
        let attrs = &player.attributes;
        let attr_rows = [
            [("Pace", attrs.pace), ("Shooting", attrs.shooting), ("Passing", attrs.passing)],
            [("Dribbling", attrs.dribbling), ("Defending", attrs.defending), ("Tackling", attrs.tackling)],
            [("Strength", attrs.strength), ("Stamina", attrs.stamina), ("Agility", attrs.agility)],
            [("Vision", attrs.vision), ("Decisions", attrs.decisions), ("Composure", attrs.composure)],
            [("Positioning", attrs.positioning), ("Aggression", attrs.aggression), ("Teamwork", attrs.teamwork)],
            [("Leadership", attrs.leadership), ("Handling", attrs.handling), ("Reflexes", attrs.reflexes)],
            [("Aerial", attrs.aerial), ("", 0), ("", 0)],
        ];
        for row in &attr_rows {
            // Skip empty cells at the end
            let cells: Vec<String> = row.iter()
                .filter(|(name, _)| !name.is_empty())
                .flat_map(|(name, val)| [name.to_string(), val.to_string()])
                .collect();
            if cells.len() >= 4 {
                output.push_str(&format!("| {} |\n", cells.join(" | ")));
            }
        }
    } else {
        // Limited detail for other teams' players (competition mode)
        output.push_str(&format!(
            "| OVR | {} |\n\
             | Form | {} |\n",
            player.ovr,
            player.condition,
        ));
        output.push_str("\n*Use `scout_send` for detailed attributes.*");
    }

    Ok(output)
}

// ─── info_finances ──────────────────────────────────────────────────────────

// ─── info_finances ──────────────────────────────────────────────────────────

pub fn info_finances(ctx: Arc<McpContext>) -> Result<String, String> {
    let response = crate::commands::finances::get_finance_snapshot_internal(
        &ctx.state_manager,
        None,
    )
    .map_err(|e| translate_error(&e))?;

    let snap = &response.snapshot;

    Ok(format!(
        "## Financial Overview\n\n\
         | Item | Amount |\n|------|--------|\n\
         | Weekly Wage Spend | {} |\n\
         | Weekly Wage Budget | {} |\n\
         | Weekly Recurring Income | {} |\n\
         | Weekly Sponsor Income | {} |\n\
         | Projected Weekly Net | {} |\n\
         | Wage Budget Usage | {}% |\n\
         | Cash Runway | {} |\n\
         | In Debt | {} |\n\
         | Over Budget | {} |\n\
         | Overall Status | {:?} |",
        snap.weekly_wage_spend,
        snap.weekly_wage_budget,
        snap.weekly_recurring_income,
        snap.weekly_sponsor_income,
        snap.projected_weekly_net,
        snap.wage_budget_usage_percent,
        snap.cash_runway_weeks.map(|w| format!("{} weeks", w)).unwrap_or_else(|| "N/A".to_string()),
        snap.currently_in_debt,
        snap.currently_over_budget,
        snap.overall_status,
    ))
}

// ─── info_season_context ────────────────────────────────────────────────────

// ─── info_season_context ────────────────────────────────────────────────────

pub fn info_season_context(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    Ok(format!(
        "## Season Context\n\n\
         | Field | Value |\n|-------|-------|\n\
         | Phase | {:?} |\n\
         | Transfer Window | {} |\n\
         | Season | {} |",
        game.season_context.phase,
        if game.season_context.transfer_window.status == domain::season::TransferWindowStatus::Open { "Open" } else { "Closed" },
        game.league.as_ref().map(|l| l.season).unwrap_or(0),
    ))
}

// ─── info_news ──────────────────────────────────────────────────────────────

// ─── info_news ──────────────────────────────────────────────────────────────

pub fn info_news(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    let news: Vec<_> = game.news.iter().take(10).collect();
    if news.is_empty() {
        return Ok("## News\n\nNo recent news.".to_string());
    }

    let mut output = format!("## Recent News\n\n| # | Headline | Date |\n|---|----------|------|\n");
    for (i, n) in news.iter().enumerate() {
        output.push_str(&format!("| {} | {} | {} |\n", i + 1, n.headline, n.date));
    }

    Ok(output)
}

// ─── info_match_preview ─────────────────────────────────────────────────────

// ─── info_match_preview ─────────────────────────────────────────────────────

pub fn info_match_preview(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let league = require_league(&game)?;
    let team_id = game.manager.team_id.as_deref().ok_or("be.error.noTeamAssigned")?;

    let next_fixture = league.fixtures.iter()
        .filter(|f| f.status != domain::league::FixtureStatus::Completed)
        .filter(|f| f.home_team_id == team_id || f.away_team_id == team_id)
        .min_by_key(|f| &f.date);

    let Some(fixture) = next_fixture else {
        return Ok("## Match Preview\n\nNo upcoming fixtures.".to_string());
    };

    let is_home = fixture.home_team_id == team_id;
    let opponent_id = if is_home { &fixture.away_team_id } else { &fixture.home_team_id };
    let opponent_name = game.teams.iter().find(|t| t.id == *opponent_id).map(|t| t.name.clone()).unwrap_or_default();
    let venue = if is_home { "Home" } else { "Away" };

    // Opponent form (last 5 results)
    let opponent_results: Vec<_> = league.fixtures.iter()
        .filter(|f| f.status == domain::league::FixtureStatus::Completed)
        .filter(|f| f.home_team_id == *opponent_id || f.away_team_id == *opponent_id)
        .collect();

    let mut form = String::new();
    for f in opponent_results.iter().rev().take(5) {
        if let Some(ref result) = f.result {
            let is_opp_home = f.home_team_id == *opponent_id;
            let opp_goals = if is_opp_home { result.home_goals } else { result.away_goals };
            let other_goals = if is_opp_home { result.away_goals } else { result.home_goals };
            let marker = if opp_goals > other_goals { "W" } else if opp_goals < other_goals { "L" } else { "D" };
            form.push_str(&format!("{} ", marker));
        }
    }

    // Opponent position (sorted standings)
    let mut sorted_standings = league.standings.clone();
    sorted_standings.sort_by(|a, b| {
        b.points.cmp(&a.points)
            .then_with(|| b.goals_for.cmp(&a.goals_for))
    });
    let opp_pos = sorted_standings.iter()
        .position(|st| st.team_id == *opponent_id)
        .map(|p| p + 1)
        .unwrap_or(0);

    Ok(format!(
        "## Match Preview\n\n\
         | Field | Value |\n|-------|-------|\n\
         | Opponent | {} |\n\
         | Venue | {} |\n\
         | Date | {} |\n\
         | Matchday | {} |\n\
         | Opponent Position | {} |\n\
         | Opponent Form | {} |",
        opponent_name, venue, fixture.date, fixture.matchday, opp_pos, form.trim(),
    ))
}

// ─── club_upgrade_facility ──────────────────────────────────────────────────

// ─── info_player_stats ──────────────────────────────────────────────────────

pub fn info_player_stats(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    let response = crate::commands::stats::get_player_stats_overview_internal(
        &ctx.state_manager,
        &player_id,
    )
    .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or(player_id);

    Ok(format!("## Player Stats: {}\n\n{}", player_name, serde_json::to_string_pretty(&response).unwrap_or_else(|_| "Stats available".to_string())))
}

// ─── info_player_match_history ──────────────────────────────────────────────

// ─── info_player_match_history ──────────────────────────────────────────────

pub fn info_player_match_history(ctx: Arc<McpContext>, player_id: String, limit: Option<usize>) -> Result<String, String> {
    let response = crate::commands::stats::get_player_match_history_internal(
        &ctx.state_manager,
        &player_id,
        limit,
    )
    .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or(player_id);

    if response.is_empty() {
        return Ok(format!("## Match History: {}\n\nNo match data available.", player_name));
    }

    let mut output = format!("## Match History: {} ({} matches)\n\n| # | Date | Opponent | Mins | Goals | Assists |\n|---|------|----------|--------|-------|--------|\n", player_name, response.len());
    for (i, entry) in response.iter().enumerate() {
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            i + 1,
            entry.date,
            entry.opponent_name,
            entry.minutes_played,
            entry.goals,
            entry.assists,
        ));
    }

    Ok(output)
}

// ─── info_team_profile ──────────────────────────────────────────────────────

// ─── info_team_profile ──────────────────────────────────────────────────────

pub fn info_team_profile(ctx: Arc<McpContext>, team_id: String) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let team = game.teams.iter()
        .find(|t| t.id == team_id)
        .ok_or_else(|| format!("Team {} not found", team_id))?;

    let is_own = game.manager.team_id.as_deref() == Some(team_id.as_str());
    let squad_size = game.players.iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id.as_str()))
        .count();

    let mut output = format!(
        "## {} — Team Profile\n\n\
         | Field | Value |\n|-------|-------|\n\
         | ID | {} |\n\
         | Formation | {} |\n\
         | Play Style | {:?} |\n\
         | Squad Size | {} |\n\
         | Training | {:?} / {:?} |\n",
        team.name, team.id, team.formation, team.play_style, squad_size,
        team.training_focus, team.training_intensity,
    );

    // Standings position
    if let Some(league) = &game.league {
        let mut standings = league.standings.clone();
        standings.sort_by(|a, b| b.points.cmp(&a.points).then_with(|| b.goals_for.cmp(&a.goals_for)));
        if let Some(pos) = standings.iter().position(|st| st.team_id == team_id) {
            let s = &standings[pos];
            let gd = i64::from(s.goals_for) - i64::from(s.goals_against);
            output.push_str(&format!(
                "| League Position | {} |\n\
                 | Points | {} ({}/{}/{}) |\n\
                 | Goal Difference | {:+} |\n",
                pos + 1, s.points, s.won, s.drawn, s.lost, gd,
            ));
        }
    }

    // Recent form (last 5 results)
    if let Some(league) = &game.league {
        let recent: Vec<_> = league.fixtures.iter()
            .filter(|f| f.status == domain::league::FixtureStatus::Completed)
            .filter(|f| f.home_team_id == team_id || f.away_team_id == team_id)
            .collect();

        let mut form = String::new();
        for f in recent.iter().rev().take(5) {
            if let Some(ref result) = f.result {
                let is_home = f.home_team_id == team_id;
                let our_goals = if is_home { result.home_goals } else { result.away_goals };
                let their_goals = if is_home { result.away_goals } else { result.home_goals };
                let marker = if our_goals > their_goals { "W" } else if our_goals < their_goals { "L" } else { "D" };
                form.push_str(&format!("{} ", marker));
            }
        }
        if !form.is_empty() {
            output.push_str(&format!("| Recent Form | {} |\n", form.trim()));
        }
    }

    // Financial info for own team
    if is_own {
        if let Ok(finance_response) = crate::commands::finances::get_finance_snapshot_internal(&ctx.state_manager, Some(team_id.as_str())) {
            let snap = &finance_response.snapshot;
            output.push_str(&format!(
                "\n### Finances\n\n\
                 | Item | Value |\n|------|-------|\n\
                 | Weekly Wage Spend | {} |\n\
                 | Weekly Wage Budget | {} |\n\
                 | Projected Weekly Net | {} |\n\
                 | In Debt | {} |\n\
                 | Overall Status | {:?} |",
                snap.weekly_wage_spend,
                snap.weekly_wage_budget,
                snap.projected_weekly_net,
                snap.currently_in_debt,
                snap.overall_status,
            ));
        }
    }

    Ok(output)
}

// ─── info_team_stats ────────────────────────────────────────────────────────

// ─── info_team_stats ────────────────────────────────────────────────────────

pub fn info_team_stats(ctx: Arc<McpContext>, team_id: String) -> Result<String, String> {
    let response = crate::commands::stats::get_team_stats_overview_internal(
        &ctx.state_manager,
        &team_id,
    )
    .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let team_name = game.teams.iter()
        .find(|t| t.id == team_id)
        .map(|t| t.name.clone())
        .unwrap_or(team_id);

    match response {
        Some(stats) => Ok(format!("## Team Stats: {}\n\n{}", team_name, serde_json::to_string_pretty(&stats).unwrap_or_else(|_| "Stats available".to_string()))),
        None => Ok(format!("## Team Stats: {}\n\nNo stats available yet.", team_name)),
    }
}

// ─── info_team_match_history ────────────────────────────────────────────────

// ─── info_team_match_history ────────────────────────────────────────────────

pub fn info_team_match_history(ctx: Arc<McpContext>, team_id: String, limit: Option<usize>) -> Result<String, String> {
    let response = crate::commands::stats::get_team_match_history_internal(
        &ctx.state_manager,
        &team_id,
        limit,
    )
    .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let team_name = game.teams.iter()
        .find(|t| t.id == team_id)
        .map(|t| t.name.clone())
        .unwrap_or(team_id);

    if response.is_empty() {
        return Ok(format!("## Match History: {}\n\nNo match data available.", team_name));
    }

    let mut output = format!("## Match History: {} ({} matches)\n\n| # | Date | Opponent | Score |\n|---|------|----------|-------|\n", team_name, response.len());
    for (i, entry) in response.iter().enumerate() {
        output.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            i + 1,
            entry.date,
            entry.opponent_name,
            format!("{}-{}", entry.goals_for, entry.goals_against),
        ));
    }

    Ok(output)
}

// ─── info_finance_snapshot ──────────────────────────────────────────────────

// ─── info_finance_snapshot ──────────────────────────────────────────────────

pub fn info_finance_snapshot(ctx: Arc<McpContext>, team_id: Option<String>) -> Result<String, String> {
    let response = crate::commands::finances::get_finance_snapshot_internal(
        &ctx.state_manager,
        team_id.as_deref(),
    )
    .map_err(|e| translate_error(&e))?;

    let snap = &response.snapshot;

    Ok(format!(
        "## Detailed Financial Snapshot\n\n\
         | Metric | Value |\n|--------|-------|\n\
         | Annual Wage Bill | {} |\n\
         | Weekly Wage Spend | {} |\n\
         | Weekly Wage Budget | {} |\n\
         | Weekly Recurring Income | {} |\n\
         | Weekly Sponsor Income | {} |\n\
         | Projected Weekly Net | {} |\n\
         | Cash Runway | {} |\n\
         | Wage Budget Usage | {}% |\n\
         | In Debt | {} |\n\
         | Over Budget | {} |\n\
         | Budget Status | {:?} |\n\
         | Runway Status | {:?} |\n\
         | Overall Status | {:?} |",
        snap.annual_wage_bill,
        snap.weekly_wage_spend,
        snap.weekly_wage_budget,
        snap.weekly_recurring_income,
        snap.weekly_sponsor_income,
        snap.projected_weekly_net,
        snap.cash_runway_weeks.map(|w| format!("{} weeks", w)).unwrap_or_else(|| "N/A".to_string()),
        snap.wage_budget_usage_percent,
        snap.currently_in_debt,
        snap.currently_over_budget,
        snap.wage_budget_status,
        snap.runway_status,
        snap.overall_status,
    ))
}

// ─── club_request_board_support ─────────────────────────────────────────────
