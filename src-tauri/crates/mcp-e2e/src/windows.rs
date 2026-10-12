//! Which windows an app has mapped on the X display it was given.
//!
//! Read straight from the X server, so a test checks what is on the display rather than what the
//! app says about itself.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, MapState};
use x11rb::rust_connection::RustConnection;

/// A top-level window that is mapped (viewable) on the display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedWindow {
    pub id: u32,
    /// The process that owns it, from `_NET_WM_PID`, when the toolkit sets it.
    pub pid: Option<u32>,
}

/// Every mapped top-level window on the display named by `$DISPLAY`.
pub fn mapped_windows() -> Result<Vec<MappedWindow>, String> {
    let (connection, screen_number) =
        RustConnection::connect(None).map_err(|e| format!("could not reach the X display: {e}"))?;
    let root = connection.setup().roots[screen_number].root;
    let pid_atom = connection
        .intern_atom(false, b"_NET_WM_PID")
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?
        .atom;

    let tree = connection
        .query_tree(root)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?;
    let mut mapped = Vec::new();
    for window in tree.children {
        let Ok(attributes) = connection
            .get_window_attributes(window)
            .map_err(|e| e.to_string())?
            .reply()
        else {
            continue;
        };
        if attributes.map_state != MapState::VIEWABLE {
            continue;
        }
        let pid = connection
            .get_property(false, window, pid_atom, AtomEnum::CARDINAL, 0, 1)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .and_then(|reply| reply.value32().and_then(|mut values| values.next()));
        mapped.push(MappedWindow { id: window, pid });
    }
    Ok(mapped)
}
