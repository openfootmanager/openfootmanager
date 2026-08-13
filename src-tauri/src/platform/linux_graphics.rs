//! WebKitGTK rendering policy for Linux.
//!
//! Tauri renders through WebKitGTK on Linux, and WebKitGTK's DMABuf renderer has a long history
//! of failing on NVIDIA's proprietary driver. On this project's reference machine (Wayland, KDE,
//! GTX 1060) the app does not start at all without a workaround: it dies with
//! `Error 71 (Protocol error) dispatching to Wayland display`.
//!
//! The app used to work around that with `WEBKIT_DISABLE_DMABUF_RENDERER=1` on every Linux
//! machine. That cures the crash and costs **13x on compositing** — measured 224 ms and 372 ms per
//! composited frame against 17 ms with the renderer left enabled. The variable is heavier than it
//! looks: it once selected WebKitGTK's WPE/X11 fallback renderer, which was still
//! hardware-accelerated, but that renderer was removed during the 2.43 cycle, so today it means
//! compositing on the CPU.
//!
//! `__NV_DISABLE_EXPLICIT_SYNC=1` fixes the same crash while keeping the DMABuf renderer — one
//! rung cheaper on Tauri's ladder of workarounds, and the one this now prefers.
//!
//! Full measurements, including the options that turned out to be dead ends, are in
//! `docs/LINUX_GRAPHICS.md`.

use std::path::PathBuf;
use std::sync::OnceLock;

/// What `configure` decided, held until there is a logger to write it to.
///
/// `configure` has to run before `tauri::Builder`, because WebKitGTK and the NVIDIA driver read
/// these variables when the web process starts. That is also before `tauri-plugin-log` is
/// installed, so logging the decision at the point it is made silently discards it — which is
/// exactly the message we most want in a bug report. Stash it and let `setup` emit it.
static DECISION: OnceLock<String> = OnceLock::new();

/// What the user asked for, via `OFM_GPU_PROFILE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GpuProfile {
    /// Choose based on the hardware present. The default.
    Auto,
    /// Force the conservative workaround. Slow, but starts on hardware where nothing else does.
    Safe,
    /// Set nothing at all. The measurement baseline.
    Off,
}

/// What we actually do about it. Separate from [`GpuProfile`] because `Auto` resolves to
/// different policies on different hardware, and because the policy is what the tests assert on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Policy {
    /// Touch nothing. Correct on AMD and Intel, where the DMABuf renderer works.
    Nothing,
    /// Keep the DMABuf renderer, but disable NVIDIA's explicit sync. Accelerated and stable.
    NvidiaExplicitSyncOff,
    /// Disable the DMABuf renderer outright. Costs the accelerated compositing path.
    DisableDmabuf,
}

/// The facts the policy is derived from. Passed in rather than read inside, so the decision is
/// testable on a machine with no GPU at all.
#[derive(Debug, Clone, Default)]
struct GraphicsEnvironment {
    /// DRM driver names, e.g. `["nvidia", "i915"]`.
    gpu_drivers: Vec<String>,
    /// How many launches in a row failed to survive startup. See [`startup_state_path`].
    consecutive_failures: u32,
}

/// Failed launches tolerated before giving up on the accelerated path.
///
/// A count rather than a flag, because a single failure is not good evidence. The process can die
/// young for reasons that have nothing to do with rendering — the user closing a mistakenly
/// launched app, a `kill`, an OOM, a panic elsewhere in startup — and a flag would read all of
/// those as "the GPU path is broken" and impose the 13x compositing penalty on hardware that was
/// never faulty.
///
/// With a count, one spurious failure costs nothing: the next launch retries the fast path and,
/// on surviving, clears the counter. Only a genuinely repeatable failure reaches the threshold.
const ESCALATE_AFTER: u32 = 2;

impl GraphicsEnvironment {
    fn has_nvidia(&self) -> bool {
        self.gpu_drivers.iter().any(|driver| driver == "nvidia")
    }
}

impl GpuProfile {
    /// Parse the `OFM_GPU_PROFILE` value. Unrecognised values fall back to [`GpuProfile::Auto`]
    /// so a typo degrades to the supported path rather than to no workaround at all.
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "safe" => Self::Safe,
            "off" | "none" => Self::Off,
            _ => Self::Auto,
        }
    }
}

