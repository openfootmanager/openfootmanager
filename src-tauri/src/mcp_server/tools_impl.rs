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

// ─── squad_set_play_style ───────────────────────────────────────────────────

pub fn squad_set_play_style(ctx: Arc<McpContext>, play_style: String) -> Result<String, String> {
    crate::commands::squad::set_play_style_internal(&ctx.state_manager, &play_style)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Play Style Changed\n\n**New Style**: {:?}", team.play_style))
}

// ─── squad_set_match_roles ───────────────────────────────────────────────────

pub fn squad_set_match_roles(
    ctx: Arc<McpContext>,
    captain: Option<String>,
    vice_captain: Option<String>,
    penalty_taker: Option<String>,
    free_kick_taker: Option<String>,
    corner_taker: Option<String>,
) -> Result<String, String> {
    let match_roles = domain::team::MatchRoles {
        captain,
        vice_captain,
        penalty_taker,
        free_kick_taker,
        corner_taker,
    };

    crate::commands::squad::set_team_match_roles_internal(&ctx.state_manager, match_roles)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("## Match Roles Updated\n\nCaptain, set-piece takers set as specified.".to_string())
}

// ─── squad_auto_set_pieces ──────────────────────────────────────────────────

pub fn squad_auto_set_pieces(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    let result = crate::commands::squad::auto_select_set_pieces_internal(
        &ctx.state_manager,
        &team.starting_xi_ids,
    )
    .map_err(|e| translate_error(&e))?;

    // Apply the auto-selected roles
    let match_roles = domain::team::MatchRoles {
        captain: result.get("captain").and_then(|v| v.as_str()).map(|s| s.to_string()),
        vice_captain: None,
        penalty_taker: result.get("penalty_taker").and_then(|v| v.as_str()).map(|s| s.to_string()),
        free_kick_taker: result.get("free_kick_taker").and_then(|v| v.as_str()).map(|s| s.to_string()),
        corner_taker: result.get("corner_taker").and_then(|v| v.as_str()).map(|s| s.to_string()),
    };

    crate::commands::squad::set_team_match_roles_internal(&ctx.state_manager, match_roles)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;

    // Format the result
    let mut names = Vec::new();
    if let Some(ref id) = game.teams.iter().find(|t| t.id == team.id).unwrap().match_roles.captain {
        if let Some(p) = game.players.iter().find(|p| p.id == *id) {
            names.push(format!("Captain: {}", p.match_name));
        }
    }
    if let Some(ref id) = game.teams.iter().find(|t| t.id == team.id).unwrap().match_roles.penalty_taker {
        if let Some(p) = game.players.iter().find(|p| p.id == *id) {
            names.push(format!("Penalties: {}", p.match_name));
        }
    }
    if let Some(ref id) = game.teams.iter().find(|t| t.id == team.id).unwrap().match_roles.free_kick_taker {
        if let Some(p) = game.players.iter().find(|p| p.id == *id) {
            names.push(format!("Free Kicks: {}", p.match_name));
        }
    }
    if let Some(ref id) = game.teams.iter().find(|t| t.id == team.id).unwrap().match_roles.corner_taker {
        if let Some(p) = game.players.iter().find(|p| p.id == *id) {
            names.push(format!("Corners: {}", p.match_name));
        }
    }

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Auto-Assigned Set Pieces\n\n{}", names.join("\n")))
}

// ─── squad_set_player_role ──────────────────────────────────────────────────

pub fn squad_set_player_role(ctx: Arc<McpContext>, player_id: String, squad_role: String) -> Result<String, String> {
    crate::commands::squad::set_player_squad_role_internal(&ctx.state_manager, &player_id, &squad_role)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or(player_id);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Player Role Updated\n\n**{}**: {}", player_name, squad_role))
}

// ─── training_get ───────────────────────────────────────────────────────────

