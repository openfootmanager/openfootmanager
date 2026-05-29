# MCP_SERVER_PLAN.md — Second Review

This review approaches the plan from angles not covered in the first review: game design and competition fairness, MCP protocol specifics, the existing command layer's subtleties and how they'll need adapting, security, testing strategy, and the agent's actual experience using these tools.

---

## 1. Competition Design: All 8 Agents Managing the Same Team

The plan says "all 8 agents manage the same team." This is the simplest starting point, but it creates a fundamental design tension: **every agent starts from an identical position, but divergent decisions create different worlds** — not just different league positions, but different transfer markets, different AI team behaviours, different injury outcomes. The league tables in the 8 instances will not be comparable.

### The comparability problem

If Agent A buys a star striker, that striker leaves the transfer market in Agent A's world but remains available in Agents B–H's worlds. AI teams in Agent A's world now have different rosters and thus different match outcomes. After just a few transfers, the 8 worlds have diverged enough that "1st in Agent A's league" and "1st in Agent B's league" measure different things.

**Recommendation**: This is acceptable for a fun competition, but the plan should explicitly acknowledge it. Two alternatives worth considering for future iterations:
- **Shared world**: All 8 agents play in the same league (8 different teams in one world). This makes league positions directly comparable, but requires a centralised world server or turn-based advancement protocol — significantly more complex.
- **Normalised scoring**: Instead of raw league position, score agents by how much they *improve* their team relative to the starting position. For example, `(starting_reputation - ending_reputation)` or points-per-match relative to the team's expected performance. This normalises across divergent worlds.

### Starting with the same team also means same budget, same squad

This is actually a feature for fairness — every agent gets the same resources. But consider: if the team is weak (e.g., a relegation candidate), the competition tests who can survive. If the team is strong, it tests who can dominate. The plan should specify what kind of team agents will manage, as this shapes what skills the competition measures.

**Recommendation**: Add a brief section on team selection criteria for the competition. A mid-table team is probably the most interesting — it tests both survival and ambition.

---

## 2. The Agent's Actual Experience: Tool Usability

I've read every existing Tauri command and the domain types they return. The MCP tool descriptions in the plan are good, but several have implicit dependencies that agents will struggle with unless the descriptions are very explicit.

### ID discovery is the biggest usability problem

Most tools take entity IDs (`player_id`, `team_id`, `scout_id`, `staff_id`, `offer_id`). But how does an agent discover these IDs? The plan has `squad_get` returning a table with player names, and `info_player_profile` taking a player ID — but the `squad_get` response must include player IDs for the agent to do anything useful.

Looking at the `squad_get` example format, it shows `# | Name | Pos | Age | OVR | ...` — the `#` column appears to be a shirt number, not a player ID. **The agent needs the player ID to call `squad_set_starting_xi` or `training_set_player_focus`.**

**Recommendation**: Every tabular response that lists entities must include their IDs. The `squad_get` table should have a `ID` column (or at least include player IDs alongside names). This applies to:
- `squad_get` → player IDs
- `staff_get` → staff IDs (especially scout IDs for `scout_send`)
- `transfer_market_browse` → player IDs and their current team IDs
- `inbox_get_messages` → message IDs and action IDs
- `info_standings` → team IDs
- `info_fixtures` → fixture IDs
- `jobs_available` → team IDs

This sounds obvious but is easy to overlook when writing markdown formatters for human readability. Agents cannot click on a player name to navigate to their profile — they need the raw ID.

### Nested workflows require guidance in tool descriptions

Several game actions are multi-step workflows. An agent encountering them for the first time may not know the sequence:

| Workflow | Steps | What can go wrong |
|---|---|---|
| **Contract renewal** | `contract_preview_renewal` → `contract_propose_renewal` → (maybe repeat with different wage) → agent or player accepts | Agent may not know what wage to offer; renewal can stall or be blocked |
| **Transfer bid** | `transfer_market_browse` → `transfer_preview_bid` → `transfer_make_bid` → (if counter-offer) `transfer_make_bid` again with higher fee | Negotiation has rounds, feedback mood/tension, suggested fees — agent needs to understand the multi-round protocol |
| **Scouting** | `staff_get` (find scout ID) → `scout_send` → wait N days → `scout_get_reports` | Agent must wait (advance time) before report is ready |
| **Responding to offer** | `inbox_get_messages` → find offer → `transfer_respond_to_offer` or `transfer_counter_offer` | Agent needs to correlate message with the offer, extract player_id and offer_id |
| **Season end** | `season_check_complete` → `season_advance` → (maybe `game_is_finished`) | Agent may not know when to check |

