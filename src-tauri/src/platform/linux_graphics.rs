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
pub enum GpuProfile {
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
pub struct GraphicsEnvironment {
    /// DRM driver names, e.g. `["nvidia", "i915"]`.
    pub gpu_drivers: Vec<String>,
    /// Whether the previous launch failed to survive startup. See [`startup_sentinel_path`].
    pub previous_launch_failed: bool,
}

impl GraphicsEnvironment {
    fn has_nvidia(&self) -> bool {
        self.gpu_drivers.iter().any(|driver| driver == "nvidia")
    }
}

impl GpuProfile {
    /// Parse the `OFM_GPU_PROFILE` value. Unrecognised values fall back to [`GpuProfile::Auto`]
    /// so a typo degrades to the supported path rather than to no workaround at all.
    pub fn parse(raw: &str) -> Self {
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
        // A launch that did not survive startup is evidence the accelerated path does not work
        // here, whatever the hardware claims. Drop to the configuration that always starts.
        GpuProfile::Auto if env.previous_launch_failed => Policy::DisableDmabuf,
        GpuProfile::Auto if env.has_nvidia() => Policy::NvidiaExplicitSyncOff,
        GpuProfile::Auto => Policy::Nothing,
    }
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

/// Marker file proving a launch got past startup.
///
/// Written before the webview exists and removed once the app has been alive long enough to have
/// rendered. If a launch dies the way the NVIDIA failures do — quickly, before anything is
/// painted — the marker survives and the next launch sees it.
fn startup_sentinel_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(base.join("openfootmanager").join("startup-incomplete"))
}

/// How long the app must stay up before a launch counts as successful.
///
/// The failures this guards against kill the process in well under a second, so this only needs
/// to clear that by a wide margin — not to be long enough for a human to finish anything. Erring
/// long is the expensive direction: quitting sooner than this leaves a stale marker, which costs
/// one slow launch before it clears itself.
const STARTUP_GRACE: std::time::Duration = std::time::Duration::from_secs(8);

/// Apply the rendering policy for this machine.
///
/// Must be called before Tauri builds the webview — WebKitGTK and the NVIDIA driver read these
/// variables when the web process starts, so setting them later has no effect.
pub fn configure() {
    let raw = std::env::var("OFM_GPU_PROFILE").unwrap_or_default();
    let profile = GpuProfile::parse(&raw);

    let sentinel = startup_sentinel_path();
    let previous_launch_failed = sentinel.as_ref().is_some_and(|path| path.exists());

    let env = GraphicsEnvironment {
        gpu_drivers: detect_gpu_drivers(),
        previous_launch_failed,
    };

    let policy = policy_for(profile, &env);
    let to_set = vars_to_set(profile, &env, |name| std::env::var(name).is_ok());
    for (name, value) in &to_set {
        std::env::set_var(name, value);
    }

    let mut decision = format!(
        "Linux graphics: profile={profile:?} policy={policy:?} gpus={:?} (OFM_GPU_PROFILE={}), set {to_set:?}",
        env.gpu_drivers,
        if raw.is_empty() { "<unset>" } else { &raw },
    );
    if previous_launch_failed {
        decision.push_str(
            " — previous launch did not survive startup, so the conservative path was chosen; \
             see docs/LINUX_GRAPHICS.md",
        );
    }
    let _ = DECISION.set(decision);

    if let Some(path) = sentinel {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, b"");
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

    let Some(path) = startup_sentinel_path() else {
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
            previous_launch_failed: false,
        }
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
    fn a_failed_previous_launch_drops_to_the_conservative_path() {
        // The safety net for the new default: if the accelerated path did not survive startup
        // here, stop trying it. Applies regardless of hardware.
        let env = GraphicsEnvironment {
            gpu_drivers: vec!["nvidia".to_owned()],
            previous_launch_failed: true,
        };
        assert_eq!(policy_for(GpuProfile::Auto, &env), Policy::DisableDmabuf);
        assert_eq!(
            vars_to_set(GpuProfile::Auto, &env, nothing_set),
            vec![("WEBKIT_DISABLE_DMABUF_RENDERER", "1")]
        );
    }

    #[test]
    fn explicit_profiles_ignore_the_failure_marker() {
        // `off` is the measurement baseline and must stay inert, or the matrix silently
        // measures the fallback instead of the row it claims to.
        let env = GraphicsEnvironment {
            gpu_drivers: vec!["nvidia".to_owned()],
            previous_launch_failed: true,
        };
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
    fn a_user_set_variable_is_never_overridden() {
        let already = |name: &str| name == "__NV_DISABLE_EXPLICIT_SYNC";
        assert!(vars_to_set(GpuProfile::Auto, &env_with(&["nvidia"]), already).is_empty());
    }
}
