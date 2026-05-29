//! Real implementations for MCP tools.
//!
//! Each function takes `Arc<McpContext>` and returns formatted text.
//! They call the same `*_internal` functions used by Tauri commands,
//! then format the result as markdown for agent readability.

use std::sync::Arc;

use chrono::Datelike;
use ofm_core::state::StateManager;

use crate::mcp_server::context::McpContext;
use crate::mcp_server::formatting::translate_error;

/// Get the active game from StateManager, returning a formatted error if none.
fn require_game(state_manager: &StateManager) -> Result<ofm_core::game::Game, String> {
    state_manager
        .get_game(|g| g.clone())
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())
}

/// Get the user's team from the game.
fn user_team(game: &ofm_core::game::Game) -> Result<&domain::team::Team, String> {
    let team_id = game
        .manager
        .team_id
        .as_deref()
        .ok_or("be.error.noTeamAssigned")?;
    game.teams
        .iter()
        .find(|t| t.id == team_id)
        .ok_or_else(|| "be.error.teamNotFound".to_string())
}

/// Get the league, returning a formatted error if none.
fn require_league(game: &ofm_core::game::Game) -> Result<&domain::league::League, String> {
    game.league
        .as_ref()
        .ok_or_else(|| "No league found. Season may not have started yet.".to_string())
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
        league.fixtures.iter().find(|f| {
            f.date >= today
                && f.status == domain::league::FixtureStatus::Scheduled
                && (f.home_team_id == team_id || f.away_team_id == team_id)
        }).map(|f| {
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

pub fn time_advance(ctx: Arc<McpContext>) -> Result<String, String> {
    // Use the delegate mode to force auto-simulation of matches
    let response = crate::application::time_advancement::advance_time_with_mode(
        &ctx.state_manager,
        "delegate",
    )
    .map_err(|e| translate_error(&e))?;

    let mut output = String::new();

    // Current date
    let date_str = if let Some(ref game) = response.game {
        game.clock.current_date.format("%d %B %Y").to_string()
    } else {
        "Unknown".to_string()
    };
    output.push_str(&format!("## Day Advanced — {}\n\n", date_str));

    // If there was a match, show round summary
    if let Some(ref round_summary) = response.round_summary {
        if !round_summary.completed_results.is_empty() {
            output.push_str("### Match Results\n\n| Home | Score | Away |\n|------|-------|------|\n");
            for result in &round_summary.completed_results {
                output.push_str(&format!(
                    "| {} | {} - {} | {} |\n",
                    result.home_team_name,
                    result.home_goals,
                    result.away_goals,
                    result.away_team_name,
                ));
            }

            // Highlight user's match
            if let Some(ref game) = response.game {
                if let Some(team_id) = &game.manager.team_id {
                    for result in &round_summary.completed_results {
                        if result.home_team_id == *team_id || result.away_team_id == *team_id {
                            let is_home = result.home_team_id == *team_id;
                            let our_goals = if is_home { result.home_goals } else { result.away_goals };
                            let their_goals = if is_home { result.away_goals } else { result.home_goals };
                            let opponent = if is_home { &result.away_team_name } else { &result.home_team_name };
                            let venue = if is_home { "H" } else { "A" };
                            let result_text = if our_goals > their_goals {
                                "won"
                            } else if our_goals < their_goals {
                                "lost"
                            } else {
                                "drew"
                            };
                            output.push_str(&format!(
                                "\nYour team {} {}-{} vs {} ({}).",
                                result_text, our_goals, their_goals, opponent, venue
                            ));
                            break;
                        }
                    }
                }
            }
        }

        // Standings update if we have a game
        if let Some(ref game) = response.game {
            if let Some(league) = &game.league {
                if let Some(team_id) = &game.manager.team_id {
                    let mut standings = league.standings.clone();
                    standings.sort_by(|a, b| b.points.cmp(&a.points).then_with(|| b.goals_for.cmp(&a.goals_for)));
                    if let Some(pos) = standings.iter().position(|s| s.team_id == *team_id) {
                        let standing = &standings[pos];
                        output.push_str(&format!(
                            "\n\n### Standings Update\n\nLeague position: {} | Points: {} | GD: {:+}",
                            pos + 1,
                            standing.points,
                            i64::from(standing.goals_for) - i64::from(standing.goals_against),
                        ));
                    }
                }
            }
        }
    }

    // Check if manager was fired during the advance
    if let Some(ref game) = response.game {
        if game.manager.team_id.is_none() {
            output.push_str("\n\n**⚠️ You have been fired!** Use `jobs_available` to find a new position.");
        }
    }

    // Notify GUI about state change
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(output)
}

// ─── squad_get ──────────────────────────────────────────────────────────────

pub fn squad_get(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;
    let team_id = team.id.as_str();

    let mut squad: Vec<&domain::player::Player> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .collect();

    // Sort: starting XI first (by starting_xi_ids order), then rest by OVR descending
    squad.sort_by(|a, b| {
        let a_in_xi = team.starting_xi_ids.contains(&a.id);
        let b_in_xi = team.starting_xi_ids.contains(&b.id);
        match (a_in_xi, b_in_xi) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.ovr.cmp(&a.ovr),
        }
    });

    let mut rows = String::new();
    for p in &squad {
        let in_xi = if team.starting_xi_ids.contains(&p.id) { "★" } else { "" };
        let pos = format_position(&p.position);
        let inj = if p.injury.is_some() { "⚠" } else { "" };
        rows.push_str(&format!(
            "| {} | {}{} | {} | {} | {} | {} | {} | {} | {} |\n",
            p.id,
            p.match_name,
            inj,
            pos,
            age_from_dob(&p.date_of_birth, &game),
            p.ovr,
            p.condition,
            p.morale,
            p.wage,
            p.contract_end.as_deref().unwrap_or("-"),
        ));
    }

    // Starting XI summary
    let xi_names: Vec<String> = team.starting_xi_ids.iter()
        .filter_map(|id| game.players.iter().find(|p| p.id == *id))
        .map(|p| format!("{} {}", format_position(&p.position), p.match_name))
        .collect();

    Ok(format!(
        "## {} — Squad Overview\n\n\
         | ID | Name | Pos | Age | OVR | Con | Mor | Wage | Contract |\n\
         |----|-------|-----|-----|-----|-----|-----|------|----------|\n\
         {}\
         \n**Starting XI**: {}\n\
         **Formation**: {} | **Play Style**: {:?}",
        team.name,
        rows,
        xi_names.join(", "),
        team.formation,
        team.play_style,
    ))
}

fn format_position(pos: &domain::player::Position) -> &'static str {
    match pos {
        domain::player::Position::Goalkeeper => "GK",
        domain::player::Position::Defender => "DF",
        domain::player::Position::Midfielder => "MF",
        domain::player::Position::Forward => "FW",
        domain::player::Position::RightBack => "RB",
        domain::player::Position::CenterBack => "CB",
        domain::player::Position::LeftBack => "LB",
        domain::player::Position::RightWingBack => "RWB",
        domain::player::Position::LeftWingBack => "LWB",
        domain::player::Position::DefensiveMidfielder => "DM",
        domain::player::Position::CentralMidfielder => "CM",
        domain::player::Position::AttackingMidfielder => "AM",
        domain::player::Position::RightMidfielder => "RM",
        domain::player::Position::LeftMidfielder => "LM",
        domain::player::Position::RightWinger => "RW",
        domain::player::Position::LeftWinger => "LW",
        domain::player::Position::Striker => "ST",
    }
}

fn age_from_dob(dob: &str, game: &ofm_core::game::Game) -> String {
    let dob_date = match chrono::NaiveDate::parse_from_str(dob, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return "?".to_string(),
    };
    let ref_date = game.clock.current_date.date_naive();
    let mut age = i32::from(ref_date.year()) - i32::from(dob_date.year());
    if (ref_date.month(), ref_date.day()) < (dob_date.month(), dob_date.day()) {
        age -= 1;
    }
    age.to_string()
}

// ─── game_save ──────────────────────────────────────────────────────────────

pub fn game_save(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let save_id = ctx.state_manager
        .get_save_id()
        .ok_or("be.error.noActiveSaveSession")?;

    let stats_state = ctx.state_manager
        .get_stats_state(|s| s.clone())
        .unwrap_or_default();

    {
        let mut sm = ctx.save_manager_state.0.lock().map_err(|_| "be.error.saveManagerUnavailable".to_string())?;
        sm.save_game_with_stats(&game, &stats_state, &save_id)?;
    }

    Ok(format!("## Game Saved\n\n**Save ID**: {}\n**Date**: {}", save_id, game.clock.current_date.format("%d %B %Y")))
}

// ─── squad_set_starting_xi ─────────────────────────────────────────────────

pub fn squad_set_starting_xi(ctx: Arc<McpContext>, player_ids: Vec<String>) -> Result<String, String> {
    // Call the internal function from commands/squad.rs
    crate::commands::squad::set_starting_xi_internal(&ctx.state_manager, player_ids.clone())
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    // Format the starting XI
    let xi_names: Vec<String> = player_ids.iter()
        .map(|id| {
            game.players.iter()
                .find(|p| p.id == *id)
                .map(|p| format!("{} {}", format_position(&p.position), p.match_name))
                .unwrap_or_else(|| id.clone())
        })
        .collect();

    // Notify GUI
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Starting XI Updated\n\n{}\n**Formation**: {}", xi_names.join(", "), team.formation))
}

// ─── squad_set_formation ────────────────────────────────────────────────────

pub fn squad_set_formation(ctx: Arc<McpContext>, formation: String) -> Result<String, String> {
    crate::commands::squad::set_formation_internal(&ctx.state_manager, &formation)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    // Notify GUI
    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Formation Changed\n\n**New Formation**: {}\n**Note**: Outfield player positions have been reassigned based on defending ability.", team.formation))
}