**Recommendation**: Tool descriptions should include brief workflow guidance. For example, `transfer_make_bid`'s description could say: "Initiates a transfer negotiation. If the selling club counters, call this tool again with a higher fee. Check `suggested_fee` in the response for a hint. Use `transfer_preview_bid` first to check financial impact."

Similarly, `scout_send` should say: "Takes several in-game days. Advance time with `time_advance` and then check `scout_get_reports`."

### Transfer market browse needs filtering

The plan's `transfer_market_browse` tool returns "available players on transfer market." In a world with ~352 players (16 teams × 22 players), even a subset could be large. An agent calling this with no filters gets a wall of text.

**Recommendation**: `transfer_market_browse` should accept optional filters: `position`, `max_price`, `min_ovr`, `sort_by` (value/ovr/age). This isn't just convenience — it's necessary to keep responses within token limits.

---

## 3. Command Layer Subtleties: What the MCP Tools Must Reproduce

After reading every command file (`game.rs`, `time.rs`, `squad.rs`, `transfers.rs`, `contracts.rs`, `messages.rs`, `finances.rs`, `club.rs`, `staff.rs`, `jobs.rs`, `season.rs`, `live_match.rs`, `stats/`), here are the subtle behaviours the MCP tools must correctly replicate:

### 3.1 `set_formation` reassigns player positions

The existing `set_formation` command doesn't just change the team's formation string — it also **reassigns outfield player positions** based on defensive ability sorting. Players are sorted by `defending + tackling + strength` and slotted into DEF/MID/FWD according to the formation's slot counts.

This is a significant side effect. An agent calling `squad_set_formation("4-3-3")` may find that several players have had their `position` field changed. The MCP tool response should make this visible — perhaps by returning the updated squad overview.

**Recommendation**: `squad_set_formation`'s response should include a brief summary of position reassignments (e.g., "3 players reassigned: C. Midfielder → Defender, ..."). Without this, agents won't understand why their squad suddenly looks different.

### 3.2 Transfer negotiations are multi-round with state

The `make_transfer_bid` implementation in `ofm_core::transfers` supports multi-round negotiations with `TransferNegotiationDecision::{Accepted, Rejected, CounterOffer}` and `NegotiationFeedback` with mood, tension, patience, and round number. The existing Tauri command returns all of this.

The plan's `transfer_make_bid` tool needs to return enough information for the agent to decide whether to bid again:
- `decision` (Accepted/Rejected/CounterOffer)
- `suggested_fee` (the counter-offer hint)
- `feedback.mood`, `feedback.tension`, `feedback.patience`, `feedback.round`
- Whether the negotiation is terminal (`is_terminal`)

**Recommendation**: The plan's response format examples should include a `transfer_make_bid` example showing the negotiation feedback. The markdown format for this is non-trivial — how do you render mood/tension/patience in a way an LLM understands? Something like:

```markdown
## Transfer Bid Result

**Player**: P. Two | **Bid**: €1,050,000
**Decision**: Counter-offer | **Suggested fee**: €950,000
**Negotiation**: Round 2 | Mood: Firm | Tension: 40/100 | Patience: 60/100
**Status**: Negotiation continues — make another bid or walk away.
```

### 3.3 Contract renewal has a cooling-off mechanic

Looking at `contracts.rs`, when a renewal is rejected or stalls, there's a `manager_blocked_until` date — the agent can't propose again until that date passes. The `RenewalSessionStatus` can be `Blocked`. The MCP tool must communicate this clearly, or an agent will keep calling `contract_propose_renewal` and getting errors.

**Recommendation**: `contract_propose_renewal` should include in its response: if the session is blocked, the date when the agent can try again. E.g., "Contract negotiation blocked. Try again after 2026-09-15."

### 3.4 `resolve_message_action` dispatches to multiple subsystems