pub fn training_get(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let team = user_team(&game)?;

    let team_id = team.id.as_str();

    // Count players by condition level
    let players: Vec<_> = game.players.iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .collect();

    let avg_condition = if players.is_empty() { 0u32 } else {
        players.iter().map(|p| p.condition as u32).sum::<u32>() / players.len() as u32
    };
    let avg_fitness = if players.is_empty() { 0u32 } else {
        players.iter().map(|p| p.fitness as u32).sum::<u32>() / players.len() as u32
    };
    let injured_count = players.iter().filter(|p| p.injury.is_some()).count();

    Ok(format!(
        "## Training Settings — {}\n\n\
         **Focus**: {:?}\n\
         **Intensity**: {:?}\n\
         **Schedule**: {:?}\n\
         **Training Groups**: {}\n\n\
         ### Squad Fitness Overview\n\
         | Metric | Value |\n|--------|-------|\n\
         | Avg Condition | {}% |\n\
         | Avg Fitness | {}% |\n\
         | Injured Players | {} |",
        team.name,
        team.training_focus,
        team.training_intensity,
        team.training_schedule,
        team.training_groups.len(),
        avg_condition,
        avg_fitness,
        injured_count,
    ))
}

// ─── training_set_focus_intensity ──────────────────────────────────────────

pub fn training_set_focus_intensity(ctx: Arc<McpContext>, focus: String, intensity: String) -> Result<String, String> {
    crate::commands::squad::set_training_internal(&ctx.state_manager, &focus, &intensity)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Training Updated\n\n**Focus**: {}\n**Intensity**: {}", focus, intensity))
}

// ─── training_set_schedule ─────────────────────────────────────────────────

pub fn training_set_schedule(ctx: Arc<McpContext>, schedule: String) -> Result<String, String> {
    crate::commands::squad::set_training_schedule_internal(&ctx.state_manager, &schedule)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Training Schedule Updated\n\n**Schedule**: {}", schedule))
}

// ─── training_set_groups ────────────────────────────────────────────────────

pub fn training_set_groups(ctx: Arc<McpContext>, groups_json: String) -> Result<String, String> {
    let groups: Vec<domain::team::TrainingGroup> = serde_json::from_str(&groups_json)
        .map_err(|e| format!("Invalid training groups JSON: {}", e))?;

    crate::commands::squad::set_training_groups_internal(&ctx.state_manager, groups)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok("## Training Groups Updated".to_string())
}

// ─── training_set_player_focus ──────────────────────────────────────────────

pub fn training_set_player_focus(ctx: Arc<McpContext>, player_id: String, focus: Option<String>) -> Result<String, String> {
    crate::commands::squad::set_player_training_focus_internal(&ctx.state_manager, &player_id, focus.as_deref())
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or(player_id);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Player Training Focus Updated\n\n**{}**: {}", player_name, focus.as_deref().unwrap_or("cleared (team default)")))
}

// ─── transfer_toggle_listed ────────────────────────────────────────────────

pub fn transfer_toggle_listed(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    crate::commands::transfers::toggle_transfer_list_internal(&ctx.state_manager, &player_id)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let is_listed = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.transfer_listed)
        .unwrap_or(false);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let status = if is_listed { "Transfer Listed ✓" } else { "Not Listed" };
    Ok(format!("## Transfer Status Updated\n\n**{}**: {}", player_name, status))
}

// ─── transfer_toggle_loan ──────────────────────────────────────────────────

pub fn transfer_toggle_loan(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    crate::commands::transfers::toggle_loan_list_internal(&ctx.state_manager, &player_id)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let is_loaned = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.loan_listed)
        .unwrap_or(false);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let status = if is_loaned { "Loan Listed ✓" } else { "Not Listed" };
    Ok(format!("## Loan Status Updated\n\n**{}**: {}", player_name, status))
}

// ─── transfer_make_bid ──────────────────────────────────────────────────────

pub fn transfer_make_bid(ctx: Arc<McpContext>, player_id: String, fee: u64) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    let response = crate::commands::transfers::make_transfer_bid_internal(
        &ctx.state_manager,
        &player_id,
        fee,
    )
    .map_err(|e| translate_error(&e))?;

    let mut output = format!("## Transfer Bid: {} — {} 💰\n\n", player_name, fee);

    output.push_str(&format!("**Decision**: {:?}\n", response.decision));
    if let Some(suggested) = response.suggested_fee {
        output.push_str(&format!("**Suggested Fee**: {}\n", suggested));
    }
    output.push_str(&format!("**Terminal**: {}\n", response.is_terminal));
    output.push_str(&format!("**Mood**: {:?}\n", response.feedback.mood));
    output.push_str(&format!("**Tension**: {}/100\n", response.feedback.tension));
    output.push_str(&format!("**Patience**: {}/100\n", response.feedback.patience));
    output.push_str(&format!("**Round**: {}\n", response.feedback.round));

    if response.is_terminal {
        output.push_str("\n✅ Negotiation complete.");
    } else {
        output.push_str("\n🔄 Negotiation continues — make another bid or walk away.");
    }

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(output)
}

