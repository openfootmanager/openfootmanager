use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;

use crate::client::Client;

const READY_WITHIN: Duration = Duration::from_secs(60);
const CALL_TIMEOUT: Duration = Duration::from_secs(120);
const GRACEFUL_STOP_WITHIN: Duration = Duration::from_secs(10);
const LOG_LINES_IN_BUNDLE: usize = 50;
const LAST_EXCHANGES_IN_BUNDLE: usize = 10;
const SHORTENED_TO: usize = 400;
const STATE_CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Sandbox,
    Competition,
}

impl Mode {
    fn flag(self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::Competition => "competition",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub mode: Mode,
    /// Extra arguments, such as `--mcp-auto-start random,<team>` and `--mcp-seed 7`.
    pub extra_args: Vec<String>,
    /// The app binary, instead of `OFM_E2E_BIN` or the debug build next to this crate.
    pub binary: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mode: Mode::Sandbox,
            extra_args: Vec::new(),
            binary: None,
        }
    }
}

#[derive(Debug)]
pub enum LaunchError {
    /// `DISPLAY` points at a desktop session, where the app's window would open in front of you.
    HostDisplay,
    /// No built binary where one was expected. Build with `--features mcp`.
    BinaryMissing(PathBuf),
    /// Neither `DISPLAY` nor `WAYLAND_DISPLAY` is set, and the app cannot start without one.
    NoDisplay,
    /// The process started and never answered `initialize`. The bundle says what it printed.
    NotReady(String),
    Io(String),
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BinaryMissing(path) => write!(
                f,
                "no app binary at {}: build it with `cargo build --features mcp` or set OFM_E2E_BIN",
                path.display()
            ),
            Self::HostDisplay => write!(
                f,
                "DISPLAY is a desktop session (XWayland): the app would open a window in front of you. \
                 Run under `xvfb-run -a`"
            ),
            Self::NoDisplay => write!(
                f,
                "no DISPLAY: run under `xvfb-run -a`; the app is forced onto X11 and cannot start without one"
            ),
            Self::NotReady(bundle) => write!(f, "{bundle}"),
            Self::Io(detail) => write!(f, "{detail}"),
        }
    }
}

impl std::error::Error for LaunchError {}

/// One running app: its process, its private data directories and its MCP session.
pub struct App {
    child: Option<Child>,
    port: u16,
    binary: PathBuf,
    args: Vec<String>,
    dir: Option<TempDir>,
    log: PathBuf,
    client: Option<Client>,
}

impl App {
    /// Starts the app on a free port with private `XDG_DATA_HOME` and `XDG_CONFIG_HOME`, and
    /// returns once `initialize` succeeds. Fails loudly; there is no skip.
    pub fn launch(config: &AppConfig) -> Result<Self, LaunchError> {
        let binary = config.binary.clone().unwrap_or_else(app_binary);
        if !binary.is_file() {
            return Err(LaunchError::BinaryMissing(binary));
        }
        require_virtual_display()?;

        let dir = tempfile::tempdir().map_err(|e| LaunchError::Io(e.to_string()))?;
        for sub in ["data", "config", "cache", "state"] {
            fs::create_dir_all(dir.path().join(sub)).map_err(|e| LaunchError::Io(e.to_string()))?;
        }
        let port = free_port().map_err(|e| LaunchError::Io(e.to_string()))?;
        let log = dir.path().join("app.log");
        let log_file = fs::File::create(&log).map_err(|e| LaunchError::Io(e.to_string()))?;

        let mut args = vec![
            "--no-gui".to_string(),
            "--mcp-port".to_string(),
            port.to_string(),
            "--mcp-mode".to_string(),
            config.mode.flag().to_string(),
            "--auto-save-interval-days".to_string(),
            "0".to_string(),
        ];
        args.extend(config.extra_args.iter().cloned());

        let child = app_command(&binary, &args, dir.path())
            .stdin(Stdio::null())
            .stdout(Stdio::from(
                log_file
                    .try_clone()
                    .map_err(|e| LaunchError::Io(e.to_string()))?,
            ))
            .stderr(Stdio::from(log_file))
            .spawn()
            .map_err(|e| LaunchError::Io(format!("could not start {}: {e}", binary.display())))?;

        let mut app = Self {
            child: Some(child),
            port,
            binary,
            args,
            dir: Some(dir),
            log,
            client: None,
        };
        match app.wait_until_ready() {
            Ok(client) => {
                app.client = Some(client);
                Ok(app)
            }
            Err(detail) => {
                let bundle = app.failure_bundle(&detail);
                app.keep_directory();
                Err(LaunchError::NotReady(bundle))
            }
        }
    }