The `resolve_message_action` command tries, in order:
1. `player_events::apply_player_response` (player conversation choices)
2. `random_events::apply_event_response` (random event choices)
3. `job_offers::apply_job_offer_response` (job offer accept/decline)
4. `scouting::apply_youth_recruitment_response` (youth scouting prospect decisions)
5. Falls back to just marking the action as resolved

This is a single Tauri command that branches into 4+ different subsystems based on the message content. The MCP tool's description needs to make this clear, or agents won't understand why the same tool call produces vastly different effects depending on the message.

**Recommendation**: The `inbox_resolve_action` description should explain: "Resolves a message action. Effects depend on the message type: job offers (accept/decline), player conversations, random events, youth scouting prospects. Read the message's actions list to determine available options and their IDs."

### 3.5 `advance_to_next_season` can fire the manager

The `season.rs` command calls `check_manager_firing` after processing end-of-season objectives. If the manager is fired, `game.manager.team_id` becomes `None`, and the response includes `action: "fired"`. An MCP agent that calls `season_advance` and gets fired is now an unemployed manager — it needs to know about `jobs_available` and `jobs_apply`.

**Recommendation**: `season_advance`'s response must clearly indicate if the manager was fired, and suggest the next action. The response format example should include a fired-manager scenario.

### 3.6 Some Tauri commands are `async`, some are not

Several commands use `async fn` (e.g., `propose_renewal`, `delegate_renewals`, `get_finance_snapshot`, `request_board_support`) while most are synchronous. This doesn't matter for the MCP layer directly, but it's worth noting because the MCP handlers will call the same internal functions. Since the MCP handlers run in a tokio runtime, calling the `*_internal` functions (which are synchronous) from an async context is fine — just be aware that some internal functions may do I/O (save to disk) and could block the tokio task briefly.

---

## 4. MCP Protocol & Transport Concerns

### 4.1 SSE connection lifecycle

The plan specifies `GET /sse` for the event stream and `POST /messages` for client messages. The MCP SSE transport requires:
- A unique endpoint URL per client (the server sends an `endpoint` event on connection)
- Proper CORS headers if agents run from browsers
- Connection keep-alive (SSE can time out on idle)

The plan doesn't discuss what happens when an agent disconnects and reconnects. Does the game state persist? Can the agent resume mid-session?

**Recommendation**: The game state lives in `StateManager`, independent of any MCP connection. An agent that disconnects and reconnects should be able to continue. Document this explicitly. Also document that SSE connections should use `Cache-Control: no-cache` and a reasonable keep-alive interval.

### 4.2 Concurrent MCP connections

Can multiple agents (or multiple connections from one agent) connect to the same MCP server simultaneously? The plan doesn't discuss this. With `std::sync::Mutex` on the game state, two concurrent `time_advance` calls would serialise — but they might interleave in unexpected ways (Agent A's squad change between Agent B's time advance clone and write-back).

**Recommendation**: For the competition, restrict to one MCP connection per server. For sandbox mode, document that concurrent connections are not supported and may cause inconsistent state.

### 4.3 Tool schema complexity and agent context windows

The plan lists ~55 tools. Each tool has a name, description, and input schema. When an MCP client connects, it receives all of these via `tools/list`. Even with concise descriptions, 55 tools × ~100 tokens each = ~5,500 tokens just for the tool definitions. This is manageable for modern LLMs, but the tool descriptions should be kept concise.

**Recommendation**: Keep each tool description under 2 sentences. Put detailed usage guidance in tool *responses* (error messages, workflow hints), not in descriptions. This keeps the `tools/list` payload small.

---

## 5. Security & Fairness

### 5.1 Agents can inspect AI team data

The `info_game_state` tool returns the *entire* `Game` struct, which includes all teams, all players, all AI manager decisions, and all internal state. In a competition, this gives an agent perfect information about every other team's finances, squad, and contract situations — information a human manager would never have.

**Recommendation**: In `competition` mode, `info_game_state` should be disabled (added to the competition-mode denylist). The existing `info_*` tools already provide appropriate information channels — `info_standings`, `info_player_profile`, `info_team_profile` — which only show what a real manager could know. But even these need review: can an agent call `info_player_profile` for a player on another team and see their exact attributes? In real football management, you'd need a scout report for that level of detail.

