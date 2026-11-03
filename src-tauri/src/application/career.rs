use domain::stats::StatsState;
use ofm_core::{game::Game, state::StateManager};

/// Install a created or loaded career together with its stats and save identity.
///
/// Ordinary same-career writers also use `set_game`, so reset the transient
/// session only here. Sharing the lifecycle lock prevents a concurrent start
/// or finish from publishing the previous career's session or result afterward.
///
/// The game is stamped with a fresh career token, which is returned so a caller that hands the game
/// to the screen straight away (a new game, a chosen club) hands it the stamped one.
pub(crate) fn install_career(
    state: &StateManager,
    mut game: Game,
    stats: StatsState,
    save_id: Option<String>,
) -> u64 {
    let _operation = super::live_session::operation();
    drop(state.take_live_match());
    let career_token = state.next_career_token();
    game.career_token = career_token;
    state.set_game(game);
    state.set_stats_state(stats);
    match save_id {
        Some(id) => state.set_save_id(id),
        None => state.clear_save_id(),
    }
    career_token
}

/// [`install_career`] for a game that goes to the screen straight away (a new game, a chosen
/// club): returns the game as the screen must hold it, carrying the installed career's token.
/// A screen holding the unstamped game would send token 0 with every tactics write and have each
/// refused as another career's.
pub(crate) fn install_career_for_screen(
    state: &StateManager,
    game: Game,
    stats: StatsState,
    save_id: Option<String>,
) -> Game {
    let mut shown = game.clone();
    shown.career_token = install_career(state, game, stats, save_id);
    shown
}

/// The active game together with the save it belongs to, read under the career lock
/// `install_career` holds. Both come from one career: a load or a new game installed meanwhile
/// cannot hand back one career's game with another's save id.
pub(crate) fn snapshot_career(state: &StateManager) -> Option<(Game, Option<String>)> {
    let _operation = super::live_session::operation();
    let game = state.get_game(|game| game.clone())?;
    Some((game, state.get_save_id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use ofm_core::clock::GameClock;
    use std::sync::mpsc;
    use std::time::Duration;

    fn game_managed_by(first_name: &str) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr1".to_string(),
            first_name.to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    /// Given two careers installed one after the other
    /// When each is read back
    /// Then each game carries its own token, the one install_career returned, and they differ.
    #[test]
    fn each_installed_career_is_stamped_with_a_token_of_its_own() {
        let state = StateManager::new();

        let first = install_career(
            &state,
            game_managed_by("Alice"),
            StatsState::default(),
            None,
        );
        let first_seen = state.get_game(|game| game.career_token).unwrap();
        let second = install_career(
            &state,
            game_managed_by("Beatrix"),
            StatsState::default(),
            None,
        );
        let second_seen = state.get_game(|game| game.career_token).unwrap();

        assert_eq!((first_seen, second_seen), (first, second));
        assert_ne!(first, second);
        assert_ne!(first, 0);
    }

    /// Given career A is installed and a load of career B is mid-install, holding the career lock
    /// When a snapshot is requested meanwhile
    /// Then it waits for the lock and returns career B's game with career B's save id, never A's
    /// game paired with B's id or the reverse.
    #[test]
    fn a_snapshot_waits_for_an_install_in_progress_and_returns_one_careers_pair() {
        let state = std::sync::Arc::new(StateManager::new());
        state.set_game(game_managed_by("Alice"));
        state.set_save_id("save-a".to_string());

        let install_in_progress = super::super::live_session::operation();
        let (sender, receiver) = mpsc::channel();
        let snapshotting = std::sync::Arc::clone(&state);
        let reader = std::thread::spawn(move || {
            sender.send(snapshot_career(&snapshotting)).unwrap();
        });
        // Long enough for a snapshot that ignored the lock to have read career A.
        std::thread::sleep(Duration::from_millis(200));
        assert!(
            receiver.try_recv().is_err(),
            "the snapshot must wait for the install"
        );

        state.set_game(game_managed_by("Beatrix"));
        state.set_save_id("save-b".to_string());
        drop(install_in_progress);

        let (snapshot_game, snapshot_save_id) = receiver.recv().unwrap().unwrap();
        reader.join().unwrap();
        assert_eq!(snapshot_game.manager.first_name, "Beatrix");
        assert_eq!(snapshot_save_id.as_deref(), Some("save-b"));
    }
}
