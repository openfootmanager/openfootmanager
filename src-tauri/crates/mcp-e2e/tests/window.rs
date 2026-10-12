//! A headless run must never put a window on the display.
//!
//! Needs a built `--features mcp` binary and an X display (`xvfb-run -a`). One test only, so no
//! other scenario in this binary maps a window while it looks.

use std::time::Duration;

use mcp_e2e::windows::mapped_windows;
use mcp_e2e::{gui_launch_maps_a_window, App, AppConfig, Mode};

fn launch(config: AppConfig, what: &str) -> App {
    App::launch(&config).unwrap_or_else(|error| panic!("could not start {what}: {error}"))
}

fn assert_no_window_of(app: &App, mode: &str) {
    // The window is decided at the start of `setup`, before the MCP server answers, so by now it
    // is either mapped or it never will be; the pause covers a late map by the toolkit.
    std::thread::sleep(Duration::from_secs(3));
    let pid = app.pid().expect("the app is running");
    let windows: Vec<_> = mapped_windows()
        .unwrap()
        .into_iter()
        .filter(|window| window.pid == Some(pid))
        .collect();
    assert!(windows.is_empty(), "{mode} --no-gui mapped {windows:?}");
}

/// Given a normal launch, then `--no-gui` in sandbox mode, then `--no-gui` in competition mode
/// When each is given a few seconds on a virtual display
/// Then the normal launch maps a window (so the detector can see it) and the headless ones never do.
#[test]
#[ignore = "needs a built MCP binary and a display; run via --ignored by the nightly"]
fn a_headless_run_never_maps_a_window() {
    let gui = gui_launch_maps_a_window(Duration::from_secs(60)).unwrap();
    assert!(
        gui.mapped_a_window,
        "a normal launch should map a window, or this test cannot see one. The app printed:\n{}",
        gui.log_tail
    );

    let sandbox = launch(AppConfig::default(), "the sandbox app");
    assert_no_window_of(&sandbox, "sandbox");
    drop(sandbox);

    let club_id = {
        let probe = launch(AppConfig::default(), "the probe app");
        let game = mcp_e2e::career::create_world(&probe, mcp_e2e::career::seed(), "seasonStart")
            .unwrap_or_else(|error| panic!("could not create the probe world: {error}"));
        mcp_e2e::career::top_club_of(&game, "ENG").unwrap()
    };
    let competition = launch(
        AppConfig {
            mode: Mode::Competition,
            extra_args: vec![
                "--mcp-auto-start".to_string(),
                format!("random,{club_id}"),
                "--mcp-seed".to_string(),
                mcp_e2e::career::seed().to_string(),
            ],
            ..AppConfig::default()
        },
        "the competition app",
    );
    assert_no_window_of(&competition, "competition");
}