Consider a visibility model:
- **Own team players**: Full attributes, contract details, morale
- **Other team players**: Only OVR, position, age, form — unless a scout report exists
- **AI team finances**: Only what's public (league position, recent transfers)

This would make scouting actually important for agents, rather than just a nice-to-have.

### 5.2 The `--mcp-auto-start` bootstrap creates a manager identity

When auto-starting, the process creates a manager. What name/nationality does it use? If all 8 agents get the same manager name, that's confusing in logs. If they get different ones, how are those names assigned?

**Recommendation**: `--mcp-auto-start` should accept manager identity parameters, or default to `Agent N` / nationality of the team's country. E.g., `--mcp-auto-start world.json,team_abc123 --manager-name "Agent 1"`.

### 5.3 RNG determinism across instances

All 8 instances start from the same world JSON, but each will produce different match results because the RNG is seeded differently (or from system entropy). This is fine for a fun competition, but it means luck plays a role — one agent might face more injuries, another might get lucky bounce-back wins.

**Recommendation**: For strict fairness, consider making RNG seeds deterministic based on instance number (seed = base_seed + instance_index). This makes the competition reproducible. It's optional but worth mentioning as a future `--rng-seed` CLI arg.

---

## 6. Testing Strategy for the MCP Server

The plan's phases mention testing only in Phase 5 ("Test multi-instance isolation"). But the MCP server layer needs its own testing strategy.

### 6.1 Unit testing the tool handlers

Every existing Tauri command has `*_internal` functions that take `&StateManager` and can be tested without a Tauri runtime (the existing tests in `commands/*.rs` do exactly this). The MCP tool handlers should follow the same pattern: thin wrappers that call `*_internal` functions and format the result.

This means the MCP tool handlers are trivially testable — you create a `StateManager`, set up a `Game`, call the handler, and check the formatted markdown output.

**Recommendation**: Add a test strategy section. Phase 1 should include: "Each MCP tool handler has a corresponding unit test that exercises it against a `StateManager` with a pre-built `Game`, following the same pattern as `commands/*_internal` tests."

### 6.2 Integration testing the SSE transport

Phase 1 says "Verify: agent connects via SSE, calls ping, gets response." This is the right end-to-end gate, but it requires running a Tauri process, which is heavyweight for CI.

**Recommendation**: Consider testing the axum SSE server in isolation (without the full Tauri app) by constructing a `StateManager` manually and passing it to the router. This allows CI-friendly integration tests.

### 6.3 Testing tool restriction modes

The `--mcp-mode competition` and `--mcp-disable-tools` flags affect which tools are registered. This needs testing: verify that disabled tools don't appear in `tools/list` and that calling them returns a proper MCP error.

---

## 7. The Formatting Module: A Bigger Undertaking Than It Appears

The plan puts formatting in Phase 2 ("Formatting module for markdown output") and Phase 5 ("Rich markdown formatting for all tool responses"). Having read every domain type — `Player` (50+ fields), `Team` (30+ fields), `Game` (the root), `League`, `Fixture`, `StandingEntry`, `InboxMessage` (with nested `MessageAction` and `MessageContext`), `NewsArticle`, `Manager`, `Staff`, `BoardObjective`, `ScoutingAssignment`, `SeasonContext`, `TransferOffer`, `NegotiationFeedback`, `PlayerSeasonStats`, `CareerEntry`, `TeamFinanceSnapshot`, `Facilities`, `Sponsorship`, `FinancialTransaction` — the formatting module is substantial.

### Every tool response needs a formatter

There are ~55 tools. Each returns a different slice of the game state. Each needs a function that takes the domain types and produces readable markdown. Rough estimate: ~55 formatter functions, each 20–50 lines = 1,100–2,750 lines of formatting code.

The example formats in the plan (`info_standings`, `info_player_profile`, `info_game_summary`, `squad_get`) are well-designed, but they only cover 4 of 55 tools. The remaining tools have more complex output types:

| Tool | Complex output type |
|---|---|
| `transfer_make_bid` | `TransferNegotiationOutcome` (decision + feedback + suggested_fee + is_terminal) |
| `contract_propose_renewal` | `RenewalCommandResponse` (outcome + game + suggested_wage + years + session_status + is_terminal + cooled_off + feedback) |
| `delegate_renewals` | `DelegatedRenewalCommandResponse` (game + `DelegatedRenewalReport` with per-player cases) |
| `contract_terminate` | `ContractTerminationCommandResponse` (game + severance_cost + `SquadSafetyReport`) |
| `get_finance_snapshot` | `FinanceSnapshotCommandResponse` (snapshot + `FinanceActionPreviews`) |
| `inbox_get_messages` | `Vec<InboxMessage>` — potentially dozens of messages with nested actions and context |
| `inbox_resolve_action` | JSON with game + effect + i18n data |
| `season_advance` | JSON with game + `EndOfSeasonSummary` |
| `get_season_awards` | `SeasonAwards` (7 award categories, each with top-3 entries) |

**Recommendation**: 
- Start formatting in Phase 2 but acknowledge it will be iterative. Early phases can use minimal formatting (e.g., debug-style `{:?}` output or simple JSON) and upgrade to rich markdown in Phase 5.
- Consider a `format_entity()` trait or convention that each domain type implements, rather than 55 standalone functions. This reduces duplication (e.g., player formatting is shared between `squad_get`, `info_player_profile`, `transfer_market_browse`).

---

## 8. The `AppHandle` Dependency

Several existing Tauri commands take `tauri::AppHandle` as a parameter:
- `write_temp_database` (in `world.rs`)
- `export_world_database` (in `world.rs`)
- `get_manager_profiles`, `save_manager_profile`, etc. (in `profiles.rs`)
- The `setup` hook uses `app.path()` to resolve data directories

The MCP server runs inside the Tauri app and will need access to `AppHandle` for:
- `app.emit("game-state-changed", ())` — the GUI auto-refresh
- Resolving paths (data dir, saves dir)
- Accessing managed state (`StateManager`, `SaveManagerState`)

**Recommendation**: The MCP server bootstrap should receive an `AppHandle` clone during Tauri setup. Store it in the axum state alongside `Arc<StateManager>` and `Arc<SaveManagerState>`. This is straightforward but should be explicit in the architecture.

---

## 9. `--no-gui` Implementation Reality

The plan mentions headless mode ("the Tauri window is created but immediately hidden, or the webview is skipped entirely if Tauri v2 allows"). In Tauri v2, the webview/window is created by the frontend process — you can't easily skip it without modifying the Tauri builder. Options:

1. **Create the window but hide it**: `window.hide()` after creation. The webview process still runs and consumes RAM, just less (no rendering).
2. **Don't create the window at all**: Tauri v2 requires at least one window in the default configuration. You'd need to create a minimal invisible window.
3. **Separate binary**: A `bin/mcp_headless` that doesn't depend on `tauri` at all, just builds `StateManager` + `SaveManager` + axum server. This is the cleanest but requires duplicating the setup logic.

**Recommendation**: Option 1 (hide the window) is the simplest and probably saves ~50MB RAM (no GPU compositing). Option 3 is the most resource-efficient but requires extracting the game initialization logic into a shared module. Start with Option 1 and move to Option 3 if RAM is a bottleneck.

---

## 10. The `--mcp-auto-start` Bootstrap Mechanism

This is one of the more complex additions. Currently, `start_new_game` is a Tauri command — it's called from the frontend after the user fills in a form. The `--mcp-auto-start` flag needs to replicate this flow programmatically, without a frontend, before the MCP server starts accepting connections.

Looking at `commands/game.rs`, `start_new_game` does a LOT:
1. Parse and validate manager name, DOB, nationality
2. Generate or load world data
3. Build a `Game` from the world data (with startup options for year, phase, history depth)
4. Set the game in `StateManager`
5. Set the stats state in `StateManager`
6. The user then calls `select_team` separately

