use std::collections::HashSet;
use std::sync::Arc;

use log::info;
use tauri::State;

use domain::message::InboxMessage;
use ofm_core::inbox_archive::{ArchiveCursor, ArchivedMessagesPage};
use ofm_core::state::{Career, StateManager};

// These commands only ever touch the inbox, so they mutate the game in place
// (no upfront whole-world clone) and return just the updated message list. The
// frontend patches its message slice rather than replacing the entire game —
// which is what made opening/deleting messages hitch.

/// A page of the archive (read mail older than half a year), newest first. Pass the previous page's
/// `next_before` as `before` to continue.
pub fn get_archived_messages_internal(
    state: &StateManager,
    before: Option<ArchiveCursor>,
    limit: usize,
) -> Result<ArchivedMessagesPage, String> {
    state
        .get_game(|game| ofm_core::inbox_archive::page(game, before.as_ref(), limit))
        .ok_or("be.error.noActiveGameSession".to_string())
}

/// A page of archived mail, newest first. `before` is the previous page's `next_before` (leave it
/// out for the newest page); `limit` is how many to return, kept between 1 and 200.
#[tauri::command]
pub fn get_archived_messages(
    state: State<'_, Arc<StateManager>>,
    before: Option<ArchiveCursor>,
    limit: usize,
) -> Result<ArchivedMessagesPage, String> {
    get_archived_messages_internal(&state, before, limit)
}

#[tauri::command]
pub fn mark_message_read(
    state: State<'_, Arc<StateManager>>,
    message_id: String,
    career_token: u64,
) -> Result<Vec<InboxMessage>, String> {
    mark_message_read_internal(&state, Career::Token(career_token), &message_id)
}

pub fn mark_message_read_internal(
    state: &StateManager,
    career: Career,
    message_id: &str,
) -> Result<Vec<InboxMessage>, String> {
    log::debug!("[cmd] mark_message_read: {}", message_id);
    state.update_game_for(career, |game| {
        if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
            msg.read = true;
        }
        game.messages.clone()
    })
}

#[tauri::command]
pub fn delete_message(
    state: State<'_, Arc<StateManager>>,
    message_id: String,
    career_token: u64,
) -> Result<Vec<InboxMessage>, String> {
    delete_message_internal(&state, Career::Token(career_token), &message_id)
}

pub fn delete_message_internal(
    state: &StateManager,
    career: Career,
    message_id: &str,
) -> Result<Vec<InboxMessage>, String> {
    log::debug!("[cmd] delete_message: {}", message_id);
    state.update_game_for(career, |game| {
        game.messages.retain(|message| message.id != message_id);
        game.messages.clone()
    })
}

#[tauri::command]
pub fn delete_messages(
    state: State<'_, Arc<StateManager>>,
    message_ids: Vec<String>,
    career_token: u64,
) -> Result<Vec<InboxMessage>, String> {
    delete_messages_internal(&state, Career::Token(career_token), message_ids)
}

pub fn delete_messages_internal(
    state: &StateManager,
    career: Career,
    message_ids: Vec<String>,
) -> Result<Vec<InboxMessage>, String> {
    log::debug!("[cmd] delete_messages: {}", message_ids.len());
    let message_ids: HashSet<String> = message_ids.into_iter().collect();
    state.update_game_for(career, |game| {
        game.messages
            .retain(|message| !message_ids.contains(&message.id));
        game.messages.clone()
    })
}

#[tauri::command]
pub fn mark_all_messages_read(
    state: State<'_, Arc<StateManager>>,
    career_token: u64,
) -> Result<Vec<InboxMessage>, String> {
    mark_all_messages_read_internal(&state, Career::Token(career_token))
}

pub fn mark_all_messages_read_internal(
    state: &StateManager,
    career: Career,
) -> Result<Vec<InboxMessage>, String> {
    log::debug!("[cmd] mark_all_messages_read");
    state.update_game_for(career, |game| {
        for msg in game.messages.iter_mut() {
            msg.read = true;
        }
        game.messages.clone()
    })
}

