# MCP Server for OpenFoot Manager

## Goal

Run ~8 copies of the game with the same initial save, each driven by a different AI agent via a text-based MCP server. Agents compete to achieve the best league position. A human can watch any game through its GUI window in real time.

---

## Architecture

### MCP Server Inside the Tauri App

Each of the 8 game instances is one Tauri process with **both** a GUI window and an MCP SSE endpoint. They share the same `StateManager`, so the GUI updates live as the agent acts.

```
┌──────────────────────────────────────────────────────┐
│                Tauri App (one process)                 │
│                                                       │
│  ┌──────────┐     ┌────────────────────────────────┐  │
│  │   GUI    │     │       MCP SSE Server           │  │
│  │ (React)  │     │     (axum on :3001)             │  │
│  └────┬─────┘     └──────────────┬─────────────────┘  │
│       │                          │                    │
│       └──────────┬───────────────┘                    │
│                  ▼                                    │
│     ┌──────────────────────────────┐                  │
│     │        StateManager          │                  │
│     │  Mutex<Option<Game>>         │                  │
│     │  Mutex<Option<StatsState>>   │                  │
│     │  Mutex<Option<LiveMatch>>    │                  │
│     │  Mutex<Option<String>>       │  save_id         │
│     └──────────────────────────────┘                  │
│                  │                                    │
│     ┌────────────┴──────────────┐                     │
│     │  SaveManagerState         │                     │
│     │  Mutex<SaveManager>       │                     │
│     └───────────────────────────┘                     │
│                  │                                    │
│     ┌────────────────────────────┐                     │
│     │  AppHandle (Arc)           │  for emit(), paths │
│     └────────────────────────────┘                     │
│                  │                                    │
│           ┌──────┴──────┐                              │
│           │  ofm_core   │                              │
│           │  domain/db  │                              │
│           └─────────────┘                              │
└──────────────────────────────────────────────────────┘
```

**Why not a separate binary?** The user wants to watch the games. If the MCP server were a separate process, we'd need IPC to sync state to the GUI — complex and fragile. Building it into the Tauri app means both the GUI and the MCP server access the same `StateManager` directly.

**Launch**: `npm run tauri dev -- --mcp-port 3001` (or the built binary with `--mcp-port 3001`).  
**8 instances** = 8 processes with ports 3001–3008, each with its own window.

### Concurrency: `std::sync::Mutex` across two thread pools

Tauri commands run on the Tauri runtime thread pool; MCP axum handlers run on the tokio runtime. These are **different thread pools** sharing the same `std::sync::Mutex` locks. A long-held lock on either side could block the other, causing UI freezes or MCP timeouts.

**Mandatory pattern for all MCP tool implementations** (matches existing Tauri command code):

1. Acquire the mutex
2. Clone the `Game` / `StatsState` / etc.
3. Release the lock immediately (drop the guard)
4. Do all processing on the clone
5. Re-acquire the lock only to write back via `set_game()` / `set_stats_state()`

Never hold a lock while doing simulation work (e.g. `process_day()`). This is already the pattern in `commands/game.rs` — the MCP tools just need to follow it religiously.

### GUI Auto-Refresh

When the MCP server modifies game state, it emits a Tauri event using `app.emit("game-state-changed", ())`. The frontend adds a listener in `App.tsx` (or `gameStore.ts`) that calls `listen('game-state-changed', ...)` and, on receipt, invokes `get_active_game` to refresh `gameStore`.

We emit a **dirty signal** (empty payload) rather than the full `GameStateData` — this avoids large event payloads and lets the frontend fetch only what it needs. The cost is one extra `invoke()` round-trip, but the GUI is for human observation only, so a few hundred ms of latency is acceptable.

### SSE Transport

Agents connect via MCP over SSE:

- `GET /sse` — server-to-client event stream
- `POST /messages` — client-to-server JSON-RPC messages

This allows agents to run remotely, not just on the same machine.

**Connection lifecycle**: Game state lives in `StateManager`, independent of any MCP connection. An agent that disconnects and reconnects can continue where it left off. SSE connections use `Cache-Control: no-cache` and a 30-second keep-alive interval.

