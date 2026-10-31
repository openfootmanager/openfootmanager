use std::fs;
use std::net::TcpListener;
use std::os::unix::process::CommandExt;
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
/// How long `--build-revision` gets to answer. It needs no display and takes milliseconds; a
/// binary that is still silent after this is too old to know the flag.
const PROBE_WITHIN: Duration = Duration::from_secs(15);

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
    /// The source revision the binary must report, instead of the working tree's own.
    pub expected_revision: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mode: Mode::Sandbox,
            extra_args: Vec::new(),
            binary: None,
            expected_revision: None,
        }
    }
}

pub enum LaunchError {
    /// `DISPLAY` points at a desktop session, where the app's window would open in front of you.
    HostDisplay,
    /// No built binary where one was expected. Build with `--features mcp`.
    BinaryMissing(PathBuf),
    /// Neither `DISPLAY` nor `WAYLAND_DISPLAY` is set, and the app cannot start without one.
    NoDisplay,
    /// The binary was built from other sources than the ones under test.
    StaleBinary {
        binary_revision: String,
        expected_revision: String,
    },
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
                "DISPLAY is not evidently a virtual display (it may be a desktop session, an XWayland \
                 server, a native X server, or an Xvfb that `xvfb-run` did not start), so the app's \
                 window could open in front of you. Run under `xvfb-run -a`, or set \
                 OFM_E2E_ALLOW_DISPLAY=1 to vouch for your own Xvfb or Xephyr"
            ),
            Self::NoDisplay => write!(
                f,
                "no DISPLAY: run under `xvfb-run -a`; the app is forced onto X11 and cannot start without one"
            ),
            Self::StaleBinary {
                binary_revision,
                expected_revision,
            } => write!(
                f,
                "the app binary was built from sources `{binary_revision}`, but the tree under test is \
                 `{expected_revision}`: rebuild it with `cargo build --features mcp --bin openfootmanager` \
                 from src-tauri, or set OFM_E2E_ALLOW_STALE=1 to test a binary you know is different"
            ),
            Self::NotReady(bundle) => write!(f, "{bundle}"),
            Self::Io(detail) => write!(f, "{detail}"),
        }
    }
}

/// `unwrap()` on a failed launch prints this, so it shows the message and not the variant.
impl std::fmt::Debug for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
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
    mode: Mode,
}

impl App {
    /// Starts the app on a free port with private `XDG_DATA_HOME` and `XDG_CONFIG_HOME`, and
    /// returns once `initialize` succeeds. Fails loudly; there is no skip.
    pub fn launch(config: &AppConfig) -> Result<Self, LaunchError> {
        Self::launch_after(config, require_virtual_display)
    }

