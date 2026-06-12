//! MCP tool implementations: live match and match interaction tools.

use std::sync::Arc;

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools_impl::helpers::require_game;
use crate::mcp_server::formatting::translate_error;

/// Format a match event as a readable string.
fn fmt_event(event: &engine::MatchEvent) -> String {
    let side = match event.side {
        engine::Side::Home => "Home",
        engine::Side::Away => "Away",
    };
    let player = event.player_id.as_deref().unwrap_or("?");
    let desc = format!("{:?}", event.event_type);
    format!("{}': {} {} ({})", event.minute, side, desc, player)
}

/// Start a live match for a given fixture index.
/// mode: "live" | "spectator" | "instant"
pub fn match_start(
    ctx: Arc<McpContext>,
    fixture_index: u32,
    mode: String,
    allows_extra_time: Option<bool>,
) -> Result<String, String> {
    let fixture_idx = fixture_index as usize;
    let allows_et = allows_extra_time.unwrap_or(true);

    let snapshot = crate::application::live_match::start_live_match(
        &ctx.state_manager,
        fixture_idx,
        &mode,
        allows_et,
    )
    .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!(
        "## Live Match Started\n\n**Fixture Index**: {}\n**Mode**: {}\n**Minute**: {}\n**Score**: {} - {}\n\nUse `match_step` to advance, `match_command` to issue tactical commands, and `match_finish` to end.",
        fixture_index, mode, snapshot.current_minute, snapshot.home_score, snapshot.away_score
    ))
}

/// Step the live match forward by N minutes.
pub fn match_step(ctx: Arc<McpContext>, minutes: u16) -> Result<String, String> {
    let results = crate::application::live_match::step_live_match(
        &ctx.state_manager,
        minutes,
    )
    .map_err(|e| translate_error(&e))?;

    let mut lines: Vec<String> = Vec::new();
    for result in &results {
        for event in &result.events {
            lines.push(fmt_event(event));
        }
    }

    // Get the latest snapshot for score
    let snapshot = crate::application::live_match::get_match_snapshot(&ctx.state_manager)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let events_text = if lines.is_empty() {
        "No events occurred.".to_string()
    } else {
        lines.join("\n")
    };

    Ok(format!(
        "## Match Advanced {} Minutes\n\n**Minute**: {}\n**Score**: {} - {}\n\n### Events\n{}",
        minutes,
        snapshot.current_minute,
        snapshot.home_score,
        snapshot.away_score,
        events_text
    ))
}

/// Apply a match command (substitution, tactic change, set piece taker, etc.)
pub fn match_command(
    ctx: Arc<McpContext>,
    command_json: String,
) -> Result<String, String> {
    let command: engine::MatchCommand = serde_json::from_str(&command_json)
        .map_err(|e| format!("Invalid match command JSON: {}", e))?;

    let snapshot = crate::application::live_match::apply_match_command(
        &ctx.state_manager,
        command,
    )
    .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format!(
        "## Command Applied\n\n**Minute**: {}\n**Score**: {} - {}",
        snapshot.current_minute,
        snapshot.home_score,
        snapshot.away_score
    ))
}

/// Get current match snapshot without advancing time.
pub fn match_snapshot(ctx: Arc<McpContext>) -> Result<String, String> {
    let snapshot = crate::application::live_match::get_match_snapshot(&ctx.state_manager)
        .map_err(|e| translate_error(&e))?;

    Ok(format!(
        "## Match Snapshot\n\n**Minute**: {}\n**Score**: {} - {}\n**Phase**: {:?}\n**Possession**: Home {:.0}% / Away {:.0}%",
        snapshot.current_minute,
        snapshot.home_score,
        snapshot.away_score,
        snapshot.phase,
        snapshot.home_possession_pct * 100.0,
        snapshot.away_possession_pct * 100.0,
    ))
}

/// Finish the live match: generate report, update game state, clean up.
pub fn match_finish(ctx: Arc<McpContext>) -> Result<String, String> {
    let response = crate::application::live_match::finish_live_match(&ctx.state_manager)
        .map_err(|e| translate_error(&e))?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let round_text = if let Some(ref summary) = response.round_summary {
        let results: Vec<String> = summary.completed_results.iter().map(|r| {
            format!("- {} {} - {} {}", r.home_team_name, r.home_goals, r.away_goals, r.away_team_name)
        }).collect();
        format!("\n\n### Round Results\n{}", results.join("\n"))
    } else {
        String::new()
    };

    Ok(format!(
        "## Match Finished\n\n**Date**: {}{}",
        response.game.clock.current_date.format("%d %B %Y"),
        round_text
    ))
}

/// Apply a team talk during a match (half-time or full-time).
/// tone: "calm" | "motivational" | "assertive" | "aggressive" | "praise" | "disappointed"
/// context: "winning" | "losing" | "drawing"
pub fn match_team_talk(
    ctx: Arc<McpContext>,
    tone: String,
    context: String,
) -> Result<String, String> {
    let mut game = require_game(&ctx.state_manager)?;

    let seed = rand::random::<u64>();
    let results = crate::commands::live_match::apply_team_talk_internal(
        &mut game, &tone, &context, seed,
    )?;

    ctx.state_manager.set_game(game);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let mut lines = Vec::new();
    for result in &results {
        let pid = result["player_id"].as_str().unwrap_or("?");
        let delta = result["delta"].as_i64().unwrap_or(0);
        let emoji = if delta > 0 { "📈" } else if delta < 0 { "📉" } else { "➡️" };
        lines.push(format!("- {} {}: morale {:+}", emoji, pid, delta));
    }

    let reactions = if lines.is_empty() {
        "No morale changes.".to_string()
    } else {
        lines.join("\n")
    };

    Ok(format!(
        "## Team Talk Applied\n\n**Tone**: {}\n**Context**: {}\n\n### Player Reactions\n{}",
        tone, context, reactions
    ))
}