    pub fn client(&self) -> &Client {
        self.client
            .as_ref()
            .expect("a launched app always has a client")
    }

    /// The process id of the app, which `_NET_WM_PID` carries on its windows.
    pub fn pid(&self) -> Option<u32> {
        self.child.as_ref().map(Child::id)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// The directory the app writes saves under, for reading the file the game holds.
    pub fn saves_dir(&self) -> Option<PathBuf> {
        let data = self.dir.as_ref()?.path().join("data");
        find_saves_dir(&data)
    }

    /// The app's private data root: where it keeps its saves, and where a failure leaves evidence.
    pub fn data_dir(&self) -> Option<&Path> {
        self.dir.as_ref().map(TempDir::path)
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/mcp", self.port)
    }

    fn wait_until_ready(&mut self) -> Result<Client, String> {
        let started = Instant::now();
        loop {
            if let Some(child) = self.child.as_mut() {
                if let Ok(Some(status)) = child.try_wait() {
                    return Err(format!("the app exited before it was ready: {status}"));
                }
            }
            match Client::connect(&self.url(), Duration::from_secs(5)) {
                Ok(client) => return client.with_timeout(CALL_TIMEOUT).map_err(|e| e.to_string()),
                Err(error) if started.elapsed() > READY_WITHIN => {
                    return Err(format!("no `initialize` within {READY_WITHIN:?}: {error}"));
                }
                Err(_) => std::thread::sleep(Duration::from_millis(250)),
            }
        }
    }

    fn failure_bundle(&self, detail: &str) -> String {
        let log = fs::read_to_string(&self.log).unwrap_or_default();
        let lines: Vec<&str> = log.lines().collect();
        let tail = &lines[lines.len().saturating_sub(LOG_LINES_IN_BUNDLE)..];
        let kept = self
            .dir
            .as_ref()
            .map_or_else(|| "?".to_string(), |d| d.path().display().to_string());
        let evidence = self.write_evidence();
        format!(
            "{detail}\nrepro: {}\nrevision: {}\nbinary: {}\nport: {}\nargs: {}\ndata kept in: {kept}\n\
             complete transcript and game state: {evidence}\n\
             last exchanges:\n{}\nlast {} log lines:\n{}",
            repro_line(),
            revision(),
            self.binary.display(),
            self.port,
            self.args.join(" "),
            self.last_exchanges(),
            tail.len(),
            tail.join("\n")
        )
    }

    /// Writes the complete transcript and, if the app still answers, the game as it stands into
    /// the data directory, so a failure after a mutation keeps every earlier call and the state
    /// they left. Returns where, or why not.
    fn write_evidence(&self) -> String {
        let Some(dir) = self.dir.as_ref().map(|d| d.path().to_path_buf()) else {
            return "(the data directory is gone)".to_string();
        };
        let Some(client) = self.client.as_ref() else {
            return "(no session was open)".to_string();
        };
        let transcript: Vec<serde_json::Value> = client
            .transcript()
            .into_iter()
            .map(|exchange| {
                serde_json::json!({"request": exchange.request, "response": exchange.response})
            })
            .collect();
        let transcript_file = dir.join("transcript.json");
        let wrote_transcript = serde_json::to_string_pretty(&transcript)
            .ok()
            .is_some_and(|text| fs::write(&transcript_file, text).is_ok());

        // A fresh short-lived session: the scenario's own client may be the one that hung.
        let state_file = dir.join("game-state.json");
        let state = Client::connect(&self.url(), STATE_CAPTURE_TIMEOUT)
            .and_then(|probe| probe.call_value("info_game_state", serde_json::json!({})));
        let state_note = match state {
            Ok(state) => match fs::write(&state_file, state.to_string()) {
                Ok(()) => state_file.display().to_string(),
                Err(error) => format!("could not write the game state: {error}"),
            },
            Err(error) => format!("no game state: {error}"),
        };
        let transcript_note = if wrote_transcript {
            transcript_file.display().to_string()
        } else {
            "could not write the transcript".to_string()
        };
        format!("{transcript_note}, {state_note}")
    }

    /// The last requests sent and what came back, shortened; the last one with a reply is the last
    /// successful action.
    fn last_exchanges(&self) -> String {
        let Some(client) = self.client.as_ref() else {
            return "(no session was open)".to_string();
        };
        let transcript = client.transcript();
        let recent = &transcript[transcript.len().saturating_sub(LAST_EXCHANGES_IN_BUNDLE)..];
        recent
            .iter()
            .map(|exchange| {
                let reply = exchange
                    .response
                    .as_ref()
                    .map_or_else(|| "no reply".to_string(), |r| shortened(&r.to_string()));
                format!("> {}\n< {reply}", shortened(&exchange.request.to_string()))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn keep_directory(&mut self) {
        if let Some(dir) = self.dir.take() {
            let kept = dir.keep();
            eprintln!("kept {}", kept.display());
        }
    }

    /// Stops the app and reports whether it left anything behind.
    pub fn shutdown(mut self) -> Teardown {
        let reaped = self.stop();
        Teardown {
            process_reaped: reaped,
            port_free: self.port_is_free(),
        }
    }

    /// Writes `content` where a nightly run collects artifacts: `$OFM_E2E_ARTIFACTS`, or
    /// `target/e2e-artifacts`.
    pub fn record_artifact(&self, name: &str, content: &str) {
        let dir = std::env::var_os("OFM_E2E_ARTIFACTS")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/e2e-artifacts")
            });
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(dir.join(name), content);
    }

    /// Stops the app: an interrupt first, a kill after a bounded wait, then reaps the process.
    /// Returns whether the process was reaped.
    fn stop(&mut self) -> bool {
        let Some(mut child) = self.child.take() else {
            return true;
        };
        let _ = Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .stderr(Stdio::null())
            .status();
        let started = Instant::now();
        while started.elapsed() < GRACEFUL_STOP_WITHIN {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = child.kill();
        child.wait().is_ok()
    }

    /// True when nothing listens on the app's port any more.
    pub fn port_is_free(&self) -> bool {
        TcpListener::bind(("127.0.0.1", self.port)).is_ok()
    }
}

/// The renderer-fallback counter the app keeps under the cache directory. A scenario that touched
/// the real one could pin a developer's own app to CPU compositing.
pub fn real_startup_failures_file() -> PathBuf {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".cache")))
        .unwrap_or_default();
    cache.join("openfootmanager").join("startup-failures")
}