// ─── transfer_preview_bid ──────────────────────────────────────────────────

pub fn transfer_preview_bid(ctx: Arc<McpContext>, player_id: String, fee: u64) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    let response = crate::commands::transfers::preview_transfer_bid_financial_impact_internal(
        &ctx.state_manager,
        &player_id,
        fee,
    )
    .map_err(|e| translate_error(&e))?;

    Ok(format!(
        "## Transfer Bid Preview: {} — {} 💰\n\n**Projection**: (financial details from projection)\nThis is a preview — no bid was made.",
        player_name, fee,
    ))
}

// ─── transfer_respond_to_offer ──────────────────────────────────────────────

pub fn transfer_respond_to_offer(ctx: Arc<McpContext>, player_id: String, offer_id: String, accept: bool) -> Result<String, String> {
    crate::commands::transfers::respond_to_offer_internal(
        &ctx.state_manager,
        &player_id,
        &offer_id,
        accept,
    )
    .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let action = if accept { "accepted" } else { "rejected" };
    Ok(format!("## Offer {}\n\nOffer {} for player {}.", action, offer_id, player_id))
}

// ─── transfer_counter_offer ─────────────────────────────────────────────────

pub fn transfer_counter_offer(ctx: Arc<McpContext>, player_id: String, offer_id: String, requested_fee: u64) -> Result<String, String> {
    let response = crate::commands::transfers::counter_offer_internal(
        &ctx.state_manager,
        &player_id,
        &offer_id,
        requested_fee,
    )
    .map_err(|e| translate_error(&e))?;

    let mut output = format!("## Counter Offer: {} 💰\n\n", requested_fee);
    output.push_str(&format!("**Decision**: {:?}\n", response.decision));
    if let Some(suggested) = response.suggested_fee {
        output.push_str(&format!("**Suggested Fee**: {}\n", suggested));
    }
    output.push_str(&format!("**Terminal**: {}\n", response.is_terminal));

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(output)
}

// ─── contract_propose_renewal ───────────────────────────────────────────────

pub fn contract_propose_renewal(ctx: Arc<McpContext>, player_id: String, weekly_wage: u32, contract_years: u32) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_default();

    let response = crate::commands::contracts::propose_renewal_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
        contract_years,
    )
    .map_err(|e| translate_error(&e))?;

    let mut output = format!("## Contract Renewal: {} — {}💰/wk × {}yr\n\n", player_name, weekly_wage, contract_years);
    output.push_str(&format!("**Outcome**: {:?}\n", response.outcome));
    if let Some(wage) = response.suggested_wage {
        output.push_str(&format!("**Suggested Wage**: {}/wk\n", wage));
    }
    if let Some(years) = response.suggested_years {
        output.push_str(&format!("**Suggested Years**: {}\n", years));
    }
    output.push_str(&format!("**Session**: {}\n", response.session_status));
    output.push_str(&format!("**Terminal**: {}\n", response.is_terminal));
    output.push_str(&format!("**Cooled Off**: {}\n", response.cooled_off));

    if let Some(ref feedback) = response.feedback {
        output.push_str(&format!("**Mood**: {:?}\n", feedback.mood));
        output.push_str(&format!("**Tension**: {}/100\n", feedback.tension));
        output.push_str(&format!("**Patience**: {}/100\n", feedback.patience));
    }

    if response.is_terminal {
        output.push_str("\n✅ Negotiation complete.");
    } else if response.cooled_off {
        output.push_str("\n❄️ Player has cooled off — wait before re-offering.");
    } else {
        output.push_str("\n🔄 Negotiation continues — adjust your offer.");
    }

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(output)
}

// ─── contract_delegate_renewals ─────────────────────────────────────────────

pub fn contract_delegate_renewals(ctx: Arc<McpContext>, player_ids: Option<Vec<String>>, max_wage_increase_pct: u32, max_contract_years: u32) -> Result<String, String> {
    let response = crate::commands::contracts::delegate_renewals_internal(
        &ctx.state_manager,
        player_ids,
        max_wage_increase_pct,
        max_contract_years,
    )
    .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Delegated Renewals\n\n**Results**: {} success, {} failed, {} stalled.\n**Max Wage Increase**: {}%\n**Max Years**: {}", response.report.success_count, response.report.failure_count, response.report.stalled_count, max_wage_increase_pct, max_contract_years))
}

