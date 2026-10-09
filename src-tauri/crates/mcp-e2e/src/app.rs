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
            Self::NoDisplay => write!(
                f,
                "no display: set DISPLAY or run under `xvfb-run`; the app cannot start without one"
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
        if !display_available(
            std::env::var_os("DISPLAY").as_deref(),
            std::env::var_os("WAYLAND_DISPLAY").as_deref(),
        ) {
            return Err(LaunchError::NoDisplay);
        }

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

        let child = Command::new(&binary)
            .args(&args)
            .env("XDG_DATA_HOME", dir.path().join("data"))
            .env("XDG_CONFIG_HOME", dir.path().join("config"))
            .env("XDG_CACHE_HOME", dir.path().join("cache"))
            .env("XDG_STATE_HOME", dir.path().join("state"))
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

    pub fn port(&self) -> u16 {
        self.port
    }

    /// The directory the app writes saves under, for reading the file the game holds.
    pub fn saves_dir(&self) -> Option<PathBuf> {
        let data = self.dir.as_ref()?.path().join("data");
        find_saves_dir(&data)
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
                Ok(client) => return Ok(client.with_timeout(CALL_TIMEOUT)),
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
        format!(
            "{detail}\nbinary: {}\nport: {}\nargs: {}\ndata kept in: {}\nlast {} log lines:\n{}",
            self.binary.display(),
            self.port,
            self.args.join(" "),
            self.dir
                .as_ref()
                .map_or_else(|| "?".to_string(), |d| d.path().display().to_string()),
            tail.len(),
            tail.join("\n")
        )
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

/// The app opens a window even when hidden, so it cannot start without a display.
fn display_available(x11: Option<&std::ffi::OsStr>, wayland: Option<&std::ffi::OsStr>) -> bool {
    [x11, wayland]
        .into_iter()
        .flatten()
        .any(|display| !display.is_empty())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::display_available;

    /// Given neither DISPLAY nor WAYLAND_DISPLAY (or only empty ones)
    /// When the fixture checks for a display
    /// Then there is none, and launch fails with that named instead of timing out.
    #[test]
    fn a_missing_display_is_detected_before_launch() {
        assert!(!display_available(None, None));
        assert!(!display_available(Some(OsStr::new("")), None));
        assert!(display_available(Some(OsStr::new(":0")), None));
        assert!(display_available(None, Some(OsStr::new("wayland-0"))));
    }
}