**Concurrent connections**: One MCP connection per server. Multiple connections from one agent, or connections from multiple agents, are not supported and may cause inconsistent state (interleaved mutations via `std::sync::Mutex`).

---

## Branch & File Structure

**Branch**: `feat/mcp-server` off `develop`

```
src-tauri/
├── Cargo.toml                  # Add [features] mcp = ["rmcp", "axum", ...]
├── src/
│   ├── main.rs                 # Parse --mcp-port CLI arg
│   ├── lib.rs                  # Conditionally start MCP server in setup
│   ├── mcp_server/
│   │   ├── mod.rs              # Server bootstrap, SSE endpoint, axum router
│   │   ├── tools.rs            # All MCP tool definitions + handlers
│   │   ├── formatting.rs       # Text/markdown formatters for agent-readable output
│   │   └── help.rs             # Tool search & category discovery
│   ├── commands/               # Existing — unchanged
│   └── application/            # Existing — unchanged
```

The `mcp` feature flag means normal builds (`cargo build`) don't include axum/rmcp. Only `--features mcp` compiles the server in.

---

## Dependencies (behind `mcp` feature)

| Crate | Purpose |
|-------|---------|
| `rmcp` | MCP protocol types, server handler trait, SSE transport |
| `axum` | HTTP server for SSE endpoint |
| `tower` | Middleware (used by axum) |
| `tokio` | Runtime (already transitive via Tauri; add explicit with `net` feature) |

---

## CLI Arguments

