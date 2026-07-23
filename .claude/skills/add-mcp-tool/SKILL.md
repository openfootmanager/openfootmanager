---
name: add-mcp-tool
description: Add a tool to the MCP server that lets AI agents play OpenFoot Manager. Follows the six-step checklist in tools.rs — route registration, catalog entry, implementation in the right tools_impl module, game-state-changed emission, competition-mode gating, and the docs/MCP_SERVER.md tables.
when_to_use: Adding or changing an MCP tool, exposing an existing Tauri command to agents, or when a tool exists but help_find_tool cannot see it.
argument-hint: "[tool name and what it does]"
allowed-tools: Read, Edit, Write, Grep, Glob, Bash(cargo build*), Bash(cargo test*), Bash(cargo clippy*)
---

# Adding an MCP tool

The MCP server (`src-tauri/src/mcp_server/`, behind the `mcp` Cargo feature) lets AI agents play
the game over JSON-RPC. It is used for agent competitions — see `docs/MCP_SERVER.md`.

The authoritative checklist lives in the code, at `tool_catalog()` in
`src-tauri/src/mcp_server/tools.rs`. Read it; this skill expands on it.

**The router and the catalog are two separate lists that must stay in sync.** `help_find_tool`
searches only the catalog, so a tool registered in the router but missing from the catalog works
if an agent guesses its name and is otherwise invisible. Nothing checks this for you.

---

## 1. Implement it in the right `tools_impl/` module

`src-tauri/src/mcp_server/tools_impl/` — `info.rs`, `time.rs`, `squad.rs`, `training.rs`,
`transfers.rs`, `contracts.rs`, `inbox.rs`, `club.rs`, `scouting.rs`, `season.rs`, `game.rs`,
`live_match.rs`, `help.rs`. Shared helpers (`require_game`, `user_team`, …) are in `helpers.rs`.

**Call the same `_internal` function the Tauri command calls.** Never reimplement game logic here.
If the behaviour has no `_internal` split yet, do that first — see `/add-tauri-command`.

Return text an agent can actually use. Tools return prose or JSON, not opaque ids: an agent
reading `info_standings` should be able to act on it without a second call. Follow the formatting
of the neighbouring tools in the module, and translate error keys through `formatting.rs`.

**Respect competition-mode information limits.** Full detail for the agent's own team; OVR,
position, age, and condition only for other teams unless the player has been scouted. A new
`info_*` tool that leaks opponent attributes silently breaks competition fairness.

## 2. Register the route

In `build_tool_router()` in `tools.rs`. Three macros cover most cases:

- `real_tool!` — no parameters
- `id_tool!` — one required string parameter (the schema key and the impl function's second
  parameter name must match)
- `custom_tool!` — custom JSON schema plus custom extraction

Every registration is wrapped in a `disabled.contains(…)` check, which is how mode restrictions
and `--mcp-disable-tools` work.

## 3. Add the catalog entry

`tool_catalog()` in the same file: `(name, description, category)`. Use an existing category
string exactly — a typo creates a phantom category in `help_list_categories`.

The description is what an agent reads when deciding whether to call the tool. Say what it does
*and* when to use it.

## 4. Emit `game-state-changed` if it mutates

Any state-mutating tool must emit `app.emit("game-state-changed", ())` via `ctx.app_handle`, so a
GUI watching the agent play refreshes. Read-only tools must not.

## 5. Decide competition-mode visibility

If agents must not have this tool in a competition, add its name to the `Competition` arm of
`disabled_tools()` in `src-tauri/src/mcp_server/config.rs`. Currently disabled there: `game_new`,
`game_select_team`, `game_export_world`, `game_exit`, `game_load_save` — anything that would let
an agent change its own starting conditions.

## 6. Update `docs/MCP_SERVER.md`

Add the row to the right category table **and** update the two counts in the header line
("N tools are available across M categories") and the per-category heading ("### Squad (7 tools)").
Stale counts are how this document rots.

---

## Verify

```bash
cargo build --manifest-path src-tauri/Cargo.toml --features mcp
cargo clippy --manifest-path src-tauri/Cargo.toml --features mcp --all-targets
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```

The `mcp` feature is not compiled by default, so a normal `cargo check` will **not** catch a
mistake in this code. Always build with `--features mcp`.

End-to-end, against a running instance:

```bash
# tools/list — confirm the tool appears (and does NOT in competition mode if gated)
curl -X POST http://localhost:3001/mcp -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'

# call it
curl -X POST http://localhost:3001/mcp -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"your_tool","arguments":{}}}'
```

`docs/MCP_SERVER.md` §"Quick Start" has the full launch command.

---

## Checklist

- [ ] Implementation in the right `tools_impl/` module, calling the shared `_internal` function
- [ ] Route registered in `build_tool_router()`
- [ ] Entry added to `tool_catalog()` with an existing category string
- [ ] `game-state-changed` emitted if and only if the tool mutates state
- [ ] Competition-mode gating decided; `config.rs` `disabled_tools()` updated if needed
- [ ] Information-visibility limits respected for other teams' players
- [ ] `docs/MCP_SERVER.md` table row added **and** both tool counts updated
- [ ] Builds and lints clean with `--features mcp`
- [ ] Verified via `tools/list` and `tools/call` against a running instance