/// Decide what to do. The whole policy, in one readable place.
fn policy_for(profile: GpuProfile, env: &GraphicsEnvironment) -> Policy {
    match profile {
        GpuProfile::Off => Policy::Nothing,
        GpuProfile::Safe => Policy::DisableDmabuf,
        // Repeated failures to survive startup are evidence the accelerated path does not work
        // here, whatever the hardware claims. Drop to the configuration that always starts.
        GpuProfile::Auto if env.consecutive_failures >= ESCALATE_AFTER => Policy::DisableDmabuf,
        GpuProfile::Auto if env.has_nvidia() => Policy::NvidiaExplicitSyncOff,
        GpuProfile::Auto => Policy::Nothing,
    }
}

/// Whether this launch should be counted if it fails to survive startup.
///
/// Only `auto` gets to fall back, and only when it is attempting a path that might not start.
/// Two cases must stay out of it:
///
/// - `off` and `safe` are explicit user choices. `off` in particular is the measurement baseline
///   and is *expected* to crash on this hardware; letting it record a failure would push the next
///   `auto` launch onto the slow path because of a deliberate experiment.
/// - `auto` that already resolved to `DisableDmabuf` **is** the fallback, so there is nothing
///   further to fall back to. Crucially this also means the fallback never touches the counter,
///   so it cannot erase the evidence that put it there — an earlier version cleared the marker on
///   every successful launch, which made a permanently broken machine alternate crash/works/crash
///   forever instead of settling on the path that works.
fn should_track_startup(profile: GpuProfile, policy: Policy) -> bool {
    profile == GpuProfile::Auto && policy != Policy::DisableDmabuf
}

fn vars_for(policy: Policy) -> &'static [(&'static str, &'static str)] {
    match policy {
        Policy::Nothing => &[],
        Policy::NvidiaExplicitSyncOff => &[("__NV_DISABLE_EXPLICIT_SYNC", "1")],
        Policy::DisableDmabuf => &[("WEBKIT_DISABLE_DMABUF_RENDERER", "1")],
    }
}

/// The variables that should actually be set, given what the environment already defines.
///
/// A variable the user set themselves is never overridden — an explicit
/// `WEBKIT_DISABLE_DMABUF_RENDERER=0` has to survive, because it is WebKitGTK's own documented
/// way to opt back into the accelerated path (it compares the value against the string `0`).
fn vars_to_set(
    profile: GpuProfile,
    env: &GraphicsEnvironment,
    already_set: impl Fn(&str) -> bool,
) -> Vec<(&'static str, &'static str)> {
    vars_for(policy_for(profile, env))
        .iter()
        .filter(|(name, _)| !already_set(name))
        .copied()
        .collect()
}

/// DRM driver names for every card the kernel knows about.
///
/// Reads `/sys/class/drm/card*/device/uevent`, which is plain text and always readable. Any
/// failure yields an empty list, which resolves to "change nothing" — the safe direction.
fn detect_gpu_drivers() -> Vec<String> {
    let mut drivers = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return drivers;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // `card0`, but not `card0-HDMI-A-1` (a connector) or `renderD128`.
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let Ok(uevent) = std::fs::read_to_string(entry.path().join("device/uevent")) else {
            continue;
        };
        for line in uevent.lines() {
            if let Some(driver) = line.strip_prefix("DRIVER=") {
                let driver = driver.trim().to_owned();
                if !drivers.contains(&driver) {
                    drivers.push(driver);
                }
            }
        }
    }
    drivers
}

/// Counter file recording consecutive launches that did not survive startup.
///
/// Incremented before the webview exists and deleted once the app has been alive long enough to
/// have rendered. A launch that dies the way the NVIDIA failures do — quickly, before anything is
/// painted — leaves the increment behind for the next launch to see.
///
/// Deleting this file by hand is the documented way to retry the accelerated path after the app
/// has settled on the conservative one.
fn startup_state_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(base.join("openfootmanager").join("startup-failures"))
}