    /// [`App::launch`] with the display check passed in, so a test can see that the stale-binary
    /// refusal comes before it.
    fn launch_after(
        config: &AppConfig,
        display_check: impl FnOnce() -> Result<(), LaunchError>,
    ) -> Result<Self, LaunchError> {
        let binary = config.binary.clone().unwrap_or_else(app_binary);
        if !binary.is_file() {
            return Err(LaunchError::BinaryMissing(binary));
        }
        // Before the display and before the app boots: the binary names its own revision, so a
        // stale build is refused the same way whatever the display is doing.
        refuse_stale_binary(&binary, expected_revision(config).as_deref(), PROBE_WITHIN)?;
        display_check()?;

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
        // The harness reads the whole game through `dev_full_game_state`, which only a sandbox
        // server may offer: the app refuses the flag in competition mode.
        if config.mode == Mode::Sandbox {
            args.push("--dev-full-state".to_string());
        }
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
            mode: config.mode,
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

    /// The mode the app was started in. A competition server has no state oracle by design.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The binary this app was started from.
    pub fn binary(&self) -> &Path {
        &self.binary
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
            .and_then(|probe| probe.call_value("dev_full_game_state", serde_json::json!({})));
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
                // The WebKit helpers belong to the app's process group and can outlive it long
                // enough to recreate directories under the scratch path.
                signal_group(child.id(), "-KILL");
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = child.kill();
        let reaped = child.wait().is_ok();
        signal_group(child.id(), "-KILL");
        reaped
    }

    /// True when nothing listens on the app's port any more.
    pub fn port_is_free(&self) -> bool {
        TcpListener::bind(("127.0.0.1", self.port)).is_ok()
    }
}

/// Sends `signal` to the whole process group the app started, which has the app's pid as its id.
fn signal_group(pid: u32, signal: &str) {
    let _ = Command::new("kill")
        .args([signal, "--", &format!("-{pid}")])
        .stderr(Stdio::null())
        .status();
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

/// What a normal (GUI) launch did within the time it was given.
pub struct GuiLaunch {
    pub mapped_a_window: bool,
    /// The last lines the app printed, for the failure message.
    pub log_tail: String,
}

/// Starts the app the way a player does, with no MCP arguments, and reports whether it maps a
/// window on the display within `timeout`. The positive control for the headless checks: it proves
/// the window detector can see this app's window at all. The whole process group is stopped, so
/// no WebKit helper outlives the scratch directory.
pub fn gui_launch_maps_a_window(timeout: Duration) -> Result<GuiLaunch, LaunchError> {
    let binary = app_binary();
    if !binary.is_file() {
        return Err(LaunchError::BinaryMissing(binary));
    }
    require_virtual_display()?;
    let dir = tempfile::tempdir().map_err(|e| LaunchError::Io(e.to_string()))?;
    let log_path = dir.path().join("app.log");
    let log = fs::File::create(&log_path).map_err(|e| LaunchError::Io(e.to_string()))?;
    let mut child = app_command(&binary, &[], dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::from(
            log.try_clone()
                .map_err(|e| LaunchError::Io(e.to_string()))?,
        ))
        .stderr(Stdio::from(log))
        .spawn()
        .map_err(|e| LaunchError::Io(format!("could not start {}: {e}", binary.display())))?;
    let outcome = wait_for_window_of(child.id(), timeout);
    let _ = child.kill();
    let _ = child.wait();
    signal_group(child.id(), "-KILL");
    let log = fs::read_to_string(&log_path).unwrap_or_default();
    let lines: Vec<&str> = log.lines().collect();
    Ok(GuiLaunch {
        mapped_a_window: outcome?,
        log_tail: lines[lines.len().saturating_sub(LOG_LINES_IN_BUNDLE)..].join("\n"),
    })
}

fn wait_for_window_of(pid: u32, timeout: Duration) -> Result<bool, LaunchError> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(500));
        let windows = crate::windows::mapped_windows().map_err(LaunchError::Io)?;
        if windows.iter().any(|window| window.pid == Some(pid)) {
            return Ok(true);
        }
    }
    Ok(false)
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
        .process_group(0)
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env("XDG_SESSION_TYPE", "x11")
        .env("GDK_BACKEND", "x11");
    command
}

/// The revision the binary must report: the config's, else the working tree's. `None` when git
/// cannot say, or the run vouches for a different binary with `OFM_E2E_ALLOW_STALE=1`.
fn expected_revision(config: &AppConfig) -> Option<String> {
    if std::env::var_os("OFM_E2E_ALLOW_STALE").as_deref() == Some(std::ffi::OsStr::new("1")) {
        return None;
    }
    config.expected_revision.clone().or_else(|| {
        crate::revision::source_revision(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
    })
}

/// The command that asks a binary for its version line: the app's own environment (`app_command`,
/// so the two cannot drift) with no display at all.
///
/// A binary built before the flag existed ignores it and starts up. In the app's environment it
/// has no display to open a window on (`GDK_BACKEND=x11`, the Wayland variables removed, no
/// `DISPLAY`), so it fails within a second; it also writes only under `dir`, never into the
/// developer's own data, settings or cache, where it would otherwise leave a startup-failure mark.
fn probe_command(binary: &Path, dir: &Path) -> Command {
    let mut command = app_command(binary, &["--build-revision".to_string()], dir);
    command.env_remove("DISPLAY");
    command
}

/// Asks the binary for its version line (`0.3.0+<revision>`) with `--build-revision`, which the app
/// answers before it touches a display or any toolkit. `None` when it does not answer in time or
/// at all, as a binary too old to know the flag does not.
pub fn probe_build_version(binary: &Path, within: Duration) -> Option<String> {
    let dir = tempfile::tempdir().ok()?;
    for sub in ["data", "config", "cache", "state"] {
        fs::create_dir_all(dir.path().join(sub)).ok()?;
    }
    let mut child = probe_command(binary, dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    let finished = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() < within => std::thread::sleep(Duration::from_millis(25)),
            _ => break None,
        }
    };
    let mut output = String::new();
    let read = match (finished, child.stdout.take()) {
        (Some(status), Some(mut stdout)) if status.success() => {
            std::io::Read::read_to_string(&mut stdout, &mut output).is_ok()
        }
        _ => false,
    };
    // A binary that ignored the flag may have started its helpers in the process group.
    let _ = child.kill();
    let _ = child.wait();
    signal_group(child.id(), "-KILL");
    if !read {
        return None;
    }
    output
        .lines()
        .map(str::trim)
        .rev()
        .find(|line| line.starts_with(|c: char| c.is_ascii_digit()))
        .map(str::to_string)
}

