use std::collections::HashSet;
use std::sync::Arc;

use log::info;
use tauri::State;

use domain::message::InboxMessage;
use ofm_core::state::StateManager;

// These commands only ever touch the inbox, so they mutate the game in place
// (no upfront whole-world clone) and return just the updated message list. The
// frontend patches its message slice rather than replacing the entire game —
// which is what made opening/deleting messages hitch.

#[tauri::command]
pub fn mark_message_read(
    state: State<'_, Arc<StateManager>>,
    message_id: String,
) -> Result<Vec<InboxMessage>, String> {
    mark_message_read_internal(&state, &message_id)
}

pub fn mark_message_read_internal(
    state: &StateManager,
    message_id: &str,
) -> Result<Vec<InboxMessage>, String> {
    log::debug!("[cmd] mark_message_read: {}", message_id);
    state
        .update_game(|game| {
            if let Some(msg) = game.messages.iter_mut().find(|m| m.id == message_id) {
                msg.read = true;
            }
            game.messages.clone()
        })
        .ok_or("be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub fn delete_message(
    state: State<'_, Arc<StateManager>>,
    message_id: String,
) -> Result<Vec<InboxMessage>, String> {
    delete_message_internal(&state, &message_id)
}

pub fn delete_message_internal(
    state: &StateManager,
    message_id: &str,
) -> Result<Vec<InboxMessage>, String> {
    log::debug!("[cmd] delete_message: {}", message_id);
    state
        .update_game(|game| {
            game.messages.retain(|message| message.id != message_id);
            game.messages.clone()
        })
        .ok_or("be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub fn delete_messages(
    state: State<'_, Arc<StateManager>>,
    message_ids: Vec<String>,
) -> Result<Vec<InboxMessage>, String> {
    delete_messages_internal(&state, message_ids)
}

pub fn delete_messages_internal(
    state: &StateManager,
    message_ids: Vec<String>,
) -> Result<Vec<InboxMessage>, String> {
    log::debug!("[cmd] delete_messages: {}", message_ids.len());
    let message_ids: HashSet<String> = message_ids.into_iter().collect();
    state
        .update_game(|game| {
            game.messages
                .retain(|message| !message_ids.contains(&message.id));
            game.messages.clone()
        })
        .ok_or("be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub fn mark_all_messages_read(
    state: State<'_, Arc<StateManager>>,
) -> Result<Vec<InboxMessage>, String> {
    mark_all_messages_read_internal(&state)
}

pub fn mark_all_messages_read_internal(state: &StateManager) -> Result<Vec<InboxMessage>, String> {
    log::debug!("[cmd] mark_all_messages_read");
    state
        .update_game(|game| {
            for msg in game.messages.iter_mut() {
                msg.read = true;
            }
            game.messages.clone()
        })
        .ok_or("be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub fn clear_old_messages(
    state: State<'_, Arc<StateManager>>,
) -> Result<Vec<InboxMessage>, String> {
    clear_old_messages_internal(&state)
}

pub fn clear_old_messages_internal(state: &StateManager) -> Result<Vec<InboxMessage>, String> {
    log::debug!("[cmd] clear_old_messages");
    state
        .update_game(|game| {
            let current_date = game.clock.current_date.format("%Y-%m-%d").to_string();
            // Keep only: unread messages, messages with unresolved actions, and messages from recent 14 days
            game.messages.retain(|m| {
                if !m.read {
                    return true;
                }
                if m.actions.iter().any(|a| !a.resolved) {
                    return true;
                }
                // Keep recent messages (within 14 days)
                if let Ok(msg_date) = chrono::NaiveDate::parse_from_str(&m.date, "%Y-%m-%d") {
                    if let Ok(cur_date) =
                        chrono::NaiveDate::parse_from_str(&current_date, "%Y-%m-%d")
                    {
                        return (cur_date - msg_date).num_days() <= 14;
                    }
                }
                false
            });
            game.messages.clone()
        })
        .ok_or("be.error.noActiveGameSession".to_string())
}

#[tauri::command]
pub fn resolve_message_action(
    state: State<'_, Arc<StateManager>>,
    message_id: String,
    action_id: String,
    option_id: Option<String>,
) -> Result<serde_json::Value, String> {
    resolve_message_action_internal(&state, &message_id, &action_id, option_id.as_deref())
}

pub fn resolve_message_action_internal(
    state: &StateManager,
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
    let (game, effect, effect_i18n_key, effect_i18n_params) = state
        .update_game(|game| {
            // Try to apply player conversation or random event response
            let (effect, effect_i18n_key, effect_i18n_params) = if let Some(opt) = option_id {
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
            (game.clone(), effect, effect_i18n_key, effect_i18n_params)
        })
        .ok_or("be.error.noActiveGameSession".to_string())?;

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
        mark_all_messages_read_internal, mark_message_read_internal,
        resolve_message_action_internal,
    };
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::message::{ActionType, InboxMessage, MessageAction};
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;
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

    #[test]
    fn clear_old_messages_internal_keeps_only_expected_messages() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = clear_old_messages_internal(&state).expect("response");
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

        let response = resolve_message_action_internal(&state, "msg-1", "action-msg-1", None)
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

        let response = mark_message_read_internal(&state, "keep-unread").expect("response");

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

        let response = mark_all_messages_read_internal(&state).expect("response");

        assert!(response.iter().all(|message| message.read));

        let stored_game = state.get_game(|game| game.clone()).expect("stored game");
        assert!(stored_game.messages.iter().all(|message| message.read));
    }

    #[test]
    fn delete_message_internal_updates_state() {
        let state = StateManager::new();
        state.set_game(make_game());

        let response = delete_message_internal(&state, "remove-stale").expect("response");

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
}
