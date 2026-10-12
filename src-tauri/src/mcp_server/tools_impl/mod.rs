//! Real implementations for MCP tools.
//!
//! Each function takes `Arc<McpContext>` and returns the tool's result struct from the
//! `mcp-results` crate, whose `Display` is the markdown an agent reads and whose `Serialize`
//! is the `structuredContent`. They call the same `*_internal` functions used by Tauri commands.
//!
//! ⚠️  WHEN ADDING A NEW TOOL IMPLEMENTATION:
//!     - Add the `pub fn` in the appropriate sub-module (or create a new one)
//!     - Add its result struct to `crates/mcp-results` (a tool cannot return anything else)
//!     - If the tool mutates game state, emit `"game-state-changed"` via
//!       `ctx.app_handle.emit("game-state-changed", ())` so the GUI refreshes
//!     - Register the tool in `tools.rs` `build_tool_router()`
//!     - Add it to `tool_catalog()` in `tools.rs` so `help_find_tool` finds it
//!     - Update `docs/MCP_SERVER.md` tool tables

pub mod club;
pub mod contracts;
pub mod game;
pub mod help;
pub mod helpers;
pub mod inbox;
pub mod info;
pub mod live_match;
pub mod scouting;
pub mod season;
pub mod squad;
pub mod time;
pub mod training;
pub mod transfers;