/// Refuses a binary built from other sources than `expected`, learning its revision from the binary
/// itself. No expectation, no probe.
fn refuse_stale_binary(
    binary: &Path,
    expected: Option<&str>,
    within: Duration,
) -> Result<(), LaunchError> {
    if expected.is_none() {
        return Ok(());
    }
    let reported = probe_build_version(binary, within);
    check_revision(reported.as_deref().unwrap_or(""), expected)
}

/// Compares what the binary reports (`0.3.0+<revision>`) with the revision expected.
fn check_revision(server_version: &str, expected: Option<&str>) -> Result<(), LaunchError> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let reported = server_version.split_once('+').map(|(_, revision)| revision);
    if reported == Some(expected) {
        return Ok(());
    }
    Err(LaunchError::StaleBinary {
        binary_revision: reported.unwrap_or("unknown").to_string(),
        expected_revision: expected.to_string(),
    })
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
    let made_by_xvfb_run = std::env::var_os("XAUTHORITY")
        .map(PathBuf::from)
        .filter(|path| path.to_string_lossy().contains("xvfb-run"))
        .and_then(|path| fs::read(path).ok())
        .zip(
            display
                .as_deref()
                .and_then(|d| d.to_str())
                .and_then(display_number),
        )
        .is_some_and(|(authority, number)| authority_is_only_for(&authority, number));
    check_virtual_display(
        made_by_xvfb_run,
        std::env::var_os("OFM_E2E_ALLOW_DISPLAY").as_deref() == Some(std::ffi::OsStr::new("1")),
        is_xwayland,
    )
}

/// The display number in `DISPLAY` (`:99`, `:99.0`, `host:99.0`).
fn display_number(display: &str) -> Option<&str> {
    let after_host = display.rsplit_once(':')?.1;
    let number = after_host.split('.').next()?;
    (!number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())).then_some(number)
}

/// Whether an Xauthority file holds entries for display `number` and for no other display, so the
/// file `xvfb-run` made cannot also grant access to a desktop session.
fn authority_is_only_for(authority: &[u8], number: &str) -> bool {
    let mut rest = authority;
    let mut entries = 0;
    let take = |len: usize, rest: &mut &[u8]| -> Option<Vec<u8>> {
        let (head, tail) = rest.split_at_checked(len)?;
        *rest = tail;
        Some(head.to_vec())
    };
    while !rest.is_empty() {
        let mut entry_number = None;
        // family, then four length-prefixed fields: address, display number, name, data.
        if take(2, &mut rest).is_none() {
            return false;
        }
        for field in 0..4 {
            let Some(len) =
                take(2, &mut rest).map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])))
            else {
                return false;
            };
            let Some(bytes) = take(len, &mut rest) else {
                return false;
            };
            if field == 1 {
                entry_number = Some(bytes);
            }
        }
        if entry_number.as_deref() != Some(number.as_bytes()) {
            return false;
        }
        entries += 1;
    }
    entries > 0
}