// ─── contract_preview_renewal ───────────────────────────────────────────────

pub fn contract_preview_renewal(ctx: Arc<McpContext>, player_id: String, weekly_wage: u32) -> Result<String, String> {
    let response = crate::commands::contracts::preview_renewal_financial_impact_internal(
        &ctx.state_manager,
        &player_id,
        weekly_wage,
    )
    .map_err(|e| translate_error(&e))?;

    Ok(format!("## Renewal Preview\n\n**Wage Offer**: {}/wk\nThis is a preview — no offer was made.", weekly_wage))
}

// ─── contract_set_exit_intent ───────────────────────────────────────────────

pub fn contract_set_exit_intent(ctx: Arc<McpContext>, player_id: String, reason: Option<String>) -> Result<String, String> {
    crate::commands::contracts::set_contract_exit_intent_internal(&ctx.state_manager, &player_id, reason)
        .map_err(|e| translate_error(&e))?;

    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or(player_id);

    Ok(format!("## Exit Intent Set\n\n**{}**: Contract will be allowed to expire.", player_name))
}

// ─── contract_clear_exit_intent ─────────────────────────────────────────────

pub fn contract_clear_exit_intent(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    crate::commands::contracts::clear_contract_exit_intent_internal(&ctx.state_manager, &player_id)
        .map_err(|e| translate_error(&e))?;

    Ok("## Exit Intent Cleared\n\nContract will proceed normally.".to_string())
}

// ─── contract_preview_termination ───────────────────────────────────────────

pub fn contract_preview_termination(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    let response = crate::commands::contracts::preview_contract_termination_internal(
        &ctx.state_manager,
        &player_id,
    )
    .map_err(|e| translate_error(&e))?;

    Ok(format!("## Termination Preview\n\n**Cost**: (see projection details)\nThis is a preview — no contract was terminated."))
}

// ─── contract_terminate ─────────────────────────────────────────────────────

pub fn contract_terminate(ctx: Arc<McpContext>, player_id: String) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;
    let player_name = game.players.iter()
        .find(|p| p.id == player_id)
        .map(|p| p.match_name.clone())
        .unwrap_or_else(|| player_id.clone());

    crate::commands::contracts::terminate_contract_now_internal(&ctx.state_manager, &player_id)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Contract Terminated\n\n**{}** has been released.", player_name))
}

// ─── inbox_get_messages ─────────────────────────────────────────────────────

pub fn inbox_get_messages(ctx: Arc<McpContext>, category: Option<String>, unread_only: Option<bool>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    let messages: Vec<_> = game.messages.iter()
        .filter(|m| {
            if let Some(ref cat) = category {
                format!("{:?}", m.category) == *cat
            } else {
                true
            }
        })
        .filter(|m| {
            if let Some(true) = unread_only {
                !m.read
            } else {
                true
            }
        })
        .collect();

    if messages.is_empty() {
        return Ok("## Inbox\n\nNo messages.".to_string());
    }

    let mut output = format!("## Inbox ({} messages)\n\n| ID | Subject | Category | Read | Date |\n|----|---------|----------|------|------|\n", messages.len());
    for m in messages.iter().take(20) {
        let read_marker = if m.read { "✓" } else { "●" };
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            m.id,
            m.subject,
            format!("{:?}", m.category),
            read_marker,
            m.date,
        ));
    }
    if messages.len() > 20 {
        output.push_str(&format!("\n... and {} more.", messages.len() - 20));
    }

    Ok(output)
}

// ─── inbox_mark_read ────────────────────────────────────────────────────────

pub fn inbox_mark_read(ctx: Arc<McpContext>, message_id: String) -> Result<String, String> {
    crate::commands::messages::mark_message_read_internal(&ctx.state_manager, &message_id)
        .map_err(|e| translate_error(&e))?;

    Ok("Message marked as read.".to_string())
}

// ─── inbox_mark_all_read ────────────────────────────────────────────────────

pub fn inbox_mark_all_read(ctx: Arc<McpContext>) -> Result<String, String> {
    crate::commands::messages::mark_all_messages_read_internal(&ctx.state_manager)
        .map_err(|e| translate_error(&e))?;

    Ok("All messages marked as read.".to_string())
}

// ─── inbox_delete ───────────────────────────────────────────────────────────

