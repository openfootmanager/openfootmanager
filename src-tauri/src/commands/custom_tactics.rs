//! The player's saved tactics (#390): list, save and delete. The rules are in
//! `ofm_core::custom_tactics`; these only reach the active game.

use log::info;
use std::sync::Arc;
use tauri::State;

use domain::custom_tactic::CustomTactic;
use ofm_core::state::{Career, StateManager};

use crate::commands::util::mutate_active_game_for;

const NO_ACTIVE_GAME: &str = "be.error.noActiveGameSession";

/// The career's saved tactics, in the order the player keeps them.
pub fn list_custom_tactics_internal(state: &StateManager) -> Result<Vec<CustomTactic>, String> {
    state
        .get_game(|game| game.custom_tactics.clone())
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

/// Validates and stores the tactic; returns the whole list, or the key of what is wrong.
pub fn save_custom_tactic_internal(
    state: &StateManager,
    tactic: &CustomTactic,
    career: Career,
) -> Result<Vec<CustomTactic>, String> {
    mutate_active_game_for(state, career, |game| {
        ofm_core::custom_tactics::save_custom_tactic(game, tactic)
    })
    .map(|game| game.custom_tactics)
}

/// Removes the tactic with `id`; returns the list that is left, or the not-found key.
pub fn delete_custom_tactic_internal(
    state: &StateManager,
    id: &str,
    career: Career,
) -> Result<Vec<CustomTactic>, String> {
    mutate_active_game_for(state, career, |game| {
        ofm_core::custom_tactics::delete_custom_tactic(game, id)
    })
    .map(|game| game.custom_tactics)
}

/// Whether the old browser library still has to be copied into this career. It is copied once: a
/// tactic the player deletes afterwards must not be copied back on the next load.
pub fn legacy_tactics_import_pending_internal(state: &StateManager) -> Result<bool, String> {
    state
        .get_game(|game| !game.legacy_tactics_imported)
        .ok_or_else(|| NO_ACTIVE_GAME.to_string())
}

/// Records that the old browser library has been copied in, so it never is again.
pub fn mark_legacy_tactics_imported_internal(
    state: &StateManager,
    career: Career,
) -> Result<(), String> {
    mutate_active_game_for(state, career, |game| {
        game.legacy_tactics_imported = true;
        Ok(())
    })
    .map(|_| ())
}

/// The career's saved tactics.
#[tauri::command]
pub fn list_custom_tactics(
    state: State<'_, Arc<StateManager>>,
) -> Result<Vec<CustomTactic>, String> {
    list_custom_tactics_internal(&state)
}

/// Adds the tactic, or replaces the one with the same id; returns the whole list.
#[tauri::command]
pub fn save_custom_tactic(
    state: State<'_, Arc<StateManager>>,
    tactic: CustomTactic,
    career_token: u64,
) -> Result<Vec<CustomTactic>, String> {
    info!("[cmd] save_custom_tactic: {}", tactic.id);
    save_custom_tactic_internal(&state, &tactic, Career::Token(career_token))
}

/// Removes the tactic; returns the list that is left.
#[tauri::command]
pub fn delete_custom_tactic(
    state: State<'_, Arc<StateManager>>,
    id: String,
    career_token: u64,
) -> Result<Vec<CustomTactic>, String> {
    info!("[cmd] delete_custom_tactic: {id}");
    delete_custom_tactic_internal(&state, &id, Career::Token(career_token))
}

/// Whether the old browser library still has to be copied in.
#[tauri::command]
pub fn legacy_tactics_import_pending(state: State<'_, Arc<StateManager>>) -> Result<bool, String> {
    legacy_tactics_import_pending_internal(&state)
}

/// Marks the old browser library as copied in.
#[tauri::command]
pub fn mark_legacy_tactics_imported(
    state: State<'_, Arc<StateManager>>,
    career_token: u64,
) -> Result<(), String> {
    info!("[cmd] mark_legacy_tactics_imported");
    mark_legacy_tactics_imported_internal(&state, Career::Token(career_token))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;

    fn state_with_game() -> StateManager {
        let state = StateManager::new();
        let clock = GameClock::new(Utc.with_ymd_and_hms(2031, 7, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "m".into(),
            "A".into(),
            "B".into(),
            "1980-01-01".into(),
            "England".into(),
        );
        state.set_game(Game::new(clock, manager, vec![], vec![], vec![], vec![]));
        state
    }

    fn tactic(id: &str) -> CustomTactic {
        CustomTactic {
            id: id.into(),
            name: format!("Tactic {id}"),
            description: String::new(),
            formation: "4-3-3".into(),
            play_style: "Possession".into(),
            source_preset_name: None,
        }
    }

    /// Given an active career
    /// When tactics are saved, listed, and one deleted
    /// Then each call returns the library as it now stands, and the game keeps it.
    #[test]
    fn saving_listing_and_deleting_go_through_the_active_game() {
        let state = state_with_game();
        assert!(list_custom_tactics_internal(&state).unwrap().is_empty());

        save_custom_tactic_internal(&state, &tactic("a"), Career::Active).unwrap();
        let after_second =
            save_custom_tactic_internal(&state, &tactic("b"), Career::Active).unwrap();
        assert_eq!(after_second.len(), 2);

        let left = delete_custom_tactic_internal(&state, "a", Career::Active).unwrap();
        assert_eq!(
            left.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["b"]
        );
        assert_eq!(list_custom_tactics_internal(&state).unwrap(), left);
    }

    /// Given a career whose old browser library has not been copied in
    /// When the import is marked done
    /// Then it is pending before, not pending after, and marking twice changes nothing.
    #[test]
    fn the_legacy_import_is_pending_until_it_is_marked_done() {
        let state = state_with_game();
        assert_eq!(legacy_tactics_import_pending_internal(&state), Ok(true));

        mark_legacy_tactics_imported_internal(&state, Career::Active).unwrap();
        mark_legacy_tactics_imported_internal(&state, Career::Active).unwrap();

        assert_eq!(legacy_tactics_import_pending_internal(&state), Ok(false));
    }

    /// Given no game at all
    /// When the import state is asked or marked
    /// Then each is the no-active-game key.
    #[test]
    fn the_legacy_import_needs_a_game() {
        let empty = StateManager::new();
        assert_eq!(
            legacy_tactics_import_pending_internal(&empty),
            Err(NO_ACTIVE_GAME.to_string())
        );
        assert_eq!(
            mark_legacy_tactics_imported_internal(&empty, Career::Active),
            Err(NO_ACTIVE_GAME.to_string())
        );
    }

    /// Given career A is active and a screen holds A's token, and then career B is installed
    /// When the screen's save, delete and import mark arrive
    /// Then each is refused with the career-changed key, nothing reaches B, and B's save, written
    ///      to a .db and read back with SQL, holds no tactic and no import mark.
    #[test]
    fn a_request_for_a_replaced_career_is_refused_and_writes_nothing_to_the_new_one() {
        use crate::application::career::install_career;
        use db::game_database::GameDatabase;
        use db::game_persistence::GamePersistenceWriter;
        use domain::stats::StatsState;

        let state = state_with_game();
        let game_of = |first_name: &str| {
            let manager = Manager::new(
                "m".into(),
                first_name.into(),
                "B".into(),
                "1980-01-01".into(),
                "England".into(),
            );
            let clock = GameClock::new(Utc.with_ymd_and_hms(2031, 7, 1, 12, 0, 0).unwrap());
            Game::new(clock, manager, vec![], vec![], vec![], vec![])
        };
        let token_a = install_career(&state, game_of("Alice"), StatsState::default(), None);
        save_custom_tactic_internal(&state, &tactic("a"), Career::Token(token_a)).unwrap();
        let token_b = install_career(&state, game_of("Beatrix"), StatsState::default(), None);
        assert_ne!(token_a, token_b);

        let refusals = [
            save_custom_tactic_internal(&state, &tactic("late"), Career::Token(token_a))
                .map(|_| ()),
            delete_custom_tactic_internal(&state, "a", Career::Token(token_a)).map(|_| ()),
            mark_legacy_tactics_imported_internal(&state, Career::Token(token_a)),
        ];

        for refusal in refusals {
            assert_eq!(refusal, Err("be.error.careerChanged".to_string()));
        }
        let career_b = state.get_game(|game| game.clone()).unwrap();
        assert!(career_b.custom_tactics.is_empty());
        let db = GameDatabase::open_in_memory().unwrap();
        GamePersistenceWriter::write_game(&db, &career_b, "save-b", "B").unwrap();
        let stored_tactics: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM custom_tactics", [], |row| row.get(0))
            .unwrap();
        let stored_mark: bool = db
            .conn()
            .query_row("SELECT legacy_tactics_imported FROM game_meta", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(stored_tactics, 0);
        assert!(!stored_mark);
        // And B's own token still works.
        assert!(save_custom_tactic_internal(&state, &tactic("b"), Career::Token(token_b)).is_ok());
    }

    /// Given an invalid tactic, a missing id, and no game at all
    /// When they are sent
    /// Then each is a translation key, and the library is unchanged.
    #[test]
    fn failures_are_translation_keys_and_change_nothing() {
        let state = state_with_game();
        save_custom_tactic_internal(&state, &tactic("a"), Career::Active).unwrap();

        let mut bad = tactic("b");
        bad.formation = "1-1-1".into();
        assert_eq!(
            save_custom_tactic_internal(&state, &bad, Career::Active),
            Err("be.error.customTactic.invalidFormation".to_string())
        );
        assert_eq!(
            delete_custom_tactic_internal(&state, "missing", Career::Active),
            Err("be.error.customTactic.notFound".to_string())
        );
        assert_eq!(list_custom_tactics_internal(&state).unwrap().len(), 1);

        let empty = StateManager::new();
        assert_eq!(
            list_custom_tactics_internal(&empty),
            Err(NO_ACTIVE_GAME.to_string())
        );
    }
}