#[tauri::command]
pub fn clear_old_messages(
    state: State<'_, Arc<StateManager>>,
    career_token: u64,
) -> Result<Vec<InboxMessage>, String> {
    clear_old_messages_internal(&state, Career::Token(career_token))
}

pub fn clear_old_messages_internal(
    state: &StateManager,
    career: Career,
) -> Result<Vec<InboxMessage>, String> {
    log::debug!("[cmd] clear_old_messages");
    state.update_game_for(career, |game| {
        let current_date = game.clock.current_date.format("%Y-%m-%d").to_string();
        // Keep only: unread messages, messages with unresolved actions, and messages from recent 14 days
        game.messages.retain(|m| {
            if !m.read {
                return true;
            }
            if m.actions.iter().any(|a| !a.resolved) {
                return true;
            }
            // Keep recent messages (within 14 days).
            //
            // Compare on the day prefix: message dates come in two shapes, a
            // bare `YYYY-MM-DD` and an RFC3339 timestamp. `parse_from_str`
            // with "%Y-%m-%d" errors on the trailing time, so parsing the
            // whole string sent every timestamped message — match reports,
            // pre-match previews, fitness warnings, the weekly digest — to
            // the `false` branch and purged it however recent it was.
            let message_day = ofm_core::slices::news::article_day(&m.date);
            if let Ok(msg_date) = chrono::NaiveDate::parse_from_str(message_day, "%Y-%m-%d") {
                if let Ok(cur_date) = chrono::NaiveDate::parse_from_str(&current_date, "%Y-%m-%d") {
                    return (cur_date - msg_date).num_days() <= 14;
                }
            }
            false
        });
        game.messages.clone()
    })
}

#[tauri::command]
pub fn resolve_message_action(
    state: State<'_, Arc<StateManager>>,
    message_id: String,
    action_id: String,
    option_id: Option<String>,
    career_token: u64,
) -> Result<serde_json::Value, String> {
    resolve_message_action_internal(
        &state,
        Career::Token(career_token),
        &message_id,
        &action_id,
        option_id.as_deref(),
    )
}

