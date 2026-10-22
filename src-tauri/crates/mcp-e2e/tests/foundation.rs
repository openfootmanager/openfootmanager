//! The fixture itself: start, answer, stop, and report what broke.
//!
//! Every test needs a built `--features mcp` binary and a display, so they are ignored by default
//! and run with `--ignored` (under `xvfb-run` where there is no display).

use mcp_e2e::tools::CATALOG;
use mcp_e2e::{App, AppConfig, LaunchError, Snapshot};
use serde_json::json;

/// Given the app built with `--features mcp`, a free port and private data directories
/// When the fixture launches it with `--no-gui --mcp-mode sandbox --auto-save-interval-days 0`
/// Then `initialize` has succeeded and `ping` answers.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn the_app_starts_and_answers_the_handshake() {
    let app = App::launch(&AppConfig::default()).unwrap();

    let pong = app.client().ping().unwrap();

    assert!(pong.message.contains("alive"));
}

/// Given a running app
/// When `initialize` reports its version and the binary is asked `--build-revision`
/// Then the two are the same string: the fixture's stale check and the server's `serverInfo` read
///      one source.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn the_server_reports_the_version_the_binary_prints() {
    let app = App::launch(&AppConfig::default()).unwrap();

    let printed = mcp_e2e::probe_build_version(app.binary(), std::time::Duration::from_secs(15));

    assert_eq!(printed.as_deref(), Some(app.client().server_version()));
}

/// Given no binary where one is expected
/// When the fixture launches
/// Then it fails naming the missing binary, before any assertion.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn a_missing_binary_fails_loudly() {
    let config = AppConfig {
        binary: Some("/nonexistent/openfootmanager".into()),
        ..AppConfig::default()
    };

    match App::launch(&config) {
        Err(LaunchError::BinaryMissing(path)) => assert!(path.ends_with("openfootmanager")),
        Err(other) => panic!("expected a missing-binary error, got {other}"),
        Ok(_) => panic!("launched a binary that does not exist"),
    }
}

/// Given a finished scenario
/// When the fixture drops
/// Then the process is gone and its port is free.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn teardown_leaves_nothing_behind() {
    let real_counter = mcp_e2e::real_startup_failures_file();
    let counter_before = std::fs::metadata(&real_counter)
        .and_then(|m| m.modified())
        .ok();
    let app = App::launch(&AppConfig::default()).unwrap();
    let port = app.port();
    assert!(!app.port_is_free(), "the app should be listening on {port}");

    let scratch = app.data_dir().unwrap().to_path_buf();

    let left_behind = app.shutdown();

    assert!(left_behind.is_clean(), "{left_behind:?}");
    // WebKit's helpers can recreate directories under the scratch path after the app exits, so
    // give a straggler time to show itself.
    std::thread::sleep(std::time::Duration::from_secs(3));
    assert!(
        !scratch.exists(),
        "the scratch directory came back: {scratch:?}"
    );
    let counter_after = std::fs::metadata(&real_counter)
        .and_then(|m| m.modified())
        .ok();
    assert_eq!(
        counter_before, counter_after,
        "the scenario touched the real renderer-fallback counter {real_counter:?}"
    );
}

/// Given a game and a call that mutates it
/// When snapshots are taken around the call
/// Then they differ; around a read-only call they do not.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn the_unchanged_helper_detects_change() {
    let app = App::launch(&AppConfig::default()).unwrap();
    mcp_e2e::career::begin(&app, mcp_e2e::career::seed(), "seasonStart", "ENG").unwrap();

    let before = Snapshot::take(&app).unwrap();
    app.client().info_game_summary().unwrap();
    let after_read = Snapshot::take(&app).unwrap();
    app.client().squad_set_formation("3-5-2").unwrap();
    let after_write = Snapshot::take(&app).unwrap();

    assert!(before.differences(&after_read).is_empty());
    assert!(!after_read.differences(&after_write).is_empty());
}

/// Given a fresh sandbox
/// When the client initializes, lists tools and pings
/// Then the session works and every tool carries an object schema; the catalog is recorded, and
/// nothing hard-codes how many tools there are.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn the_sandbox_catalog_is_usable_over_the_wire() {
    let app = App::launch(&AppConfig::default()).unwrap();

    let tools = app.client().list_tools().unwrap();
    app.client().ping().unwrap();

    assert!(!tools.is_empty());
    for tool in &tools {
        assert_eq!(tool["inputSchema"]["type"], json!("object"), "{tool}");
    }
    app.record_artifact(
        "catalog.json",
        &serde_json::to_string_pretty(&tools).unwrap(),
    );
}

