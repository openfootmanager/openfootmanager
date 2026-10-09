//! The smoke subset: the scenarios that run first in the nightly. A failure here stops the rest.
//!
//! Every test needs a built `--features mcp` binary and a display. Reproduce one with
//! `OFM_E2E_SEED=<seed> xvfb-run cargo test -p mcp-e2e --test smoke <scenario> -- --ignored --nocapture`.

use mcp_e2e::career::{self, begin, fixtures, game_state, today};
use mcp_e2e::{assert_refusal_changes_nothing, App, AppConfig, CallError, Mode, Snapshot};
use serde_json::{json, Value};

fn is_mine(fixture: &Value, club: &str) -> bool {
    fixture["home_team_id"] == club || fixture["away_team_id"] == club
}

/// Given a seeded English pyramid and `seasonStart`
/// When a manager is created and the highest-reputation English club selected
/// Then the career opens on its first day, no fixture is Completed, contracts start on or before
/// today, and a save exists under the temp data root.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn northern_season_start_opens_on_the_clubs_first_day() {
    let app = App::launch(&AppConfig::default()).unwrap();

    let career = begin(&app, career::seed(), "seasonStart", "ENG").unwrap();

    let game = game_state(&app).unwrap();
    assert_eq!(game["manager"]["team_id"], json!(career.club_id));
    assert_eq!(game["clock"]["current_date"], game["clock"]["start_date"]);
    assert!(fixtures(&game)
        .iter()
        .all(|(_, f)| f["status"] != "Completed"));
    let opening = today(&game);
    let late_contracts: Vec<&str> = game["players"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["team_id"] == json!(career.club_id))
        .filter(|p| {
            p["contract_start"]
                .as_str()
                .is_some_and(|start| start > opening.as_str())
        })
        .filter_map(|p| p["id"].as_str())
        .collect();
    assert!(
        late_contracts.is_empty(),
        "contracts dated after today: {late_contracts:?}"
    );
    let saves = app
        .saves_dir()
        .expect("a saves directory under the temp data root");
    assert!(std::fs::read_dir(saves)
        .unwrap()
        .flatten()
        .any(|entry| entry.path().extension().is_some_and(|ext| ext == "db")));
}

/// Given the first match day
/// When `time_advance` is called
/// Then my fixture is Completed once, no fixture dated that day or earlier is Scheduled in any
/// competition, and the date is one day later.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn a_delegated_matchday_plays_everything_dated_today() {
    let app = App::launch(&AppConfig::default()).unwrap();
    let career = begin(&app, career::seed(), "seasonStart", "ENG").unwrap();
    app.client().time_skip_to_match_day().unwrap();
    let match_day = game_state(&app).map(|game| today(&game)).unwrap();

    app.client().time_advance().unwrap();

    let game = game_state(&app).unwrap();
    let all = fixtures(&game);
    let mine_completed = all
        .iter()
        .filter(|(_, f)| is_mine(f, &career.club_id) && f["status"] == "Completed")
        .count();
    assert_eq!(
        mine_completed, 1,
        "my fixture should be Completed exactly once"
    );
    let stranded: Vec<String> = all
        .iter()
        .filter(|(_, f)| {
            f["status"] == "Scheduled" && f["date"].as_str().unwrap_or("9999") <= match_day.as_str()
        })
        .map(|(competition, f)| format!("{competition}/{}", f["id"]))
        .collect();
    assert!(
        stranded.is_empty(),
        "still Scheduled on or before {match_day}: {stranded:?}"
    );
    assert_ne!(today(&game), match_day);
}

/// Given a saved game
/// When the `.db` is opened with the `db` crate
/// Then it equals `info_game_state` (catches a field with no column).
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn the_file_on_disk_holds_what_the_game_holds() {
    let app = App::launch(&AppConfig::default()).unwrap();
    let career = begin(&app, career::seed(), "seasonStart", "ENG").unwrap();
    app.client().game_save().unwrap();
    let live = by_national_team_id(game_state(&app).unwrap());

    let mut saves = db::save_manager::SaveManager::init(&app.saves_dir().unwrap()).unwrap();
    let on_disk = saves.load_game(&career.save_id).unwrap();

    let on_disk = by_national_team_id(serde_json::to_value(&on_disk).unwrap());
    let mut differing = Vec::new();
    json_differences("game", &live, &on_disk, &mut differing);
    assert!(
        differing.is_empty(),
        "{} places differ after a save and load, first ten:\n{}{}",
        differing.len(),
        differing
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
        shapes(&differing)
    );
}

/// The save does not keep national teams in the order the live game holds them, and nothing reads
/// that order, so the comparison puts both in the same one.
fn by_national_team_id(mut game: Value) -> Value {
    if let Some(teams) = game["national_teams"].as_array_mut() {
        teams.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    }
    game
}