/// What a stopped app left behind.
#[derive(Debug)]
pub struct Teardown {
    pub process_reaped: bool,
    pub port_free: bool,
}

impl Teardown {
    pub fn is_clean(&self) -> bool {
        self.process_reaped && self.port_free
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let failed = std::thread::panicking();
        if failed {
            eprintln!("{}", self.failure_bundle("the scenario failed"));
        }
        self.stop();
        if failed {
            self.keep_directory();
        }
    }
}

/// Starts the app the way a player does, with no MCP arguments, and reports whether it maps a
/// window on the display within `timeout`. The positive control for the headless checks: it proves
/// the window detector can see this app's window at all.
pub fn gui_launch_maps_a_window(timeout: Duration) -> Result<bool, LaunchError> {
    let binary = app_binary();
    if !binary.is_file() {
        return Err(LaunchError::BinaryMissing(binary));
    }
    require_virtual_display()?;
    let dir = tempfile::tempdir().map_err(|e| LaunchError::Io(e.to_string()))?;
    let mut child = app_command(&binary, &[], dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| LaunchError::Io(format!("could not start {}: {e}", binary.display())))?;
    let started = Instant::now();
    let mut mapped = false;
    while started.elapsed() < timeout && !mapped {
        std::thread::sleep(Duration::from_millis(500));
        mapped = crate::windows::mapped_windows()
            .map_err(LaunchError::Io)?
            .iter()
            .any(|window| window.pid == Some(child.id()));
    }
    let _ = child.kill();
    let _ = child.wait();
    Ok(mapped)
}

/// The command that starts the app, with its environment pinned to the scratch directory and to X11.
///
/// GTK prefers Wayland when it can see a Wayland session, and `xvfb-run` only sets `DISPLAY`. An app
/// that inherits `WAYLAND_DISPLAY` opens its window on the developer's own desktop instead of the
/// virtual one, so the Wayland variables are removed and the backend is forced to X11.
fn app_command(binary: &Path, args: &[String], dir: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .args(args)
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env("XDG_SESSION_TYPE", "x11")
        .env("GDK_BACKEND", "x11");
    command
}

