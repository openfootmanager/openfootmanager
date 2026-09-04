use log::info;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

pub use crate::application::live_match::FinishLiveMatchResponse;
use crate::application::live_match::{
    apply_match_command as apply_match_command_service,
    finish_live_match as finish_live_match_service,
    get_match_snapshot as get_match_snapshot_service, start_live_match as start_live_match_service,
    step_live_match as step_live_match_service,
};
use crate::application::press_conference::{
    first_player_outside_squad, last_completed_match, todays_article_id,
};
use crate::application::team_talk::apply_team_talk as apply_team_talk_service;
use crate::commands::util::user_team_id;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

#[derive(Debug, Deserialize)]
pub struct PressConferenceAnswer {
    question_id: String,
    response_id: String,
    #[serde(rename = "response_tone")]
    _response_tone: String,
    response_text: String,
    #[serde(default)]
    response_text_key: String,
    #[serde(default)]
    response_text_params: HashMap<String, String>,
    question_text: String,
    #[serde(default)]
    player_id: String,
}

#[derive(Debug, Serialize)]
struct LocalizedPressQuote {
    #[serde(skip_serializing_if = "String::is_empty")]
    key: String,
    fallback: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    params: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Live Match Commands
// ---------------------------------------------------------------------------

pub fn finish_live_match_internal(state: &StateManager) -> Result<FinishLiveMatchResponse, String> {
    finish_live_match_service(state)
}

pub fn apply_team_talk_internal(
    game: &mut Game,
    tone: &str,
    context: &str,
    seed: u64,
) -> Result<Vec<serde_json::Value>, String> {
    apply_team_talk_service(game, tone, context, seed)
}

/// Start a live match for a given fixture.
/// mode: "live" | "spectator" | "instant"
#[tauri::command]
pub fn start_live_match(
    state: State<'_, Arc<StateManager>>,
    fixture_index: usize,
    mode: String,
    allows_extra_time: bool,
    home_team_id: Option<String>,
    away_team_id: Option<String>,
) -> Result<engine::MatchSnapshot, String> {
    start_live_match_service(
        &state,
        fixture_index,
        &mode,
        allows_extra_time,
        home_team_id.as_deref(),
        away_team_id.as_deref(),
    )
}

/// Step the live match forward by N minutes. Returns the events from each minute.
#[tauri::command]
pub fn step_live_match(
    state: State<'_, Arc<StateManager>>,
    minutes: u16,
) -> Result<Vec<engine::MinuteResult>, String> {
    step_live_match_service(&state, minutes)
}

/// Apply a match command (substitution, tactic change, set piece taker, etc.)
#[tauri::command]
pub fn apply_match_command(
    state: State<'_, Arc<StateManager>>,
    command: engine::MatchCommand,
) -> Result<engine::MatchSnapshot, String> {
    apply_match_command_service(&state, command)
}

/// Get current match snapshot without advancing time.
#[tauri::command]
pub fn get_match_snapshot(
    state: State<'_, Arc<StateManager>>,
) -> Result<engine::MatchSnapshot, String> {
    get_match_snapshot_service(&state)
}

/// Finish the live match: generate report, update game state, clean up.
#[tauri::command]
pub fn finish_live_match(
    state: State<'_, Arc<StateManager>>,
) -> Result<FinishLiveMatchResponse, String> {
    finish_live_match_internal(&state)
}

/// Apply a team talk and return per-player morale changes.
/// tone: "calm" | "motivational" | "assertive" | "aggressive" | "praise" | "disappointed"
/// context: "winning" | "losing" | "drawing"
#[tauri::command]
pub fn apply_team_talk(
    state: State<'_, Arc<StateManager>>,
    tone: String,
    context: String,
) -> Result<Vec<serde_json::Value>, String> {
    info!("[cmd] apply_team_talk: tone={}, context={}", tone, context);
    let seed = rand::rng().random::<u64>();
    // apply_team_talk validates (team assigned) before mutating morale.
    state
        .update_game(|game| apply_team_talk_internal(game, &tone, &context, seed))
        .unwrap_or_else(|| Err("be.error.noActiveGameSession".to_string()))
}

/// Process press conference answers: generate news article, affect squad morale.
#[tauri::command]
pub fn submit_press_conference(
    state: State<'_, Arc<StateManager>>,
    answers: Vec<PressConferenceAnswer>,
) -> Result<serde_json::Value, String> {
    info!("[cmd] submit_press_conference: {} answers", answers.len());
    state
        .update_game(|game| apply_press_conference(game, &answers))
        .unwrap_or_else(|| Err("be.error.noActiveGameSession".to_string()))
}

/// Applies a press conference to the game: individual morale, squad morale, and the news article.
///
/// Every failure is resolved before the first mutation. `update_game` cannot roll back a closure
/// that returns `Err`, so a check placed after a morale change would leave the squad's mood moved
/// with no article to explain it.
fn apply_press_conference(
    game: &mut Game,
    answers: &[PressConferenceAnswer],
) -> Result<serde_json::Value, String> {
    // One conference per game day. See `todays_article_id` for what a second one costs.
    let (article_id, already_held) = todays_article_id(game);
    if already_held {
        return Err("be.error.liveMatch.pressConferenceAlreadyHeld".to_string());
    }

    // Who is speaking, and about which match — both taken from the game rather than from the
    // caller. The team id, the team name and the whole scoreline used to arrive over IPC, so the
    // article reported whatever it was handed: a result that never happened, filed under a club
    // the manager does not manage, with that club's morale moved to match.
    let user_team_id = user_team_id(game)?;
    let user_team_name = game
        .teams
        .iter()
        .find(|t| t.id == user_team_id)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| user_team_id.clone());

