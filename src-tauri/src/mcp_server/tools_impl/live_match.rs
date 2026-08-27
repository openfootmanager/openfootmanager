//! MCP tool implementations: live match and match interaction tools.

use std::sync::Arc;

use crate::application::press_conference::{
    first_player_outside_squad, last_completed_match, todays_article_id,
    MAX_PRESS_CONFERENCE_ANSWERS, MAX_RESPONSE_TEXT_CHARS,
};
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
#[derive(Debug)]
struct PressConferenceOutcome {
    squad_morale_delta: i16,
    /// How many of the user's players actually ended the conference on a different morale, and
    /// how many there were. Both effects clamp, so the delta alone says nothing about movement.
    squad_players_moved: usize,
    squad_size: usize,
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

    // One conference per game day.
    let (article_id, already_held) = todays_article_id(game);
    if already_held {
        return Err("A press conference has already been held today.".to_string());
    }

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

    // Every named player must be one the user actually manages, checked before anything moves.
    let outsider = first_player_outside_squad(
        game,
        &user_team_id,
        answers.iter().map(|answer| answer.player_id.as_str()),
    );
    if let Some(id) = outsider {
        return Err(format!("Player {} is not in your squad.", id));
    }

    // The match the conference is about — the last one played in any competition, not the last
    // league match. Copied out of the borrow so the morale loops below can take `game` mutably.
    let (home_team_id, away_team_id, home_score, away_score) =
        last_completed_match(game, &user_team_id)
            .ok_or("No completed match found for your team")?;

    let team_name = |id: &str| {
        game.teams
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let home_team_name = team_name(&home_team_id);
    let away_team_name = team_name(&away_team_id);

    // The squad's morale before anything moves, so the outcome can report what changed rather
    // than what was asked for. Both effects below clamp to 10..=100, so a squad already at the
    // ceiling absorbs a "+3" whole — reporting the nominal delta would tell the caller its praise
    // landed when not one player moved. Indices are stable: nothing here adds or removes players.
    let squad_before: Vec<(usize, u8)> = game
        .players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.team_id.as_deref() == Some(&user_team_id))
        .map(|(index, p)| (index, p.morale))
        .collect();

    let mut morale_delta: i16 = 0;
    let mut mentioned_player_ids: Vec<String> = Vec::new();
    // Only the first quote is ever rendered; the rest only decide singular vs plural wording.
    let mut first_quote: Option<String> = None;
    let mut quote_count: usize = 0;

    // Past this point nothing returns `Err` — see the note above.
    for answer in answers {
        if !answer.response_text.is_empty() {
            quote_count += 1;
            first_quote.get_or_insert_with(|| answer.response_text.clone());
        }
        if !answer.player_id.is_empty() {
            mentioned_player_ids.push(answer.player_id.clone());
        }

        let rid = answer.response_id.as_str();
        // Saturating, because the answer list is agent-supplied and unbounded. Plain `+=` on an
        // i16 overflows after roughly 10,900 positive answers, and this loop runs while the game
        // mutex is held — a panic here would poison it and take the session down, after some
        // morale had already moved. The final clamp to ±8 puts the saturation point out of reach
        // for any input that is not already nonsense.
        match rid {
            "humble" | "fair" | "positive" | "focused" | "grateful" | "patience" | "appreciate"
            | "understand" => morale_delta = morale_delta.saturating_add(2),
            "confident" | "ambitious" | "shared" => morale_delta = morale_delta.saturating_add(3),
            // Deliberately neutral rather than unhandled.
            "defiant" | "frustrated" => {}
            "curt" | "evasive" => morale_delta = morale_delta.saturating_sub(1),
            "accept" | "detailed" | "apologize" => morale_delta = morale_delta.saturating_add(1),
            "deflect" => {}
            "praise" => morale_delta = morale_delta.saturating_add(4),
            "demanding" => morale_delta = morale_delta.saturating_add(1),
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

    let squad_players_moved = squad_before
        .iter()
        .filter(|(index, before)| game.players[*index].morale != *before)
        .count();

    let result_str = format!(
        "{} {} - {} {}",
        home_team_name, home_score, away_score, away_team_name
    );
    let headline_key = if quote_count == 0 {
        "be.news.pressConference.headlinePostMatch"
    } else {
        "be.news.pressConference.headlineManagerQuote"
    };
    let body_key = if quote_count > 1 {
        "be.news.pressConference.bodyMultiple"
    } else if quote_count == 1 {
        "be.news.pressConference.bodySingle"
    } else {
        "be.news.pressConference.bodyNone"
    };

    let mut i18n_params = std::collections::HashMap::new();
    i18n_params.insert("team".to_string(), user_team_name);
    i18n_params.insert("result".to_string(), result_str);
    if let Some(quote) = first_quote {
        i18n_params.insert("quote".to_string(), quote.trim_matches('"').to_string());
    }

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
        squad_players_moved,
        squad_size: squad_before.len(),
        home_team_name,
        away_team_name,
        home_score,
        away_score,
    })
}