fn read_consecutive_failures() -> u32 {
    startup_state_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| raw.trim().parse().ok())
        .unwrap_or(0)
}

/// How long the app must stay up before a launch counts as successful.
///
/// The failures this guards against kill the process in well under a second, so this only needs
/// to clear that by a wide margin — not to be long enough for a human to finish anything. A quit
/// inside the window costs one increment, which the next successful launch clears.
const STARTUP_GRACE: std::time::Duration = std::time::Duration::from_secs(8);

/// Whether this launch is counting itself, so `watch_startup` knows whether the counter is its
/// to clear. A launch that already fell back must leave it alone.
static TRACKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Apply the rendering policy for this machine.
///
/// Must be called before Tauri builds the webview — WebKitGTK and the NVIDIA driver read these
/// variables when the web process starts, so setting them later has no effect.
pub fn configure_graphics() {
    let raw = std::env::var("OFM_GPU_PROFILE").unwrap_or_default();
    let profile = GpuProfile::parse(&raw);

    let env = GraphicsEnvironment {
        gpu_drivers: detect_gpu_drivers(),
        consecutive_failures: read_consecutive_failures(),
    };

    let policy = policy_for(profile, &env);
    // `var_os`, not `var`: a variable set to a non-UTF-8 value is still set, and overriding it
    // would contradict the promise that a user's own choice always wins.
    let to_set = vars_to_set(profile, &env, |name| std::env::var_os(name).is_some());
    for (name, value) in &to_set {
        std::env::set_var(name, value);
    }

    let tracking = should_track_startup(profile, policy);
    TRACKING.store(tracking, std::sync::atomic::Ordering::Relaxed);

    let mut decision = format!(
        "Linux graphics: profile={profile:?} policy={policy:?} gpus={:?} (OFM_GPU_PROFILE={}), set {to_set:?}",
        env.gpu_drivers,
        if raw.is_empty() { "<unset>" } else { &raw },
    );
    if policy == Policy::DisableDmabuf && profile == GpuProfile::Auto {
        decision.push_str(&format!(
            " — {} launches in a row failed to survive startup, so the conservative path was \
             chosen. Delete {} to retry the accelerated path; see docs/LINUX_GRAPHICS.md",
            env.consecutive_failures,
            startup_state_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "the startup-failures file".to_owned()),
        ));
    }
    let _ = DECISION.set(decision);

    // Pessimistic: record the failure up front and let a launch that survives take it back. The
    // alternative — writing on the way down — cannot work, because the failures this guards
    // against kill the process without unwinding.
    if tracking {
        if let Some(path) = startup_state_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&path, (env.consecutive_failures + 1).to_string());
        }
    }
}

