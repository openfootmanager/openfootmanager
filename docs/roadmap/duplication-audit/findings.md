# Single-source-of-truth audit — OpenFoot Manager

Read-only audit, 22 Sep 2026. Written to disk 24 Sep after the original report was lost with a
wiped session — see the note at the bottom.

**What prompted it.** Four bugs in three days turned out to be one shape: a backend rule
reimplemented somewhere else, then drifting from it. Three of the four were causing visible,
player-facing breakage.

| # | The duplicated rule | What the drift did |
|---|---|---|
| 1 | Promotion/relegation pyramid, re-implemented in `src/lib/pyramid.ts` | Drew a 4-club relegation zone in Argentina's Apertura and a 4-club promotion zone in the Clausura — two halves of the *same* division, same 20 clubs, where nobody can ever move |
| 2 | "The season is over", re-decided in `src/pages/Dashboard.tsx` | Asked `competitions[0]` (always Argentina) instead of the player's league, so the end-of-season screen never appeared and the rollover was unreachable (#562) |
| 3 | Player match ratings, invented in `PostMatchHelpers.tsx` from a 6.0 baseline | The engine has no rating field at all |
| 4 | `snap_player`, written twice in `engine/` | Both copies carried the same `players[0]` panic; fixing one would have left the other |

---

## Re-verified against develop, 24 Sep 2026

These three were checked again today and still hold.

### A1 — MCP season tools: an agent cannot finish a season at all
`src-tauri/src/mcp_server/tools_impl/season.rs:9`

`season_check_complete` counts unfinished fixtures on the legacy `game.league` mirror. It does
not call `ofm_core::end_of_season::is_season_complete`, and it does not filter to
`counts_for_league_standings()`, so cup and friendly fixtures keep the season "in progress".

Worse, `season_advance` (same file) only calls `advance_time_with_mode(state, "delegate")` — one
day — and tells the agent to "Continue advancing to reach next season". The only non-test caller
of `process_end_of_season` is the Tauri command at `src-tauri/src/commands/season.rs:27`. There is
no MCP path to the rollover, so an agent is stuck at season end exactly as the reported player
was, and promotion never runs for them.

`docs/MCP_SERVER.md` describes `season_advance` as "Advance through the off-season". It does not.

**Source of truth:** `is_season_complete`, and `advance_to_next_season`'s body factored into an
`_internal` that both the command and the MCP tool call — which is what `src-tauri/CLAUDE.md` §4
already requires.

### A2 — Round summaries diff against the wrong table
`src-tauri/src/application/time_advancement.rs:27`, `crates/ofm_core/src/game.rs:258`

`round_context_for_today` calls `game.primary_competition()`, which is literally
`competitions.first().or(self.league.as_ref())`. Competitions are ordered by country code, so for
any career outside the first country that is a foreign league on another calendar. The matchday
number and the "previous standings" used for position-change arrows describe that league while the
fixtures being diffed are the user's. On a day the first competition has no fixture, the function
returns `None` and no round summary is produced at all.

`crates/ofm_core/src/inbox.rs:276` already carries a comment warning that `primary_competition()`
is just `competitions.first()` — somebody noticed and worked around it locally.

**Source of truth:** `Game::user_competition()` (`game.rs:250`), which already exists.
`primary_competition()` should keep only its persistence-metadata caller
(`crates/db/src/game_persistence.rs:125`), where "the first competition" is genuinely what is meant.

### A3 — `engine` has two match engines, and they have drifted
Batch: `crates/engine/src/engine/{mod.rs,resolution.rs,fouls.rs}` ·
Live: `crates/engine/src/live_match/{zone_resolution.rs,helpers.rs}`

Not the deliberate `engine`/`domain` type mirror — two full simulation loops.
`resolve_action`, `resolve_buildup`, `resolve_midfield`, `resolve_attacking_third`, `resolve_shot`,
`maybe_foul`, `maybe_card`, `effective_press`, `effective_midfield` and `snap_player` all exist twice.

Verified divergences: the live loop applies `condition_adjusted_skill` per player
(`zone_resolution.rs:100`) and the batch loop does not (`resolution.rs:81`, which only depletes a
team-level `home_condition`); the live loop emits `EventDetail::Foul { severity }` and the batch
loop emits none.