/// Rejects a request too large to walk while holding the game mutex.
///
/// Called before `press_conference_on`, deliberately: the loop it guards runs with the mutex
/// held, so the point is to refuse an oversized request *without* having taken the lock.
fn check_answer_limits(answers: &[PressAnswer]) -> Result<(), String> {
    if answers.len() > MAX_PRESS_CONFERENCE_ANSWERS {
        return Err(format!(
            "Too many answers: {} submitted, at most {} accepted.",
            answers.len(),
            MAX_PRESS_CONFERENCE_ANSWERS
        ));
    }

    if let Some(answer) = answers
        .iter()
        .find(|answer| answer.response_text.chars().count() > MAX_RESPONSE_TEXT_CHARS)
    {
        return Err(format!(
            "Answer to `{}` is too long: {} characters, at most {} accepted.",
            answer.question_id,
            answer.response_text.chars().count(),
            MAX_RESPONSE_TEXT_CHARS
        ));
    }

    Ok(())
}

/// Runs a press conference against the active game.
///
/// The morale it moves and the article it files are one press conference, so they go in under a
/// single lock rather than being computed against a clone that only partly makes it back.
///
/// Split from the tool wrapper so the write-back itself is reachable from a test: the wrapper
/// needs a Tauri `AppHandle`, this needs only a `StateManager`. Testing `apply_press_conference`
/// alone leaves the persistence — the part that actually regressed — uncovered.
fn press_conference_on(
    state: &ofm_core::state::StateManager,
    answers: &[PressAnswer],
) -> Result<PressConferenceOutcome, String> {
    state
        .update_game(|game| apply_press_conference(game, answers))
        .ok_or_else(|| "be.error.noActiveGameSession".to_string())?
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

    check_answer_limits(&answers)?;

    let outcome = press_conference_on(&ctx.state_manager, &answers)?;

    {
        use tauri::Emitter;
        let _ = ctx.app_handle.emit("game-state-changed", ());
    }

    Ok(format_outcome(&outcome))
}