/// Report the rendering decision and clear the startup marker once the app has clearly survived.
///
/// Call from Tauri's `setup` hook — by then there is a logger, and reaching this point at all is
/// most of the evidence that the chosen path works.
pub fn watch_startup() {
    match DECISION.get() {
        Some(decision) => log::info!("{decision}"),
        None => log::warn!("Linux graphics policy was never configured"),
    }

    // Only a launch that is counting itself may clear the counter. A launch that already fell
    // back to the conservative path must not, or it would erase the very evidence that sent it
    // there and the next launch would retry the path that just failed — forever.
    if !TRACKING.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    let Some(path) = startup_state_path() else {
        return;
    };
    std::thread::spawn(move || {
        std::thread::sleep(STARTUP_GRACE);
        let _ = std::fs::remove_file(&path);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nothing_set(_: &str) -> bool {
        false
    }

    fn env_with(drivers: &[&str]) -> GraphicsEnvironment {
        GraphicsEnvironment {
            gpu_drivers: drivers.iter().map(|d| (*d).to_owned()).collect(),
            consecutive_failures: 0,
        }
    }

    fn env_failing(drivers: &[&str], failures: u32) -> GraphicsEnvironment {
        GraphicsEnvironment {
            consecutive_failures: failures,
            ..env_with(drivers)
        }
    }

    /// Replays consecutive launches, returning the policy each one chose.
    ///
    /// `survives` decides whether a launch lives long enough to clear the counter, which is the
    /// only way to exercise the interaction between escalation and reset.
    ///
    /// Caveat worth knowing before trusting a green run: this models the counter bookkeeping that
    /// `configure_graphics` and `watch_startup` perform, rather than calling them — they touch
    /// the real filesystem and process environment. It pins the *decision sequence*, so a change
    /// to when the counter is written could drift from this model and still pass. Keep the two in
    /// step by hand.
    fn replay(launches: usize, survives: impl Fn(Policy) -> bool) -> Vec<Policy> {
        let mut failures = 0;
        let mut chosen = Vec::new();
        for _ in 0..launches {
            let env = env_failing(&["nvidia"], failures);
            let policy = policy_for(GpuProfile::Auto, &env);
            let tracking = should_track_startup(GpuProfile::Auto, policy);
            if tracking {
                failures += 1; // written up front, before the webview exists
            }
            if survives(policy) && tracking {
                failures = 0; // the launch lived; take the increment back
            }
            chosen.push(policy);
        }
        chosen
    }

    #[test]
    fn parses_the_documented_profile_names() {
        assert_eq!(GpuProfile::parse("auto"), GpuProfile::Auto);
        assert_eq!(GpuProfile::parse("safe"), GpuProfile::Safe);
        assert_eq!(GpuProfile::parse("off"), GpuProfile::Off);
    }

    #[test]
    fn profile_names_are_case_and_whitespace_insensitive() {
        assert_eq!(GpuProfile::parse("  SAFE "), GpuProfile::Safe);
        assert_eq!(GpuProfile::parse("Off"), GpuProfile::Off);
    }

    #[test]
    fn unknown_profile_falls_back_to_auto() {
        assert_eq!(GpuProfile::parse("banana"), GpuProfile::Auto);
        assert_eq!(GpuProfile::parse(""), GpuProfile::Auto);
    }

    #[test]
    fn nvidia_gets_the_accelerated_workaround() {
        // The measured win: 17ms per composited frame instead of 224ms.
        assert_eq!(
            vars_to_set(GpuProfile::Auto, &env_with(&["nvidia"]), nothing_set),
            vec![("__NV_DISABLE_EXPLICIT_SYNC", "1")]
        );
    }

    #[test]
    fn optimus_laptops_count_as_nvidia() {
        // The reference machine is Intel + NVIDIA. Order must not matter.
        assert_eq!(
            policy_for(GpuProfile::Auto, &env_with(&["i915", "nvidia"])),
            Policy::NvidiaExplicitSyncOff
        );
        assert_eq!(
            policy_for(GpuProfile::Auto, &env_with(&["nvidia", "i915"])),
            Policy::NvidiaExplicitSyncOff
        );
    }

    #[test]
    fn amd_and_intel_machines_are_left_alone() {
        // These never had the bug, and previously every one of them was pushed onto CPU
        // compositing by the blanket workaround.
        for drivers in [&["amdgpu"][..], &["i915"][..], &["nouveau"][..]] {
            assert!(
                vars_to_set(GpuProfile::Auto, &env_with(drivers), nothing_set).is_empty(),
                "expected no variables for {drivers:?}"
            );
        }
    }

    #[test]
    fn undetectable_hardware_changes_nothing() {
        // detect_gpu_drivers() returns empty on any read failure; that must not be read as
        // "NVIDIA" nor trigger the slow path.
        assert_eq!(
            policy_for(GpuProfile::Auto, &GraphicsEnvironment::default()),
            Policy::Nothing
        );
    }

    #[test]
    fn one_failure_is_not_enough_to_give_up_the_fast_path() {
        // A process can die young for reasons that have nothing to do with rendering. Treating a
        // single failure as proof would impose the 13x compositing penalty on healthy hardware.
        let env = env_failing(&["nvidia"], 1);
        assert_eq!(
            policy_for(GpuProfile::Auto, &env),
            Policy::NvidiaExplicitSyncOff
        );
    }

    #[test]
    fn repeated_failures_drop_to_the_conservative_path() {
        let env = env_failing(&["nvidia"], ESCALATE_AFTER);
        assert_eq!(policy_for(GpuProfile::Auto, &env), Policy::DisableDmabuf);
        assert_eq!(
            vars_to_set(GpuProfile::Auto, &env, nothing_set),
            vec![("WEBKIT_DISABLE_DMABUF_RENDERER", "1")]
        );
    }

    #[test]
    fn a_permanently_broken_machine_settles_instead_of_alternating() {
        // The bug this guards: the fallback used to clear the counter when it survived, so the
        // next launch retried the path that had just crashed. Users saw crash / works / crash /
        // works forever. The fallback must stay put once chosen.
        let chosen = replay(6, |policy| policy == Policy::DisableDmabuf);
        assert_eq!(
            chosen,
            vec![
                Policy::NvidiaExplicitSyncOff, // crashes
                Policy::NvidiaExplicitSyncOff, // crashes again — now we believe it
                Policy::DisableDmabuf,
                Policy::DisableDmabuf,
                Policy::DisableDmabuf,
                Policy::DisableDmabuf,
            ],
        );
    }

    #[test]
    fn healthy_hardware_never_degrades_however_often_it_is_relaunched() {
        // Every launch survives, so the counter is taken back each time and the fast path is
        // kept indefinitely.
        let chosen = replay(6, |_| true);
        assert!(chosen.iter().all(|p| *p == Policy::NvidiaExplicitSyncOff));
    }

    #[test]
    fn a_single_quick_quit_costs_nothing() {
        // Closing a mistakenly launched app leaves one increment behind. The next launch must
        // still take the fast path, and on surviving must clear it.
        let mut failures = 1; // the quick quit
        let env = env_failing(&["nvidia"], failures);
        let policy = policy_for(GpuProfile::Auto, &env);
        assert_eq!(policy, Policy::NvidiaExplicitSyncOff);
        failures += 1;
        if should_track_startup(GpuProfile::Auto, policy) {
            failures = 0; // survives
        }
        assert_eq!(failures, 0);
    }

    #[test]
    fn explicit_profiles_ignore_the_failure_counter() {
        // `off` is the measurement baseline and must stay inert, or the matrix silently
        // measures the fallback instead of the row it claims to.
        let env = env_failing(&["nvidia"], 99);
        assert_eq!(policy_for(GpuProfile::Off, &env), Policy::Nothing);
        assert_eq!(policy_for(GpuProfile::Safe, &env), Policy::DisableDmabuf);
    }

    #[test]
    fn safe_profile_still_means_what_the_app_used_to_do() {
        assert_eq!(
            vars_to_set(GpuProfile::Safe, &env_with(&["nvidia"]), nothing_set),
            vec![("WEBKIT_DISABLE_DMABUF_RENDERER", "1")]
        );
    }

    #[test]
    fn off_sets_nothing_at_all() {
        assert!(vars_to_set(GpuProfile::Off, &env_with(&["nvidia"]), nothing_set).is_empty());
    }

    #[test]
    fn only_auto_records_a_startup_attempt() {
        // `off` is expected to crash on NVIDIA -- it is the measurement baseline. If it planted
        // the marker, running the benchmark matrix would force the next `auto` launch onto the
        // slow path because of a deliberate experiment.
        assert!(!should_track_startup(GpuProfile::Off, Policy::Nothing));
        assert!(!should_track_startup(GpuProfile::Safe, Policy::DisableDmabuf));
    }

    #[test]
    fn the_fallback_path_does_not_track_itself() {
        // Nothing left to fall back to, so recording a failure would achieve nothing.
        assert!(!should_track_startup(
            GpuProfile::Auto,
            Policy::DisableDmabuf
        ));
    }

    #[test]
    fn auto_tracks_the_paths_that_might_not_start() {
        assert!(should_track_startup(
            GpuProfile::Auto,
            Policy::NvidiaExplicitSyncOff
        ));
        assert!(should_track_startup(GpuProfile::Auto, Policy::Nothing));
    }

    #[test]
    fn a_user_set_variable_is_never_overridden() {
        let already = |name: &str| name == "__NV_DISABLE_EXPLICIT_SYNC";
        assert!(vars_to_set(GpuProfile::Auto, &env_with(&["nvidia"]), already).is_empty());
    }
}
