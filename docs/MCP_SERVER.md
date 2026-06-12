# MCP Server for AI Agent Competition

OpenFoot Manager includes a built-in **Model Context Protocol (MCP) server** that allows AI agents to play the game through a text-based interface. Multiple agents can each run their own game instance and compete to achieve the best league position.

This feature is behind the `mcp` Cargo feature flag — normal builds are completely unaffected.

---

## Quick Start

### 1. Build with MCP support

```bash
cd src-tauri
cargo build --features mcp
```

### 2. Generate a world JSON

Start the game normally (GUI), create a new manager, select a team, then use the in-app export to save a `world.json` file. Note the team ID of the team you want agents to manage.

Alternatively, use the `game_export_world` MCP tool after creating a game.

### 3. Launch an agent instance

```bash
openfootmanager \
  --mcp-port 3001 \
  --mcp-mode competition \
  --mcp-auto-start "/path/to/world.json" \
  --no-gui \
  --manager-name "Agent Alpha" \
  --manager-nationality "England" \
  --auto-save-interval-days 7
```

The MCP server starts on port 3001. Your AI agent connects via SSE and can immediately begin calling tools.

### 4. Connect from an agent

Agents connect using the MCP SSE protocol at:

```text
http://localhost:3001/mcp
```

The endpoint speaks JSON-RPC 2.0 over Server-Sent Events. Any MCP-compatible client library works (e.g. `rmcp` in Rust, the `@modelcontextprotocol/sdk` in TypeScript, or raw HTTP).

---

## CLI Arguments

All MCP-related arguments are only recognized when the `mcp` feature is compiled in.

| Argument | Required | Default | Description |
|----------|----------|---------|-------------|
| `--mcp-port <PORT>` | **Yes** | — | Port for the MCP SSE server. Without this flag, no MCP server starts. |
| `--mcp-mode <MODE>` | No | `sandbox` | `sandbox` = all tools available. `competition` = restricted tool set (see below). |
| `--mcp-auto-start <WORLD[,TEAM]>` | No* | — | Bootstrap a game before MCP starts. Format: `"/path/to/world.json"` or `"/path/to/world.json,team_id"`. Team ID is optional for HistoricalSnapshot worlds where the manager already has a team assigned. \*\*Required in competition mode\*\* (enforced after CLI parsing, at startup). |
| `--no-gui` | No | off | Hide the GUI window (headless). Saves ~150MB RAM per instance. |
| `--manager-name <NAME>` | No | `Agent` | Manager first name for auto-start. |
| `--manager-last-name <NAME>` | No | `Manager` | Manager last name for auto-start. |
| `--manager-nationality <NAT>` | No | `England` | Manager nationality for auto-start. |
| `--auto-save-interval-days <N>` | No | `7` | Auto-save every N in-game days. `0` disables. |
| `--min-tick-delay-ms <MS>` | No | `0` | Minimum delay between `time_advance` completions. Prevents agents from advancing too fast for the GUI to follow. |
| `--mcp-disable-tools <LIST>` | No | — | Comma-separated tool names to disable on top of mode restrictions. |

\* `--mcp-auto-start` is **required** when `--mcp-mode competition` is used (the app will exit with an error otherwise). The team ID is optional — omit it if the exported world's manager already has a team assigned.

---

## Competition Mode

In competition mode, the following tools are **completely omitted** from registration — they never appear in `tools/list` and cannot be called:

- `game_new` — agents cannot create new games
- `game_select_team` — team is pre-assigned via `--mcp-auto-start`
- `game_load_save` — agents cannot load arbitrary saves
- `game_exit` — agents cannot quit to menu
- `game_export_world` — agents cannot export the world

This ensures all agents start from the same state and cannot manipulate the game setup.

---

## Tool Reference

89 tools are available across 15 categories. Use the built-in `help_list_categories` and `help_find_tool` tools to discover tools at runtime.

> **Adding a new tool?** Follow the checklist in `src-tauri/src/mcp_server/tools.rs` at `tool_catalog()` — register the route, add to the catalog, add the implementation, emit `game-state-changed` if it mutates state, update competition-mode disabled list if needed, and update this document.

