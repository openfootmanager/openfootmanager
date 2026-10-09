//! Starting a career the way an agent does, and reading the game back.

use serde_json::{json, Value};

use crate::app::App;
use crate::client::CallError;

/// The seed every scenario uses unless `OFM_E2E_SEED` says otherwise.
pub const DEFAULT_SEED: u64 = 7;

pub fn seed() -> u64 {
    std::env::var("OFM_E2E_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_SEED)
}

/// A career the scenario began.
#[derive(Debug, Clone)]
pub struct Career {
    pub club_id: String,
    pub save_id: String,
}

/// Creates a generated world from `seed`, leaves the manager clubless, and returns the game.
pub fn create_world(app: &App, seed: u64, start_phase: &str) -> Result<Value, CallError> {
    app.client().call_value(
        "game_new",
        json!({
            "first_name": "Ada",
            "last_name": "Lovelace",
            "nationality": "England",
            "seed": seed,
            "start_year": 2026,
            "start_phase": start_phase,
        }),
    )?;
    game_state(app)
}

/// The whole game, as `info_game_state` reports it.
pub fn game_state(app: &App) -> Result<Value, CallError> {
    Ok(app.client().call_value("info_game_state", json!({}))?["game"].clone())
}

/// The highest-reputation club of `country` (a country code such as "ENG"). Targets are chosen by
/// rule at run time, never by a hard-coded id.
pub fn top_club_of(game: &Value, country: &str) -> Option<String> {
    game["teams"]
        .as_array()?
        .iter()
        .filter(|team| team["country"] == country)
        .max_by_key(|team| team["reputation"].as_i64().unwrap_or(0))
        .and_then(|team| team["id"].as_str())
        .map(str::to_string)
}

/// Creates the world, picks the top club of `country`, and starts the career.
pub fn begin(app: &App, seed: u64, start_phase: &str, country: &str) -> Result<Career, CallError> {
    let game = create_world(app, seed, start_phase)?;
    let club_id = top_club_of(&game, country).ok_or_else(|| CallError::Shape {
        tool: "game_new".to_string(),
        detail: format!("no club of {country} in the generated world"),
    })?;
    let selected = app
        .client()
        .call_value("game_select_team", json!({"team_id": club_id}))?;
    Ok(Career {
        club_id,
        save_id: selected["save_id"].as_str().unwrap_or_default().to_string(),
    })
}

/// Every fixture of every competition, as (competition id, fixture).
pub fn fixtures(game: &Value) -> Vec<(String, Value)> {
    game["competitions"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|competition| {
            let id = competition["id"].as_str().unwrap_or_default().to_string();
            competition["fixtures"]
                .as_array()
                .into_iter()
                .flatten()
                .map(move |fixture| (id.clone(), fixture.clone()))
        })
        .collect()
}

/// The game's date as `YYYY-MM-DD`.
pub fn today(game: &Value) -> String {
    game["clock"]["current_date"]
        .as_str()
        .unwrap_or_default()
        .chars()
        .take(10)
        .collect()
}
