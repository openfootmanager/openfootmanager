//! Drives the built app the way an agent does: one real process per scenario, spoken to over
//! MCP, with the results read back as the typed structs the server itself serializes.
//!
//! - [`App`] starts and stops the process, and says what broke when it won't start.
//! - [`Client`] speaks the protocol. It never retries: a call that timed out may have run.
//! - [`tools`] is one typed method per tool.
//! - [`assert_refusal_changes_nothing`] proves a refused call left every layer as it was.
//! - [`expect_bug!`] records a known-broken route and tells you when it is fixed.

mod app;
mod bug;
pub mod career;
mod client;
mod snapshot;
pub mod tools;

pub use app::{real_startup_failures_file, App, AppConfig, LaunchError, Mode, Teardown};
pub use bug::expect_bug;
pub use client::{CallError, Client, Refusal};
pub use snapshot::{assert_refusal_changes_nothing, Snapshot};

pub use mcp_results as results;