### Information (15 tools)

| Tool | Description |
|------|-------------|
| `info_game_summary` | High-level overview: date, league position, finances, next match, unread messages |
| `info_game_state` | Full game state as JSON (programmatic access) |
| `info_standings` | Full league table with goal difference |
| `info_fixtures` | Upcoming fixtures and recent results for your team |
| `info_match_preview` | Next opponent details, form, and standings comparison |
| `info_season_context` | Current season phase and transfer window status |
| `info_player_profile` | Detailed player card: attributes, contract, morale (own team) or limited info (other teams) |
| `info_player_stats` | Season and career statistics for a player |
| `info_player_match_history` | Match-by-match performance for a player |
| `info_team_profile` | Team details: squad size, standings position, recent form, finances |
| `info_team_stats` | Season statistics for a team |
| `info_team_match_history` | Match-by-match results for a team |
| `info_finances` | Financial overview: wage spend, budget, projected net, debt status |
| `info_finance_snapshot` | Detailed financial snapshot with all health metrics |
| `info_news` | Recent news articles |

### Time (3 tools)

| Tool | Description |
|------|-------------|
| `time_advance` | Advance one day. Matches are auto-simulated in delegate mode. Returns round summary on match days. |
| `time_skip_to_match_day` | Fast-forward day-by-day until next fixture. Use `time_advance` to then play the match. |
| `time_check_blockers` | Check for live matches, pending transfer offers, or other blockers before advancing. |

### Squad (7 tools)

| Tool | Description |
|------|-------------|
| `squad_get` | Full squad listing with player IDs, positions, OVR, condition, morale, wages, contract end, starting XI markers |
| `squad_set_formation` | Change formation (e.g. `4-4-2`, `4-3-3`). Outfield positions are reassigned by defending ability. |
| `squad_set_starting_xi` | Set the starting eleven by player IDs (must be exactly 11) |
| `squad_set_play_style` | Change play style: `Attacking`, `Defensive`, `Possession`, `Counter`, `HighPress`, `Balanced` |
| `squad_set_match_roles` | Assign captain, penalty taker, free kick taker, corner taker by player ID |
| `squad_auto_set_pieces` | Auto-assign best set-piece takers based on attributes |
| `squad_set_player_role` | Set player squad role: `Senior` or `Youth` |

### Training (5 tools)

| Tool | Description |
|------|-------------|
| `training_get` | Current training settings and squad fitness overview |
| `training_set_focus_intensity` | Set focus (`Physical`/`Technical`/`Tactical`/`Defending`/`Attacking`/`Recovery`) and intensity (`Low`/`Medium`/`High`) |
| `training_set_schedule` | Weekly schedule: `Intense`, `Balanced`, `Light` |
| `training_set_groups` | Set training groups (JSON array of TrainingGroup objects) |
| `training_set_player_focus` | Individual player training focus (omit to clear) |

### Transfers (9 tools)

| Tool | Description |
|------|-------------|
| `transfer_market_browse` | Browse players with optional position, max price, and listed-only filters |
| `transfer_make_bid` | Make a transfer bid. Returns negotiation feedback (mood, tension, patience, suggested fee). |
| `transfer_preview_bid` | Preview financial impact of a bid without making it |
| `transfer_respond_to_offer` | Accept or reject an incoming transfer offer |
| `transfer_counter_offer` | Counter an incoming offer with a different fee |
| `transfer_toggle_listed` | Toggle a player's transfer-listed status |
| `transfer_toggle_loan` | Toggle a player's loan-listed status |
| `transfer_free_agent_offer` | Offer a contract to a free agent |
| `transfer_free_agent_preview` | Preview free agent contract financial impact |

### Contracts (7 tools)