The `--mcp-auto-start` bootstrap needs to do all of this. It also needs the `SaveManager` to be initialised first (since it's set up in the Tauri setup hook).

**Recommendation**: Implement `--mcp-auto-start` as a function called during the Tauri `setup` hook, *after* `SaveManager` is initialised but *before* the MCP server starts listening. This function should:
1. Create a `Manager` with a default identity (or from CLI args)
2. Load the world from the specified JSON file
3. Build the `Game` and set it in `StateManager`
4. Call the equivalent of `select_team` to assign the manager to the specified team
5. Optionally do an initial save
6. Then start the MCP server

If any of these steps fail, the process should exit with a clear error message.

---

## 11. Orchestration Phase: Missing Details

Phase 6 is left for later, but the plan's competition design depends on it. A few things to consider now:

### Process management

Starting 8 Tauri processes, each with its own port and saves directory, is a shell script. But managing their lifecycles (starting, waiting for MCP readiness, detecting completion, stopping) requires:
- A readiness signal: how does the orchestrator know the MCP server is accepting connections? (Check if the port is listening?)
- A completion signal: `game_is_finished` is the right tool, but the orchestrator needs to poll it.
- Crash handling: what if one agent's process crashes? Does the competition continue with 7?

### Agent process management

Each agent is a separate process (likely a Python or Node script connecting via MCP). The orchestrator needs to:
- Start each agent process pointing at the correct MCP port
- Monitor for agent crashes or hangs
- Set a time limit (wall-clock) for the competition

**Recommendation**: Add a brief orchestration design section (even if deferred). At minimum: a `start_competition.sh` script that starts processes, waits for readiness, launches agents, and collects results.

---

## 12. Minor Observations

1. **`transfer_market_browse` has no Tauri command equivalent.** The existing frontend browses the transfer market by filtering `game.players` on the client side — there's no dedicated backend command. The MCP tool will need a new backend function that filters and sorts players by transfer/loan status and formats the results. This isn't just wrapping an existing command; it's new logic.

2. **`scout_get_reports` has no Tauri command equivalent either.** Scout reports are currently generated as `InboxMessage` objects with a `scout_report` context. The frontend reads messages. The MCP tool will need to extract and format scout reports from the message list — or a new backend function that returns structured scout report data.

3. **`training_set_groups` takes `Vec<TrainingGroup>` as input.** `TrainingGroup` is a complex domain type with player lists and focus overrides. The MCP tool needs to define how agents specify this — probably as a JSON object parameter.

4. **`squad_set_match_roles` takes `MatchRoles` as input.** This is another complex type (captain, vice_captain, penalty_taker, etc.). The MCP tool should accept these as separate named parameters rather than requiring agents to construct a JSON object.

5. **`info_game_state` should be removed or severely limited.** Even with the warning in the plan, agents will call it because it's the easiest way to get everything. In competition mode, disable it. In sandbox mode, truncate it or paginate it.

6. **The `round_summary` data from `time_advance` needs formatting.** When a match day occurs during `time_advance`, the delegate path returns a `RoundSummaryDto` with all match results for that day. This is important information for the agent — it tells them the scores of all matches. The plan says results are "returned in the `time_advance` response" but doesn't show a format example.

7. **`info_match_preview` needs data that may not exist.** The plan says it shows "likely lineups" — but the current codebase doesn't have a function that predicts opponent lineups. This tool will need new prediction logic, or it should be simplified to just show the opponent's recent form and squad (without predicting the XI).

---

## Summary of Key Recommendations

| # | Recommendation | Priority |
|---|---|---|
| 1 | Include entity IDs in all tabular tool responses | Critical — agents cannot function without them |
| 2 | Add workflow hints to multi-step tool descriptions | High — reduces agent confusion |
| 3 | Add filters to `transfer_market_browse` | High — token limit management |
| 4 | Restrict information visibility in competition mode | Medium — fairness |
| 5 | Implement `--mcp-auto-start` in Tauri setup hook | High — required for competition flow |
| 6 | Start with minimal formatting, upgrade iteratively | Medium — practical phasing |
| 7 | Test tool handlers with `StateManager` + pre-built `Game` | High — essential for quality |
| 8 | Document that `set_formation` reassigns positions | Medium — agent surprise prevention |
| 9 | Format negotiation feedback in transfer/contract responses | High — agents need this to make decisions |
| 10 | Consider normalised scoring for competition fairness | Low — future improvement |
| 11 | Add `--rng-seed` for reproducibility | Low — future improvement |
| 12 | Briefly design the orchestration phase | Low — deferred but shouldn't be forgotten |