AI-vs-AI matchdays use the batch loop (`crates/ofm_core/src/turn/mod.rs:516`), and `sim-bench`
benchmarks only the batch loop. The player's own match uses the live loop. So balance is tuned
against an engine the player never sees, and AI results are produced by different physics from the
player's.

---

## Verified 22 Sep, not re-checked since

Still worth treating as real; confirm before acting.

- **`turn/mod.rs` bridge plays injured players.** `build_engine_team` (`turn/mod.rs:371`) passes
  every player with a matching `team_id` — no `injury.is_none()`, no `squad_role`, no XI — while
  `live_match_manager/team_builder.rs:416` filters to the starting XI and uncured players. The two
  also disagree on position: the turn bridge uses `to_group_position()`, the team builder uses the
  deployed slot. `domain_to_engine_role` and `domain_to_engine_tactics` live only in
  `team_builder.rs`, contradicting `src-tauri/CLAUDE.md` §1 ("`ofm_core/turn/` … the only place
  that conversion is allowed to live").
- **`src/lib/finance.ts` recomputes the finance snapshot and gets a shorter runway.** It omits
  matchday income and the sponsorship bonus that `crates/ofm_core/src/finances.rs:274` includes, so
  `dashboardHelpers.ts` can fire a `finance_crisis` alert for a club the backend rates "watch", and
  the Finances tab visibly changes numbers once the backend snapshot loads.
- **Player profile age is frozen.** `PlayerProfile.helpers.ts:67` defaults `asOfDate` to
  `"2026-07-01"`; `src/lib/valueFormatting.ts:44` already fixed this exact bug by using the game
  clock. After one rollover the profile and the squad table disagree.
- **Seven age helpers, three algorithms.** Day-of-year (`aging.rs:10`, `contracts/helpers.rs:149`,
  `season_awards.rs:61`), `(month, day)` (`commands/squad.rs:19`, MCP `helpers.rs:59`, TS
  `valueFormatting.ts:55`), and year-only (`player_rating.rs:92`) — the last drives
  `generate_potential` bands and the wonderkid gate, so a December birth is treated a year older
  all year.
- **Backend-key locale coverage omits Indonesian.** `src/utils/backendI18n.localeCoverage.test.ts:19`
  lists eleven locales; `src/i18n/localeCoverage.test.ts:23` lists twelve. Neither reads
  `SUPPORTED_LANGUAGES`. A backend key missing from `id.json` passes CI.
- **`turn/news.rs:94` sorts standings by points then goal difference only**, where
  `League::sorted_standings` (`domain/src/league.rs:478`) is points → GD → goals for. The standings
  news article can name a different leader on a tie. Six TypeScript copies of the same comparator
  exist and are currently in sync.
- **Press conference is implemented twice with different arithmetic.**
  `src/commands/live_match.rs:154` uses randomised morale deltas and translation-key errors; the
  MCP copy at `src/mcp_server/tools_impl/live_match.rs:249` uses fixed deltas and English error
  strings. "Demanding" is mildly positive for agents and can be negative for humans.
- **`docs/ARCHITECTURE.md:178` documents 29 commands**; `src-tauri/src/lib.rs:192` registers 123.
  `choose_team` is documented and not registered.
- **`game.league` is a second copy of the user's competition**, kept in sync by discipline: one
  canonical writer (`sync_legacy_league`) plus five ad-hoc `game.league = Some(...)` assignments.

## Checked and genuinely in sync — do not re-investigate

Formation slot layout (`SquadTab.helpers.ts:272` vs `player_rating.rs:101`); the role table
(`src/lib/playerRoles.ts:9` vs `commands/squad.rs:483`); transfer-window arithmetic; marketing
cooldown and wage/runway thresholds; facility cost; youth-academy age; every hand-mirrored enum in
`src/store/types.ts`; the MCP tool catalog against `docs/MCP_SERVER.md` (89 = 89);
`docs/GAME_SYSTEMS.md` fitness tiers and training multipliers. `ofm-cli` links `ofm_core`, so
package validation is one implementation by construction.

---

## Why this file exists

The original report was a subagent message in a session that has since been wiped, and it was
never written to disk — so it evaporated and had to be reconstructed from conversation context two
days later. Working notes for anything spanning more than one session belong here, not in the
scratchpad.