pub fn inbox_delete(ctx: Arc<McpContext>, message_id: String) -> Result<String, String> {
    crate::commands::messages::delete_message_internal(&ctx.state_manager, &message_id)
        .map_err(|e| translate_error(&e))?;

    Ok("Message deleted.".to_string())
}

// ─── inbox_clear_old ────────────────────────────────────────────────────────

pub fn inbox_clear_old(ctx: Arc<McpContext>) -> Result<String, String> {
    crate::commands::messages::clear_old_messages_internal(&ctx.state_manager)
        .map_err(|e| translate_error(&e))?;

    Ok("Old messages cleared.".to_string())
}

// ─── inbox_resolve_action ───────────────────────────────────────────────────

pub fn inbox_resolve_action(ctx: Arc<McpContext>, message_id: String, action_id: String, option_id: Option<String>) -> Result<String, String> {
    crate::commands::messages::resolve_message_action_internal(
        &ctx.state_manager,
        &message_id,
        &action_id,
        option_id.as_deref(),
    )
    .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Action Resolved\n\nMessage {} — action {} completed.", message_id, action_id))
}

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

    // Opponent position
    let opp_pos = league.standings.iter()
        .filter(|s| s.team_id == *opponent_id)
        .position(|s| true)
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

pub fn club_upgrade_facility(ctx: Arc<McpContext>, facility: String) -> Result<String, String> {
    crate::commands::club::upgrade_facility_internal(&ctx.state_manager, &facility)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Facility Upgraded\n\n**{}** upgraded.", facility))
}

// ─── staff_get ──────────────────────────────────────────────────────────────

pub fn staff_get(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    let mut output = String::from("## Staff\n\n| ID | Name | Role | Team |\n|----|------|------|------|\n");

    for s in &game.staff {
        let team = s.team_id.as_deref()
            .and_then(|tid| game.teams.iter().find(|t| t.id == tid))
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Unattached".to_string());
        output.push_str(&format!("| {} | {} {} | {:?} | {} |\n", s.id, s.first_name, s.last_name, s.role, team));
    }

    Ok(output)
}

// ─── staff_hire ──────────────────────────────────────────────────────────────

pub fn staff_hire(ctx: Arc<McpContext>, staff_id: String) -> Result<String, String> {
    crate::commands::staff::hire_staff_internal(&ctx.state_manager, &staff_id)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Staff Hired\n\nStaff member {} hired.", staff_id))
}

// ─── staff_release ───────────────────────────────────────────────────────────

pub fn staff_release(ctx: Arc<McpContext>, staff_id: String) -> Result<String, String> {
    crate::commands::staff::release_staff_internal(&ctx.state_manager, &staff_id)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!("## Staff Released\n\nStaff member {} released.", staff_id))
}

// ─── season_check_complete ──────────────────────────────────────────────────

pub fn season_check_complete(ctx: Arc<McpContext>) -> Result<String, String> {
    let game = require_game(&ctx.state_manager)?;

    if let Some(league) = &game.league {
        let incomplete = league.fixtures.iter()
            .filter(|f| f.status != domain::league::FixtureStatus::Completed)
            .count();
        if incomplete == 0 && !league.fixtures.is_empty() {
            return Ok("## Season Status: Complete ✅\n\nAll fixtures played. Use `season_advance` to proceed.".to_string());
        }
        return Ok(format!("## Season Status: In Progress\n\n**Remaining fixtures**: {}", incomplete));
    }

    Ok("## Season Status: No league active.".to_string())
}

// ─── season_advance ─────────────────────────────────────────────────────────

pub fn season_advance(ctx: Arc<McpContext>) -> Result<String, String> {
    // The season advance is handled by advancing time through the off-season.
    // In competition mode, this uses delegate mode.
    let response = crate::application::time_advancement::advance_time_with_mode(
        &ctx.state_manager,
        "delegate",
    )
    .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    if let Some(ref game) = response.game {
        if game.manager.team_id.is_none() {
            return Ok("## Season Advance\n\n**⚠️ You have been fired!** Use `jobs_available` to find a new position.".to_string());
        }

        Ok(format!("## Day Advanced\n\nDate: {}. Continue advancing to reach next season.", game.clock.current_date.format("%d %B %Y")))
    } else {
        Ok("## Season Advance\n\nGame state lost during advance.".to_string())
    }
}

// ─── help_find_tool ─────────────────────────────────────────────────────────