fn shapes(paths: &[String]) -> String {
    let mut shapes: std::collections::BTreeMap<String, usize> = Default::default();
    for path in paths {
        let path = path.split(": ").next().unwrap_or(path);
        let mut shape = String::new();
        let mut in_index = false;
        for c in path.chars() {
            match c {
                '[' => {
                    in_index = true;
                    shape.push_str("[]");
                }
                ']' => in_index = false,
                _ if !in_index => shape.push(c),
                _ => {}
            }
        }
        *shapes.entry(shape).or_default() += 1;
    }
    format!("\nshapes:\n{shapes:#?}")
}

/// Where two JSON documents differ, as paths.
fn json_differences(path: &str, live: &Value, on_disk: &Value, found: &mut Vec<String>) {
    match (live, on_disk) {
        (Value::Object(a), Value::Object(b)) => {
            for key in a.keys().chain(b.keys().filter(|k| !a.contains_key(*k))) {
                json_differences(
                    &format!("{path}.{key}"),
                    a.get(key).unwrap_or(&Value::Null),
                    b.get(key).unwrap_or(&Value::Null),
                    found,
                );
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                json_differences(&format!("{path}[{i}]"), x, y, found);
            }
        }
        _ if live != on_disk => found.push(format!("{path}: live {live} / on disk {on_disk}")),
        _ => {}
    }
}

/// Given a sandbox with no game
/// When `time_advance` or a squad mutation is called
/// Then the no-active-game key is returned, ping still works, creation still works.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn a_call_before_any_game_is_refused_and_the_service_survives() {
    let app = App::launch(&AppConfig::default()).unwrap();

    assert_refusal_changes_nothing(&app, "be.error.noActiveGameSession", || {
        app.client().time_advance()
    });
    assert_refusal_changes_nothing(&app, "be.error.noActiveGameSession", || {
        app.client().squad_set_formation("4-4-2")
    });

    app.client().ping().unwrap();
    career::create_world(&app, career::seed(), "seasonStart").unwrap();
}

/// Given required fixtures still to play
/// When `season_advance` is called
/// Then it is refused with season-not-complete and nothing changes.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn rolling_over_before_completion_is_refused() {
    let app = App::launch(&AppConfig::default()).unwrap();
    begin(&app, career::seed(), "seasonStart", "ENG").unwrap();

    assert_refusal_changes_nothing(&app, "be.error.seasonNotComplete", || {
        app.client().season_advance()
    });
}

/// Starts a competition-mode app whose manager already manages `club_id`.
fn competition_app() -> App {
    let club_id = {
        let sandbox = App::launch(&AppConfig::default()).unwrap();
        let game = career::create_world(&sandbox, career::seed(), "seasonStart").unwrap();
        career::top_club_of(&game, "ENG").unwrap()
    };
    App::launch(&AppConfig {
        mode: Mode::Competition,
        extra_args: vec![
            "--mcp-auto-start".to_string(),
            format!("random,{club_id}"),
            "--mcp-seed".to_string(),
            career::seed().to_string(),
        ],
        ..AppConfig::default()
    })
    .unwrap()
}

/// Given competition mode
/// When tools are listed, and the lifecycle tool is called directly
/// Then it is absent, and the direct call is an unknown-tool error with state and saves unchanged.
fn assert_lifecycle_tool_is_hidden(tool: &str) {
    let app = competition_app();

    let listed = app.client().list_tools().unwrap();
    assert!(
        listed.iter().all(|t| t["name"] != tool),
        "{tool} should be absent in competition mode"
    );

    let before = Snapshot::take(&app).unwrap();
    let outcome: Result<Value, CallError> = app.client().call_value(tool, json!({}));
    let after = Snapshot::take(&app).unwrap();
    assert!(
        matches!(&outcome, Err(CallError::Transport(detail)) if detail.contains("tools/call")),
        "{tool} should be an unknown-tool protocol error, got {outcome:?}"
    );
    assert!(
        before.differences(&after).is_empty(),
        "{:?}",
        before.differences(&after)
    );
}

macro_rules! hidden_in_competition_mode {
    ($($test:ident => $tool:literal),+ $(,)?) => {$(
        #[test]
        #[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
        fn $test() {
            assert_lifecycle_tool_is_hidden($tool);
        }
    )+};
}

hidden_in_competition_mode!(
    competition_omits_and_rejects_game_new => "game_new",
    competition_omits_and_rejects_game_select_team => "game_select_team",
    competition_omits_and_rejects_game_export_world => "game_export_world",
    competition_omits_and_rejects_game_exit => "game_exit",
    competition_omits_and_rejects_game_load_save => "game_load_save",
);