| Arg | Purpose | Example |
|-----|---------|---------|
| `--mcp-port` | Start MCP SSE server on this port | `--mcp-port 3001` |
| `--saves-dir` | Override saves directory (for isolation) | `--saves-dir ./saves/agent-1` |
| `--mcp-mode` | Tool restriction mode (see below) | `--mcp-mode competition` |
| `--mcp-disable-tools` | Comma-separated denylist of individual tool names | `--mcp-disable-tools game_new,game_export_world` |
| `--mcp-auto-start` | World JSON path + team ID to auto-bootstrap before MCP starts | `--mcp-auto-start world.json,team_abc123` |
| `--no-gui` | Start without a GUI window (headless, saves ~150MB RAM per instance) | `--no-gui` |
| `--min-tick-delay-ms` | Minimum delay between `time_advance` calls (prevents GUI starvation) | `--min-tick-delay-ms 100` |
| `--auto-save-interval-days` | Auto-save every N in-game days (default 7, 0 to disable) | `--auto-save-interval-days 7` |
| `--manager-name` | Manager name for `--mcp-auto-start` (default: "Agent N") | `--manager-name "Agent 1"` |
| `--manager-nationality` | Manager nationality for `--mcp-auto-start` (default: team's country) | `--manager-nationality England` |

---

## Tool Restrictions

Agents should not be able to cheat by creating their own fresh saves or exporting the world to inspect other agents' data. Two mechanisms control which tools are available:

### Mode flag (`--mcp-mode`)

| Mode | Description | Disabled tools |
|------|-------------|----------------|
| `sandbox` | All tools available (default) | None |
| `competition` | Game lifecycle tools + full-state inspection locked out; game must be pre-loaded | `game_new`, `game_select_team`, `game_export_world`, `game_exit`, `game_load_save`, `info_game_state` |

In `competition` mode, the game is loaded and the team is selected **before** the MCP server starts accepting connections. The agent can only play the hand it's dealt — it can save, advance time, manage the squad, etc., but cannot reset or create a new game.

### Denylist flag (`--mcp-disable-tools`)

For fine-grained control beyond the mode presets. Takes a comma-separated list of tool names to disable regardless of mode.

Example: `--mcp-disable-tools game_new,game_export_world,contract_terminate`

Both flags compose: `--mcp-mode competition --mcp-disable-tools club_upgrade_facility` would omit the competition-mode set **plus** `club_upgrade_facility`.

Disabled tools are **not registered at all** — they don't appear in `tools/list` and cannot be called. The agent never knows they exist, and their descriptions don't bloat the agent's context window.

### Competition-mode startup flow

```
1. Orchestrator generates a world and exports it to world.json
2. For each agent (1–8):
   a. Start Tauri process: --mcp-port 300N --mcp-mode competition --saves-dir ./saves/agent-N
   b. Process auto-creates a manager, loads world.json, selects the designated team
   c. MCP server starts accepting connections only AFTER the game is fully initialised
   d. Agent connects and begins play
```

This requires a new CLI arg for the pre-loaded game setup:

| Arg | Purpose | Example |
|-----|---------|---------|
| `--mcp-auto-start` | Path to world JSON + team ID to auto-bootstrap before MCP starts | `--mcp-auto-start world.json,team_abc123` |

When `--mcp-mode competition` is set **and** `--mcp-auto-start` is provided, the app boots straight into the game (no menu, no team selection) and then opens the MCP port. If `--mcp-mode competition` is set without `--mcp-auto-start`, the process exits with an error — there's nothing for the agent to play.

---

## World Cloning Workflow

All 8 agents manage the same team. Divergence is purely in their decisions.

```
1. Orchestrator calls `game_new` (via a temporary sandbox-mode process or a CLI helper) → generates world
2. Orchestrator calls `game_export_world` → saves world.json to shared path
3. For each agent (1–8), start a competition-mode process:
   `--mcp-port 300N --mcp-mode competition --mcp-auto-start world.json,team_abc123 --saves-dir ./saves/agent-N`
4. Each process auto-bootstraps the game and opens its MCP port
5. Agents connect and play — each has its own SQLite DB for replay
```

Each process gets its own `SaveManager` initialised with its own saves directory, so replay data is fully isolated.

---

## MCP Tools — Full Inventory

Every existing Tauri command gets an MCP tool wrapper. Tools return **formatted text** (markdown tables, structured paragraphs), not raw JSON — agents are text-based. Naming convention: flat `category_verb` so tools sort logically in `tools/list`.

### Tool Design Principles

1. **Entity IDs in every response.** Agents cannot click on a name to navigate — they need raw IDs to call subsequent tools. Every tabular response that lists entities must include their ID column. This applies to squad lists, standings, fixtures, messages, staff, transfer targets, etc.
2. **Concise descriptions.** Keep each tool description under 2 sentences. Put detailed workflow guidance in tool *responses* (error messages, hints), not in descriptions — this keeps the `tools/list` payload small (~55 tools × ~50 tokens each).
3. **Workflow hints in responses.** Multi-step workflows (transfers, contracts, scouting) include brief next-step guidance in their response text, not just raw data.
4. **Complex input types as flat parameters.** Where Tauri commands take structured types like `MatchRoles` or `TrainingGroup`, the MCP tool accepts separate named parameters instead of requiring agents to construct JSON objects.

### Game Lifecycle

| Tool | Description |
|------|-------------|
| `game_new` | Create manager + generate/load world. Parameters: `first_name`, `last_name`, `dob` (YYYY-MM-DD), `nationality`, `world_source?` ("random" or `file:/path`), `startup_options?` (`start_year`, `start_phase`, `history_depth_years`) |
| `game_select_team` | Pick a team to manage. Parameters: `team_id` |
| `game_load_save` | Load an existing save. Parameters: `save_id` |
| `game_save` | Persist current game |
| `game_exit` | Save and return to menu |
| `game_export_world` | Export world to JSON file. Parameters: `export_path` |
| `game_is_finished` | Returns `{ finished: bool, reason? }` — true if season is complete and all fixtures played. Useful for orchestration scripts to detect when an agent's run is done |

### Time Advancement

| Tool | Description |
|------|-------------|
| `time_advance` | Advance one day (match forced to delegate mode). On match days, response includes a round summary with all match scores for that day |
| `time_skip_to_match_day` | Fast-forward to next fixture |
| `time_check_blockers` | Check if anything blocks advancement |

### Squad Management

| Tool | Description |
|------|-------------|
| `squad_get` | Squad overview (all players with key stats) |
| `squad_set_formation` | Change formation. **Note**: this also reassigns outfield player positions based on defending ability sorting. Response includes a summary of any position reassignments |
| `squad_set_starting_xi` | Set starting eleven by player IDs |
| `squad_set_play_style` | Change play style |
| `squad_set_match_roles` | Set captain, set-piece takers. Separate parameters: `captain_id`, `vice_captain_id`, `penalty_taker_id`, `free_kick_taker_id`, `corner_taker_id` |
| `squad_set_player_role` | Set player squad role (Senior/Youth) |
| `squad_auto_set_pieces` | Auto-assign best set-piece takers |

### Training

| Tool | Description |
|------|-------------|
| `training_get` | Current training settings + fitness overview |
| `training_set_focus_intensity` | Set team training focus + intensity |
| `training_set_schedule` | Set weekly training schedule |
| `training_set_groups` | Set training groups. Parameters: groups as JSON array of `{ player_ids: string[], focus: string? }` |
| `training_set_player_focus` | Set individual player training focus |

### Transfers

| Tool | Description |
|------|-------------|
| `transfer_market_browse` | Browse available players on transfer/loan market. Optional filters: `position`, `max_price`, `min_ovr`, `sort_by` (ovr/value/age). **New backend logic required** — no existing Tauri command; currently the frontend filters `game.players` client-side |
| `transfer_make_bid` | Make a transfer bid for a player. Response includes negotiation state: decision (Accepted/Rejected/CounterOffer), suggested_fee, mood, tension, patience, round number, and whether negotiation is terminal. If counter-offered, call again with a higher fee |
| `transfer_preview_bid` | Preview financial impact of a bid |
| `transfer_respond_to_offer` | Accept/reject/withdraw an incoming offer |
| `transfer_counter_offer` | Counter an incoming offer |
| `transfer_toggle_listed` | Toggle player transfer listed status |
| `transfer_toggle_loan` | Toggle player loan listed status |
| `transfer_free_agent_offer` | Offer contract to a free agent |
| `transfer_free_agent_preview` | Preview free agent contract impact |

### Contracts

| Tool | Description |
|------|-------------|
| `contract_propose_renewal` | Propose contract renewal to a player. Response includes outcome, suggested_wage, session status, and if blocked — the date when you can try again |
| `contract_delegate_renewals` | Delegate renewals to assistant |
| `contract_preview_renewal` | Preview renewal financial impact |
| `contract_set_exit_intent` | Mark contract to let expire |
| `contract_clear_exit_intent` | Remove exit intent |
| `contract_preview_termination` | Preview cost of terminating contract |
| `contract_terminate` | Terminate contract immediately |

### Scouting

| Tool | Description |
|------|-------------|
| `scout_send` | Send scout to report on a player. Takes several in-game days — advance time with `time_advance`, then check `scout_get_reports` |
| `scout_get_reports` | Get completed scout reports. **New backend logic required** — currently scout reports are generated as `InboxMessage` objects; this tool extracts and formats structured report data from the message list |
| `scout_youth_start` | Start youth scouting assignment |
| `scout_youth_cancel` | Cancel youth scouting |
| `scout_youth_reassign` | Reassign youth scouting parameters |

### Staff

| Tool | Description |
|------|-------------|
| `staff_get` | List all staff (your team + available) |
| `staff_hire` | Hire an unattached staff member |
| `staff_release` | Release a staff member |

### Inbox

| Tool | Description |
|------|-------------|
| `inbox_get_messages` | Get messages (filterable by category, read status) |
| `inbox_mark_read` | Mark a message as read |
| `inbox_mark_all_read` | Mark all messages as read |
| `inbox_delete` | Delete message(s) |
| `inbox_clear_old` | Clear old messages |
| `inbox_resolve_action` | Resolve a message action. Effects depend on the message type: job offers (accept/decline), player conversations, random events, youth scouting prospects. Read the message's actions list to determine available options and their IDs |

### Information / Queries

| Tool | Description |
|------|-------------|
| `info_game_summary` | High-level: date, phase, league position, finances, next match, unread messages count |
| `info_game_state` | Full raw game state as JSON. **Warning**: can be very large (all players, teams, messages, league). Agents should prefer `info_game_summary` and drill-down tools instead |
| `info_standings` | League table as markdown |
| `info_fixtures` | Upcoming/past fixtures as markdown |
| `info_player_profile` | Detailed player card (attributes, season stats, match history, contract, morale) |
| `info_player_stats` | Focused season + career stats for a specific player |
| `info_player_match_history` | Match-by-match stats for a specific player |
| `info_team_profile` | Detailed team view (squad, form, finances) |
| `info_team_stats` | Focused season stats for a specific team |
| `info_team_match_history` | Match-by-match stats for a specific team |
| `info_finances` | Financial overview + ledger |
| `info_finance_snapshot` | Detailed financial snapshot |
| `info_news` | Recent news articles |
| `info_season_context` | Season phase, transfer window status |
| `info_match_preview` | Preview of next match (opponent, recent form, squad overview). Does **not** predict opponent lineups — that logic doesn't exist yet |

### Facilities / Club

| Tool | Description |
|------|-------------|
| `club_upgrade_facility` | Upgrade a facility level |
| `club_request_board_support` | Request board financial support |
| `club_request_marketing` | Request marketing campaign |
| `club_request_sponsor_pitch` | Request sponsor pitch |

### Season

| Tool | Description |
|------|-------------|
| `season_check_complete` | Check if season is finished |
| `season_advance` | Advance to next season. **Warning**: if board objectives are not met, the manager may be fired. Response indicates whether the manager was fired and suggests next actions (e.g. `jobs_available`, `jobs_apply`) |
| `season_get_awards` | Get end-of-season awards |

### Jobs

| Tool | Description |
|------|-------------|
| `jobs_available` | List available job openings |
| `jobs_apply` | Apply for a job |

### Help / Discovery

| Tool | Description |
|------|-------------|
| `help_find_tool` | Search tools by keyword or description (e.g. "I want to buy a player" → suggests `transfer_make_bid`, `transfer_market_browse`). **Phase 4+ only** — MCP clients already receive tool lists via `tools/list`; this is a convenience for agents with many tools |
| `help_list_categories` | List all tool categories with tool counts. **Phase 4+ only** |

---

## Match Handling — Delegate Only (For Now)

All `time_advance` calls force `mode = "delegate"`. No live match tools are exposed. The match is simulated instantly and results are returned in the `time_advance` response (including a round summary of all matches played that day).

**Team talks and press conferences are auto-resolved in delegate mode.** Agents do not get to influence morale through post-match talks. This keeps the MCP surface simple. If this proves important for competition quality, team talk / press conference tools can be added later (see Future section below).

### Future: Live Match Interaction

Live match interaction could be added after initial implementation by:

1. Adding `match_start`, `match_step`, `match_apply_command` tools
2. Changing `time_advance` to support `mode = "live"`
3. Adding half-time tools: `match_team_talk`, `match_substitution`, `match_formation_change`
4. Adding press conference tools: `match_press_conference`

The engine already supports step-by-step simulation and tactical commands — the MCP server would just need to expose those operations as tools instead of delegating.

---

## Winning & Evaluation

**Primary metric**: League position at end of season.

**Secondary metrics** (available via tools so evaluation scripts can penalise):
- Financial health / bankruptcy risk (`info_finances`)
- Fan approval / manager satisfaction (`info_game_summary`)
- Trophies (`season_get_awards`)

Agents that go bankrupt or get fired score poorly regardless of league position.

### Competition Comparability

All 8 agents manage the same team, but each instance is an independent world. After a few transfers, the 8 worlds diverge — different transfer markets, different AI team rosters, different match outcomes. "1st in Agent A's league" and "1st in Agent B's league" measure different things after enough divergence. This is acceptable for a fun competition but should be acknowledged.

Future improvements:
- **Shared world**: All 8 agents play as 8 different teams in one world. Makes league positions directly comparable, but requires a centralised server or turn-based protocol — significantly more complex.
- **Normalised scoring**: Score agents by improvement relative to starting position (e.g., points-per-match vs. expected performance), which normalises across divergent worlds.

### Information Visibility in Competition Mode

In competition mode, agents should not have perfect information about other teams. The following visibility model applies:

| Data | Own team | Other teams |
|------|----------|-------------|
| Player attributes | Full detail | Only OVR, position, age, form — unless a scout report exists |
| Contract details | Full | Not visible |
| Morale / fitness | Full | Not visible |
| Team finances | Full | Not visible |
| Transfer offers | Full | Not visible |

This makes scouting strategically important rather than decorative. The `info_player_profile` and `info_team_profile` tools will respect this model based on the current `--mcp-mode`.

---

## Error Handling

All existing Tauri commands return `Result<T, String>` with error keys like `"be.error.noActiveGameSession"`. MCP tools will map these to MCP error responses with **human-readable descriptions** that help agents self-correct, not just the backend key.

Mapping examples:

| Backend error key | MCP error message |
|---|---|
| `be.error.noActiveGameSession` | "No active game session. Start or load a game first." |
| `be.error.teamNotFound` | "Team not found. Check the team ID and try again." |
| `be.error.saveManagerUnavailable` | "Save manager is unavailable. This is an internal error." |
| `be.error.createManager.nameRequired` | "Manager first and last name are required." |

A helper function `translate_error(key: &str) -> String` will handle the mapping, with a fallback that returns the raw key if no mapping exists (so new backend errors are at least identifiable).

---

## Auto-Save

Agents can crash mid-season, losing all progress. The MCP server will auto-save every **7 in-game days** (once per in-game week) after `time_advance`. This is configurable via the `--auto-save-interval-days` CLI arg (default 7, set to 0 to disable). Auto-saves write to the per-agent SQLite DB.

---

## Testing Strategy

### Unit testing tool handlers

Every existing Tauri command has `*_internal` functions that take `&StateManager` and can be tested without a Tauri runtime (the existing tests in `commands/*.rs` do exactly this). The MCP tool handlers follow the same pattern: thin wrappers that call `*_internal` functions and format the result.

Each MCP tool handler has a corresponding unit test that exercises it against a `StateManager` with a pre-built `Game`, following the same pattern as the existing command tests.

### Integration testing the SSE transport

Test the axum SSE server in isolation (without the full Tauri app) by constructing a `StateManager` manually and passing it to the router. This allows CI-friendly integration tests.

### Testing tool restriction modes

Verify that disabled tools don't appear in `tools/list` and that calling a disabled tool's endpoint returns a proper MCP error. Test both `--mcp-mode competition` and `--mcp-disable-tools` individually and composed.

### Testing the information visibility model

In competition mode, verify that `info_player_profile` for another team's player returns limited data (OVR, position, age only) unless a scout report exists. In sandbox mode, verify full data is returned.

---

## Response Format Examples

### `info_standings`

```markdown
## Premier Division — Season 2032

| # | Team          | P  | W | D | L | GF | GA | GD  | Pts |
|---|---------------|----|---|---|---|----|----|-----|-----|
| 1 | Alpha FC      | 15 | 10| 3 | 2 | 28 | 12 | +16 | 33  |
| 2 | Beta FC       | 15 | 8 | 4 | 3 | 22 | 15 | +7  | 28  |
| 3 | Gamma United  | 15 | 7 | 5 | 3 | 20 | 14 | +6  | 26  |
```

### `info_player_profile`

```markdown
## A. Striker — Striker
**Team**: Alpha FC | **Age**: 26 | **Nationality**: England
**OVR**: 74 | **Potential**: 80 | **Condition**: 85 | **Morale**: 72

### Attributes
| Pac | Sho | Pas | Dri | Def | Phy |
|-----|-----|-----|-----|-----|-----|
| 72  | 79  | 60  | 73  | 28  | 70  |

### Contract
Wage: €18,000/wk | Expires: 2025-06-30 | Market Value: €2.4M

### Season Stats (2032)
Apps: 15 | Goals: 8 | Assists: 3 | Avg Rating: 7.2 | Yellow: 2 | Red: 0
```

### `info_game_summary`

```markdown
## Game Summary — 12 November 2032

**Manager**: Alex Manager | **Team**: Alpha FC
**Season Phase**: InSeason | **Transfer Window**: Open (14 days remaining)

### Position & Form
League Position: 1st | Form: W-W-D-W-L

### Finances
Balance: €4.2M | Wage Budget: €180K/wk | Transfer Budget: €1.5M

### Squad Health
Avg Condition: 82% | Avg OVR: 68 | Injured: 1 | Exhausted: 2

### Next Match
vs Beta FC (A) — 15 November 2032

### Unread Messages: 3
```

### `squad_get`

```markdown
## Alpha FC — Squad Overview

| ID | Name | Pos | Age | OVR | Pot | Con | Mor | Wage | Contract |
|----|------|-----|-----|-----|-----|-----|-----|------|----------|
| player_1 | A. Keeper | GK | 31 | 68 | 73 | 90 | 70 | €12K/wk | 2024-06 |
| player_5 | B. Defender | DF | 24 | 71 | 78 | 85 | 75 | €15K/wk | 2026-06 |
| player_8 | C. Midfielder | MF | 27 | 73 | 76 | 78 | 68 | €18K/wk | 2025-06 |
| player_9 | A. Striker | FW | 26 | 74 | 80 | 85 | 72 | €18K/wk | 2025-06 |

**Starting XI**: GK A. Keeper, DF B. Defender, MF C. Midfielder, FW A. Striker, ...
**Formation**: 4-4-2 | **Play Style**: Balanced
```

### `time_advance` (match day)

```markdown
## Day Advanced — 15 November 2032

### Match Results
| Home | Score | Away |
|------|-------|------|
| Alpha FC | 2 - 1 | Beta FC |
| Gamma United | 0 - 3 | Delta City |

Your team won 2-1 vs Beta FC. Scorers: A. Striker (23', 67').

### Standings Update
| # | Team | Pts | GD |
|---|------|-----|----|
| 1 | Alpha FC | 33 | +16 |
| 2 | Beta FC | 28 | +7 |
```

### `transfer_make_bid`

```markdown
## Transfer Bid Result

**Player**: P. Two (player_42) | **Bid**: €1,050,000
**Decision**: Counter-offer | **Suggested fee**: €950,000
**Negotiation**: Round 2 | Mood: Firm | Tension: 40/100 | Patience: 60/100
**Status**: Negotiation continues — make another bid or walk away.
```

### `contract_propose_renewal`

```markdown
## Contract Renewal — A. Striker (player_9)

**Outcome**: Player wants higher wage
**Suggested wage**: €22,000/wk | **Suggested years**: 3
**Session status**: Open | **Round**: 1
**Next step**: Call contract_propose_renewal again with a different wage, or accept suggested terms.
```

```markdown
## Contract Renewal — B. Defender (player_5)

**Outcome**: Negotiation blocked
**Blocked until**: 2026-09-15
**Reason**: Player is unhappy with recent offers. Wait until the block date before trying again.
```

### `season_advance` (manager fired)

```markdown
## Season Complete — 2032/33

**Final Position**: 14th | **Points**: 38
**Board verdict**: Objectives not met. You have been fired.

**Next steps**: Use `jobs_available` to find a new position, then `jobs_apply`.
```

---

## Implementation Phases

### Phase 1 — Infrastructure

The hard part. Everything else is mechanical wrapping of existing commands.

- Add `mcp` feature flag + dependencies to `Cargo.toml`
- Create `mcp_server/mod.rs` with axum SSE server
- Parse `--mcp-port`, `--saves-dir`, `--mcp-mode`, `--mcp-auto-start`, `--no-gui`, `--min-tick-delay-ms`, `--auto-save-interval-days` CLI args in `main.rs`
- Wire `StateManager` + `SaveManagerState` + `AppHandle` access into MCP handlers (store `Arc`s in axum state)
- Implement `--mcp-auto-start` bootstrap in Tauri setup hook (create manager, load world, select team, initial save — before MCP server starts listening)
- Implement tool registration that respects `--mcp-mode` and `--mcp-disable-tools` (tools not registered = invisible to agents)
- Implement `rmcp::ServerHandler` trait
- Get a single `ping` tool working end-to-end
- Add Tauri event emission on state changes (for GUI auto-refresh)
- Implement `--no-gui` (create window then immediately hide it via `window.hide()`)
- Verify: agent connects via SSE, calls ping, gets response
- Unit test: tool handler with `StateManager` + pre-built `Game`

### Phase 2 — Game Lifecycle + Information

- `game_new`, `game_select_team`, `game_save`, `game_load_save`, `game_export_world`, `game_is_finished`
- `info_game_summary`, `info_game_state`, `info_standings`, `info_fixtures`
- Formatting module — start with minimal formatting (structured text), upgrade to rich markdown later
- Every tabular response includes entity IDs

### Phase 3 — Core Gameplay Loop

- `time_advance` (delegate mode), `time_skip_to_match_day`, `time_check_blockers`
- `squad_get`, `squad_set_formation`, `squad_set_starting_xi`, `squad_set_play_style`
- `training_get`, `training_set_focus_intensity`

### Phase 4 — Full Tool Surface

- All remaining tools: transfers, contracts, scouting, staff, inbox, finances, facilities, season, jobs
- New backend logic for `transfer_market_browse` (filter/sort players by transfer status) and `scout_get_reports` (extract scout reports from messages)
- Information visibility model for competition mode (own team full detail, other teams limited)
- `help_find_tool`, `help_list_categories`

### Phase 5 — Formatting & Polish

- Rich markdown formatting for all tool responses (upgrade from Phase 2 minimal formatting)
- Consider a `FormatEntity` trait on domain types to reduce duplication across tools
- Verify GUI auto-refresh works correctly during agent-driven actions
- Test multi-instance isolation (8 processes, 8 ports, 8 save dirs)
- Test tool restriction modes (verify disabled tools absent from `tools/list`)
- Edge cases: what happens if agent makes invalid moves, tries to advance during a blocker, etc.
- Integration test: axum SSE server with manually constructed `StateManager` (CI-friendly)

### Phase 6 — Orchestration

Separate work, to be done later. A script that:
- Starts 8 Tauri processes with ports 3001–3008
- Generates one world and distributes it
- Launches 8 agent processes pointing at the 8 MCP servers
- Collects results at season end

---

## Open Questions / Future Considerations

- **rmcp crate maturity**: If `rmcp` doesn't support SSE transport well, we'll implement the MCP SSE protocol manually using axum. The protocol is JSON-RPC 2.0 over SSE — not complex.
- **Feature flag in production**: The `mcp` feature should not be enabled in normal release builds. It's only for the agent competition scenario.
- **`--saves-dir` and `save_index.json`**: `SaveManager::init(&saves_dir)` expects the directory to contain both `.db` files and the index. With 8 parallel directories, each `SaveManager` is independent. Confirm this works correctly with the existing `init()` method — it should, since each process has its own `SaveManager` instance pointed at its own directory.
- **Headless mode (`--no-gui`)**: Phase 1 uses `window.hide()` after creation (saves ~50MB RAM by avoiding GPU compositing). If RAM is still a bottleneck, a future `bin/mcp_headless` binary that doesn't depend on `tauri` at all could save another ~100MB — but requires extracting game initialization logic into a shared module.
- **Rate limiting (`--min-tick-delay-ms`)**: Enforces a minimum interval between `time_advance` completions. Default 0 (no limit). Set to e.g. 100ms to keep the GUI responsive during rapid agent advancement. Implemented as a simple timestamp check inside the `time_advance` handler — if called too soon, it sleeps for the remainder before proceeding.
- **SSE connection lifecycle**: Game state lives in `StateManager`, independent of any MCP connection. An agent that disconnects and reconnects can continue where it left off. SSE connections should use `Cache-Control: no-cache` and a reasonable keep-alive interval. **One MCP connection per server** — concurrent connections are not supported and may cause inconsistent state.
- **`--mcp-auto-start` manager identity**: Accept optional `--manager-name` and `--manager-nationality` CLI args. If not provided, defaults to `Agent N` with the nationality of the managed team's country.
- **RNG reproducibility**: Consider a future `--rng-seed` CLI arg. Seed = `base_seed + instance_index` makes competitions reproducible. Not required for v1 but worth reserving the CLI arg name.
- **AppHandle dependency**: The MCP server needs `AppHandle` for `app.emit()` (GUI refresh) and path resolution. Store `Arc<AppHandle>` in the axum router state alongside `Arc<StateManager>` and `Arc<SaveManagerState>`.
- **Competition team selection**: A mid-table team is probably the most interesting for competition — it tests both survival and ambition. The specific team should be configurable via the `--mcp-auto-start` team ID.
- **Orchestration design (Phase 6)**: At minimum, a `start_competition.sh` script that: starts 8 processes with sequential ports and saves dirs, waits for port readiness (TCP check), launches agent processes, monitors for completion via `game_is_finished`, and collects results. Handle agent crashes gracefully (competition continues with remaining agents).