/// Given the typed client's catalog
/// When it is compared with the running server's `tools/list`
/// Then a tool added, removed or given another parameter fails here by name.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn catalog_matches_the_live_server() {
    let app = App::launch(&AppConfig::default()).unwrap();
    let live = app.client().list_tools().unwrap();

    let mut problems = Vec::new();
    for tool in &live {
        let name = tool["name"].as_str().unwrap_or_default();
        let properties: Vec<(String, String)> = tool["inputSchema"]["properties"]
            .as_object()
            .map(|map| {
                let mut found: Vec<(String, String)> = map
                    .iter()
                    .map(|(name, schema)| {
                        (
                            name.clone(),
                            schema["type"].as_str().unwrap_or_default().to_string(),
                        )
                    })
                    .collect();
                found.sort();
                found
            })
            .unwrap_or_default();
        let mut required: Vec<String> = tool["inputSchema"]["required"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|i| i.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        required.sort();
        match CATALOG.iter().find(|(known, _, _)| *known == name) {
            None => problems.push(format!("{name}: no typed method")),
            Some((_, known_properties, known_required)) => {
                let known: Vec<(String, String)> = known_properties
                    .iter()
                    .map(|(n, t)| (n.to_string(), t.to_string()))
                    .collect();
                if properties != known || required != *known_required {
                    problems.push(format!("{name}: parameters or their types changed"));
                }
            }
        }
    }
    for (known, _, _) in CATALOG {
        if !live.iter().any(|tool| tool["name"] == *known) {
            problems.push(format!(
                "{known}: typed method for a tool the server no longer has"
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "regenerate or edit tools.rs:\n{}",
        problems.join("\n")
    );
}

/// Given a scenario that fails after it changed the game
/// When the fixture is dropped by the failure
/// Then the kept data directory holds the complete transcript, with the earlier calls, and the game as it stands.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn a_failed_scenario_keeps_its_transcript_and_state() {
    let app = App::launch(&AppConfig::default()).unwrap();
    mcp_e2e::career::begin(&app, mcp_e2e::career::seed(), "seasonStart", "ENG").unwrap();
    let formation_before = my_formation(&mcp_e2e::career::game_state(&app).unwrap())
        .expect("the manager's club has a formation before the change");
    assert_ne!(
        formation_before, "3-5-2",
        "the change below must be a change"
    );
    app.client().squad_set_formation("3-5-2").unwrap();
    let kept = app.data_dir().unwrap().to_path_buf();

    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _app = app;
        panic!("the scenario failed");
    }));

    assert!(failure.is_err());
    let transcript = std::fs::read_to_string(kept.join("transcript.json")).unwrap();
    assert!(
        transcript.contains("game_select_team"),
        "an early call is missing"
    );
    assert!(
        transcript.contains("squad_set_formation"),
        "the last call is missing"
    );
    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(kept.join("game-state.json")).unwrap())
            .unwrap();
    assert_eq!(
        my_formation(&state["game"]).expect("the captured state has the club's formation"),
        "3-5-2",
        "the captured state should hold the change made before the failure"
    );
    let _ = std::fs::remove_dir_all(kept);
}

/// The formation of the manager's own club in `game`.
fn my_formation(game: &serde_json::Value) -> Option<String> {
    let club = &game["manager"]["team_id"];
    game["teams"]
        .as_array()
        .and_then(|teams| teams.iter().find(|team| team["id"] == *club))
        .and_then(|team| team["formation"].as_str())
        .map(str::to_string)
}

/// Given a binary built from sources other than the ones under test
/// When the fixture launches it
/// Then it refuses, naming both revisions and the rebuild command.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn a_stale_binary_is_refused_with_the_rebuild_command() {
    let config = AppConfig {
        expected_revision: Some("0123456789ab".to_string()),
        ..AppConfig::default()
    };

    match App::launch(&config) {
        Err(mcp_e2e::LaunchError::StaleBinary {
            expected_revision, ..
        }) => {
            assert_eq!(expected_revision, "0123456789ab");
        }
        Err(other) => panic!("expected a stale-binary refusal, got {other}"),
        Ok(_) => panic!("launched a binary that was not built from the sources under test"),
    }
}