pub fn help_find_tool(ctx: Arc<McpContext>, query: String) -> Result<String, String> {
    // Simple keyword search across all tool names and descriptions
    let query_lower = query.to_lowercase();
    let all_tools: Vec<(&str, &str)> = vec![
        ("info_game_summary", "Overview of current game state"),
        ("info_standings", "League table"),
        ("info_fixtures", "Upcoming and recent fixtures"),
        ("info_player_profile", "Detailed player card"),
        ("info_player_stats", "Player season/career stats"),
        ("info_team_profile", "Detailed team view"),
        ("info_team_stats", "Team season stats"),
        ("info_finances", "Financial overview"),
        ("info_news", "Recent news"),
        ("info_season_context", "Season phase and transfer window"),
        ("info_match_preview", "Next match preview"),
        ("time_advance", "Advance one day"),
        ("time_skip_to_match_day", "Fast-forward to match day"),
        ("squad_get", "Squad overview"),
        ("squad_set_formation", "Change formation"),
        ("squad_set_starting_xi", "Set starting eleven"),
        ("squad_set_play_style", "Change play style"),
        ("squad_set_match_roles", "Set captain and set-piece takers"),
        ("squad_auto_set_pieces", "Auto-assign set-piece takers"),
        ("squad_set_player_role", "Set player squad role"),
        ("training_get", "Training settings"),
        ("training_set_focus_intensity", "Set training focus and intensity"),
        ("transfer_make_bid", "Make a transfer bid"),
        ("transfer_toggle_listed", "Toggle transfer-listed status"),
        ("transfer_toggle_loan", "Toggle loan-listed status"),
        ("contract_propose_renewal", "Propose contract renewal"),
        ("contract_delegate_renewals", "Delegate contract renewals"),
        ("inbox_get_messages", "Get inbox messages"),
        ("club_upgrade_facility", "Upgrade facility"),
        ("staff_hire", "Hire staff"),
        ("season_advance", "Advance season"),
        ("game_save", "Save current game"),
        ("game_is_finished", "Check if game is finished"),
    ];

    let matches: Vec<_> = all_tools.iter()
        .filter(|(name, desc)| name.contains(&query_lower) || desc.to_lowercase().contains(&query_lower))
        .collect();

    if matches.is_empty() {
        return Ok(format!("## Tool Search: '{}'\n\nNo tools found matching your query. Try `help_list_categories`.", query));
    }

    let mut output = format!("## Tool Search: '{}'\n\n| Tool | Description |\n|------|-------------|\n", query);
    for (name, desc) in matches {
        output.push_str(&format!("| {} | {} |\n", name, desc));
    }

    Ok(output)
}

// ─── help_list_categories ───────────────────────────────────────────────────

pub fn help_list_categories() -> String {
    let categories: Vec<(&str, &[&str])> = vec![
        ("📋 Information", &["info_game_summary", "info_standings", "info_fixtures", "info_player_profile", "info_finances", "info_news", "info_season_context", "info_match_preview"]),
        ("⏰ Time", &["time_advance", "time_skip_to_match_day", "time_check_blockers"]),
        ("⚽ Squad", &["squad_get", "squad_set_formation", "squad_set_starting_xi", "squad_set_play_style", "squad_set_match_roles", "squad_auto_set_pieces"]),
        ("🏋️ Training", &["training_get", "training_set_focus_intensity", "training_set_schedule", "training_set_groups"]),
        ("💰 Transfers", &["transfer_make_bid", "transfer_toggle_listed", "transfer_toggle_loan", "transfer_respond_to_offer", "transfer_counter_offer", "transfer_free_agent_offer"]),
        ("📝 Contracts", &["contract_propose_renewal", "contract_delegate_renewals", "contract_set_exit_intent", "contract_terminate"]),
        ("📬 Inbox", &["inbox_get_messages", "inbox_mark_read", "inbox_mark_all_read", "inbox_resolve_action"]),
        ("🏢 Club", &["club_upgrade_facility", "staff_hire", "staff_release"]),
        ("🗓️ Season", &["season_check_complete", "season_advance"]),
        ("💾 Game", &["game_save", "game_is_finished"]),
        ("❓ Help", &["help_find_tool", "help_list_categories"]),
    ];

    let mut output = String::from("## Tool Categories\n\n");
    for (cat, tools) in &categories {
        output.push_str(&format!("**{}** ({} tools): {}\n\n", cat, tools.len(), tools.join(", ")));
    }

    output
}
