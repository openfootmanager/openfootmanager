//! The weekly auto-save (#1181). After a screen's advance moves the game into a new in-game week,
//! the career is saved when the player's Auto-Save setting is on.
//!
//! Once a week rather than every day: on a full world one save costs about twenty days of play
//! (1.5 s against 70 ms for 440 clubs, measured 4 Nov 2026), so saving every day would make every
//! Continue wait. It runs on the advance's worker thread, never the webview's.

use chrono::{Datelike, NaiveDate};
use db::save_manager::SaveManager;
use ofm_core::state::{Career, StateManager};
use std::sync::Mutex;

/// Whether the game moved from `before` into a later ISO week by `after`.
pub(crate) fn crossed_into_new_week(before: NaiveDate, after: NaiveDate) -> bool {
    after > before && after.iso_week() != before.iso_week()
}

/// The active game's date, read before an advance so [`save_if_a_new_week`] can tell whether the
/// advance crossed a week.
pub(crate) fn game_date(state: &StateManager) -> Option<NaiveDate> {
    state.get_game(|game| game.clock.current_date.date_naive())
}

/// Saves the active career when the advance that began on `before` has crossed into a new week
/// and `enabled` (the player's setting, read only then) says so. Returns whether it saved.
///
/// A save that cannot be made (an unsaved career, a live match in progress, a disk error) is logged
/// and the advance stands: the manual Save still works, and the next week tries again.
pub(crate) fn save_if_a_new_week(
    state: &StateManager,
    saves: &Mutex<SaveManager>,
    enabled: impl FnOnce() -> bool,
    before: Option<NaiveDate>,
    career: Career,
) -> bool {
    let (Some(before), Some(after)) = (before, game_date(state)) else {
        return false;
    };
    if !crossed_into_new_week(before, after) || !enabled() {
        return false;
    }
    let Ok(mut save_manager) = saves.lock() else {
        log::warn!("[auto-save] skipped on {after}: the save manager is unavailable");
        return false;
    };
    match super::saving::persist_active_game(state, &mut save_manager, career) {
        Ok(()) => {
            log::info!("[auto-save] saved on {after}");
            true
        }
        Err(error) => {
            log::warn!("[auto-save] skipped on {after}: {error}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use ofm_core::game::Game;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    /// Given a Sunday and the Monday after it, when the advance is checked, then it crossed a week.
    #[test]
    fn a_day_into_a_new_week_crosses_it() {
        assert!(crossed_into_new_week(day("2026-08-16"), day("2026-08-17")));
    }

    /// Given a Monday and the Sunday that ends its week, when checked, then no week was crossed.
    #[test]
    fn days_within_one_week_do_not_cross_it() {
        assert!(!crossed_into_new_week(day("2026-08-17"), day("2026-08-23")));
        assert!(!crossed_into_new_week(day("2026-08-17"), day("2026-08-17")));
    }

    /// Given a skip from a Wednesday to the Thursday two weeks on, when checked, then it crossed.
    #[test]
    fn a_skip_across_several_weeks_crosses() {
        assert!(crossed_into_new_week(day("2026-08-19"), day("2026-09-03")));
    }

    /// A saved career whose clock reads `date`, the state holding it, and its save manager.
    fn saved_career_on(
        date: (i32, u32, u32),
    ) -> (tempfile::TempDir, StateManager, Mutex<SaveManager>, String) {
        let clock = GameClock::new(
            Utc.with_ymd_and_hms(date.0, date.1, date.2, 12, 0, 0)
                .unwrap(),
        );
        let mut manager = Manager::new(
            "manager".into(),
            "Pat".into(),
            "Doyle".into(),
            "1985-03-01".into(),
            "Ireland".into(),
        );
        manager.hire("club".into());
        let club = Team::new(
            "club".into(),
            "Club".into(),
            "CLU".into(),
            "Ireland".into(),
            "Dublin".into(),
            "Ground".into(),
            10_000,
        );
        let game = Game::new(clock, manager, vec![club], vec![], vec![], vec![]);
        let dir = tempfile::tempdir().unwrap();
        let mut saves = SaveManager::init(dir.path()).unwrap();
        let save_id = saves.create_save(&game, "Auto-save").unwrap();
        let state = StateManager::new();
        state.set_game(game);
        state.set_save_id(save_id.clone());
        (dir, state, Mutex::new(saves), save_id)
    }

    fn advance(state: &StateManager, days: i64) {
        state
            .update_game_for(Career::Active, |game| game.clock.advance_days(days))
            .unwrap();
    }

    fn saved_date(saves: &Mutex<SaveManager>, save_id: &str) -> NaiveDate {
        saves
            .lock()
            .unwrap()
            .load_game(save_id)
            .unwrap()
            .clock
            .current_date
            .date_naive()
    }

    /// Given a saved career on a Sunday with Auto-Save on; when a day's advance brings Monday;
    /// then the career is saved, and the save on disk reads Monday.
    #[test]
    fn an_advance_into_a_new_week_saves_the_career() {
        let (_dir, state, saves, save_id) = saved_career_on((2026, 8, 16));
        let before = game_date(&state);
        advance(&state, 1);

        assert!(save_if_a_new_week(
            &state,
            &saves,
            || true,
            before,
            Career::Active
        ));
        assert_eq!(saved_date(&saves, &save_id), day("2026-08-17"));
    }

    /// Given the same with Auto-Save off; when the week turns; then nothing is saved.
    #[test]
    fn with_auto_save_off_the_new_week_is_not_saved() {
        let (_dir, state, saves, save_id) = saved_career_on((2026, 8, 16));
        let before = game_date(&state);
        advance(&state, 1);

        assert!(!save_if_a_new_week(
            &state,
            &saves,
            || false,
            before,
            Career::Active
        ));
        assert_eq!(saved_date(&saves, &save_id), day("2026-08-16"));
    }

    /// Given a saved career on a Monday with Auto-Save on; when a day passes within the week;
    /// then nothing is saved.
    #[test]
    fn an_advance_within_the_week_is_not_saved() {
        let (_dir, state, saves, save_id) = saved_career_on((2026, 8, 17));
        let before = game_date(&state);
        advance(&state, 1);

        assert!(!save_if_a_new_week(
            &state,
            &saves,
            || true,
            before,
            Career::Active
        ));
        assert_eq!(saved_date(&saves, &save_id), day("2026-08-17"));
    }

    /// Given a career that was never saved; when the week turns; then nothing is saved and the
    /// advance is not failed.
    #[test]
    fn an_unsaved_career_is_skipped_quietly() {
        let (_dir, state, saves, _) = saved_career_on((2026, 8, 16));
        state.clear_save_id();
        let before = game_date(&state);
        advance(&state, 1);

        assert!(!save_if_a_new_week(
            &state,
            &saves,
            || true,
            before,
            Career::Active
        ));
    }
}
