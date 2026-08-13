//! WebKitGTK rendering policy for Linux.
//!
//! Tauri renders through WebKitGTK on Linux, and WebKitGTK's DMABuf renderer has a long history
//! of failing on NVIDIA proprietary drivers — blank windows, flicker on resize, `EGL_BAD_PARAMETER`
//! aborts. The historical fix is `WEBKIT_DISABLE_DMABUF_RENDERER=1`, which this app forced on
//! every Linux machine.
//!
//! That is a heavier hammer than it looks. The variable used to select WebKitGTK's WPE/X11
//! fallback renderer, which was still hardware-accelerated; that renderer was removed during the
//! 2.43 cycle (the installed 2.52 library no longer links `libwpe` at all). Tauri's own guidance
//! ranks the variable third of four workarounds and notes it "sacrifices the faster rendering
//! pathway".
//!
//! So the workaround plausibly costs every Linux user — including AMD and Intel users who never
//! had the bug — the accelerated compositing path. Whether it actually does is a question for
//! measurement, and measurement needs a way to turn it off. That is what this module adds:
//! `OFM_GPU_PROFILE` selects the policy, and `Auto` deliberately still means "what we shipped
//! before" so this change moves no needles on its own.
//!
//! See `docs/LINUX_GRAPHICS.md` for the measurements that decide what `Auto` should become.

/// Which WebKitGTK rendering workarounds to apply.
///
/// Selected by the `OFM_GPU_PROFILE` environment variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuProfile {
    /// Pick automatically. Currently identical to [`GpuProfile::Safe`] — hardware detection
    /// lands once `docs/LINUX_GRAPHICS.md` has numbers to justify a smarter choice.
    Auto,
    /// Disable the DMABuf renderer. Cures blank windows, at the cost of the accelerated path.
    Safe,
    /// Set nothing and let WebKitGTK choose. The measurement baseline, and the right answer on
    /// hardware where the default path already works.
    Off,
}

/// Environment variables a profile wants set, as `(name, value)` pairs.
type EnvVars = &'static [(&'static str, &'static str)];

impl GpuProfile {
    /// Parse the `OFM_GPU_PROFILE` value. Unrecognised values fall back to [`GpuProfile::Auto`]
    /// so a typo degrades to the shipped behaviour rather than to no workaround at all.
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "safe" => Self::Safe,
            "off" | "none" => Self::Off,
            _ => Self::Auto,
        }
    }

    /// The variables this profile wants set.
    fn wanted_vars(self) -> EnvVars {
        match self {
            Self::Auto | Self::Safe => &[("WEBKIT_DISABLE_DMABUF_RENDERER", "1")],
            Self::Off => &[],
        }
    }
}

/// The variables that should actually be set, given what the environment already defines.
///
/// A variable the user set themselves is never overridden — an explicit
/// `WEBKIT_DISABLE_DMABUF_RENDERER=0` has to survive, because it is the documented way to opt
/// back into the accelerated path.
fn vars_to_set(
    profile: GpuProfile,
    already_set: impl Fn(&str) -> bool,
) -> Vec<(&'static str, &'static str)> {
    profile
        .wanted_vars()
        .iter()
        .filter(|(name, _)| !already_set(name))
        .copied()
        .collect()
}

/// Apply the rendering policy for this machine.
///
/// Must be called before Tauri builds the webview — WebKitGTK reads these variables when the web
/// process starts, so setting them later has no effect.
pub fn configure() {
    let raw = std::env::var("OFM_GPU_PROFILE").unwrap_or_default();
    let profile = GpuProfile::parse(&raw);

    let to_set = vars_to_set(profile, |name| std::env::var(name).is_ok());
    for (name, value) in &to_set {
        std::env::set_var(name, value);
    }

    // Logged so that every future Linux bug report arrives with the answer already in it.
    log::info!(
        "Linux graphics profile: {profile:?} (OFM_GPU_PROFILE={}), set {:?}",
        if raw.is_empty() { "<unset>" } else { &raw },
        to_set,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nothing_set(_: &str) -> bool {
        false
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
    fn auto_still_matches_the_previously_shipped_behaviour() {
        // This change must not move any needles on its own; `auto` is what every existing
        // install already gets.
        assert_eq!(
            vars_to_set(GpuProfile::Auto, nothing_set),
            vec![("WEBKIT_DISABLE_DMABUF_RENDERER", "1")]
        );
    }

    #[test]
    fn off_sets_nothing_at_all() {
        // The measurement baseline. Without this, the accelerated path is unreachable and the
        // env-var ladder in docs/LINUX_GRAPHICS.md has no row 0.
        assert!(vars_to_set(GpuProfile::Off, nothing_set).is_empty());
    }

    #[test]
    fn a_user_set_variable_is_never_overridden() {
        // `WEBKIT_DISABLE_DMABUF_RENDERER=0` is WebKitGTK's own opt-out (it compares the value
        // against "0"), so clobbering it would silently ignore the user.
        let already = |name: &str| name == "WEBKIT_DISABLE_DMABUF_RENDERER";
        assert!(vars_to_set(GpuProfile::Safe, already).is_empty());
    }
}