pub fn resolve_message_action_internal(
    state: &StateManager,
    career: Career,
    message_id: &str,
    action_id: &str,
    option_id: Option<&str>,
) -> Result<serde_json::Value, String> {
    info!(
        "[cmd] resolve_message_action: msg={}, action={}, option={:?}",
        message_id, action_id, option_id
    );
    // Mutate under the game lock (update_game) so a concurrent GUI/MCP write
    // between read and write-back is not silently discarded.
    let (game, effect, effect_i18n_key, effect_i18n_params) =
        state.update_game_for(career, |game| {
            // A message the player cannot see yet cannot be acted on. The inbox
            // list already hides future-dated mail, but this takes an id from the
            // caller — an MCP agent can name one the list never showed it. The
            // guard belongs here rather than in the MCP tool so the GUI obeys the
            // same rule through the same door.
            let today = game.clock.current_date.format("%Y-%m-%d").to_string();
            let visible = game
                .messages
                .iter()
                .find(|message| message.id == message_id)
                .is_none_or(|message| {
                    ofm_core::slices::inbox::message_is_visible(&message.date, &today)
                });
            // An action answers once. Every handler below applies its effect (a morale hit, a signed
            // sponsor) before it marks the action resolved, so a second answer would apply it again;
            // the refusal here covers all of them, for the GUI and for an MCP agent alike.
            let already_resolved = game
                .messages
                .iter()
                .find(|message| message.id == message_id)
                .and_then(|message| message.actions.iter().find(|action| action.id == action_id))
                .is_some_and(|action| action.resolved);
            if visible && already_resolved {
                return Err("be.error.actionAlreadyResolved".to_string());
            }
            // Try to apply player conversation or random event response
            let (effect, effect_i18n_key, effect_i18n_params) = if !visible {
                // Treated exactly like an id that is not in the inbox, because
                // from the player's side it is not: no effect, nothing resolved.
                (None, None, None)
            } else if let Some(opt) = option_id {
                // Try player events first, then random events
                let player_effect = ofm_core::player_events::apply_player_response(
                    game, message_id, action_id, opt,
                );
                if let Some(player_effect) = player_effect {
                    (
                        Some(player_effect.message),
                        Some(player_effect.i18n_key),
                        Some(player_effect.i18n_params),
                    )
                } else {
                    let random_effect = ofm_core::random_events::apply_event_response(
                        game, message_id, action_id, opt,
                    );
                    if let Some(effect) = random_effect {
                        (
                            Some(effect.message),
                            Some(effect.i18n_key),
                            Some(effect.i18n_params),
                        )
                    } else {
                        match ofm_core::job_offers::apply_job_offer_response(
                            game, message_id, action_id, opt,
                        ) {
                            Some(effect) => (
                                Some(effect.message),
                                Some(effect.i18n_key),
                                Some(effect.i18n_params),
                            ),
                            None => match ofm_core::scouting::apply_youth_recruitment_response(
                                game, message_id, action_id, opt,
                            ) {
                                Some(effect) => (
                                    Some(effect.message),
                                    Some(effect.i18n_key),
                                    Some(effect.i18n_params),
                                ),
                                None => (None, None, None),
                            },
                        }
                    }
                }
            } else {
                // Standard resolve — just mark action as resolved
                if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                    if let Some(action) = msg.actions.iter_mut().find(|a| a.id == action_id) {
                        action.resolved = true;
                    }
                }
                (None, None, None)
            };
            Ok((game.clone(), effect, effect_i18n_key, effect_i18n_params))
        })??;

    Ok(serde_json::json!({
        "game": game,
        "effect": effect,
        "effect_i18n_key": effect_i18n_key,
        "effect_i18n_params": effect_i18n_params
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        clear_old_messages_internal, delete_message_internal, delete_messages_internal,
        get_archived_messages_internal, mark_all_messages_read_internal,
        mark_message_read_internal, resolve_message_action_internal,
    };
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::message::{ActionOption, ActionType, InboxMessage, MessageAction};
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
    use ofm_core::state::Career;
    use ofm_core::state::StateManager;

    fn make_team() -> Team {
        let mut team = Team::new(
            "team-1".to_string(),
            "User FC".to_string(),
            "USR".to_string(),
            "England".to_string(),
            "London".to_string(),
            "User Ground".to_string(),
            25_000,
        );
        team.manager_id = Some("manager-1".to_string());
        team
    }

    fn unresolved_action_message(id: &str, date: &str) -> InboxMessage {
        let mut message = InboxMessage::new(
            id.to_string(),
            "Subject".to_string(),
            "Body".to_string(),
            "Board".to_string(),
            date.to_string(),
        );
        message.read = true;
        message.actions.push(MessageAction {
            id: format!("action-{}", id),
            label: "Review".to_string(),
            action_type: ActionType::Dismiss,
            resolved: false,
            label_key: None,
        });
        message
    }

    fn read_message(id: &str, date: &str) -> InboxMessage {
        let mut message = InboxMessage::new(
            id.to_string(),
            "Subject".to_string(),
            "Body".to_string(),
            "Board".to_string(),
            date.to_string(),
        );
        message.read = true;
        message
    }

    fn actionable_message(id: &str, date: &str) -> InboxMessage {
        let mut message = InboxMessage::new(
            id.to_string(),
            "Subject".to_string(),
            "Body".to_string(),
            "Board".to_string(),
            date.to_string(),
        );
        message.actions.push(MessageAction {
            id: format!("action-{}", id),
            label: "Acknowledge".to_string(),
            action_type: ActionType::Acknowledge,
            resolved: false,
            label_key: None,
        });
        message
    }

    fn unread_message(id: &str, date: &str) -> InboxMessage {
        InboxMessage::new(
            id.to_string(),
            "Subject".to_string(),
            "Body".to_string(),
            "Board".to_string(),
            date.to_string(),
        )
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 20, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "manager-1".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("team-1".to_string());

        let mut game = Game::new(clock, manager, vec![make_team()], vec![], vec![], vec![]);
        game.messages = vec![
            unread_message("keep-unread", "2026-07-01"),
            unresolved_action_message("keep-unresolved", "2026-07-01"),
            read_message("keep-recent", "2026-08-10"),
            read_message("remove-stale", "2026-07-01"),
        ];
        game
    }

    /// Given career B is active and a screen still holds career A's token
    /// When the screen's delete arrives through the command
    /// Then it is refused with the career-changed key and the message is still in the inbox.
    #[test]
    fn the_delete_command_for_a_replaced_career_is_refused_and_keeps_the_message() {
        use crate::commands::wrapper_testkit::WrapperApp;
        let app = WrapperApp::new(make_game(), false);
        let inbox = |app: &WrapperApp| app.state().get_game(|g| g.messages.len()).unwrap();
        let before = inbox(&app);

        let refused = super::delete_message(app.state(), "remove-stale".to_string(), app.token + 1);

        assert_eq!(
            refused.map(|_| ()),
            Err("be.error.careerChanged".to_string())
        );
        assert_eq!(inbox(&app), before);
    }

    /// Given career B is active (token 2) and a screen holds career A's token (1)
    /// When mark-read, mark-all, delete, clear and resolve arrive with A's token
    /// Then each is refused with the career-changed key and career B's inbox is untouched.
    #[test]
    fn inbox_writes_for_a_replaced_career_are_refused() {
        let state = StateManager::new();
        let mut game = make_game();
        game.career_token = 2;
        state.set_game(game);
        let before = serde_json::to_value(state.get_game(|g| g.clone()).unwrap()).unwrap();
        let stale = Career::Token(1);
        let key = Err("be.error.careerChanged".to_string());

        assert_eq!(
            mark_message_read_internal(&state, stale, "keep-unread").map(|_| ()),
            key
        );
        assert_eq!(
            mark_all_messages_read_internal(&state, stale).map(|_| ()),
            key
        );
        assert_eq!(
            delete_message_internal(&state, stale, "remove-stale").map(|_| ()),
            key
        );
        assert_eq!(
            delete_messages_internal(&state, stale, vec!["remove-stale".to_string()]).map(|_| ()),
            key
        );
        assert_eq!(clear_old_messages_internal(&state, stale).map(|_| ()), key);
        assert_eq!(
            resolve_message_action_internal(&state, stale, "msg-1", "action-msg-1", None)
                .map(|_| ()),
            key
        );

        let after = serde_json::to_value(state.get_game(|g| g.clone()).unwrap()).unwrap();
        assert_eq!(after, before);
    }

    #[test]
    fn resolve_message_action_internal_ignores_a_message_dated_ahead_of_the_clock() {
        // The inbox list hides future-dated mail, but this takes an id from the
        // caller, and an MCP agent can name one the list never showed it. Acting
        // on it would let an agent resolve an event before the player can see it.
        let state = StateManager::new();
        let mut game = make_game();
        game.messages = vec![unresolved_action_message("future", "2026-09-01")];
        state.set_game(game);

        resolve_message_action_internal(&state, Career::Active, "future", "action-future", None)
            .expect("the call itself succeeds");

        let stored = state.get_game(|game| game.clone()).expect("stored game");
        let action = &stored.messages[0].actions[0];
        assert!(
            !action.resolved,
            "an action on a message the player cannot see must not resolve"
        );
    }

    #[test]
    fn clear_old_messages_internal_keeps_recent_rfc3339_dated_messages() {
        // Match reports, pre-match previews, fitness warnings and the weekly
        // digest stamp `to_rfc3339()`. Parsing those as "%Y-%m-%d" fails on the
        // trailing time, which used to drop them to the `false` branch and purge
        // them however recent they were — re-arming their generators immediately.
        let state = StateManager::new();
        let mut game = make_game();
        game.messages = vec![read_message(
            "keep-timestamped",
            "2026-08-19T12:00:00+00:00",
        )];
        state.set_game(game);

        let response = clear_old_messages_internal(&state, Career::Active).expect("response");

        let message_ids: Vec<&str> = response.iter().map(|message| message.id.as_str()).collect();
        assert_eq!(message_ids, vec!["keep-timestamped"]);
    }

    #[test]
    fn clear_old_messages_internal_still_purges_stale_rfc3339_dated_messages() {
        let state = StateManager::new();
        let mut game = make_game();
        game.messages = vec![read_message(
            "drop-timestamped",
            "2026-07-01T12:00:00+00:00",
        )];
        state.set_game(game);

        let response = clear_old_messages_internal(&state, Career::Active).expect("response");

        assert!(response.is_empty());
    }

    #[test]
    fn clear_old_messages_internal_keeps_only_expected_messages() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = clear_old_messages_internal(&state, Career::Active).expect("response");
        let message_ids: Vec<&str> = response.iter().map(|message| message.id.as_str()).collect();

        assert_eq!(message_ids.len(), 3);
        assert!(message_ids.contains(&"keep-unread"));
        assert!(message_ids.contains(&"keep-unresolved"));
        assert!(message_ids.contains(&"keep-recent"));
        assert!(!message_ids.contains(&"remove-stale"));

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        let stored_ids: Vec<&str> = stored_game
            .messages
            .iter()
            .map(|message| message.id.as_str())
            .collect();
        assert_eq!(stored_ids.len(), 3);
        assert!(!stored_ids.contains(&"remove-stale"));
    }

    // Regression: resolve_message_action used to clone the game, mutate the
    // clone, and set_game it back — so of N concurrent resolves, only the last
    // writer's change survived. With the mutation under the game lock
    // (update_game), every resolve must land.
    #[test]
    fn concurrent_resolves_are_not_lost() {
        use std::sync::{Arc, Barrier};

        const WRITERS: usize = 8;
        let state = Arc::new(StateManager::new());
        let mut game = make_game();
        game.messages = (0..WRITERS)
            .map(|index| unresolved_action_message(&format!("concurrent-{index}"), "2026-08-01"))
            .collect();
        state.set_game(game);

        let barrier = Arc::new(Barrier::new(WRITERS));
        let handles: Vec<_> = (0..WRITERS)
            .map(|index| {
                let state = Arc::clone(&state);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    resolve_message_action_internal(
                        &state,
                        Career::Active,
                        &format!("concurrent-{index}"),
                        &format!("action-concurrent-{index}"),
                        None,
                    )
                    .expect("resolve succeeds");
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("thread completes");
        }

        let resolved = state
            .get_game(|game| {
                game.messages
                    .iter()
                    .flat_map(|message| message.actions.iter())
                    .filter(|action| action.resolved)
                    .count()
            })
            .expect("game present");
        assert_eq!(
            resolved, WRITERS,
            "every concurrently-resolved action must survive"
        );
    }

    #[test]
    fn resolve_message_action_internal_marks_action_resolved_and_updates_state() {
        let state = StateManager::new();
        let mut game = make_game();
        game.messages = vec![actionable_message("msg-1", "2026-08-20")];
        state.set_game(game);

        let response =
            resolve_message_action_internal(&state, Career::Active, "msg-1", "action-msg-1", None)
                .expect("response");

        assert!(response["effect"].is_null());
        assert!(response["effect_i18n_key"].is_null());
        assert!(response["effect_i18n_params"].is_null());
        assert_eq!(
            response["game"]["messages"][0]["actions"][0]["resolved"].as_bool(),
            Some(true)
        );

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert!(stored_game.messages[0].actions[0].resolved);
    }

    #[test]
    fn resolve_message_action_internal_returns_job_offer_i18n_effect() {
        let state = StateManager::new();
        let mut game = make_game();

        let mut vacancy_team = Team::new(
            "team-2".to_string(),
            "Vacancy FC".to_string(),
            "VAC".to_string(),
            "England".to_string(),
            "Liverpool".to_string(),
            "Vacancy Ground".to_string(),
            30_000,
        );
        vacancy_team.reputation = 450;
        game.teams.push(vacancy_team);

        let mut offer = actionable_message("job_offer_team-2_2026-08-20", "2026-08-20");
        offer.context.team_id = Some("team-2".to_string());
        game.messages = vec![offer];
        game.manager.team_id = None;
        state.set_game(game);

        let response = resolve_message_action_internal(
            &state,
            Career::Active,
            "job_offer_team-2_2026-08-20",
            "action-job_offer_team-2_2026-08-20",
            Some("decline"),
        )
        .expect("response");

        assert_eq!(
            response["effect_i18n_key"].as_str(),
            Some("be.msg.jobOffer.effects.declined")
        );
        assert_eq!(
            response["effect_i18n_params"]["team"].as_str(),
            Some("Vacancy FC")
        );
        assert_eq!(response["effect"].as_str(), Some(""));
    }

    #[test]
    fn mark_message_read_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response =
            mark_message_read_internal(&state, Career::Active, "keep-unread").expect("response");

        let message = response
            .iter()
            .find(|message| message.id == "keep-unread")
            .expect("message should exist");
        assert!(message.read);

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert!(
            stored_game
                .messages
                .iter()
                .find(|message| message.id == "keep-unread")
                .expect("stored message should exist")
                .read
        );
    }

    #[test]
    fn mark_all_messages_read_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = mark_all_messages_read_internal(&state, Career::Active).expect("response");

        assert!(response.iter().all(|message| message.read));

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert!(stored_game.messages.iter().all(|message| message.read));
    }

    #[test]
    fn delete_message_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response =
            delete_message_internal(&state, Career::Active, "remove-stale").expect("response");

        assert!(!response.iter().any(|message| message.id == "remove-stale"));

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert!(!stored_game
            .messages
            .iter()
            .any(|message| message.id == "remove-stale"));
    }

    #[test]
    fn delete_messages_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = delete_messages_internal(
            &state,
            Career::Active,
            vec!["keep-unread".to_string(), "remove-stale".to_string()],
        )
        .expect("response");

        assert!(!response.iter().any(|message| message.id == "keep-unread"));
        assert!(!response.iter().any(|message| message.id == "remove-stale"));

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert!(!stored_game
            .messages
            .iter()
            .any(|message| message.id == "keep-unread"));
        assert!(!stored_game
            .messages
            .iter()
            .any(|message| message.id == "remove-stale"));
    }

    /// Given an active game with archived mail, and no game at all
    /// When a page is asked for
    /// Then the game gives its mail newest first and the missing game gives the session key.
    #[test]
    fn archived_messages_are_paged_from_the_active_game() {
        let state = StateManager::new();
        assert_eq!(
            get_archived_messages_internal(&state, None, 10).err(),
            Some("be.error.noActiveGameSession".to_string())
        );

        let mut game = make_game();
        game.archived_messages = vec![
            InboxMessage::new(
                "a".into(),
                "S".into(),
                "B".into(),
                "X".into(),
                "2030-01-01".into(),
            ),
            InboxMessage::new(
                "b".into(),
                "S".into(),
                "B".into(),
                "X".into(),
                "2030-02-01".into(),
            ),
        ];
        state.set_game(game);

        let page = get_archived_messages_internal(&state, None, 1).unwrap();
        assert_eq!(page.items[0].id, "b");
        let next = get_archived_messages_internal(&state, page.next_before, 1).unwrap();
        assert_eq!(next.items[0].id, "a");
        assert_eq!(next.next_before, None);
    }

    fn attributes() -> PlayerAttributes {
        PlayerAttributes {
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 60,
            shooting: 60,
            tackling: 60,
            dribbling: 60,
            defending: 60,
            positioning: 60,
            vision: 60,
            decisions: 60,
            composure: 60,
            aggression: 60,
            teamwork: 60,
            leadership: 60,
            handling: 30,
            reflexes: 30,
            aerial: 60,
        }
    }

    fn squad_player(id: &str) -> Player {
        let mut player = Player::new(
            id.to_string(),
            "Player".to_string(),
            "Player".to_string(),
            "1995-01-01".to_string(),
            "England".to_string(),
            Position::Forward,
            attributes(),
        );
        player.team_id = Some("team-1".to_string());
        player.morale = 80;
        player
    }

    /// A message with one choose-an-option action, as a player event or a sponsor offer carries.
    fn choice_message(id: &str, options: &[&str]) -> InboxMessage {
        let mut message = actionable_message(id, "2026-08-20");
        message.actions[0].action_type = ActionType::ChooseOption {
            options: options
                .iter()
                .map(|option| ActionOption {
                    id: option.to_string(),
                    label: option.to_string(),
                    description: String::new(),
                    label_key: None,
                    description_key: None,
                })
                .collect(),
        };
        message
    }

    const CONCERN: &str = "contract_concern_player-1_2026-08-20";
    const CONCERN_ACTION: &str = "action-contract_concern_player-1_2026-08-20";

    /// The user's club has a player with a contract concern and a teammate who hears of the answer.
    fn state_with_a_contract_concern() -> StateManager {
        let state = StateManager::new();
        let mut game = make_game();
        game.players = vec![squad_player("player-1"), squad_player("player-2")];
        let mut message = choice_message(CONCERN, &["reassure", "noncommittal", "no_renewal"]);
        message.context.player_id = Some("player-1".to_string());
        game.messages = vec![message];
        state.set_game(game);
        state
    }

    fn morales(state: &StateManager) -> (u8, u8) {
        state
            .get_game(|game| (game.players[0].morale, game.players[1].morale))
            .expect("game")
    }

    /// Given a contract concern answered "No renewal", which costs the player and every teammate morale
    /// When the same answer arrives again (a second click, or an agent calling twice)
    /// Then the second is refused with the already-resolved key and nobody loses morale twice.
    #[test]
    fn a_player_response_answered_twice_is_refused_and_applies_once() {
        let state = state_with_a_contract_concern();
        let before = morales(&state);
        resolve_message_action_internal(
            &state,
            Career::Active,
            CONCERN,
            CONCERN_ACTION,
            Some("no_renewal"),
        )
        .expect("the first answer applies");
        let after_first = morales(&state);
        assert!(
            after_first.0 < before.0 && after_first.1 < before.1,
            "the first answer costs the player and the teammate morale"
        );

        let second = resolve_message_action_internal(
            &state,
            Career::Active,
            CONCERN,
            CONCERN_ACTION,
            Some("no_renewal"),
        );

        assert_eq!(
            second.map(|_| ()),
            Err("be.error.actionAlreadyResolved".to_string())
        );
        assert_eq!(morales(&state), after_first);
    }

    /// Given a contract concern answered "No renewal"
    /// When a different answer, "Reassure", arrives for the same action
    /// Then it is refused as well: the conversation had its answer, and morale is as the first left it.
    #[test]
    fn a_different_second_answer_to_the_same_action_is_refused() {
        let state = state_with_a_contract_concern();
        resolve_message_action_internal(
            &state,
            Career::Active,
            CONCERN,
            CONCERN_ACTION,
            Some("no_renewal"),
        )
        .expect("the first answer applies");
        let after_first = morales(&state);

        let second = resolve_message_action_internal(
            &state,
            Career::Active,
            CONCERN,
            CONCERN_ACTION,
            Some("reassure"),
        );

        assert_eq!(
            second.map(|_| ()),
            Err("be.error.actionAlreadyResolved".to_string())
        );
        assert_eq!(morales(&state), after_first);
    }

    /// Given a sponsor offer accepted, and some weeks of the deal since gone by
    /// When "Accept" arrives again
    /// Then it is refused and the deal's remaining weeks are not reset to a fresh twelve.
    #[test]
    fn a_sponsor_offer_accepted_twice_does_not_sign_the_deal_again() {
        let state = StateManager::new();
        let mut game = make_game();
        let mut offer = choice_message("sponsor_2026-08-20", &["accept", "decline"]);
        offer
            .i18n_params
            .insert("amount".to_string(), "400000".to_string());
        offer
            .i18n_params
            .insert("sponsor".to_string(), "Acme".to_string());
        game.messages = vec![offer];
        state.set_game(game);
        let remaining_weeks = |state: &StateManager| {
            state
                .get_game(|game| {
                    game.teams[0]
                        .sponsorship
                        .as_ref()
                        .map(|s| s.remaining_weeks)
                })
                .expect("game")
        };
        resolve_message_action_internal(
            &state,
            Career::Active,
            "sponsor_2026-08-20",
            "action-sponsor_2026-08-20",
            Some("accept"),
        )
        .expect("the first accept signs the deal");
        assert_eq!(remaining_weeks(&state), Some(12));
        state
            .update_game(|game| {
                if let Some(sponsorship) = game.teams[0].sponsorship.as_mut() {
                    sponsorship.remaining_weeks = 5;
                }
            })
            .expect("game");

        let second = resolve_message_action_internal(
            &state,
            Career::Active,
            "sponsor_2026-08-20",
            "action-sponsor_2026-08-20",
            Some("accept"),
        );

        assert_eq!(
            second.map(|_| ()),
            Err("be.error.actionAlreadyResolved".to_string())
        );
        assert_eq!(remaining_weeks(&state), Some(5));
    }

    /// Given a plain action (no option to choose) already resolved
    /// When it is resolved again
    /// Then the caller is told it was already resolved, the same as for an answer with an effect.
    #[test]
    fn a_plain_action_resolved_twice_is_refused() {
        let state = StateManager::new();
        let mut game = make_game();
        game.messages = vec![actionable_message("msg-1", "2026-08-20")];
        state.set_game(game);
        resolve_message_action_internal(&state, Career::Active, "msg-1", "action-msg-1", None)
            .expect("the first resolve");

        let second =
            resolve_message_action_internal(&state, Career::Active, "msg-1", "action-msg-1", None);

        assert_eq!(
            second.map(|_| ()),
            Err("be.error.actionAlreadyResolved".to_string())
        );
    }

    /// Given a message with two actions, one of them resolved
    /// When the other is resolved
    /// Then it is not refused: only an action that was itself resolved is.
    #[test]
    fn resolving_one_action_leaves_the_messages_other_action_open() {
        let state = StateManager::new();
        let mut game = make_game();
        let mut message = actionable_message("msg-1", "2026-08-20");
        message.actions[0].resolved = true;
        message.actions.push(MessageAction {
            id: "second".to_string(),
            label: "Second".to_string(),
            action_type: ActionType::Acknowledge,
            resolved: false,
            label_key: None,
        });
        game.messages = vec![message];
        state.set_game(game);

        let resolved =
            resolve_message_action_internal(&state, Career::Active, "msg-1", "second", None);

        assert!(resolved.is_ok());
    }
}
