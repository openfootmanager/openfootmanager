# Design: Stop shipping the whole world over Tauri IPC

Status: **Phase 1 complete. Phase 2 additive work complete (SessionState defined, get_session_state added). Return-type flip deferred to Phase 3 gate.**
Author: Sisyphus · Scope: backend command contracts + frontend store/tabs · Risk class: invasive (gated)

---

## 1. Problem & measured evidence

Almost every Tauri command returns the **entire `Game`** (`get_active_game` plus 27 mutating
commands via `mutate_active_game`; 41 `Result<Game>` sites total). The frontend `gameStore`
replaces its whole `gameState` on every command and re-normalizes all arrays.

Measured on a standard world (440 clubs / 9,680 players / 1,772 staff, **with news=0 and
messages=0 — i.e. best case**):

| Per-command marshalling step | Cost |
|---|---|
| `Game::clone()` (in `mutate_active_game`) | **18.7 ms** |
| serde JSON serialize (Rust side) | **62.8 ms** |
| **JSON payload size** | **17.6 MB** |
| JSON deserialize (JS `invoke` parse) | **82.4 ms** |
| **Total marshalling overhead per command** | **~145 ms+** |

This overhead is paid on **every** Continue/Skip and every squad/tactics/transfer action,
**on top of** the actual logic, and it **grows** across a season as `news`, `messages`,
`world_history`, player `career`/`stats`, and `transfer_log` accumulate (the 17.6 MB above
has none of those yet). It also forces a full frontend re-render because the whole
`gameState` object is replaced.

For comparison, the per-day *logic* costs already optimized this session were training
10 ms and finances 3.7 ms. **The IPC payload is roughly an order of magnitude larger than
any logic hotspot** and is the single biggest contributor to perceived sluggishness.

## 2. Root cause

One monolithic read model. `GameStateData` (frontend) is a near-1:1 mirror of the backend
`Game`: `clock, manager, managers, teams[], players[], staff[], messages[], news[],
competitions[], national_teams[], world_history, scouting_assignments[], board_objectives[],
…`. Every command answers "what is the entire world now?" when the caller only needed
"advance a day and tell me what changed for *me*" or "show me *this* team's squad".

The user looks at a handful of screens at a time and almost never needs all 9,680 players
serialized on a Continue click.

## 3. Goals & constraints

- **Goal:** hot commands (advance/skip/squad/tactics/transfer) ship KB, not MB; no full
  re-render fan-out; cost no longer grows unbounded with save age.
- **Constraint (yours):** targeted, **incremental, low-risk**. Each phase must be
  independently shippable, behavior-preserving, and TDD-locked.
- **Constraint:** persistence (save/load) is independent of IPC and is **out of scope** here.
- **Non-goal:** changing the match engine or game logic.

## 4. Options considered

| Option | Idea | Win | Risk | Verdict |
|---|---|---|---|---|
| **A. Scoped read model** | Small always-on "session" payload + on-demand slice queries for heavy collections | Largest (root-cause) | Medium, but **splittable into low-risk phases** | **Recommended** |
| B. Delta/patch updates | Backend tracks dirty entities; commands return only changes | Best steady-state | High (change-tracking + frontend merge) | Defer (possible Phase 4) |
| C. Cheaper codec | Return binary (postcard/bincode) instead of JSON via Tauri raw response | Medium (smaller/faster) but **still ships whole world + full re-render** | Low–medium | Band-aid; doesn't fix root |
| D. Lazy collections | One big fetch on load; commands return only a mutation result; tabs re-fetch slices | Same as A, framed differently | Medium | Folded into A |

## 5. Recommended approach — Option A, phased

Split the world into:

1. **`SessionState` (always returned by hot commands, KB-sized):** `clock`, `manager`,
   the user's `team`, the user's competition summary (standings + the user's upcoming/recent
   fixtures only), `season_context`, `board_objectives`, unread `news`/`messages` **counts**,
   and the existing advance **recap**. This is what the Home/header/Continue flow needs.