| Tool | Description |
|------|-------------|
| `contract_propose_renewal` | Propose contract renewal. Returns negotiation feedback with suggested wage/years. |
| `contract_delegate_renewals` | Delegate renewals to assistant with budget constraints |
| `contract_preview_renewal` | Preview renewal financial impact |
| `contract_set_exit_intent` | Mark a contract to expire (let player leave on free transfer) |
| `contract_clear_exit_intent` | Remove exit intent from a contract |
| `contract_preview_termination` | Preview cost of terminating a contract |
| `contract_terminate` | Terminate a contract immediately (costs compensation) |

### Inbox (6 tools)

| Tool | Description |
|------|-------------|
| `inbox_get_messages` | List messages with optional category and read-status filters |
| `inbox_mark_read` | Mark a message as read |
| `inbox_mark_all_read` | Mark all messages as read |
| `inbox_delete` | Delete a message |
| `inbox_clear_old` | Clear old/processed messages |
| `inbox_resolve_action` | Resolve a message action (job offers, scout reports, events) |

### Club & Staff (7 tools)

| Tool | Description |
|------|-------------|
| `club_upgrade_facility` | Upgrade a facility level |
| `club_request_board_support` | Request financial support from the board |
| `club_request_marketing` | Request a marketing campaign for revenue |
| `club_request_sponsor_pitch` | Request a new sponsor pitch |
| `staff_get` | List all staff (your team + available unattached) |
| `staff_hire` | Hire an unattached staff member (triggers staff market rotation) |
| `staff_release` | Release a staff member from your team |

### Scouting (5 tools)

| Tool | Description |
|------|-------------|
| `scout_send` | Send a scout to report on a specific player |
| `scout_get_reports` | View completed scout reports and active assignments |
| `scout_youth_start` | Start a youth scouting assignment (region, objective, target position) |
| `scout_youth_cancel` | Cancel an active youth scouting assignment |
| `scout_youth_reassign` | Reassign a youth scouting assignment to a different scout |

### Season (3 tools)

| Tool | Description |
|------|-------------|
| `season_check_complete` | Check if the current season is finished |
| `season_advance` | Advance through the off-season (may result in being fired) |
| `season_get_awards` | View end-of-season awards (Golden Boot, Player of the Year, etc.) |

### Game Lifecycle (10 tools)

Most of these are **disabled in competition mode** — agents use `--mcp-auto-start` instead.

| Tool | Description |
|------|-------------|
| `game_new` | Create a new manager and generate/load a world |
| `game_select_team` | Pick a team to manage (creates initial save) |
| `game_load_save` | Load an existing save |
| `game_save` | Persist the current game |
| `game_exit` | Auto-save and return to menu |
| `game_export_world` | Export the world data to JSON (saved in app data directory with auto-generated filename) |
| `game_list_saves` | List all saved games with manager name and date |
| `game_delete_save` | Permanently delete a saved game |
| `game_list_world_databases` | List available world databases (built-in random + user JSON files) |
| `game_is_finished` | Check if the current game/season is finished |

### Live Match (7 tools)

These tools allow agents to play matches interactively instead of delegating them.

| Tool | Description |
|------|-------------|
| `match_start` | Start a live match for a fixture (mode: live, spectator, instant) |
| `match_step` | Advance the live match by N minutes |
| `match_command` | Apply a tactical command (substitution, formation change, etc.) |
| `match_snapshot` | Get current match state without advancing time |
| `match_finish` | Finish the match, apply results, and clean up |
| `match_team_talk` | Apply a team talk (half-time or full-time) affecting player morale |
| `match_press_conference` | Submit press conference answers affecting squad morale |

### Jobs (2 tools)

| Tool | Description |
|------|-------------|
| `jobs_available` | List current job openings. Employed managers only see clubs that are a step up in reputation. |
| `jobs_apply` | Apply for a job (result: Hired / Rejected / InvalidTeam / AlreadyEmployed / SameTeam / NotBetterClub). Employed managers can switch to better clubs. |

> **Club Switching**: Employed managers can now receive job offers from bigger clubs and apply for them. When hired at a new club, your old club's manager slot is cleared, your career history entry for the old club is closed, and a new open entry for the new club is created. Use `jobs_available` to see opportunities and `jobs_apply` to accept one.

### Help (2 tools)

