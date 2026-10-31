//! Lets a test call a `#[tauri::command]` itself, with the `State` it takes.
//!
//! Every command is a thin wrapper: it turns the IPC `career_token` into a `Career` and calls an
//! `_internal` function that the tests cover. The wrapper was the one line no test could reach,
//! because a command takes `tauri::State` and a `State` only comes out of an app. A mutant that
//! emptied a wrapper survived for that reason (cloud mutants on the career check). `mock_app()`
//! is the app.

use std::sync::{Arc, Mutex};

use db::save_manager::SaveManager;
use domain::stats::StatsState;
use ofm_core::game::Game;
use ofm_core::state::StateManager;
use tauri::test::{mock_app, MockRuntime};
use tauri::{App, Manager};

use crate::SaveManagerState;

/// A mock app managing the same two states the real one does, with `game` installed as the
/// active career.
pub(crate) struct WrapperApp {
    pub app: App<MockRuntime>,
    /// The token `install_career` stamped on the installed game: what a screen of this career holds.
    pub token: u64,
    /// The save row `game` was created as, when the app was built with one.
    pub save_id: Option<String>,
    _saves: tempfile::TempDir,
}

impl WrapperApp {
    /// `game` installed as the active career; `with_save` also creates its save and makes it the
    /// active save id.
    pub fn new(game: Game, with_save: bool) -> Self {
        let saves = tempfile::tempdir().expect("temp dir");
        let mut save_manager = SaveManager::init(saves.path()).expect("save manager");
        let save_id = with_save.then(|| save_manager.create_save(&game, "wrapper").expect("save"));

        let state = StateManager::new();
        let token = crate::application::career::install_career(
            &state,
            game,
            StatsState::default(),
            save_id.clone(),
        );
        Self::managing(state, token, save_manager, save_id, saves)
    }

    /// An app around a state a test already built (a live match in progress, say). `token` is the
    /// one its game carries.
    pub fn around(state: StateManager, token: u64) -> Self {
        let saves = tempfile::tempdir().expect("temp dir");
        let save_manager = SaveManager::init(saves.path()).expect("save manager");
        Self::managing(state, token, save_manager, None, saves)
    }

    fn managing(
        state: StateManager,
        token: u64,
        save_manager: SaveManager,
        save_id: Option<String>,
        saves: tempfile::TempDir,
    ) -> Self {
        let app = mock_app();
        app.manage(Arc::new(state));
        app.manage(Arc::new(SaveManagerState(Mutex::new(save_manager))));
        Self {
            app,
            token,
            save_id,
            _saves: saves,
        }
    }

    pub fn state(&self) -> tauri::State<'_, Arc<StateManager>> {
        self.app.state::<Arc<StateManager>>()
    }

    pub fn saves(&self) -> tauri::State<'_, Arc<SaveManagerState>> {
        self.app.state::<Arc<SaveManagerState>>()
    }

    /// The saved copy of the active career, read back from disk.
    pub fn saved_game(&self) -> Game {
        let id = self.save_id.as_deref().expect("built with a save");
        self.saves()
            .0
            .lock()
            .expect("save manager")
            .load_game(id)
            .expect("load")
    }
}