2. **On-demand slice queries (fetched only when a tab opens):**
   `get_team_squad(team_id)`, `get_players_page(filter,page)`,
   `get_world_teams_page(...)`, `get_competition_view(competition_id)`,
   `get_news_page(page)`, `get_messages_page(page)`, `get_manager_directory()`.

This matches the backend's existing `active_scope` concept and the tab-based UI.

### Phased rollout (each phase independently shippable, committed atomically)

- **Phase 1 — Add slice query commands ✅ DONE.**
  New slice commands added: `get_players_page`, `get_teams_directory`, `get_schedule`,
  `get_news_feed`, `get_messages_page`, `get_competitions_view`.
  Players, Teams, Tournaments, News tabs now self-fetch from their slice on open.
  InboxTab backend command exists but tab still reads from `gameState.messages` (Phase 3).
  The giant payload still flows but the tabs no longer depend on it for initial render.
- **Phase 2 — Slim the hot commands (where the 17.6 MB→KB win lands). ✅ Additive done.**
  `SessionState` struct defined (`slices/session.rs`): clock, manager, user's team,
  season_context, board_objectives, scouting queues, unread counts, `UserCompetitionSummary`
  (standings with resolved names + next 3 / last 2 fixtures for the user's team).
  `project_session(game)` is the canonical projection; 6 parity tests lock the contract.
  `get_session_state` Tauri command + `sessionService.ts` added.
  **Blocked — return-type flip pending:** 35+ components still read `gameState.players/teams/
  competitions/staff/news/messages` directly. Flipping `advance_time*` / `mutate_active_game`
  return types now would clear those fields and break every non-migrated tab. The flip is
  gated on Phase 3 (store split) completing all remaining migrations.
- **Phase 3 — Split the frontend store.**
  Replace monolithic `gameState` with `sessionState` + per-slice caches; eliminates
  whole-object replacement and broad re-render fan-out. InboxTab migration from
  `gameState.messages` to `get_messages_page` slice happens here.
- **Phase 4 (optional) — Deltas** for frequently-changing slices (standings after a matchday)
  to avoid re-fetching. Only if profiling still warrants it.

## 6. TDD & verification plan

- **Parity (characterization):** each slice command must return data **identical** to the
  corresponding projection of today's full `Game`. Lock with tests that build a world and
  assert `slice == project(full_game)` before any command is slimmed.
- **Frontend safety net:** the existing per-tab tests (TournamentsTab, TeamsListTab,
  ManagerTab, ScheduleTab, Inbox, etc.) must stay green as each tab migrates to slice fetches.
- **Perf gate (new test/probe):** assert hot-command payload drops from ~17.6 MB to
  < ~100 KB and re-measure the advance round-trip; keep a probe (ignored by default) for
  before/after.
- **Full suite green** after every phase; atomic Conventional Commits per phase.

## 7. Risks & mitigations

- **Contract drift** between a slice and the full model → single shared serialization
  projection per slice + parity tests.
- **Cross-slice reads** (e.g. a player profile needs the team name) → denormalize the few
  needed display fields into the slice, or a tiny id→name lookup cache.
- **Two sources of truth** during migration → Phase 1 is additive; the full model stays
  until Phase 2 flips the hot commands, so there's always a working path.
- **Save/load** untouched (separate layer).

## 8. Decisions (recorded)

1. **Option A approved** (scoped read model, phased).
2. **Phase 1 tab migration order**: Tournaments, News, Inbox (user decision).
   - `get_competitions_view`, `get_news_feed`, `get_messages_page` added.
   - TournamentsTab, NewsTab migrated to self-fetch; InboxTab backend done, tab migration deferred to Phase 3.
3. **`get_active_game` slimmed in Phase 2** (user decision: "Slim it too.").