/// Submit press conference answers after a match.
/// Derives team names, scores, and user team from the current game state
/// to prevent fabrication of match results.
pub fn match_press_conference(
    ctx: Arc<McpContext>,
    answers_json: String,
) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct PressAnswer {
        question_id: String,
        response_id: String,
        response_text: String,
        #[serde(default)]
        player_id: String,
    }

    let answers: Vec<PressAnswer> = serde_json::from_str(&answers_json)
        .map_err(|e| format!("Invalid answers JSON: {}", e))?;

    let mut game = require_game(&ctx.state_manager)?;
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    // Derive user team and last match result from game state
    let user_team_id = game.manager.team_id.clone()
        .ok_or("No team assigned to manager")?;
    let user_team_name = game.teams.iter()
        .find(|t| t.id == user_team_id)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| user_team_id.clone());

    // Find the most recent completed fixture involving the user's team
    let last_match = game.league.as_ref()
        .and_then(|league| league.fixtures.iter()
            .filter(|f| f.result.is_some() && (f.home_team_id == user_team_id || f.away_team_id == user_team_id))
            .max_by(|a, b| a.date.cmp(&b.date)))
        .ok_or("No completed match found for your team")?;

    let (home_team_name, away_team_name) = {
        let home = game.teams.iter().find(|t| t.id == last_match.home_team_id).map(|t| t.name.clone()).unwrap_or_else(|| last_match.home_team_id.clone());
        let away = game.teams.iter().find(|t| t.id == last_match.away_team_id).map(|t| t.name.clone()).unwrap_or_else(|| last_match.away_team_id.clone());
        (home, away)
    };
    let home_score = last_match.result.as_ref().unwrap().home_goals;
    let away_score = last_match.result.as_ref().unwrap().away_goals;

    let mut morale_delta: i16 = 0;
    let mut mentioned_player_ids: Vec<String> = Vec::new();
    let mut quotes: Vec<String> = Vec::new();

    for answer in &answers {
        if !answer.response_text.is_empty() {
            quotes.push(format!("\"{}\"", answer.response_text));
        }
        if !answer.player_id.is_empty() {
            mentioned_player_ids.push(answer.player_id.clone());
        }

        let rid = answer.response_id.as_str();
        match rid {
            "humble" | "fair" | "positive" | "focused" | "grateful" | "patience" | "appreciate" | "understand" => morale_delta += 2,
            "confident" | "ambitious" | "shared" => morale_delta += 3,
            "defiant" | "frustrated" => morale_delta += 0,
            "curt" | "evasive" => morale_delta -= 1,
            "accept" | "detailed" | "apologize" => morale_delta += 1,
            "deflect" => {}
            "praise" => morale_delta += 4,
            "demanding" => morale_delta += 1,
            _ => {}
        }

        if answer.question_id == "player_focus" && !answer.player_id.is_empty() {
            let player_delta: i16 = match rid {
                "praise" => 5,
                "demanding" => 0,
                "deflect" => -1,
                _ => 2,
            };
            if let Some(p) = game.players.iter_mut().find(|p| p.id == answer.player_id) {
                p.morale = ((p.morale as i16) + player_delta).clamp(10, 100) as u8;
            }
        }
    }

    morale_delta = morale_delta.clamp(-8, 8);
    if morale_delta != 0 {
        for p in game.players.iter_mut() {
            if p.team_id.as_deref() == Some(&user_team_id) {
                p.morale = ((p.morale as i16) + morale_delta).clamp(10, 100) as u8;
            }
        }
    }

    let result_str = format!("{} {} - {} {}", home_team_name, home_score, away_score, away_team_name);
    let headline_key = if quotes.is_empty() {
        "be.news.pressConference.headlinePostMatch"
    } else {
        "be.news.pressConference.headlineManagerQuote"
    };
    let body_key = if quotes.len() > 1 {
        "be.news.pressConference.bodyMultiple"
    } else if quotes.len() == 1 {
        "be.news.pressConference.bodySingle"
    } else {
        "be.news.pressConference.bodyNone"
    };

    let mut i18n_params = std::collections::HashMap::new();
    i18n_params.insert("team".to_string(), user_team_name);
    i18n_params.insert("result".to_string(), result_str);
    if !quotes.is_empty() {
        i18n_params.insert("quote".to_string(), quotes[0].trim_matches('"').to_string());
    }

    let article_id = format!("press_conf_{}", today);
    let article = domain::news::NewsArticle::new(
        article_id,
        String::new(),
        String::new(),
        String::new(),
        today,
        domain::news::NewsCategory::MatchReport,
    )
    .with_teams(vec![user_team_id])
    .with_players(mentioned_player_ids)
    .with_i18n(headline_key, body_key, "be.source.sportsDaily", i18n_params);

    game.news.push(article);
    ctx.state_manager.set_game(game);

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let emoji = if morale_delta > 0 { "📈" } else if morale_delta < 0 { "📉" } else { "➡️" };
    Ok(format!(
        "## Press Conference Complete\n\n{} Squad morale: {:+}\n**Match**: {} {} - {} {}",
        emoji, morale_delta, home_team_name, home_score, away_score, away_team_name
    ))
}
