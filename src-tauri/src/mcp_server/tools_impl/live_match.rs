//! MCP tool implementations: live match and match interaction tools.

use std::sync::Arc;

use crate::mcp_server::context::McpContext;
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
        None,
        None,
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

    // No minute count in the header. It reported the *requested* minutes, which a step that stops
    // early at half time or full time does not deliver; counting the returned results is no better,
    // because a MinuteResult is also produced for phase transitions that play no minute (kick-off,
    // the restart after an interval). Both numbers contradicted the `Minute:` line directly below.
    //
    // The snapshot's own minute is the truth an agent needs, and the phase tells it why the step
    // stopped — which is the actionable part, since a stop means the match is waiting on a decision.
    Ok(format!(
        "## Match Advanced\n\n**Minute**: {}\n**Phase**: {:?}\n**Score**: {} - {}\n\n### Events\n{}",
        snapshot.current_minute,
        snapshot.phase,
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
    let seed = rand::random::<u64>();
    // Resolves the manager's team before the loop that adjusts morale, so an
    // error path never leaves a half-applied team talk behind.
    let results = ctx
        .state_manager
        .update_game(|game| {
            crate::commands::live_match::apply_team_talk_internal(game, &tone, &context, seed)
        })
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())??;

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

/// One press conference answer, as the agent submits it.
#[derive(serde::Deserialize)]
struct PressAnswer {
    question_id: String,
    response_id: String,
    response_text: String,
    #[serde(default)]
    player_id: String,
}

/// What a press conference did, once applied: the squad morale it moved and the match it was about.
struct PressConferenceOutcome {
    squad_morale_delta: i16,
    home_team_name: String,
    away_team_name: String,
    home_score: u8,
    away_score: u8,
}

/// Applies a press conference to the game — individual morale, squad morale, and the news article.
///
/// Every failure is resolved before the first mutation. `update_game` cannot roll back a closure
/// that returns `Err`, so a check placed after a morale change would leave the squad's mood moved
/// with no article to explain it.
fn apply_press_conference(
    game: &mut ofm_core::game::Game,
    answers: &[PressAnswer],
) -> Result<PressConferenceOutcome, String> {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    // Derive user team and last match result from game state
    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or("No team assigned to manager")?;
    let user_team_name = game
        .teams
        .iter()
        .find(|t| t.id == user_team_id)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| user_team_id.clone());

    // Find the most recent completed fixture involving the user's team. Copied out of the league
    // borrow here so the morale loops below can take `game` mutably.
    let (home_team_id, away_team_id, home_score, away_score) = {
        let last_match = game
            .league
            .as_ref()
            .and_then(|league| {
                league
                    .fixtures
                    .iter()
                    .filter(|f| {
                        f.result.is_some()
                            && (f.home_team_id == user_team_id || f.away_team_id == user_team_id)
                    })
                    .max_by(|a, b| a.date.cmp(&b.date))
            })
            .ok_or("No completed match found for your team")?;
        let result = last_match
            .result
            .as_ref()
            .expect("filtered to fixtures with a result");
        (
            last_match.home_team_id.clone(),
            last_match.away_team_id.clone(),
            result.home_goals,
            result.away_goals,
        )
    };

    let team_name = |id: &str| {
        game.teams
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let home_team_name = team_name(&home_team_id);
    let away_team_name = team_name(&away_team_id);

    let mut morale_delta: i16 = 0;
    let mut mentioned_player_ids: Vec<String> = Vec::new();
    let mut quotes: Vec<String> = Vec::new();

    // Past this point nothing returns `Err` — see the note above.
    for answer in answers {
        if !answer.response_text.is_empty() {
            quotes.push(format!("\"{}\"", answer.response_text));
        }
        if !answer.player_id.is_empty() {
            mentioned_player_ids.push(answer.player_id.clone());
        }

        let rid = answer.response_id.as_str();
        match rid {
            "humble" | "fair" | "positive" | "focused" | "grateful" | "patience" | "appreciate"
            | "understand" => morale_delta += 2,
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

    let result_str = format!(
        "{} {} - {} {}",
        home_team_name, home_score, away_score, away_team_name
    );
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

    Ok(PressConferenceOutcome {
        squad_morale_delta: morale_delta,
        home_team_name,
        away_team_name,
        home_score,
        away_score,
    })
}

/// Submit press conference answers after a match.
/// Derives team names, scores, and user team from the current game state
/// to prevent fabrication of match results.
pub fn match_press_conference(
    ctx: Arc<McpContext>,
    answers_json: String,
) -> Result<String, String> {
    let answers: Vec<PressAnswer> =
        serde_json::from_str(&answers_json).map_err(|e| format!("Invalid answers JSON: {}", e))?;

    // The morale it moves and the article it files are one press conference, so they go in under
    // a single lock rather than being computed against a clone that only partly makes it back.
    let outcome = ctx
        .state_manager
        .update_game(|game| apply_press_conference(game, &answers))
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())??;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    let emoji = if outcome.squad_morale_delta > 0 {
        "📈"
    } else if outcome.squad_morale_delta < 0 {
        "📉"
    } else {
        "➡️"
    };
    Ok(format!(
        "## Press Conference Complete\n\n{} Squad morale: {:+}\n**Match**: {} {} - {} {}",
        emoji,
        outcome.squad_morale_delta,
        outcome.home_team_name,
        outcome.home_score,
        outcome.away_score,
        outcome.away_team_name
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn attributes() -> domain::player::PlayerAttributes {
        domain::player::PlayerAttributes {
            pace: 70,
            stamina: 72,
            strength: 65,
            agility: 68,
            passing: 74,
            shooting: 61,
            tackling: 58,
            dribbling: 69,
            defending: 56,
            positioning: 67,
            vision: 73,
            decisions: 71,
            composure: 66,
            aggression: 54,
            teamwork: 76,
            leadership: 49,
            handling: 20,
            reflexes: 24,
            aerial: 44,
        }
    }

    fn player(id: &str, team_id: &str) -> domain::player::Player {
        let mut p = domain::player::Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "1998-04-02".to_string(),
            "England".to_string(),
            domain::player::Position::Midfielder,
            attributes(),
        );
        p.team_id = Some(team_id.to_string());
        p.morale = 50;
        p
    }

    fn team(id: &str, name: &str) -> domain::team::Team {
        domain::team::Team::new(
            id.to_string(),
            name.to_string(),
            name.to_string(),
            "England".to_string(),
            "Somewhere".to_string(),
            "The Ground".to_string(),
            10_000,
        )
    }

    /// A game whose user team has just played — the precondition every press conference needs.
    fn game_after_a_match() -> ofm_core::game::Game {
        let clock = ofm_core::clock::GameClock::new(
            chrono::Utc.with_ymd_and_hms(2026, 3, 14, 12, 0, 0).unwrap(),
        );
        let mut manager = domain::manager::Manager::new(
            "mgr1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());

        let players = vec![
            player("p1", "team1"),
            player("p2", "team1"),
            player("p3", "team2"),
        ];

        let fixture = domain::league::Fixture {
            id: "fix1".to_string(),
            matchday: 1,
            date: "2026-03-13".to_string(),
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: domain::league::FixtureCompetition::League,
            status: domain::league::FixtureStatus::Completed,
            result: Some(domain::league::MatchResult {
                home_goals: 2,
                away_goals: 1,
                ..Default::default()
            }),
            ..Default::default()
        };

        let league = domain::league::League {
            id: "league1".to_string(),
            name: "Test League".to_string(),
            season: 1,
            fixtures: vec![fixture],
            ..Default::default()
        };

        let mut game = ofm_core::game::Game::new(
            clock,
            manager,
            vec![team("team1", "Test FC"), team("team2", "Rival FC")],
            players,
            vec![],
            vec![],
        );
        game.league = Some(league);
        game
    }

    fn answer(question_id: &str, response_id: &str, player_id: &str) -> PressAnswer {
        PressAnswer {
            question_id: question_id.to_string(),
            response_id: response_id.to_string(),
            response_text: "We go again.".to_string(),
            player_id: player_id.to_string(),
        }
    }

    /// The regression this module exists for: morale used to be applied to a clone of the game
    /// that was then thrown away, so only the news article survived the tool call.
    #[test]
    fn squad_morale_survives_the_press_conference() {
        let mut game = game_after_a_match();

        let outcome = apply_press_conference(&mut game, &[answer("mood", "confident", "")])
            .expect("the user team has a completed fixture");

        assert_eq!(outcome.squad_morale_delta, 3);
        let moved: Vec<u8> = game
            .players
            .iter()
            .filter(|p| p.team_id.as_deref() == Some("team1"))
            .map(|p| p.morale)
            .collect();
        assert_eq!(
            moved,
            vec![53, 53],
            "every player in the user's squad should carry the morale change"
        );
    }

    #[test]
    fn only_the_user_squad_takes_the_morale_change() {
        let mut game = game_after_a_match();

        apply_press_conference(&mut game, &[answer("mood", "confident", "")]).unwrap();

        let rival = game.players.iter().find(|p| p.id == "p3").unwrap();
        assert_eq!(rival.morale, 50, "the opposition hears no team talk");
    }

    /// A `player_focus` answer moves that player on top of the squad-wide change.
    #[test]
    fn naming_a_player_moves_that_player_further() {
        let mut game = game_after_a_match();

        apply_press_conference(&mut game, &[answer("player_focus", "praise", "p1")]).unwrap();

        let named = game.players.iter().find(|p| p.id == "p1").unwrap();
        let unnamed = game.players.iter().find(|p| p.id == "p2").unwrap();
        assert_eq!(named.morale, 59, "50 + 5 individual + 4 squad-wide");
        assert_eq!(unnamed.morale, 54, "50 + 4 squad-wide");
    }

    #[test]
    fn the_conference_files_one_news_article() {
        let mut game = game_after_a_match();

        apply_press_conference(&mut game, &[answer("mood", "confident", "")]).unwrap();

        assert_eq!(game.news.len(), 1);
        assert_eq!(game.news[0].id, "press_conf_2026-03-14");
    }

    /// Nothing may be half-applied: `update_game` cannot roll back, so a rejected conference has
    /// to leave the game exactly as it found it.
    #[test]
    fn a_manager_without_a_team_changes_nothing() {
        let mut game = game_after_a_match();
        game.manager.team_id = None;

        let result = apply_press_conference(&mut game, &[answer("mood", "confident", "")]);

        assert!(result.is_err());
        assert!(game.players.iter().all(|p| p.morale == 50));
        assert!(game.news.is_empty());
    }

    #[test]
    fn a_team_that_has_not_played_changes_nothing() {
        let mut game = game_after_a_match();
        if let Some(league) = game.league.as_mut() {
            league.fixtures[0].result = None;
        }

        let result = apply_press_conference(&mut game, &[answer("mood", "confident", "")]);

        assert!(result.is_err());
        assert!(game.players.iter().all(|p| p.morale == 50));
        assert!(game.news.is_empty());
    }

    #[test]
    fn the_outcome_reports_the_match_it_covered() {
        let mut game = game_after_a_match();

        let outcome =
            apply_press_conference(&mut game, &[answer("mood", "confident", "")]).unwrap();

        assert_eq!(outcome.home_team_name, "Test FC");
        assert_eq!(outcome.away_team_name, "Rival FC");
        assert_eq!(outcome.home_score, 2);
        assert_eq!(outcome.away_score, 1);
    }
}