/// Renders the outcome for the agent that asked for it.
///
/// The movement count is not decoration: a squad already on 100 absorbs a "+3" entirely, and a
/// `deflect` on a player question moves one player while leaving the squad delta at zero.
/// Reporting the delta on its own told the caller praise had worked when nothing had happened —
/// and this call is its only window onto morale, so it would learn the wrong lesson.
fn format_outcome(outcome: &PressConferenceOutcome) -> String {
    let emoji = match (outcome.squad_players_moved, outcome.squad_morale_delta) {
        (0, _) => "➡️",
        (_, delta) if delta > 0 => "📈",
        (_, delta) if delta < 0 => "📉",
        _ => "➡️",
    };
    format!(
        "## Press Conference Complete\n\n{} Squad morale {:+}: {} of {} players moved\n**Match**: {} {} - {} {}",
        emoji,
        outcome.squad_morale_delta,
        outcome.squad_players_moved,
        outcome.squad_size,
        outcome.home_team_name,
        outcome.home_score,
        outcome.away_score,
        outcome.away_team_name
    )
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
        game.competitions.push(league);
        // What every load path does (`db::game_persistence`), so the fixture reaches the state
        // production actually reaches: `competitions` is the source of truth, `league` its mirror.
        game.promote_legacy_league();
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
        game.competitions[0].fixtures[0].result = None;

        let result = apply_press_conference(&mut game, &[answer("mood", "confident", "")]);

        assert!(result.is_err());
        assert!(game.players.iter().all(|p| p.morale == 50));
        assert!(game.news.is_empty());
    }

    /// The answer list arrives from an agent and is unbounded. `i16` accumulation overflows after
    /// roughly 10,900 positive answers, and this runs while the game mutex is held — an overflow
    /// panic would poison it and end the session, after some morale had already moved.
    #[test]
    fn an_absurd_number_of_answers_saturates_instead_of_overflowing() {
        let mut game = game_after_a_match();
        let answers: Vec<PressAnswer> = (0..20_000)
            .map(|_| answer("mood", "confident", ""))
            .collect();

        let outcome = apply_press_conference(&mut game, &answers)
            .expect("a long answer list is still a valid conference");

        assert_eq!(
            outcome.squad_morale_delta, 8,
            "clamped, not wrapped negative"
        );
        assert!(game.players.iter().all(|p| p.morale <= 100));
    }

    /// A second conference on the same day would file an article sharing the first one's id — the
    /// id the news list keys on — and re-apply the squad delta.
    #[test]
    fn a_second_conference_on_the_same_day_is_rejected() {
        let mut game = game_after_a_match();
        apply_press_conference(&mut game, &[answer("mood", "confident", "")]).unwrap();

        let result = apply_press_conference(&mut game, &[answer("mood", "confident", "")]);

        assert!(result.is_err());
        assert_eq!(game.news.len(), 1, "no second article");
        assert!(
            game.players
                .iter()
                .filter(|p| p.team_id.as_deref() == Some("team1"))
                .all(|p| p.morale == 53),
            "morale must not stack on a rejected repeat"
        );
    }

    /// The guard is keyed on the game date, so advancing the clock allows another one.
    #[test]
    fn the_next_day_allows_another_conference() {
        let mut game = game_after_a_match();
        apply_press_conference(&mut game, &[answer("mood", "confident", "")]).unwrap();

        game.clock.current_date = chrono::Utc.with_ymd_and_hms(2026, 3, 15, 12, 0, 0).unwrap();
        apply_press_conference(&mut game, &[answer("mood", "confident", "")])
            .expect("a new day is a new conference");

        assert_eq!(game.news.len(), 2);
    }

    /// Covers the write-back, not just the arithmetic. Every other test here drives
    /// `apply_press_conference` against a game it owns, which cannot tell whether the tool
    /// persists anything — the exact thing that regressed. This one goes through a real
    /// `StateManager` and reads the stored game back out.
    #[test]
    fn the_conference_reaches_the_stored_game() {
        let state = ofm_core::state::StateManager::new();
        state.set_game(game_after_a_match());

        let outcome = press_conference_on(&state, &[answer("mood", "confident", "")])
            .expect("the stored game has a completed fixture");

        assert_eq!(outcome.squad_morale_delta, 3);
        let (morale, articles) = state
            .get_game(|g| {
                (
                    g.players
                        .iter()
                        .filter(|p| p.team_id.as_deref() == Some("team1"))
                        .map(|p| p.morale)
                        .collect::<Vec<u8>>(),
                    g.news.len(),
                )
            })
            .expect("a game is active");

        assert_eq!(
            morale,
            vec![53, 53],
            "morale must survive in the stored game"
        );
        assert_eq!(articles, 1, "the article must survive in the stored game");
    }

    #[test]
    fn a_conference_without_an_active_game_is_rejected() {
        let state = ofm_core::state::StateManager::new();

        let result = press_conference_on(&state, &[answer("mood", "confident", "")]);

        assert_eq!(result.unwrap_err(), "be.error.noActiveGameSession");
    }

    fn quoted(text: &str) -> PressAnswer {
        PressAnswer {
            question_id: "mood".to_string(),
            response_id: "confident".to_string(),
            response_text: text.to_string(),
            player_id: String::new(),
        }
    }

    fn body_key_of(game: &ofm_core::game::Game) -> String {
        game.news[0]
            .body_key
            .clone()
            .expect("the article carries an i18n body key")
    }

    #[test]
    fn too_many_answers_are_refused_before_the_lock() {
        let answers: Vec<PressAnswer> = (0..MAX_PRESS_CONFERENCE_ANSWERS + 1)
            .map(|_| answer("mood", "confident", ""))
            .collect();

        let err = check_answer_limits(&answers).unwrap_err();

        assert!(err.contains("Too many answers"), "got: {err}");
        assert!(check_answer_limits(&answers[..MAX_PRESS_CONFERENCE_ANSWERS]).is_ok());
    }

    #[test]
    fn an_overlong_answer_is_refused_before_the_lock() {
        let long = "x".repeat(MAX_RESPONSE_TEXT_CHARS + 1);

        let err = check_answer_limits(&[quoted(&long)]).unwrap_err();

        assert!(err.contains("too long"), "got: {err}");
        assert!(check_answer_limits(&[quoted(&"x".repeat(MAX_RESPONSE_TEXT_CHARS))]).is_ok());
    }

    /// The limit counts characters, not bytes, so a multi-byte quote is not penalised for its
    /// encoding — and `chars().count()` on a `String` cannot panic mid-character the way slicing
    /// would.
    #[test]
    fn the_answer_limit_counts_characters_not_bytes() {
        let accented = "é".repeat(MAX_RESPONSE_TEXT_CHARS);

        assert!(check_answer_limits(&[quoted(&accented)]).is_ok());
    }

    /// Only the first quote reaches the article, but the number of them picks the wording.
    #[test]
    fn the_article_wording_follows_the_number_of_quotes() {
        let mut none = game_after_a_match();
        apply_press_conference(&mut none, &[quoted("")]).unwrap();
        assert_eq!(body_key_of(&none), "be.news.pressConference.bodyNone");

        let mut one = game_after_a_match();
        apply_press_conference(&mut one, &[quoted("We go again.")]).unwrap();
        assert_eq!(body_key_of(&one), "be.news.pressConference.bodySingle");

        let mut many = game_after_a_match();
        apply_press_conference(&mut many, &[quoted("First."), quoted("Second.")]).unwrap();
        assert_eq!(body_key_of(&many), "be.news.pressConference.bodyMultiple");
    }

    #[test]
    fn the_article_quotes_the_first_answer_given() {
        let mut game = game_after_a_match();

        apply_press_conference(&mut game, &[quoted("First."), quoted("Second.")]).unwrap();

        assert_eq!(
            game.news[0].i18n_params.get("quote").map(String::as_str),
            Some("First.")
        );
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

    /// A cup tie is a match the manager is asked about. It lives in its own `League` entry, so a
    /// lookup that reads only the legacy `game.league` mirror reports the previous *league* game
    /// instead — the wrong scoreline, filed into a news article the player reads.
    #[test]
    fn the_conference_covers_the_last_match_played_not_the_last_league_match() {
        let mut game = game_after_a_match();
        let cup_tie = domain::league::Fixture {
            id: "cup1".to_string(),
            matchday: 1,
            // The day after the league fixture in `game_after_a_match`.
            date: "2026-03-14".to_string(),
            home_team_id: "team2".to_string(),
            away_team_id: "team1".to_string(),
            competition: domain::league::FixtureCompetition::Cup,
            status: domain::league::FixtureStatus::Completed,
            result: Some(domain::league::MatchResult {
                home_goals: 0,
                away_goals: 3,
                ..Default::default()
            }),
            ..Default::default()
        };
        game.competitions.push(domain::league::League {
            id: "cup".to_string(),
            name: "Test Cup".to_string(),
            season: 1,
            kind: domain::league::CompetitionType::Cup,
            fixtures: vec![cup_tie],
            ..Default::default()
        });

        let outcome =
            apply_press_conference(&mut game, &[answer("mood", "confident", "")]).unwrap();

        assert_eq!(outcome.home_team_name, "Rival FC");
        assert_eq!(outcome.away_team_name, "Test FC");
        assert_eq!(outcome.home_score, 0);
        assert_eq!(outcome.away_score, 3);
    }

    /// A `player_focus` answer used to resolve its id against every player in the world, so
    /// praising the striker who had just knocked you out lifted *his* morale by five.
    #[test]
    fn praising_a_player_from_another_club_is_rejected() {
        let mut game = game_after_a_match();

        let result = apply_press_conference(&mut game, &[answer("player_focus", "praise", "p3")]);

        assert_eq!(result.unwrap_err(), "Player p3 is not in your squad.");
        assert!(game.players.iter().all(|p| p.morale == 50));
        assert!(game.news.is_empty());
    }

    /// The same rule for an id that only decorates the article: a player the user does not manage
    /// has no business in the conference's player list either.
    #[test]
    fn mentioning_a_player_from_another_club_is_rejected() {
        let mut game = game_after_a_match();

        let result = apply_press_conference(&mut game, &[answer("mood", "confident", "p3")]);

        assert_eq!(result.unwrap_err(), "Player p3 is not in your squad.");
        assert!(game.players.iter().all(|p| p.morale == 50));
        assert!(game.news.is_empty());
    }

    #[test]
    fn an_unknown_player_id_is_rejected() {
        let mut game = game_after_a_match();

        let result =
            apply_press_conference(&mut game, &[answer("player_focus", "praise", "nobody")]);

        assert_eq!(result.unwrap_err(), "Player nobody is not in your squad.");
        assert!(game.news.is_empty());
    }

    #[test]
    fn the_outcome_counts_the_players_that_moved() {
        let mut game = game_after_a_match();

        let outcome =
            apply_press_conference(&mut game, &[answer("mood", "confident", "")]).unwrap();

        assert_eq!(outcome.squad_morale_delta, 3);
        assert_eq!(outcome.squad_players_moved, 2);
        assert_eq!(outcome.squad_size, 2);
    }

    /// Morale clamps at 100. A squad already there absorbs the whole delta, and the old report
    /// still announced "+3" — telling the caller its praise had worked when nobody had moved.
    #[test]
    fn a_squad_already_at_maximum_morale_reports_nobody_moved() {
        let mut game = game_after_a_match();
        for player in game.players.iter_mut() {
            if player.team_id.as_deref() == Some("team1") {
                player.morale = 100;
            }
        }

        let outcome =
            apply_press_conference(&mut game, &[answer("mood", "confident", "")]).unwrap();

        assert_eq!(outcome.squad_morale_delta, 3);
        assert_eq!(outcome.squad_players_moved, 0);
        assert_eq!(outcome.squad_size, 2);
    }

    /// The mirror of the case above: `deflect` on a player question leaves the squad-wide delta at
    /// zero but still costs the named player a point, so "no change" would be just as wrong.
    #[test]
    fn a_zero_squad_delta_still_reports_the_player_that_moved() {
        let mut game = game_after_a_match();

        let outcome =
            apply_press_conference(&mut game, &[answer("player_focus", "deflect", "p1")]).unwrap();

        assert_eq!(outcome.squad_morale_delta, 0);
        assert_eq!(outcome.squad_players_moved, 1);
        let p1 = game.players.iter().find(|p| p.id == "p1").unwrap();
        assert_eq!(p1.morale, 49);
    }

    fn outcome_with(squad_morale_delta: i16, squad_players_moved: usize) -> PressConferenceOutcome {
        PressConferenceOutcome {
            squad_morale_delta,
            squad_players_moved,
            squad_size: 24,
            home_team_name: "Test FC".to_string(),
            away_team_name: "Rival FC".to_string(),
            home_score: 2,
            away_score: 1,
        }
    }

    #[test]
    fn the_report_never_claims_a_change_that_did_not_happen() {
        assert!(format_outcome(&outcome_with(3, 18)).contains("📈 Squad morale +3: 18 of 24"));
        assert!(format_outcome(&outcome_with(-2, 24)).contains("📉 Squad morale -2: 24 of 24"));
        // A delta the squad absorbed whole must not read as a rise.
        assert!(format_outcome(&outcome_with(3, 0)).contains("➡️ Squad morale +3: 0 of 24"));
        // And a flat squad delta must not read as nothing happening.
        assert!(format_outcome(&outcome_with(0, 1)).contains("➡️ Squad morale +0: 1 of 24"));
    }
}
