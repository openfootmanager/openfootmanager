//! Building the evidence a bug report carries, and sending it when the player agrees to.
//!
//! The bundle is built on the player's machine (slice 3 of #569). `relay` and `http` upload it to
//! the maintainer's inbox only after explicit consent (slice 4), and `history` remembers the
//! reference codes that came back.

pub mod bundle;
pub mod history;
pub mod http;
pub mod redact;
pub mod relay;

/// The bundle could not be written or read back. Shared by the export and the upload, which both
/// build the same file, so the two cannot drift into different words for the same failure.
pub const REPORT_BUNDLE_FAILED: &str = "be.error.report.bundleFailed";
