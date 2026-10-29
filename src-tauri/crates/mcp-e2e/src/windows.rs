//! Which windows an app has mapped on the X display it was given.
//!
//! Read straight from the X server, so a test checks what is on the display rather than what the
//! app says about itself.

use std::time::Duration;

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
    let (connection, _) = connect_to_display()?;
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
    let (connection, screen_number) = connect_to_display()?;
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

/// Connections to try before giving up on the display.
const CONNECT_ATTEMPTS: u32 = 3;
const CONNECT_PAUSE: Duration = Duration::from_millis(300);

/// Connects to `$DISPLAY`, trying a few times.
///
/// The first connection of a run can land while `xvfb-run` has only just started the server, which
/// accepts the socket and then resets it (the nightly of 16 and 17 Oct: "Connection reset by peer"
/// on the first test to touch the display, every other launch a moment later fine). That is a
/// startup race, not a leaked connection (each is dropped after use) nor the client limit (two test
/// threads). A bounded retry, logged, rides it out; a display that stays down still fails.
fn connect_to_display() -> Result<(RustConnection, usize), String> {
    retrying(
        || RustConnection::connect(None),
        CONNECT_ATTEMPTS,
        CONNECT_PAUSE,
    )
    .map_err(|e| format!("could not reach the X display: {e}"))
}

/// Runs `attempt` up to `attempts` times, pausing between tries and logging each retry to stderr;
/// the last error is the one returned.
fn retrying<T, E: std::fmt::Display>(
    mut attempt: impl FnMut() -> Result<T, E>,
    attempts: u32,
    pause: Duration,
) -> Result<T, E> {
    let mut tried = 1;
    loop {
        match attempt() {
            Ok(value) => return Ok(value),
            Err(error) if tried >= attempts => return Err(error),
            Err(error) => {
                eprintln!(
                    "[mcp-e2e] X display connection failed ({error}); retry {tried}/{}",
                    attempts - 1
                );
                tried += 1;
                std::thread::sleep(pause);
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::retrying;
    use std::cell::Cell;
    use std::time::Duration;

    /// Given a connection that is reset twice and then works
    /// When it is retried (3 attempts)
    /// Then it succeeds on the third try.
    #[test]
    fn a_connection_reset_while_the_server_starts_is_retried() {
        let tries = Cell::new(0);

        let result = retrying(
            || {
                tries.set(tries.get() + 1);
                if tries.get() < 3 {
                    Err("Connection reset by peer")
                } else {
                    Ok("connected")
                }
            },
            3,
            Duration::ZERO,
        );

        assert_eq!(result, Ok("connected"));
        assert_eq!(tries.get(), 3);
    }

    /// Given a display that never comes up
    /// When it is retried (3 attempts)
    /// Then it gives up after exactly three tries with the last error, so a dead display is still
    ///      a failure and not an endless wait.
    #[test]
    fn a_display_that_stays_down_still_fails_after_the_bound() {
        let tries = Cell::new(0);

        let result: Result<(), String> = retrying(
            || {
                tries.set(tries.get() + 1);
                Err(format!("refused {}", tries.get()))
            },
            3,
            Duration::ZERO,
        );

        assert_eq!(result, Err("refused 3".to_string()));
        assert_eq!(tries.get(), 3);
    }

    /// Given a connection that works at once
    /// When it is attempted
    /// Then it is made once, with no pause.
    #[test]
    fn a_connection_that_works_is_made_once() {
        let tries = Cell::new(0);

        let result: Result<u8, String> = retrying(
            || {
                tries.set(tries.get() + 1);
                Ok(7)
            },
            3,
            Duration::from_secs(60),
        );

        assert_eq!(result, Ok(7));
        assert_eq!(tries.get(), 1);
    }
}
