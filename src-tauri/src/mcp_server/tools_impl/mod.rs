//! Real implementations for MCP tools.
//!
//! Each function takes `Arc<McpContext>` and returns formatted text.
//! They call the same `*_internal` functions used by Tauri commands,
//! then format the result as markdown for agent readability.

pub mod helpers;
pub mod info;
pub mod time;
pub mod squad;
pub mod training;
pub mod transfers;
pub mod contracts;
pub mod inbox;
pub mod club;
pub mod scouting;
pub mod season;
pub mod game;
pub mod help;