fn check_virtual_display(
    made_by_xvfb_run: bool,
    vouched_for: bool,
    is_xwayland: bool,
) -> Result<(), LaunchError> {
    if is_xwayland {
        return Err(LaunchError::HostDisplay);
    }
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

    use super::{
        app_command, authority_is_only_for, check_revision, check_virtual_display, display_number,
        probe_build_version, probe_command, refuse_stale_binary, App, AppConfig, LaunchError,
    };
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    /// A stand-in for the app binary: a script that runs `body` when started.
    fn fake_binary(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("openfootmanager");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn config_for(binary: PathBuf, expected: &str) -> AppConfig {
        AppConfig {
            binary: Some(binary),
            expected_revision: Some(expected.to_string()),
            ..AppConfig::default()
        }
    }

    /// Given a binary built from other sources, and a display check that must not run
    /// When the app is launched expecting a revision
    /// Then the stale refusal comes back without the display being touched: it needs neither a
    ///      display nor a booted app, so a display that resets the connection cannot hide it.
    #[test]
    fn a_stale_binary_is_refused_before_the_display_is_touched() {
        let dir = tempfile::tempdir().unwrap();
        let binary = fake_binary(dir.path(), "echo 0.3.0+oldoldoldold");

        let result = App::launch_after(&config_for(binary, "newnewnewnew"), || {
            panic!("the display was touched before the stale check")
        });

        match result {
            Err(LaunchError::StaleBinary {
                binary_revision,
                expected_revision,
            }) => {
                assert_eq!(binary_revision, "oldoldoldold");
                assert_eq!(expected_revision, "newnewnewnew");
            }
            other => panic!("expected the stale refusal, got {:?}", other.err()),
        }
    }

    /// Given a binary from the right sources
    /// When the app is launched
    /// Then the display is checked next, so the stale check did not skip it.
    #[test]
    fn a_current_binary_goes_on_to_the_display_check() {
        let dir = tempfile::tempdir().unwrap();
        let binary = fake_binary(dir.path(), "echo 0.3.0+samesamesame");

        let result = App::launch_after(&config_for(binary, "samesamesame"), || {
            Err(LaunchError::NoDisplay)
        });

        assert!(matches!(result, Err(LaunchError::NoDisplay)));
    }

    /// Given a binary too old to know `--build-revision` (it prints nothing and exits), and one
    ///       that never answers
    /// When the stale check runs
    /// Then both are stale, reported as unknown: a build that cannot say what it is was not built
    ///      from the sources under test.
    #[test]
    fn a_binary_that_cannot_name_its_revision_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        let silent = fake_binary(dir.path(), "exit 1");
        let hanging_dir = tempfile::tempdir().unwrap();
        let hanging = fake_binary(hanging_dir.path(), "sleep 30");

        for binary in [silent, hanging] {
            let started = std::time::Instant::now();
            let refusal = refuse_stale_binary(&binary, Some("abc123"), Duration::from_millis(400))
                .unwrap_err();
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "the probe must give up at its bound, not wait for the binary"
            );
            match refusal {
                LaunchError::StaleBinary {
                    binary_revision, ..
                } => assert_eq!(binary_revision, "unknown"),
                other => panic!("expected a stale refusal, got {other}"),
            }
        }
    }

    /// Given a probe of a binary
    /// When its command is built
    /// Then it runs in the app's own environment: X11 forced, the Wayland variables removed, no
    ///      DISPLAY, and private XDG directories that are not the developer's.
    #[test]
    fn the_probe_runs_in_the_apps_environment_with_private_directories() {
        let dir = tempfile::tempdir().unwrap();

        let command = probe_command(Path::new("/bin/true"), dir.path());
        let environment: std::collections::HashMap<_, _> = command.get_envs().collect();

        assert_eq!(
            environment.get(OsStr::new("GDK_BACKEND")),
            Some(&Some(OsStr::new("x11")))
        );
        for removed in ["DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"] {
            assert_eq!(
                environment.get(OsStr::new(removed)),
                Some(&None),
                "{removed}"
            );
        }
        for (variable, sub) in [
            ("XDG_DATA_HOME", "data"),
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_CACHE_HOME", "cache"),
            ("XDG_STATE_HOME", "state"),
        ] {
            assert_eq!(
                environment.get(OsStr::new(variable)),
                Some(&Some(dir.path().join(sub).as_os_str())),
                "{variable}"
            );
        }
        assert_eq!(command.get_args().collect::<Vec<_>>(), ["--build-revision"]);
    }

    /// Given a stand-in binary that records the environment it was started in
    /// When it is probed
    /// Then it saw the app's environment and a private cache, not the developer's own, and the
    ///      scratch directory is gone afterwards.
    #[test]
    fn a_probed_binary_sees_the_apps_environment_and_leaves_nothing_behind() {
        let dir = tempfile::tempdir().unwrap();
        let seen = dir.path().join("seen.env");
        let binary = fake_binary(
            dir.path(),
            &format!("env > {}; echo 0.3.0+abc123abc123", seen.display()),
        );

        let version = probe_build_version(&binary, Duration::from_secs(5));

        assert_eq!(version.as_deref(), Some("0.3.0+abc123abc123"));
        let environment = std::fs::read_to_string(&seen).unwrap();
        assert!(
            environment.lines().any(|l| l == "GDK_BACKEND=x11"),
            "{environment}"
        );
        assert!(
            !environment.lines().any(|l| l.starts_with("DISPLAY=")),
            "{environment}"
        );
        let cache = environment
            .lines()
            .find_map(|l| l.strip_prefix("XDG_CACHE_HOME="))
            .expect("a private cache directory");
        assert!(
            Path::new(cache).starts_with(std::env::temp_dir()),
            "{cache} is not a private scratch directory"
        );
        assert!(
            !Path::new(cache).exists(),
            "the probe's scratch directory was left behind"
        );
    }

    /// Whether a process is still running (a zombie waiting to be reaped is not).
    fn process_is_running(pid: u32) -> bool {
        std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|stat| {
                stat.rsplit_once(')')
                    .map(|(_, rest)| rest.trim_start().to_string())
            })
            .is_some_and(|rest| !rest.starts_with('Z'))
    }

    /// Given a binary that starts a helper process of its own before it prints its version, as the
    ///       app's WebKit helpers do
    /// When it is probed
    /// Then the helper is gone afterwards: the probe kills the whole process group, not only the
    ///      binary, so nothing it started outlives the probe.
    #[test]
    fn a_probe_leaves_no_helper_process_behind() {
        let dir = tempfile::tempdir().unwrap();
        let pid_file = dir.path().join("helper.pid");
        let binary = fake_binary(
            dir.path(),
            &format!(
                "sleep 30 >/dev/null 2>&1 &\necho $! > {}\necho 0.3.0+abc123abc123",
                pid_file.display()
            ),
        );

        let version = probe_build_version(&binary, Duration::from_secs(5));

        assert_eq!(version.as_deref(), Some("0.3.0+abc123abc123"));
        let helper: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while process_is_running(helper) && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            !process_is_running(helper),
            "the helper process {helper} outlived the probe"
        );
    }

    /// Given a binary that prints its version line
    /// When it is probed
    /// Then the line is read back, whatever else is on the output before it; no expectation
    ///      skips the probe altogether.
    #[test]
    fn the_probe_reads_the_version_line_and_is_skipped_without_an_expectation() {
        let dir = tempfile::tempdir().unwrap();
        let binary = fake_binary(dir.path(), "echo warming up; echo 0.3.0+abc123abc123");

        assert_eq!(
            probe_build_version(&binary, Duration::from_secs(5)).as_deref(),
            Some("0.3.0+abc123abc123")
        );
        assert!(refuse_stale_binary(&binary, None, Duration::from_secs(5)).is_ok());
    }

    /// Given XWayland, an unidentified display, an xvfb-run display, and one vouched for
    /// When the fixture decides whether the display is virtual
    /// Then only the last two are accepted, and XWayland is refused even when vouched for.
    #[test]
    fn only_a_display_with_positive_evidence_is_virtual() {
        assert!(check_virtual_display(true, false, false).is_ok());
        assert!(check_virtual_display(false, false, false).is_err());
        assert!(check_virtual_display(false, true, false).is_ok());
        assert!(check_virtual_display(true, true, true).is_err());
    }

    fn entry(number: &str) -> Vec<u8> {
        let mut bytes = vec![0x01, 0x00]; // family
        for field in [
            &b"host"[..],
            number.as_bytes(),
            b"MIT-MAGIC-COOKIE-1",
            &[7_u8; 16],
        ] {
            bytes.extend((field.len() as u16).to_be_bytes());
            bytes.extend(field);
        }
        bytes
    }

    /// Given an Xauthority file for display 99 only, one that also has a desktop's display 0, and a torn one
    /// When the fixture asks whether it belongs to DISPLAY=:99 alone
    /// Then only the first does, so a file that could also open the desktop is not evidence.
    #[test]
    fn an_authority_must_be_for_this_display_alone() {
        let only = entry("99");
        let both = [entry("99"), entry("0")].concat();

        assert!(authority_is_only_for(&only, "99"));
        assert!(!authority_is_only_for(&both, "99"));
        assert!(!authority_is_only_for(&only, "100"));
        assert!(!authority_is_only_for(&only[..only.len() - 3], "99"));
        assert!(!authority_is_only_for(&[], "99"));
    }

    #[test]
    fn the_display_number_is_read_from_display() {
        assert_eq!(display_number(":99"), Some("99"));
        assert_eq!(display_number(":99.0"), Some("99"));
        assert_eq!(display_number("host:7.1"), Some("7"));
        assert_eq!(display_number("garbage"), None);
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

    /// Given a binary that reports the sources it was built from
    /// When the fixture compares with the sources under test
    /// Then the same revision passes, a different or missing one is stale, and no expectation passes.
    #[test]
    fn a_binary_from_other_sources_is_stale() {
        assert!(check_revision("0.3.0+abc123", Some("abc123")).is_ok());
        assert!(check_revision("0.3.0+abc123.def456", Some("abc123")).is_err());
        assert!(check_revision("0.3.0", Some("abc123")).is_err());
        assert!(check_revision("0.3.0+abc123", None).is_ok());
        let message = check_revision("0.3.0+old", Some("new"))
            .unwrap_err()
            .to_string();
        assert!(
            message.contains("cargo build --features mcp --bin openfootmanager"),
            "{message}"
        );
    }
}
