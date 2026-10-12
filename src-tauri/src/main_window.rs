//! Whether the main window is shown.
//!
//! The window is declared in `tauri.conf.json` with `"visible": false`, so it exists but is never
//! mapped until [`show_unless_headless`] says so. That is the only place that decides: a headless
//! run (`--no-gui`) never shows it, and a normal launch shows it at the start of `setup`.

use tauri::{App, Manager};

/// A `--no-gui` run is headless; everything else shows the window.
fn should_show(headless: bool) -> bool {
    !headless
}

/// `--no-gui` only means something to the MCP server, so a build without it is never headless.
#[cfg(feature = "mcp")]
fn headless_requested() -> bool {
    matches!(
        crate::mcp_server::config::parse_mcp_config_from_args(),
        Ok(Some(config)) if config.no_gui
    )
}

#[cfg(not(feature = "mcp"))]
fn headless_requested() -> bool {
    false
}

pub fn show_unless_headless(app: &App) {
    let headless = headless_requested();
    if !should_show(headless) {
        log::info!("[window] --no-gui: the main window stays hidden");
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.show() {
            log::error!("[window] could not show the main window: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::should_show;

    /// Given a normal launch and a `--no-gui` launch
    /// When the window decision is made
    /// Then only the normal launch shows the window.
    #[test]
    fn only_a_headless_run_keeps_the_window_hidden() {
        assert!(should_show(false));
        assert!(!should_show(true));
    }
}