| Tool | Description |
|------|-------------|
| `help_find_tool` | Search tools by keyword |
| `help_list_categories` | List all tool categories with tool counts |

### Utility (1 tool)

| Tool | Description |
|------|-------------|
| `ping` | Check if the MCP server is alive |

---

## Typical Agent Workflow

```text
1.  info_game_summary          → "Where am I? What day is it?"
2.  info_standings             → "What's my league position?"
3.  info_match_preview         → "Who am I playing next?"
4.  squad_get                  → "How's my squad? Anyone injured?"
5.  squad_set_formation        → "Set formation for the match"
6.  squad_set_starting_xi     → "Pick my best 11"
7.  time_advance              → "Play the match" → see result + standings update
8.  (repeat daily)
9.  inbox_get_messages         → "Any transfer offers or contract issues?"
10. transfer_market_browse     → "Looking for a striker"
11. transfer_make_bid         → "Bid on a player"
12. contract_propose_renewal  → "Renew expiring contract"
13. game_save                 → "Save progress"
```

---

## Multi-Instance Competition Setup

To run 8 agents competing in parallel:

```bash
# Generate one world.json from the GUI, then:

for i in $(seq 1 8); do
  PORT=$((3000 + i))
  SAVEDIR="/tmp/ofm-agent-$i"
  mkdir -p "$SAVEDIR"
  export TAURI_SAVE_DIR="$SAVEDIR"

  openfootmanager \
    --mcp-port $PORT \
    --mcp-mode competition \
    --mcp-auto-start "/path/to/world.json" \
    --no-gui \
    --manager-name "Agent $i" \
    --auto-save-interval-days 7 \
    --min-tick-delay-ms 100 \
    &

  echo "Agent $i → port $PORT, saves in $SAVEDIR"
done
```

Each instance gets its own port (3001–3008) and its own game state. Agents diverge only through their decisions.

**Note on save isolation**: By default, all Tauri instances share the same `app_data_dir/saves/` directory (and thus the same SQLite databases). The script above achieves per-instance isolation by setting `TAURI_SAVE_DIR` to a unique directory for each agent. Alternatively, you can build with distinct app identifiers.

---

## Transport Protocol

The MCP server uses the **Streamable HTTP** transport (SSE) as defined by the MCP specification:

- **Endpoint**: `http://localhost:<PORT>/mcp`
- **Protocol**: JSON-RPC 2.0 over Server-Sent Events
- **Keep-alive**: SSE keep-alive pings every 30 seconds
- **One connection per server**: Concurrent MCP connections are not supported. An agent that disconnects can reconnect and continue where it left off (game state persists in `StateManager`).

### Manual test with curl

```bash
# Initialize session
curl -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'

# List available tools
curl -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

# Call a tool
curl -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ping","arguments":{}}}'
```

---

## Building

### With MCP support

```bash
cd src-tauri
cargo build --features mcp
```

### Without MCP support (normal release)

```bash
cd src-tauri
cargo build
```

The `mcp` feature adds `rmcp`, `axum`, `tower`, `tokio` (with `net`), and `tokio-util` as dependencies. These are not included in normal builds.

---

## Architecture

```text
┌──────────────────────────────────────────────────┐
│                Tauri App (one process)            │
│                                                   │
│  ┌──────────┐     ┌────────────────────────────┐  │
│  │   GUI    │     │    MCP SSE Server          │  │
│  │ (React)  │     │    (axum on :3001)         │  │
│  └────┬─────┘     └──────────────┬─────────────┘  │
│       │                          │                 │
│       └──────────┬───────────────┘                 │
│                  ▼                                 │
│     ┌──────────────────────────────┐               │
│     │        StateManager          │               │
│     │  Mutex<Option<Game>>         │               │
│     │  Mutex<Option<StatsState>>   │               │
│     │  Mutex<Option<LiveMatch>>    │               │
│     │  Mutex<Option<String>>       │  ← save ID   │
│     └──────────────────────────────┘               │
│                  │                                 │
│     ┌──────────────────────────────┐               │
│     │     SaveManagerState         │               │
│     │  Mutex<SaveManager>         │               │
│     └──────────────────────────────┘               │
│                                                   │
│     AppHandle → emit("game-state-changed")        │
│       ↳ GUI fetches new state on receipt          │
└──────────────────────────────────────────────────┘
```