    // Every named player must be one the user actually manages, checked before anything moves.
    // The morale effect below resolves a named player against the whole world, so a stale or
    // mistaken id could praise an opposition striker into a better mood.
    if first_player_outside_squad(
        game,
        &user_team_id,
        answers.iter().map(|answer| answer.player_id.as_str()),
    )
    .is_some()
    {
        return Err("be.error.playerNotInSquad".to_string());
    }

    let (home_team_id, away_team_id, home_score, away_score) =
        last_completed_match(game, &user_team_id).ok_or("be.error.liveMatch.noCompletedMatch")?;
    let team_name = |id: &str| {
        game.teams
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let home_team = team_name(&home_team_id);
    let away_team = team_name(&away_team_id);

    // Past this point nothing returns `Err` — see the note above.
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let mut rng = rand::rng();

    // Build news article from press conference answers
    let mut quotes: Vec<String> = Vec::new();
    let mut localized_quotes: Vec<LocalizedPressQuote> = Vec::new();
    let mut morale_delta: i16 = 0;
    let mut mentioned_player_ids: Vec<String> = Vec::new();

    for answer in answers {
        let rid = answer.response_id.as_str();
        let text = answer.response_text.as_str();
        let qid = answer.question_id.as_str();

        let _ = &answer.question_text;

        if !text.is_empty() {
            quotes.push(format!("\"{}\"", text));
            localized_quotes.push(LocalizedPressQuote {
                key: answer.response_text_key.clone(),
                fallback: text.to_string(),
                params: answer.response_text_params.clone(),
            });
        }

        // Track player mentions
        if !answer.player_id.is_empty() {
            mentioned_player_ids.push(answer.player_id.clone());
        }

        // Morale effects based on stable response identifiers.
        match rid {
            "humble" | "fair" | "positive" | "focused" | "grateful" | "patience" | "appreciate"
            | "understand" => morale_delta += rng.random_range(1..=3),
            "confident" | "ambitious" | "shared" => morale_delta += rng.random_range(2..=5),
            "defiant" | "frustrated" => morale_delta += rng.random_range(-2..=2),
            "curt" | "evasive" => morale_delta += rng.random_range(-3..=0),
            "accept" | "detailed" | "apologize" => morale_delta += rng.random_range(0..=2),
            "deflect" => morale_delta += rng.random_range(-1..=1),
            "praise" => morale_delta += rng.random_range(3..=6),
            "demanding" => morale_delta += rng.random_range(-2..=3),
            _ => {}
        }

        // Player-focused question effects
        if qid == "player_focus" && !answer.player_id.is_empty() {
            let player_delta: i16 = match rid {
                "praise" => rng.random_range(4..=8),
                "demanding" => rng.random_range(-3..=4),
                "deflect" => rng.random_range(-2..=1),
                _ => rng.random_range(0..=3),
            };
            if let Some(p) = game.players.iter_mut().find(|p| p.id == answer.player_id) {
                p.morale = ((p.morale as i16) + player_delta).clamp(10, 100) as u8;
            }
        }
    }

    // Apply squad-wide morale effect
    morale_delta = morale_delta.clamp(-8, 8);
    if morale_delta != 0 {
        for p in game.players.iter_mut() {
            if p.team_id.as_deref() == Some(&user_team_id) {
                p.morale = ((p.morale as i16) + morale_delta).clamp(10, 100) as u8;
            }
        }
    }

    // Generate news article
    let result_str = format!(
        "{} {} - {} {}",
        home_team, home_score, away_score, away_team
    );
    let headline_key = if quotes.is_empty() {
        ("be.news.pressConference.headlinePostMatch",)
    } else if rng.random::<bool>() {
        ("be.news.pressConference.headlineManagerQuote",)
    } else {
        ("be.news.pressConference.headlinePressConf",)
    }
    .0;

    let body_key = if quotes.len() > 1 {
        ("be.news.pressConference.bodyMultiple",)
    } else if quotes.len() == 1 {
        ("be.news.pressConference.bodySingle",)
    } else {
        ("be.news.pressConference.bodyNone",)
    }
    .0;

    let mut i18n_params = HashMap::new();
    i18n_params.insert("team".to_string(), user_team_name);
    i18n_params.insert("result".to_string(), result_str);
    if !localized_quotes.is_empty() {
        if let Ok(serialized_quotes) = serde_json::to_string(&localized_quotes) {
            i18n_params.insert("quotesData".to_string(), serialized_quotes);
        }
        i18n_params.insert("quote".to_string(), quotes[0].trim_matches('"').to_string());
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

    Ok(serde_json::json!({
        "game": game,
        "morale_delta": morale_delta
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        apply_press_conference, apply_team_talk_internal, finish_live_match_internal,
        PressConferenceAnswer,
    };
    use chrono::{TimeZone, Utc};
    use domain::league::{
        CompetitionFormat, CompetitionRules, CompetitionType, Fixture, FixtureCompetition,
        FixtureStatus, KnockoutRoundState, League, StandingEntry,
    };
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, PlayerIssue, PlayerIssueCategory, Position};
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::live_match_manager::{self, MatchMode};
    use ofm_core::state::StateManager;
    use std::collections::HashMap;

    fn default_attrs(position: Position) -> PlayerAttributes {
        let is_goalkeeper = matches!(position, Position::Goalkeeper);

        PlayerAttributes {
            pace: 65,
            stamina: 65,
            strength: 65,
            agility: 65,
            passing: 65,
            shooting: if is_goalkeeper { 30 } else { 65 },
            tackling: if is_goalkeeper { 30 } else { 65 },
            dribbling: if is_goalkeeper { 30 } else { 65 },
            defending: if is_goalkeeper { 30 } else { 65 },
            positioning: 65,
            vision: 65,
            decisions: 65,
            composure: 65,
            aggression: 50,
            teamwork: 65,
            leadership: 50,
            handling: if is_goalkeeper { 75 } else { 20 },
            reflexes: if is_goalkeeper { 75 } else { 20 },
            aerial: 60,
        }
    }

    fn make_player(id: &str, name: &str, team_id: &str, position: Position) -> Player {
        let mut player = Player::new(
            id.to_string(),
            name.to_string(),
            name.to_string(),
            "1995-01-01".to_string(),
            "England".to_string(),
            position.clone(),
            default_attrs(position),
        );
        player.team_id = Some(team_id.to_string());
        player.condition = 100;
        player.morale = 70;
        player
    }

    fn make_team(id: &str, name: &str) -> Team {
        Team::new(
            id.to_string(),
            name.to_string(),
            name[..3].to_string(),
            "England".to_string(),
            "London".to_string(),
            "Stadium".to_string(),
            40_000,
        )
    }

    fn make_squad(team_id: &str, prefix: &str) -> Vec<Player> {
        let mut players = Vec::new();
        players.push(make_player(
            &format!("{}_gk", prefix),
            &format!("{} GK", prefix),
            team_id,
            Position::Goalkeeper,
        ));
        for index in 0..4 {
            players.push(make_player(
                &format!("{}_def{}", prefix, index),
                &format!("{} Def{}", prefix, index),
                team_id,
                Position::Defender,
            ));
        }
        for index in 0..4 {
            players.push(make_player(
                &format!("{}_mid{}", prefix, index),
                &format!("{} Mid{}", prefix, index),
                team_id,
                Position::Midfielder,
            ));
        }
        for index in 0..2 {
            players.push(make_player(
                &format!("{}_fwd{}", prefix, index),
                &format!("{} Fwd{}", prefix, index),
                team_id,
                Position::Forward,
            ));
        }
        players
    }

    fn make_game_with_round() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team1".to_string());

        let teams = vec![
            make_team("team1", "Home FC"),
            make_team("team2", "Away FC"),
            make_team("team3", "Third FC"),
            make_team("team4", "Fourth FC"),
        ];
        let mut players = make_squad("team1", "t1");
        players.extend(make_squad("team2", "t2"));
        players.extend(make_squad("team3", "t3"));
        players.extend(make_squad("team4", "t4"));

        let league = League {
            id: "league1".to_string(),
            name: "Test League".to_string(),
            season: 1,
            fixtures: vec![
                Fixture {
                    id: "fix1".to_string(),
                    competition_id: "league1".to_string(),
                    matchday: 1,
                    date: "2025-06-15".to_string(),
                    home_team_id: "team1".to_string(),
                    away_team_id: "team2".to_string(),
                    competition: FixtureCompetition::League,
                    status: FixtureStatus::Scheduled,
                    result: None,
                },
                Fixture {
                    id: "fix2".to_string(),
                    competition_id: "league1".to_string(),
                    matchday: 1,
                    date: "2025-06-15".to_string(),
                    home_team_id: "team3".to_string(),
                    away_team_id: "team4".to_string(),
                    competition: FixtureCompetition::League,
                    status: FixtureStatus::Scheduled,
                    result: None,
                },
            ],
            standings: vec![
                StandingEntry::new("team1".to_string()),
                StandingEntry::new("team2".to_string()),
                StandingEntry::new("team3".to_string()),
                StandingEntry::new("team4".to_string()),
            ],
            transfer_log: vec![],
            transfer_rumours: vec![],
            ..League::default()
        };

        let mut game = Game::new(clock, manager, teams, players, vec![], vec![]);
        game.league = Some(league.clone());
        // `game.competitions` is the modern source of truth; the legacy `league`
        // field is only a mirror. Real saves always have both populated, so the
        // test fixture mirrors that to exercise sync_legacy_league realistically.
        game.competitions = vec![league];
        game
    }

    fn delta_for(results: &[serde_json::Value], player_id: &str) -> i64 {
        results
            .iter()
            .find(|result| result["player_id"] == player_id)
            .and_then(|result| result["delta"].as_i64())
            .unwrap()
    }

    /// A game whose user team has played — the state a press conference is held in.
    ///
    /// `make_game_with_round` leaves both fixtures unplayed, which is the state *before* kick-off;
    /// a conference is held after. Loading through `promote_legacy_league` is what every real load
    /// path does, so the fixture reaches the state production actually reaches rather than one
    /// where only the legacy `league` mirror is populated.
    fn game_after_a_match() -> Game {
        let mut game = make_game_with_round();
        game.competitions[0].fixtures[0].status = FixtureStatus::Completed;
        game.competitions[0].fixtures[0].result = Some(domain::league::MatchResult {
            home_goals: 2,
            away_goals: 1,
            ..Default::default()
        });
        game.league = None;
        game.promote_legacy_league();
        game
    }

    /// Every player's morale, in order. Compared whole so a test asserting "nothing moved" cannot
    /// be satisfied by a fixture whose baseline happens to match a hardcoded number.
    fn morale_snapshot(game: &Game) -> Vec<u8> {
        game.players.iter().map(|p| p.morale).collect()
    }

    /// The same, for every player outside one club.
    fn morale_snapshot_excluding(game: &Game, team_id: &str) -> Vec<u8> {
        game.players
            .iter()
            .filter(|p| p.team_id.as_deref() != Some(team_id))
            .map(|p| p.morale)
            .collect()
    }

    fn press_answer(
        question_id: &str,
        response_id: &str,
        player_id: &str,
    ) -> PressConferenceAnswer {
        PressConferenceAnswer {
            question_id: question_id.to_string(),
            response_id: response_id.to_string(),
            _response_tone: String::new(),
            response_text: "Something quotable".to_string(),
            response_text_key: String::new(),
            response_text_params: HashMap::new(),
            question_text: "How was the game?".to_string(),
            player_id: player_id.to_string(),
        }
    }

    #[test]
    fn praising_a_player_from_another_club_is_rejected() {
        // The player id used to be resolved against every player in the world, so a stale or
        // mistaken id could hand an opposition striker the morale boost meant for one of ours.
        let mut game = game_after_a_match();
        let answers = vec![press_answer("player_focus", "praise", "t2_fwd0")];
        let morale_before = morale_snapshot(&game);

        let result = apply_press_conference(&mut game, &answers);

        assert_eq!(result.unwrap_err(), "be.error.playerNotInSquad");
        assert_eq!(
            morale_snapshot(&game),
            morale_before,
            "the rival's morale must not have moved"
        );
        assert!(game.news.is_empty(), "and no article may be filed");
    }

    #[test]
    fn a_press_conference_names_the_match_actually_played() {
        // The scoreline used to arrive over IPC next to the answers, so the article reported
        // whatever the caller passed. It is now read from the last completed fixture.
        let mut game = game_after_a_match();

        apply_press_conference(&mut game, &[press_answer("q1", "humble", "")])
            .expect("press conference applied");

        let params = game.news[0].i18n_params.clone();
        assert_eq!(
            params.get("result").map(String::as_str),
            Some("Home FC 2 - 1 Away FC")
        );
        assert_eq!(params.get("team").map(String::as_str), Some("Home FC"));
    }

    #[test]
    fn a_press_conference_covers_the_last_match_played_not_the_last_league_match() {
        // A cup tie played after the league game is the match the press will ask about. Reading
        // `game.league` — one competition's mirror — would report the league game instead.
        let mut game = game_after_a_match();
        let cup = League {
            id: "cup1".to_string(),
            name: "Test Cup".to_string(),
            season: 1,
            fixtures: vec![Fixture {
                id: "cup_fix1".to_string(),
                competition_id: "cup1".to_string(),
                matchday: 1,
                date: "2025-06-16".to_string(),
                home_team_id: "team3".to_string(),
                away_team_id: "team1".to_string(),
                competition: FixtureCompetition::Cup,
                status: FixtureStatus::Completed,
                result: Some(domain::league::MatchResult {
                    home_goals: 0,
                    away_goals: 3,
                    ..Default::default()
                }),
            }],
            ..League::default()
        };
        game.competitions.push(cup);

        apply_press_conference(&mut game, &[press_answer("q1", "humble", "")])
            .expect("press conference applied");

        let params = game.news[0].i18n_params.clone();
        assert_eq!(
            params.get("result").map(String::as_str),
            Some("Third FC 0 - 3 Home FC")
        );
    }

    #[test]
    fn a_second_press_conference_on_the_same_day_is_rejected() {
        // Both articles would carry `press_conf_<date>`, so the second is unreachable in a news
        // list that selects by id — and the squad would take the morale swing twice.
        let mut game = game_after_a_match();
        let answers = vec![press_answer("q1", "praise", "")];
        apply_press_conference(&mut game, &answers).expect("first conference applied");
        let morale_after_first: Vec<u8> = game.players.iter().map(|p| p.morale).collect();

        let result = apply_press_conference(&mut game, &answers);

        assert_eq!(
            result.unwrap_err(),
            "be.error.liveMatch.pressConferenceAlreadyHeld"
        );
        assert_eq!(game.news.len(), 1, "no second article");
        let morale_now: Vec<u8> = game.players.iter().map(|p| p.morale).collect();
        assert_eq!(morale_now, morale_after_first, "and no second morale swing");
    }

    #[test]
    fn a_press_conference_before_any_match_is_rejected() {
        // Nothing has been played, so there is no result to report and no article to file.
        let mut game = make_game_with_round();

        let morale_before = morale_snapshot(&game);

        let result = apply_press_conference(&mut game, &[press_answer("q1", "humble", "")]);

        assert_eq!(result.unwrap_err(), "be.error.liveMatch.noCompletedMatch");
        assert!(game.news.is_empty());
        assert_eq!(morale_snapshot(&game), morale_before);
    }

    #[test]
    fn only_the_user_squad_takes_the_press_conference_morale() {
        let mut game = game_after_a_match();
        let others_before = morale_snapshot_excluding(&game, "team1");

        apply_press_conference(&mut game, &[press_answer("q1", "confident", "")])
            .expect("press conference applied");

        assert_eq!(
            morale_snapshot_excluding(&game, "team1"),
            others_before,
            "no other club hears the manager's press conference"
        );
    }

    #[test]
    fn finish_live_match_persists_user_standings_update() {
        // Regression: finish_live_match_day calls sync_legacy_league at the end,
        // which copies game.competitions[i] back into game.league. If
        // apply_match_report's standings/fixture update only landed on the legacy
        // mirror, the sync silently wipes it out — leaving the user's team with
        // played=0 in the table even though the result mail was sent.
        let state = StateManager::new();
        let game = make_game_with_round();

        let mut session =
            live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();
        session.user_side = None;
        session.run_to_completion();

        state.set_game(game);
        state.set_live_match(session);

        finish_live_match_internal(&state).expect("finish live match response");

        // Assert against the persisted state — not just the cloned response.game
        // — so a missing state.set_game() call would surface here.
        let persisted_game = state
            .get_game(|game| game.clone())
            .expect("game persisted in state after finish_live_match_internal");

        let competition = persisted_game
            .competitions
            .first()
            .expect("user competition retained");
        let user_entry = competition
            .standings
            .iter()
            .find(|entry| entry.team_id == "team1")
            .expect("user team standings entry");
        assert_eq!(
            user_entry.played, 1,
            "user team must have played count incremented after live match",
        );
        let opp_entry = competition
            .standings
            .iter()
            .find(|entry| entry.team_id == "team2")
            .expect("opponent team standings entry");
        assert_eq!(opp_entry.played, 1, "opponent played count must also update");
        assert_eq!(
            user_entry.points + opp_entry.points,
            user_entry.won * 3
                + opp_entry.won * 3
                + (user_entry.drawn + opp_entry.drawn),
            "points must agree with W/D record",
        );

        let user_fixture = competition
            .fixtures
            .first()
            .expect("user fixture retained");
        assert!(
            matches!(user_fixture.status, domain::league::FixtureStatus::Completed),
            "user fixture must be marked Completed",
        );
        assert!(
            user_fixture.result.is_some(),
            "user fixture result must be recorded",
        );
    }

    #[test]
    fn finish_live_match_returns_completed_round_summary_response() {
        let state = StateManager::new();
        let mut game = make_game_with_round();
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        ofm_core::turn::simulate_other_matches(&mut game, &today, Some(0));
        // Mirror the GUI day-start flow, which writes the updated competition
        // back before the match finishes: finish_live_match treats
        // game.competitions as the source of truth.
        game.competitions[0] = game.league.clone().unwrap();

        let mut session =
            live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();
        session.user_side = None;
        session.run_to_completion();

        state.set_game(game);
        state.set_live_match(session);

        let response = finish_live_match_internal(&state).expect("finish live match response");

        let round_summary = response.round_summary.expect("round summary response");
        assert!(round_summary.is_complete);
        assert_eq!(round_summary.pending_fixture_count, 0);
        assert_eq!(round_summary.completed_results.len(), 2);
        assert_eq!(
            response
                .game
                .clock
                .current_date
                .format("%Y-%m-%d")
                .to_string(),
            "2025-06-16"
        );
    }

    // Regression: MCP match_finish could be called mid-match and persisted the
    // partial score (or a half-taken shootout) as the final result. The finish
    // path must run the remainder so the report describes a completed match.
    #[test]
    fn finish_live_match_completes_an_unfinished_match_first() {
        let state = StateManager::new();
        let game = make_game_with_round();

        let mut session =
            live_match_manager::create_live_match(&game, 0, MatchMode::Instant, false).unwrap();
        session.user_side = None;
        // Step only a few minutes — nowhere near full time.
        session.step_many(5);
        assert!(!session.is_finished());

        state.set_game(game);
        state.set_live_match(session);

        finish_live_match_internal(&state).expect("finish live match");

        let fixture = state
            .get_game(|g| g.competitions[0].fixtures[0].clone())
            .unwrap();
        assert_eq!(fixture.status, FixtureStatus::Completed);
        let report = fixture
            .result
            .expect("result persisted")
            .report
            .expect("compact report persisted");
        assert!(
            report.total_minutes >= 90,
            "persisted report must describe a full match, got {} minutes",
            report.total_minutes
        );
    }

    // Regression: MCP match_start called start_live_match directly, which
    // never simulated the day's other fixtures; match_finish then advanced
    // the clock, stranding them Scheduled in the past forever.
    #[test]
    fn start_live_match_simulates_other_same_day_fixtures() {
        let state = StateManager::new();
        state.set_game(make_game_with_round());

        crate::application::live_match::start_live_match(
            &state, 0, "spectator", false, None, None,
        )
        .expect("start live match");

        let (user_fixture, other_fixture) = state
            .get_game(|g| {
                let league = g.league.as_ref().unwrap();
                (league.fixtures[0].clone(), league.fixtures[1].clone())
            })
            .unwrap();
        assert_eq!(
            user_fixture.status,
            FixtureStatus::Scheduled,
            "the user's own fixture must not be pre-simulated"
        );
        assert_eq!(
            other_fixture.status,
            FixtureStatus::Completed,
            "the same-day AI fixture must be simulated"
        );
        let first_result = other_fixture.result.expect("simulated fixture has a result");

        // The modern competitions list must receive the results too.
        let competition_fixture = state
            .get_game(|g| g.competitions[0].fixtures[1].clone())
            .unwrap();
        assert_eq!(competition_fixture.status, FixtureStatus::Completed);

        // Session restore: starting again must not re-simulate completed
        // fixtures (simulate_other_matches only touches Scheduled ones).
        crate::application::live_match::start_live_match(
            &state, 0, "spectator", false, None, None,
        )
        .expect("restore live match");
        let restored_result = state
            .get_game(|g| g.league.as_ref().unwrap().fixtures[1].result.clone())
            .unwrap()
            .expect("result still present");
        assert_eq!(restored_result.home_goals, first_result.home_goals);
        assert_eq!(restored_result.away_goals, first_result.away_goals);
    }

    fn make_knockout_cup(fixture_date: &str) -> League {
        League {
            id: "cup1".to_string(),
            name: "Test Cup".to_string(),
            kind: CompetitionType::Cup,
            season: 1,
            rules: CompetitionRules {
                format: CompetitionFormat::Knockout,
                ..CompetitionRules::default()
            },
            fixtures: vec![Fixture {
                id: "cupfix1".to_string(),
                competition_id: "cup1".to_string(),
                matchday: 1,
                date: fixture_date.to_string(),
                home_team_id: "team1".to_string(),
                away_team_id: "team3".to_string(),
                competition: FixtureCompetition::Cup,
                status: FixtureStatus::Scheduled,
                result: None,
            }],
            standings: vec![],
            knockout_rounds: vec![KnockoutRoundState {
                id: "cup1-round-1".to_string(),
                name: "Final".to_string(),
                fixture_ids: vec!["cupfix1".to_string()],
                bye_team_ids: Vec::new(),
                completed: false,
            }],
            ..League::default()
        }
    }

    // Regression: finishing a live CUP match applied the report by raw index
    // into game.league — which sync_legacy_league had reset to the user's
    // domestic league — writing the cup result onto an unrelated league
    // fixture (or panicking on an out-of-range index) while the real cup
    // fixture stayed Scheduled and the bracket stalled.
    #[test]
    fn finish_live_match_applies_cup_result_to_the_cup_competition() {
        let state = StateManager::new();
        let mut game = make_game_with_round();
        let cup = make_knockout_cup("2025-06-15");
        game.competitions.push(cup.clone());

        // Mimic the GUI match-day flow: the cup is swapped into game.league,
        // the session is created against it…
        game.league = Some(cup);
        let mut session =
            live_match_manager::create_live_match(&game, 0, MatchMode::Instant, true).unwrap();
        session.user_side = None;
        session.run_to_completion();
        assert_eq!(session.competition_id, "cup1");

        // …then the day-start code restores the legacy mirror to the user's
        // domestic league before finish_live_match runs.
        game.sync_legacy_league();
        assert_eq!(game.league.as_ref().unwrap().id, "league1");

        state.set_game(game);
        state.set_live_match(session);

        let response = finish_live_match_internal(&state).expect("finish live match");

        let cup = response
            .game
            .competitions
            .iter()
            .find(|c| c.id == "cup1")
            .expect("cup competition");
        assert_eq!(cup.fixtures[0].status, FixtureStatus::Completed);
        assert!(cup.fixtures[0].result.is_some(), "cup fixture gets the result");
        assert!(cup.knockout_rounds[0].completed, "cup bracket advances");

        let league = response
            .game
            .competitions
            .iter()
            .find(|c| c.id == "league1")
            .expect("league competition");
        assert_eq!(
            league.fixtures[0].status,
            FixtureStatus::Scheduled,
            "league fixture at the same index must be untouched"
        );
        assert!(league.fixtures[0].result.is_none());
        assert!(
            league.standings.iter().all(|entry| entry.played == 0),
            "league standings must not record the cup result"
        );
    }

    #[test]
    fn finish_live_match_errors_on_out_of_range_fixture_index() {
        // Legacy-shaped save: no competitions, game.league is the only truth.
        let state = StateManager::new();
        let mut game = make_game_with_round();
        game.competitions = Vec::new();
        let mut session =
            live_match_manager::create_live_match(&game, 1, MatchMode::Instant, false).unwrap();
        session.user_side = None;
        session.run_to_completion();

        // The stored index no longer exists.
        game.league.as_mut().unwrap().fixtures.truncate(1);
        state.set_game(game);
        state.set_live_match(session);

        let result = finish_live_match_internal(&state);
        assert!(result.is_err(), "out-of-range fixture index must error, not panic");
    }

    #[test]
    fn team_talk_reactions_vary_by_player_context() {
        let mut game = make_game_with_round();
        let composed = game
            .players
            .iter_mut()
            .find(|player| player.id == "t1_mid0")
            .unwrap();
        composed.attributes.composure = 90;
        composed.attributes.leadership = 90;
        composed.attributes.aggression = 20;
        composed.morale_core.manager_trust = 80;

        let volatile = game
            .players
            .iter_mut()
            .find(|player| player.id == "t1_fwd0")
            .unwrap();
        volatile.attributes.composure = 20;
        volatile.attributes.leadership = 20;
        volatile.attributes.aggression = 90;
        volatile.morale_core.manager_trust = 25;
        volatile.morale_core.unresolved_issue = Some(PlayerIssue {
            category: PlayerIssueCategory::Morale,
            severity: 70,
        });

        let results = apply_team_talk_internal(&mut game, "aggressive", "winning", 7).unwrap();

        assert!(delta_for(&results, "t1_mid0") > delta_for(&results, "t1_fwd0"));
    }

    #[test]
    fn repeating_same_team_talk_loses_effectiveness() {
        let mut game = make_game_with_round();
        let player = game
            .players
            .iter_mut()
            .find(|player| player.id == "t1_mid0")
            .unwrap();
        player.morale = 50;
        player.morale_core.manager_trust = 70;

        let first = apply_team_talk_internal(&mut game, "motivational", "losing", 13).unwrap();
        let second = apply_team_talk_internal(&mut game, "motivational", "losing", 13).unwrap();

        assert!(delta_for(&second, "t1_mid0") <= delta_for(&first, "t1_mid0"));
    }
}
