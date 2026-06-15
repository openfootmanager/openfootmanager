//! Real implementations for MCP tools.
//!
//! Each function takes `Arc<McpContext>` and returns formatted text.
//! They call the same `*_internal` functions used by Tauri commands,
//! then format the result as markdown for agent readability.
//!
//! ⚠️  WHEN ADDING A NEW TOOL IMPLEMENTATION:
//!     - Add the `pub fn` in the appropriate sub-module (or create a new one)
//!     - If the tool mutates game state, emit `"game-state-changed"` via
//!       `ctx.app_handle.emit("game-state-changed", ())` so the GUI refreshes
//!     - Register the tool in `tools.rs` `build_tool_router()`
//!     - Add it to `tool_catalog()` in `tools.rs` so `help_find_tool` finds it
//!     - Update `docs/MCP_SERVER.md` tool tables

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
pub mod live_match;