- MCP tools call the same `_internal` functions used by Tauri commands — no duplicate business logic.
- `Arc<StateManager>` and `Arc<SaveManagerState>` are shared between the GUI thread pool and the axum/tokio thread pool.
- The **clone-inside-lock pattern** is mandatory: clone data under a Mutex lock, release the lock, process the data, re-acquire to write back.
- `app.emit("game-state-changed", ())` is called after every state-mutating tool, enabling the GUI to auto-refresh when watching an agent play.

---

## Information Visibility

In competition mode, information about other teams' players is limited:

- **Your team**: Full detail (all attributes, condition, morale, wage, contract end, injury status)
- **Other teams**: OVR, position, age, and condition only
- **Scouted players**: Full detail from scout reports (use `scout_send` → `scout_get_reports`)

This makes scouting strategically important — agents must invest scout assignments to discover player details before bidding.

---

## Rate Limiting & Auto-Save

### `--min-tick-delay-ms`

Prevents agents from advancing too fast. Each `time_advance` call sleeps for the specified duration before processing. Set to `0` (default) for unlimited speed, or `100`+ ms to keep the GUI responsive during rapid advancement.

### `--auto-save-interval-days`

Automatically saves the game every N in-game days (default: 7). Uses a simple counter — every Nth `time_advance` triggers a save. Set to `0` to disable auto-save.

---

## Troubleshooting

### MCP server doesn't start

- Ensure `--mcp-port` is specified (this is the trigger that starts the MCP server)
- Ensure the binary was built with `--features mcp`

### Connection refused

- Check the port isn't already in use: `lsof -i :3001`
- Verify the server logged: `[mcp] Starting MCP server on port 3001`

### Agent can't see tools

- Check `--mcp-mode` — competition mode hides certain tools
- Use `help_list_categories` and `help_find_tool` to discover what's available
- Use `--mcp-disable-tools` only if you need additional restrictions beyond the mode

### Agent gets "no active game session" errors

- Ensure `--mcp-auto-start` was provided (or manually call `game_new` + `game_select_team` in sandbox mode)
- Check that the world JSON path is correct and readable

### GUI doesn't update when agent acts

- This is expected in `--no-gui` mode (window is hidden)
- With GUI visible, the frontend listens for `game-state-changed` events and auto-refreshes
- If the GUI seems stuck, click any tab to force a state fetch

---

## File Structure

```text
src-tauri/src/mcp_server/
├── mod.rs           # axum SSE server startup
├── config.rs        # CLI argument parsing
├── context.rs       # McpContext (shared state holder)
├── server.rs        # OfmMcpHandler (ServerHandler trait impl)
├── tools.rs         # ToolRouter builder with macro-based registration
├── tools_impl/      # Tool implementations by category
│   ├── mod.rs       # Module re-exports
│   ├── helpers.rs   # Shared helpers (require_game, user_team, etc.)
│   ├── info.rs      # Information queries (standings, fixtures, profiles)
│   ├── time.rs      # Time advancement (advance, skip, blockers)
│   ├── squad.rs     # Squad management (formation, XI, roles)
│   ├── training.rs  # Training settings and groups
│   ├── transfers.rs # Transfer market and negotiations
│   ├── contracts.rs # Contract renewals and termination
│   ├── inbox.rs     # Message management and action resolution
│   ├── club.rs      # Club facilities, staff, and board requests
│   ├── scouting.rs  # Scout dispatch and youth scouting
│   ├── season.rs    # Season progression, awards, and jobs
│   ├── game.rs      # Game lifecycle (new, load, save, exit, export)
│   ├── live_match.rs # Live match play, team talks, press conferences
│   └── help.rs      # Tool discovery helpers
└── formatting.rs    # Error key → human-readable translation
```
