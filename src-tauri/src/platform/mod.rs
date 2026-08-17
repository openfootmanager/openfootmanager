//! Platform-specific startup configuration.
//!
//! Every function here exists on **all** platforms; only the implementation varies. Callers in
//! `lib.rs` are written without `#[cfg]`, so a platform that needs nothing gets a documented
//! no-op rather than a missing symbol, and adding Windows or macOS behaviour later has an
//! obvious home instead of another conditional at the call site.

#[cfg(target_os = "linux")]
mod linux_graphics;

#[cfg(target_os = "linux")]
pub use linux_graphics::{configure_graphics, watch_startup};

/// Windows renders through WebView2 and macOS through WKWebView. Neither has the WebKitGTK
/// DMABuf/NVIDIA problem that `linux_graphics` exists to work around, and neither reads the
/// environment variables it sets — so there is deliberately nothing to do here.
#[cfg(not(target_os = "linux"))]
pub fn configure_graphics() {}

/// See [`configure_graphics`] — no rendering policy is chosen off Linux, so there is no
/// startup attempt to record and nothing to clear.
#[cfg(not(target_os = "linux"))]
pub fn watch_startup() {}