fn app_binary() -> PathBuf {
    if let Some(path) = std::env::var_os("OFM_E2E_BIN") {
        return PathBuf::from(path);
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/openfootmanager")
}

fn free_port() -> std::io::Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

/// The `saves` directory under the app's data directory, wherever Tauri puts its identifier folder.
fn find_saves_dir(data: &Path) -> Option<PathBuf> {
    fs::read_dir(data)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("saves"))
        .find(|path| path.is_dir())
}

/// Refuses to start the app on a display that could be a desktop session.
///
/// The app is forced onto X11, so it needs `DISPLAY`; `WAYLAND_DISPLAY` is of no use to it. Beyond
/// that, a display counts as virtual only on positive evidence: `xvfb-run` points `XAUTHORITY` into
/// its own temp directory, and `OFM_E2E_ALLOW_DISPLAY=1` vouches for your own Xvfb or Xephyr. An
/// XWayland server (a Wayland desktop's X display) is refused outright.
fn require_virtual_display() -> Result<(), LaunchError> {
    let display = std::env::var_os("DISPLAY");
    if display.as_deref().is_none_or(|display| display.is_empty()) {
        return Err(LaunchError::NoDisplay);
    }
    let is_xwayland = crate::windows::display_is_a_host_session().map_err(LaunchError::Io)?;
    check_virtual_display(
        std::env::var_os("XAUTHORITY").as_deref(),
        std::env::var_os("OFM_E2E_ALLOW_DISPLAY").as_deref() == Some(std::ffi::OsStr::new("1")),
        is_xwayland,
    )
}

fn check_virtual_display(
    xauthority: Option<&std::ffi::OsStr>,
    vouched_for: bool,
    is_xwayland: bool,
) -> Result<(), LaunchError> {
    if is_xwayland {
        return Err(LaunchError::HostDisplay);
    }
    let made_by_xvfb_run =
        xauthority.is_some_and(|path| path.to_string_lossy().contains("xvfb-run"));
    if vouched_for || made_by_xvfb_run {
        Ok(())
    } else {
        Err(LaunchError::HostDisplay)
    }
}

fn shortened(text: &str) -> String {
    match text.char_indices().nth(SHORTENED_TO) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_string(),
    }
}

/// The command that reruns the failing scenario: its test name is the thread's, its story file is
/// the test binary's.
fn repro_line() -> String {
    let scenario = std::thread::current()
        .name()
        .unwrap_or("<scenario>")
        .to_string();
    let story = std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .map(|stem| {
            stem.rsplit_once('-')
                .map_or(stem.clone(), |(name, _)| name.to_string())
        })
        .unwrap_or_else(|| "<story>".to_string());
    format!(
        "OFM_E2E_SEED={} xvfb-run cargo test -p mcp-e2e --test {story} {scenario} -- --ignored --nocapture",
        crate::career::seed()
    )
}

/// The revision under test: `$OFM_E2E_REVISION`, then `$GITHUB_SHA`, then `git rev-parse`.
fn revision() -> String {
    ["OFM_E2E_REVISION", "GITHUB_SHA"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok())
        .or_else(|| {
            let output = Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()?;
            output
                .status
                .success()
                .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::{app_command, display_available};

    /// Given neither DISPLAY nor WAYLAND_DISPLAY (or only empty ones)
    /// When the fixture checks for a display
    /// Then there is none, and launch fails with that named instead of timing out.
    #[test]
    fn a_missing_display_is_detected_before_launch() {
        assert!(!display_available(None));
        assert!(!display_available(Some(OsStr::new(""))));
        assert!(display_available(Some(OsStr::new(":0"))));
    }

    /// Given a developer session that exports a Wayland display
    /// When the fixture builds the command that starts the app
    /// Then the app cannot reach that session: no Wayland variables, and GTK is forced onto X11.
    #[test]
    fn the_app_cannot_reach_the_developers_wayland_session() {
        let command = app_command(
            std::path::Path::new("/bin/true"),
            &[],
            std::path::Path::new("/tmp/scratch"),
        );

        let environment: std::collections::HashMap<_, _> = command
            .get_envs()
            .map(|(name, value)| (name.to_owned(), value.map(|v| v.to_owned())))
            .collect();

        // `Some(None)` is a variable the command removes from the inherited environment.
        assert_eq!(environment.get(OsStr::new("WAYLAND_DISPLAY")), Some(&None));
        assert_eq!(environment.get(OsStr::new("WAYLAND_SOCKET")), Some(&None));
        assert_eq!(
            environment.get(OsStr::new("GDK_BACKEND")),
            Some(&Some(OsStr::new("x11").to_owned()))
        );
    }
}
