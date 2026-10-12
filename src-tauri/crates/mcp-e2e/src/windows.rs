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

/// Whether the display belongs to a host desktop session (an XWayland server) rather than a
/// virtual one. `xvfb` never offers the `XWAYLAND` extension.
pub fn display_is_a_host_session() -> Result<bool, String> {
    let (connection, _) =
        RustConnection::connect(None).map_err(|e| format!("could not reach the X display: {e}"))?;
    let extensions = connection
        .list_extensions()
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?;
    Ok(extensions.names.iter().any(|name| name.name == b"XWAYLAND"))
}

/// Every mapped window on the display named by `$DISPLAY` that says which process owns it, down
/// to `MAX_DEPTH` levels below the root.
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

    let mut mapped = Vec::new();
    collect_mapped(&connection, root, pid_atom, MAX_DEPTH, &mut mapped)?;
    Ok(mapped)
}

/// How far below the root to look: a reparenting window manager puts a frame (which carries no
/// `_NET_WM_PID`) between the root and the client window that does.
const MAX_DEPTH: u8 = 4;

fn collect_mapped(
    connection: &RustConnection,
    parent: u32,
    pid_atom: u32,
    depth: u8,
    found: &mut Vec<MappedWindow>,
) -> Result<(), String> {
    if depth == 0 {
        return Ok(());
    }
    let tree = connection
        .query_tree(parent)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?;
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
        if pid.is_some() {
            found.push(MappedWindow { id: window, pid });
        }
        collect_mapped(connection, window, pid_atom, depth - 1, found)?;
    }
    Ok(())
}
