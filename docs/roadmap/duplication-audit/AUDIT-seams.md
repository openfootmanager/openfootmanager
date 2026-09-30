# Duplication audit — the seams

Scope: `src-tauri/src/commands/`, `src-tauri/src/mcp_server/`, `src-tauri/src/application/`,
`src-tauri/crates/db/src/`, `src-tauri/crates/domain/src/`. Branch `fix/pyramid-promotion-relegation`
(this worktree; `commands/game.rs` is still the 4,358-line single file here, not the `game/`
directory). All paths below are relative to `src-tauri/`. Line numbers are from this tree.

Not re-reported (known): `season_check_complete` / `season_advance`, `round_context_for_today`
using `primary_competition()`, the press-conference pair, `game_load_save`, the ARCHITECTURE.md
command count. `engine` mirroring `domain` types is deliberate and is never proposed as a fix.

Findings are ranked by observable impact: save-file first, then things an MCP agent gets wrong
today, then latent, then hygiene. The MCP coverage table is at the end.

---

## 1. Every SQLite enum column is written with `{:?}` and read with a hand-written `match` that has a silent default — and one variant is already missing

**Concept.** The write side of every repository serialises enums with `format!("{:?}", …)`; the
read side is a hand-maintained `match` on strings ending in `_ => <some default>`. A domain
variant that is not in the read-side list compiles, saves, and reloads as the default.

**Locations.**
Write side (`{:?}`): `crates/db/src/repositories/player_repo.rs:36-41,121`, `team_repo.rs:34-38`,
`league_repo.rs:34-35`, `message_repo.rs:25-26`, `news_repo.rs:17`, `staff_repo.rs:11-12`,
`crates/db/src/game_persistence.rs:126,159,183-188`.
Read side (all 21 `parse_*` / `*_to_string` helpers, with what falls through to the default):

| helper | arms / variants | silently-defaulted variants |
|---|---|---|
| `repositories/message_repo.rs:59 parse_category` | 13 / 15 | **`JobOffer` → `System`** |
| `repositories/league_repo.rs:139 parse_fixture_competition` | 2 / 8 | `Cup`, `ContinentalClub`, `InternationalClub`, `InternationalNation`, `FriendlyCup` → `League` |
| `repositories/stats_repo.rs:21 parse_competition` | 7 / 8 | (default is `League`, in sync) |
| `repositories/player_repo.rs:143 parse_position` | 17 / 17 | none; default `Midfielder` |
| `game_persistence.rs:590 parse_position` | 17 / 17 | none; default `Midfielder` (second copy of the line above) |
| `repositories/player_repo.rs:166/174/181`, `team_repo.rs:100/111/122/130`, `staff_repo.rs:47/57`, `news_repo.rs:58`, `message_repo.rs:78`, `league_repo.rs:131`, `competition_repo.rs:30/41` | in sync today | each has a `_ =>` default that will absorb the next variant added |

Two in-repo precedents for the right shape already exist. `kit_pattern` is written through
`KitPattern`'s `Display` (`team_repo.rs:35`, impl at `domain/src/team.rs:337`) and read through
its `FromStr` (`domain/src/team.rs:350`) — the enum owns its own round-trip and the repository has
no table for it. And `game_persistence.rs:580 parse_objective_type`, `:613 parse_youth_region`,
`:621 parse_youth_objective` return `Err(be.error.gamePersistence.loadFailed)` on an unknown
string instead of guessing.

**Drifted or in sync.** Drifted, and it hits save files today:

- `MessageCategory::JobOffer` is written by `ofm_core/src/job_offers.rs:353` and `:493`; after a
  save/load cycle every job-offer message comes back as `MessageCategory::System`. Anything that
  filters the inbox by category — the GUI inbox filter, `inbox_get_messages(category="JobOffer")`
  (`mcp_server/tools_impl/inbox.rs:10`) — loses them once the save is reloaded. GUI player and MCP
  agent both affected; the save file is silently rewritten with the wrong category on the next save.
- `league_repo.rs:139` and `stats_repo.rs:21` are two parsers for the same `FixtureCompetition`
  enum in the same crate and disagree on six of eight variants. This one is masked on v3+ saves:
  `game_persistence.rs:341 promote_legacy_league()` → `ofm_core/src/game.rs:188 sync_legacy_league`
  overwrites `game.league` with a clone from `competitions`, which `competition_repo.rs:64-65`
  stores as serde JSON (correct types). It bites only when `competitions` is empty
  (`game_persistence.rs:262-266` promotes the collapsed mirror). Latent.
- `parse_position` exists twice with identical bodies (`game_persistence.rs:590`, `player_repo.rs:143`);
  the next `Position` variant must be added in two places or youth-scouting target positions and
  player positions will disagree after a reload.

**Source of truth.** `domain`'s serde derives. Every one of these columns holds a serde-compatible
string already (`{:?}` of a fieldless enum equals its serde name), so the read side can be
`serde_json::from_value(Value::String(s))` or a `FromStr` derived next to the enum in `domain`,
returning `Err` on unknown — as `parse_objective_type` does. That deletes 21 hand tables and the
two-copy `parse_position`.

**Severity.** Wrong today (JobOffer). Latent for every other enum — each is one added variant
away from the same silent corruption. Note the `_ =>` defaults also make the round-trip tests in
these files unable to catch it: a test that saves `JobOffer` and asserts the count of messages
still passes.

---

## 2. `time_check_blockers` reports a different set of blockers than the GUI, and MCP time advancement never enforces either

**Concept.** "What stops the clock" is defined once for the GUI in
`application/time_blockers.rs:321 compute_blocking_actions` (seven blockers: injured XI,
incomplete XI, squad-size crisis, planned contract-exit crisis, key-contract risk, contract-wage
risk, urgent unread mail). The MCP tool reimplements the question with two unrelated checks.

**Locations.**
- `mcp_server/tools_impl/time.rs:208-243 time_check_blockers` — checks (a) live match in
  progress, (b) pending transfer offers on own players. Neither is a GUI blocker; none of the seven
  GUI blockers is checked. Line 232-236 is a comment admitting the contract check was skipped.
- `mcp_server/tools_impl/time.rs:10-138 time_advance` → `advance_time_with_mode("delegate")`
  (`application/time_advancement.rs:87`), which has no blocker check at all.
- `mcp_server/tools_impl/time.rs:144-202 time_skip_to_match_day` — loops `advance_time_with_mode("delegate")`
  with no blocker check and no fired-during-skip detection.
- GUI counterparts that *do* enforce: `commands/time.rs:245 advance_one_day_internal` (blockers at
  267, stops on match day at 257), `:105 skip_to_match_day_internal` (blockers 187, match day 156,
  60-day cap 143), `:366 advance_to_next_event_internal`.

**Drift.** An MCP agent that dutifully calls `time_check_blockers` before `time_advance` is told
"No Blockers" while the GUI would refuse to advance (two injured starters, a squad that cannot
field eleven, a wage-risk warning). Conversely the agent is told to resolve "pending transfer
offers" that the GUI never blocks on. `time_skip_to_match_day` also:
- targets the next fixture in `game.league` only (`:146,151`), so a **cup** match between now and
  the next league game is auto-simulated in delegate mode on the way past — the GUI stops on it
  (`ofm_core/src/game.rs:202 user_has_scheduled_match_on`, across all competitions);
- uses `f.date > today` strict and a 365-day cap instead of 60;
- re-reads the whole game each iteration via `require_game` (`helpers.rs:7`, a full clone).

**Source of truth.** `compute_blocking_actions` for the question, and `commands/time.rs:105
skip_to_match_day_internal` for the skip — it is already `pub fn(&StateManager)` and returns a
JSON value with `action`/`blockers`/`results`; the MCP tool can call it today and format the
result. `time_advance` should call `advance_one_day_internal` (`:245`), which returns
`match_day` / `blocked` / `fired` / `advanced`, and surface those; the "delegate" auto-sim of the
user's own match should be an explicit agent choice, not the default of every tick.

**Severity.** Wrong today.

---

## 3. `game_select_team` and `game_new` skip four steps of the GUI's `select_team`

**Concept.** Team selection is the moment the world is shaped around the user's club. The GUI
command does this in `commands/game.rs:1824 select_team`; the two MCP entry points call the inner
`bootstrap_team_selection` (`:1552`) directly and skip everything `select_team` does around it.

**Locations.**
- `mcp_server/tools_impl/game.rs:95-126 game_select_team` — calls `start_phase_for_game` +
  `bootstrap_team_selection` + `create_new_save`.
- `commands/game.rs:1988-2106 bootstrap_game_for_mcp` (behind `#[cfg(feature = "mcp")]`, called
  from `mcp_server/tools_impl/game.rs:66 game_new`) — a second copy of `start_new_game`
  (`:1705`) + `select_team` (`:1824`) with the same omissions.
- What both skip, all in `select_team`: `ensure_multi_competition_foundations` (`:1838,1851`),
  the hemisphere clock anchor (`:1844-1854 team_season_anchor` / `rebuild_competitions_for_management_date`),
  `resolve_simulation_scope` (`:1856-1859`, sets `active_region_ids` / `active_competition_ids`),
  `upgrade_game_player_identities` (`:1869`). `bootstrap_team_selection` (`:1552-1568`) itself
  calls none of these.
- `bootstrap_game_for_mcp` additionally skips `nameMaxLength` (`:1723`), the DOB format / min-age
  / max-age checks (`:1732,1781-1786`), and instead invents a DOB 45×365 days before the clock
  (`:2039`); manager id is `mgr_user` in both paths, but the MCP one decides whether to honour the
  CLI name by comparing against the literals `"Agent"`, `"Manager"`, `"England"` (`:2023-2031`).

**Drift.** An MCP-created career has no simulation scope (`active_competition_ids` empty ⇒
`ofm_core/src/game.rs:252 competition_in_active_scope` returns true for everything ⇒ the whole
world is simulated daily, the unscoped cost the scope feature exists to avoid), no hemisphere alignment (a southern
club starts in July), and legacy-bucket positions until the first reload. The GUI player never
sees any of this; the MCP agent gets a measurably different (and slower) world from the same
world file. `game_select_team` also adds a guard the GUI lacks (`:98-100`, English text).

**Source of truth.** `select_team`. Factor its body into `select_team_internal(&StateManager,
&SaveManagerState, team_id, region_ids, competition_ids) -> Result<Game, String>` and have both
MCP tools and `bootstrap_game_for_mcp` call it; `bootstrap_game_for_mcp` should then be
`start_new_game_internal` + `select_team_internal`, not a third copy.

**Severity.** Wrong today.

---

## 4. `match_start` gives league matches extra time by default, and cannot start a cup match at all

**Concept.** Whether a level match goes to extra time and penalties is a property of the fixture
(knockout round or not), derived in one place for the GUI.

**Locations.**
- GUI: `application/time_advancement.rs:75 fixture_allows_extra_time` — true only when
  `league.is_knockout_fixture(&fixture.id)` (`domain/src/league.rs:470`); used at `:121` and `:175`.
- MCP: `mcp_server/tools_impl/live_match.rs:29 allows_extra_time.unwrap_or(true)`, passed
  straight into `application/live_match.rs:125 start_live_match`, which hands it to the session
  unchanged (`:185-190`). The engine branches on it with no competition awareness:
  `crates/engine/src/live_match/simulation.rs:97 if self.allows_extra_time && self.home_score == self.away_score`.
- MCP passes `None, None` for `home_team_id` / `away_team_id` (`live_match.rs:36-37`), so the
  cross-competition fixture resolution in `application/live_match.rs:154-178` never runs and the
  index is taken against `game.league` (the domestic mirror). The tool description (`tools.rs:794`)
  says "fixture index" without saying which competition.

**Drift.** An agent that calls `match_start(fixture_index, "live")` on its league fixture and
draws 1-1 at 90' plays extra time and a shootout; the same fixture in the GUI ends 1-1. That is a
numeric outcome (points, goals, fatigue, stats history) that differs by front end. And a cup
fixture cannot be started live via MCP at all — `time_advance` (delegate) auto-simulates it.

**Source of truth.** `fixture_allows_extra_time` — make it `pub(crate)` and have
`start_live_match` compute it from the fixture when the caller passes `None`, with the explicit
`bool` kept only for session restore. `match_start` should resolve the user's fixture on the
current date across competitions (the same `scheduled_user_fixture_index` at
`application/time_advancement.rs:37`) instead of taking an index.

**Severity.** Wrong today.

---

## 5. Five copies of a league-table ordering that disagrees with `domain`

**Concept.** How standings are ranked. `domain/src/league.rs:476 League::sorted_standings` is
points → goal difference → goals for. `StandingEntry::goal_difference()` is at `:384`.

**Locations** (all sort by points then **goals for**, skipping goal difference, and recompute GD
by hand):
- `mcp_server/tools_impl/time.rs:81-82,89`
- `mcp_server/tools_impl/info.rs:30-31,39` (`info_game_summary`), `:172-175,186,216`
  (`info_standings`), `:552-556` (`info_match_preview`), `:662-663,666` (`info_team_profile`)
- `commands/stats/team.rs:56` (GD by hand only)
- ofm_core counterpart, for the record: `crates/ofm_core/src/turn/round_summary.rs:332 sort_standings`
  is a third implementation with the correct three keys — it should also call
  `sorted_standings`, but that is outside this area.

**Drift.** Whenever two teams are level on points, the MCP table places the higher-scoring team
above the one with the better goal difference. `info_game_summary`'s "League Position: N" and the
"Standings Update" after every `time_advance` are therefore wrong on ties. `info_standings` also
builds the whole table twice (`:177-200` is dead — the comment at `:199` says so).

**Source of truth.** `League::sorted_standings()` and `StandingEntry::goal_difference()`.

**Severity.** Wrong today on every tie.

---

## 6. `season_get_awards` is computed over the whole world; the GUI shows the user's division

**Locations.** `mcp_server/tools_impl/season.rs:129` calls
`ofm_core::season_awards::compute_season_awards(&game)`. `commands/season.rs:49-65 get_season_awards`
resolves `ofm_core::end_of_season::user_division` (`crates/ofm_core/src/end_of_season.rs:825`) and
calls `compute_division_season_awards`, falling back to the world-wide version only when the
manager is unemployed.

**Drift.** In a 440-team world the MCP Golden Boot is whoever scored most anywhere; the GUI's is
the user's division. Two different answers to "who won the awards".

**Source of truth.** The command. Factor its closure into `season_awards_internal(&StateManager)`
and call it from the tool.

**Severity.** Wrong today.

---

## 7. `transfer_market_browse` prices players by wage × 52 while the GUI shows `market_value`

**Locations.** `mcp_server/tools_impl/transfers.rs:193-257`; the `max_price` filter at `:220-226`
is `(p.wage as u64 * 52) <= max` ("Rough annual cost estimate"). The GUI's market view is
`commands/slices.rs:20 get_players_page` → `crates/ofm_core/src/slices/players.rs:27 PlayersPageQuery`
(`position`, `team_id`, `status`, sort, paging) producing `PlayerSummary.market_value` (`:49`).
`Player.market_value` is a stored field (`player_repo.rs:44` column list).

**Drift.** `max_price=5_000_000` returns players whose *annual wage* is under 5M, not players whose
fee is under 5M; an agent budgeting a bid from this list is working from the wrong number. The
position filter (`:212-214`) also matches on the MCP-local short code (`helpers.rs:34 format_position`)
or the Debug name, while the slice matches `Position` exactly.

**Source of truth.** `query_page` with a `PlayersPageQuery`, and `market_value`.

**Severity.** Wrong today.

---

## 8. The `info_*` tools re-derive schedule and form from the legacy `game.league` mirror

**Concept.** The GUI reads fixtures and standings through `commands/slices.rs:41 get_schedule`
(`crates/ofm_core/src/slices/schedule.rs`) and `:71 get_competitions_view`; both walk
`game.competitions`. The MCP info tools rebuild the same views by hand over `game.league`.

**Locations.** `mcp_server/tools_impl/info.rs:28-69,99-116` (`info_game_summary`: position, form,
next match), `:268-329` (`info_fixtures`), `:515-573` (`info_match_preview`), `:661-696`
(`info_team_profile`), `:478` (`info_season_context` season number), `:242-262` (`game_is_finished`).
`helpers.rs:27-31 require_league` guards all of them with an English error.

**Drift.** `sync_legacy_league` (`crates/ofm_core/src/game.rs:188-200`) does mirror the *user's*
league, so these are not the wrong league — but: (a) cup, continental and international fixtures
never appear in `info_fixtures`, `info_match_preview` or "Next Match" (the comment at
`game.rs:196-199` says the mirror "misses cups and isn't reliable while the turn loop swaps
competitions through it"); (b) when the manager is unemployed the mirror falls back to
`competitions.first()` (`:198`). The standings/fixture tools are protected from that by their own
`team_id` guard (`info.rs:169,272,518`, and `:25` via `user_team`), so an unemployed agent gets
`be.error.noTeamAssigned` rather than a wrong table; the two readers that do consume the fallback
are `info_season_context` (`:478`, reports the first competition's season number) and
`info_team_profile` (`:661-674`, shows a league position only when the queried club happens to be
in the mirrored competition, so an out-of-division club silently gets none); (c) `game_is_finished`
(`:251-258`) is a third copy of the known-wrong
`season_check_complete` body (counts unfinished fixtures in the mirror, ignores
`counts_for_league_standings()`, never calls `ofm_core::end_of_season::is_season_complete`
at `crates/ofm_core/src/end_of_season.rs:91`).

**Source of truth.** `query_schedule` / `query_competitions` for the views, `is_season_complete`
for completeness, `user_has_scheduled_match_on` for "is today a match day".

**Severity.** Wrong today for any club in a cup, and for unemployed managers.

---

## 9. Youth-scouting input parsing exists three times with three different accepted vocabularies

**Locations.**
- `commands/transfers.rs:471 parse_youth_region`, `:479 parse_youth_objective`, `:488 parse_youth_target_position`
  — accept `""`/`None` as default, the exact variant names, and `"Goalkeeper"|"Defender"|"Midfielder"|"Forward"`;
  errors are the keys `be.error.transfers.invalidYouthScouting{Region,Objective,TargetPosition}` (`:17-21`).
- `mcp_server/tools_impl/scouting.rs:115,123,132` — accept aliases (`"Potential"`, `"Immediate"`),
  and 30 short codes (`"CB"`, `"CDM"`, `"LS"`…) folded to the legacy buckets; errors are English
  `format!("Unknown …: {}")`, which then pass through `translate_error` untouched.
- `crates/db/src/game_persistence.rs:613,621` — the persistence-side pair, `Err` on unknown.

**Drift.** `scout_youth_start(target_position="Goalkeeper")` works in both; `"GK"` works only in
MCP; `"Potential"` works only in MCP; the same invalid input yields a translation key from the GUI
and English from MCP. The MCP version also folds `"RWB"` to `Defender` where the GUI would reject
it — the two front ends disagree on what a valid request is.

**Source of truth.** One `FromStr`/parse per enum next to `YouthScoutingRegion` / `YouthScoutingObjective`
in `ofm_core::game`, returning the `be.error.transfers.*` keys; the MCP tool should call
`start_youth_scouting` through a `_internal` in `commands/transfers.rs` (there is none — the four
scouting commands at `:401-469` call `ofm_core` directly inside `mutate_active_game`).

**Severity.** Wrong today (divergent validation); low blast radius.

---

## 10. Six MCP tools use the `get_game` → clone → `set_game` pattern the backend rules forbid

**Concept.** `src-tauri/CLAUDE.md` §3: mutate through `update_game`; the clone-and-write-back
sequence loses updates when the GUI and an agent act concurrently (`fix/lost-update-races`).
`commands/util.rs:42 mutate_active_game` exists for exactly this.

**Locations** (MCP side → the command that already does it right):
- `mcp_server/tools_impl/season.rs:189-191 jobs_apply` → `commands/jobs.rs:25 apply_for_job` (`update_game`)
- `mcp_server/tools_impl/scouting.rs:11-14 scout_send` → `commands/transfers.rs:410`
- `scouting.rs:90-105 scout_youth_start` → `commands/transfers.rs:431`
- `scouting.rs:152-155 scout_youth_cancel` → `commands/transfers.rs:451`
- `scouting.rs:170-173 scout_youth_reassign` → `commands/transfers.rs:466`
- `mcp_server/tools_impl/live_match.rs:169-176 match_team_talk` → `commands/live_match.rs:128-130 apply_team_talk` (`update_game`)
- (`live_match.rs:222,330 match_press_conference` is the same pattern; known, not re-reported.)

Each also clones the entire `Game` twice (`helpers.rs:7 require_game` is `get_game(|g| g.clone())`,
then `set_game`), which `util.rs:32-37` documents as the thing `mutate_active_game` replaced.

**Source of truth.** The commands. `jobs_apply` and `apply_team_talk` need a `_internal` split
(their bodies are inline in the `#[tauri::command]`); the four scouting commands likewise.

**Severity.** Latent — a lost update needs a concurrent GUI write; the double world clone is a
cost on every call.

---

## 11. Three MCP save paths write an empty stats history where the command refuses

**Locations.** `mcp_server/tools_impl/game.rs:37-39 game_save`, `:163-165 game_exit`,
`mcp_server/tools_impl/time.rs:119-121` (auto-save inside `time_advance`) all do
`get_stats_state(|s| s.clone()).unwrap_or_default()`. The commands (`commands/game.rs:1955 save_game`,
`:1973 exit_to_menu`) call `require_active_stats_state` (`:74`) and fail with
`be.error.noActiveStatsSession`.

**Drift.** If the stats mutex is `None` for any reason, the MCP save overwrites the save file's
`player_match_stats` / `team_match_stats` with empty tables where the GUI would have refused.
The divergence is the finding; I did not trace which paths can leave the mutex empty. Also
`game_exit` clears the save id with `set_save_id(String::new())` (`game.rs:171`) instead of
`clear_save_id()` (`commands/game.rs:1979`, `crates/ofm_core/src/state.rs:129`), so
`get_save_id()` returns `Some("")` afterwards and the GUI's `get_active_save_id` reports an active
save that does not exist.

**Source of truth.** Factor `save_game` / `exit_to_menu` into `_internal`s taking
`(&StateManager, &SaveManagerState)`; the MCP tools already own both.

**Severity.** Latent (data loss when it fires).

---

## 12. `translate_error` is a hand copy of `en.json` with 44 arms against 160 distinct `be.error.*` keys in the tree, and can never match a parameterised key

**Locations.** `mcp_server/formatting.rs:7-53`. Applied twice on every path: inside each tool
(`.map_err(|e| translate_error(&e))`, 57 sites across `tools_impl/*.rs`) and again by the router
(`tools.rs:93-95 err_result`).

**Drift.**
- The tree emits 160 distinct `be.error.*` keys; the map has 44 arms. After excluding the
  package / competition-definition / persistence / settings families (which only the GUI reaches),
  49 keys that `commands/`, `application/` or `ofm_core` can return to an MCP tool have no entry:
  every `be.error.finance.*` except three, every `be.error.transfers.*` loan key, all six
  `be.error.scouting.*` youth keys, `be.error.noActiveLiveMatch`, `be.error.liveMatch.*`,
  `be.error.jerseyNumber*`, `be.error.kitChangesLockedInSeason`, `be.error.roleNotValidForPosition`,
  `be.error.invalidPlayerRole`, `be.error.playerNotOnTeam`, `be.error.taskJoinFailed`, the four
  `be.error.world*` keys. An agent hitting any of these sees `Error: be.error.transfers.loanCounterMustImproveTerms`.
- Keys that carry `?param=` never match the exact-string arms: `be.error.saveNotFound` is only
  ever emitted as `be.error.saveNotFound?saveId=…` (`crates/db/src/save_manager.rs:43`), likewise
  `gamePersistence.managerNotFound?managerId=` (`game_persistence.rs:221`) and
  `package.invalidPackageId?id=` (`commands/world.rs:355`). The `saveNotFound` arm at
  `formatting.rs:37` is dead.
- The double application means a mapped error reads `Error: Player not found. Check the player ID…`
  (the translated sentence falls into the `_ => format!("Error: {}", key)` arm on the second pass).

**Source of truth.** The locale files — `src/i18n/en.json` already has every `be.error.*` string
and the `?param=` parser (`src/utils/backendI18n.ts`). The MCP server could load `en.json` (or a
build-time include of it) and resolve keys the way the frontend does; failing that, the map needs a
gate test that diffs it against the keys in the tree. Remove the inner `map_err` layer either way.

**Severity.** Wrong today for agents (opaque errors on ~half the failure paths); cosmetic prefix
on the rest.

---

## 13. `game_delete_save` guards the active save; `delete_save` does not

**Locations.** `mcp_server/tools_impl/game.rs:210-215` vs `commands/game.rs:1894-1901` (calls
`sm.delete_save` directly). `settings.rs:129 clear_all_saves` deletes everything including the
active save with no guard either.

**Drift.** Deleting the active save from the GUI leaves `StateManager` holding a save id whose
`.db` is gone; the next `save_game` errors with `saveNotFound`. I cannot see whether the frontend
prevents this — treat as "verify the frontend guards it". The rule belongs in `SaveManager` or an
`_internal` shared by both.

**Severity.** Latent.

---

## 14. Hand-rolled "the user's team" lookups next to `commands/util.rs`, with the same failure under two keys

**Concept.** `commands/util.rs:10 user_team_id` / `:23 user_team_mut` exist so that "no team" is
`be.error.noTeamAssigned` and "team id matches no club" is `be.error.teamNotFound`, and so the
lookup cannot silently no-op (the doc comment at `:17-22` explains the bug it prevents). Only five
command files use them (`club.rs`, `contracts.rs`, `squad.rs`, `staff.rs`, `transfers.rs`).

**Locations** re-implementing it:
- `commands/finances.rs:54-60,98-102,116-120,134-138` (four copies of `manager.team_id.clone().ok_or("be.error.noTeamAssigned")`, then `be.error.managedTeamNotFound` at `:65`)
- `commands/club.rs:35,50` (`managedTeamNotFound`)
- `commands/squad.rs:408-412` (`set_team_kit_pattern_internal`: calls `user_team_id` then hand-rolls `teams.iter_mut().find()` + `teamNotFound` — the exact body of `user_team_mut`)
- `application/team_talk.rs:318-322`
- `application/time_blockers.rs:8-18 user_team_context`
- `mcp_server/tools_impl/helpers.rs:14-24 user_team` (a fourth spelling, returning `&Team`)
- `commands/stats/shared.rs:66-78 ensure_team_exists`

**Drift.** The same condition — the managed club is missing from `game.teams` — is reported as
`be.error.teamNotFound` (`util.rs:7`, `staff.rs:160,181`, `squad.rs:414`, `game.rs:1290,1372,1420,1507`,
`stats/shared.rs:76`) and as `be.error.managedTeamNotFound` (`finances.rs:65`, `club.rs:35,50`,
plus nine sites in `ofm_core/src/finances.rs`). Two locale strings, one bug.

**Source of truth.** `util.rs`; add a `user_team(&Game) -> Result<&Team>` there and delete the
MCP helper. Pick one key.

**Severity.** Latent (silent no-op risk the util comment describes); hygiene otherwise.

---

## 15. Three age calculators in the seams (and three more in `ofm_core`)

**Locations.** `commands/game.rs:181 age_on_date(NaiveDate, NaiveDate) -> i64`,
`commands/squad.rs:19 player_age_on(NaiveDate, &str) -> Option<i32>`,
`mcp_server/tools_impl/helpers.rs:57 age_from_dob(&str, &Game) -> String`.
`ofm_core` counterparts: `crates/ofm_core/src/contracts/helpers.rs:149 player_age_on` (`pub(crate)`,
returns **30** on a bad date), `aging.rs:10`, `season_awards.rs:61`, `player_rating.rs:95 player_age`
(year-only), `training.rs:406 estimate_age` (year-only), `generator/mod.rs:147 opening_player_age`.

**Drift.** Same arithmetic today; the failure modes differ (`"?"` string, `None` → `be.error.invalidDateOfBirth`,
`30`). The youth-academy rule at `squad.rs:303-308` (`age > 21` → `youthAcademyOverage`) and the
generator's `OPENING_YOUTH_MAX_AGE = 21` (`generator/mod.rs:47`) are the same threshold held in two
places with no shared constant.

**Source of truth.** One `pub fn age_on(dob: &str, on: NaiveDate) -> Option<u8>` in `ofm_core`
(or `domain` — it is pure), and `YOUTH_MAX_AGE` exported next to it.

**Severity.** Latent.

---

## 16. Five builders of the `key?param=value` error convention, three encoding strategies

**Locations.** `commands/game.rs:40-66 first_package_error_message` + `encode_error_param`
(full percent-encoding of everything outside `[A-Za-z0-9-_.~]`), `commands/world.rs:15
backend_text_with_param` (no encoding), `commands/world.rs:367 encode_param_value` (five
characters only; the comment at `:359-366` argues that is sufficient), `crates/db/src/game_persistence.rs:199
backend_error_with_param` (no encoding), `crates/db/src/save_manager.rs:33` (byte-identical copy
of the previous one). `ofm_core/src/generator/world_io.rs:44` is a sixth, outside this area.

**Drift.** A save id or manager id containing `&` or `=` breaks the frontend's `URLSearchParams`
parse on the db paths but not on the `game.rs` path. Two of the five are the same function
pasted between two files in one crate.

**Source of truth.** One `backend_message(key, &[(name, value)])` in `domain` or a tiny
`ofm_core::i18n` module, with one encoder.

**Severity.** Latent.

---

## 17. Three formation-string parsers in the area

**Locations.** `commands/sim_lab.rs:552 parse_formation -> (u8,u8,u8)` (folds a 4-part
formation's two midfield lines together, falls back to 4-4-2 if the sum ≠ 10),
`crates/db/src/save_manager.rs:736 formation_row_lengths -> Vec<usize>` (row lengths, falls back
to 4-4-2 on anything not 3 or 4 parts), `crates/ofm_core/src/player_rating.rs:104 formation_slot_rows`
(the canonical one, granular `Position`s per row). `save_manager.rs:757 is_mirrored_side_pair` is
lineup knowledge that also belongs beside `formation_slot_rows`. (The engine's
`live_match/substitution.rs:158 parse_formation` is a deliberate mirror and is not counted.)

**Drift.** `sim_lab`'s "sum must be 10" rule does not exist in `ofm_core`; `db` depends on
`ofm_core` already (it imports `formation_slots` at `save_manager.rs:683`) so its private copy is
pure duplication.

**Source of truth.** `player_rating::formation_slot_rows` (make it `pub`, derive row lengths from it).

**Severity.** Latent.

---

## 18. Smaller copies inside `commands/` and `db/`

Each is one concept in two places; all in sync today; all latent.

- **`player_has_active_or_pending_loan`** — `commands/transfers.rs:25` and
  `crates/ofm_core/src/transfers/mod.rs:38`, identical, the `ofm_core` one private. Make it `pub`.
- **`FixtureCompetition` → display string** — `commands/stats/shared.rs:4 competition_label` and
  `crates/db/src/repositories/stats_repo.rs:8 competition_to_string`, identical eight-arm tables;
  both equal `format!("{:?}")`.
- **`RoundSummaryDto`** — `commands/round_summary.rs:8-30` is a field-for-field copy of
  `crates/ofm_core/src/turn/round_summary.rs:9 RoundSummary` (seven fields, `From` impl does no
  transformation). A field added to `RoundSummary` is silently absent from the IPC/MCP response.
  Re-export the core type (it already derives `Serialize`).
- **`Manager::full_name()`** (`domain/src/manager.rs:120`) — reimplemented as
  `format!("{} {}", first_name, last_name)` at `commands/game.rs:1872,1916,2093`,
  `mcp_server/tools_impl/game.rs:111,139`, `crates/db/src/save_index.rs:172`; called once
  (`save_manager.rs:1650`, a test). `Staff` has no `full_name()`; `scouting.rs:18,73` hand-roll it.
- **`map_save_manager_lock_error`** (`commands/game.rs:68`) vs inline
  `.lock().map_err(|_| "be.error.saveManagerUnavailable")` at `commands/settings.rs:131-134`,
  `mcp_server/tools_impl/game.rs:12,42,113,133,166,217`, and `if let Ok(mut sm) = …lock()` at
  `tools_impl/time.rs:122` (swallows the poisoned-lock case entirely).
- **`role_valid_for_position`** (`commands/squad.rs:483`) — the only granular role/position
  validity table in Rust; the engine holds a coarse-bucket one (`crates/engine/src/live_match/mod.rs:433`,
  deliberate mirror, in agreement at bucket level). The rule is game logic and belongs in
  `ofm_core` beside `deployed_position`; leaving it in a command module means the MCP
  `squad_set_player_role` path and any future `ofm_core` caller (AI managers) must reach into
  `commands/`.
- **Atomic file write** — `commands/package_editor.rs:39 write_json_atomic` (fixed `.json.tmp`
  name, no cleanup on rename failure) and `commands/portraits.rs:312 write_cache_file_atomically`
  (counter-suffixed temp, cleans up, tolerates `AlreadyExists`). Two answers to "publish a file
  atomically"; the package editor's loses on a concurrent write.
- **`NO_ACTIVE_GAME` constant** — `commands/util.rs:5`, `commands/slices.rs:16`, and the literal
  `"be.error.noActiveGameSession"` at 56 other sites.

---

## 19. Error keys: English literals, and one failure under several keys

**English literals returned by commands** (category 5, "a command returning an English literal
instead of a key"):
- `commands/portraits.rs:178,202,209,264,271,315,323,334,352` — ten `format!("failed to …")` messages.
- `commands/sim_lab.rs:143,150` — `"{name} must be between …"`.
- `commands/time.rs:74` — `format!("be.error.taskJoinFailed: {err}")`: a key with an English
  suffix, which `resolveBackendError` will not parse as `key?param=`.
- `commands/game.rs:2080` — the `--mcp-auto-start requires a team_id …` sentence.
- MCP-side English that shadows an existing key: `helpers.rs:30` (`"No league found…"`),
  `info.rs:339` (`"Player {} not found"` — `be.error.playerNotFound` is mapped at `formatting.rs:12`),
  `info.rs:641` (`"Team {} not found"` — `be.error.teamNotFound`), `game.rs:99` (`"Already have a
  team assigned…"`), `game.rs:213`, `time.rs:165` (`"Date parse error"`), `live_match.rs:227`
  (`"No team assigned to manager"` — `be.error.noTeamAssigned`).

**The same failure under different keys** (emitter locations; ofm_core sites listed only to show
the pair):
- "That player is not yours": `be.error.playerNotInSquad` (`commands/squad.rs:300`),
  `be.error.playerNotOnTeam` (`squad.rs:621`), `be.error.playerNotInClub` (`ofm_core/src/squad_safety.rs:9`),
  `be.error.transfers.playerNotOwnedByUser` (`commands/transfers.rs:22`, `ofm_core/src/transfers/consts.rs:25`),
  `be.error.contracts.playerNotOwnedByClub` (`ofm_core/src/contracts/consts.rs:12`). Five keys.
- "Managed club not in `teams`": `be.error.teamNotFound` vs `be.error.managedTeamNotFound` (finding 14).
- "Staff member not found / not on your staff": `be.error.staffMemberNotFound` (`commands/staff.rs:27,43,288,303`)
  vs `be.error.scouting.scoutNotFound` (`ofm_core/src/scouting.rs:11`); `be.error.staffMemberNotInTeam`
  (`staff.rs:291`) vs `be.error.scouting.scoutNotInTeam` (`scouting.rs:13`).
- "Player has no club": `be.error.transfers.playerHasNoTeam` vs `be.error.contracts.playerHasNoTeam`.
- "Player is on loan": `be.error.transfers.playerAlreadyLoaned` (`commands/transfers.rs:23`) vs
  `be.error.contracts.playerOnActiveLoan`.

Each pair is two locale strings (×12 locales) for one condition, and two arms for
`translate_error` to forget (it has neither `playerNotOnTeam` nor any `contracts.playerOn*`).

**Severity.** Wrong today for the portrait/sim-lab literals (untranslatable); hygiene for the pairs.

---

## 20. `domain` doc comments that state a rule the code no longer honours

- **Wonderkid** (the known example, recorded because it lives in this area):
  `domain/src/player.rs:549` — `// age <= 21 && potential >= 85 && (potential - ovr) >= 10`.
  Code: `crates/ofm_core/src/player_rating.rs:4-8` — `20 / 90 / 14`, with a comment at `:7-8`
  saying the tightening happened in v0.2.1. The domain comment was never updated.
- **Position-gated traits**: `player.rs:541-542` say `(GK only)`, `:545` says `FWD:`, `:546`
  says `MID:`. `compute_traits` (`:553`) takes `_position` and ignores it; its own comments at
  `:604` ("any player with high GK stats can earn these") and `:621` ("purely attribute-based")
  contradict the enum's. An outfield player with `handling >= 85` is tagged `SafeHands`.
- Checked and **not** stale: `league.rs:468-469` ("knockout pairings are single-leg") — only
  `group_stage_legs` exists (`league.rs:42`, `ofm_core/src/group_stage.rs:19`); no two-legged
  knockout support anywhere.

**`impl` methods callers recompute**: `StandingEntry::goal_difference` (finding 5, six sites),
`League::sorted_standings` (finding 5), `Manager::full_name` (finding 18). `Team::remove_player_references`
(`team.rs:506`) is used correctly everywhere it applies.

**Severity.** Latent (misleading the next reader).

---

## MCP sweep — every tool in `tool_catalog()` (`mcp_server/tools.rs:892`)

Verdicts: **shared** = calls the same `_internal` / `application` fn the command calls;
**delegates** = calls `ofm_core` directly, and the command does the same thing with no extra logic
(or there is no command and the tool is a read-only formatter over `Game`); **duplicated** = a
second implementation of something a command or `application` fn does. "Counterpart" is the
Tauri command (`lib.rs` handler list) or `application` fn.

| tool | impl | counterpart | verdict | note |
|---|---|---|---|---|
| ping | tools.rs:245 | — | delegates | no logic |
| info_game_summary | info.rs:21 | get_session_state / get_schedule / get_competitions_view | duplicated | findings 5, 8 |
| info_game_state | info.rs:13 | get_active_game | delegates | serialises `Game` |
| info_standings | info.rs:165 | get_competitions_view | duplicated | finding 5, 8; dead code 177-200 |
| info_fixtures | info.rs:268 | get_schedule | duplicated | finding 8 |
| info_match_preview | info.rs:515 | get_schedule | duplicated | findings 5, 8 |
| info_player_profile | info.rs:335 | get_players_page / get_squad | delegates | English not-found (19) |
| info_player_stats | info.rs:579 | get_player_stats_overview | shared | |
| info_team_profile | info.rs:637 | get_teams_directory + get_finance_snapshot | duplicated | findings 5, 8 |
| info_team_stats | info.rs:726 | get_team_stats_overview | shared | |
| info_finances | info.rs:428 | get_finance_snapshot | shared | |
| info_news | info.rs:484 | get_news_feed | delegates | uses `article_is_visible`; no team-name map |
| info_season_context | info.rs:467 | get_session_state | delegates | season from mirror (8) |
| info_finance_snapshot | info.rs:786 | get_finance_snapshot | shared | |
| info_player_match_history | info.rs:599 | get_player_match_history | shared | |
| info_team_match_history | info.rs:749 | get_team_match_history | shared | |
| time_advance | time.rs:10 | advance_time_with_mode / advance_one_day | duplicated | finding 2, 5, 11 |
| time_skip_to_match_day | time.rs:144 | skip_to_match_day | duplicated | finding 2 |
| time_check_blockers | time.rs:208 | check_blocking_actions | duplicated | finding 2 |
| squad_get | squad.rs:10 | get_squad | delegates | formatter |
| squad_set_formation | squad.rs:107 | set_formation | shared | |
| squad_set_starting_xi | squad.rs:76 | set_starting_xi | shared | |
| squad_set_play_style | squad.rs:127 | set_play_style | shared | |
| squad_set_match_roles | squad.rs:146 | set_team_match_roles | shared | |
| squad_auto_set_pieces | squad.rs:177 | auto_select_set_pieces + set_team_match_roles | shared | composes two internals |
| squad_set_player_role | squad.rs:236 | set_player_squad_role | shared | (squad role, not tactical role) |
| training_get | training.rs:10 | get_active_game | delegates | formatter |
| training_set_focus_intensity | training.rs:55 | set_training | shared | |
| training_set_schedule | training.rs:71 | set_training_schedule | shared | |
| training_set_groups | training.rs:87 | set_training_groups | shared | |
| training_set_player_focus | training.rs:106 | set_player_training_focus | shared | |
| transfer_market_browse | transfers.rs:193 | get_players_page | duplicated | finding 7 |
| transfer_make_bid | transfers.rs:68 | make_transfer_bid | shared | |
| transfer_preview_bid | transfers.rs:112 | preview_transfer_bid_financial_impact | shared | |
| transfer_respond_to_offer | transfers.rs:143 | respond_to_offer | shared | |
| transfer_counter_offer | transfers.rs:165 | counter_offer | shared | |
| transfer_toggle_listed | transfers.rs:10 | toggle_transfer_list | shared | |
| transfer_toggle_loan | transfers.rs:39 | toggle_loan_list | shared | |
| transfer_free_agent_offer | transfers.rs:263 | offer_free_agent_contract | shared | |
| transfer_free_agent_preview | transfers.rs:284 | preview_free_agent_contract_impact | shared | |
| contract_propose_renewal | contracts.rs:10 | propose_renewal | shared | |
| contract_delegate_renewals | contracts.rs:63 | delegate_renewals | shared | |
| contract_set_exit_intent | contracts.rs:99 | set_contract_exit_intent | shared | |
| contract_clear_exit_intent | contracts.rs:121 | clear_contract_exit_intent | shared | |
| contract_terminate | contracts.rs:151 | terminate_contract_now | shared | |
| contract_preview_renewal | contracts.rs:84 | preview_renewal_financial_impact | shared | discards the projection it fetched |
| contract_preview_termination | contracts.rs:137 | preview_contract_termination | shared | discards the preview it fetched |
| inbox_get_messages | inbox.rs:10 | get_messages_page | delegates | `MessagesQuery` is empty; filter is MCP-only |
| inbox_mark_read | inbox.rs:53 | mark_message_read | shared | |
| inbox_mark_all_read | inbox.rs:69 | mark_all_messages_read | shared | |
| inbox_delete | inbox.rs:85 | delete_message | shared | |
| inbox_clear_old | inbox.rs:101 | clear_old_messages | shared | |
| inbox_resolve_action | inbox.rs:117 | resolve_message_action | shared | |
| club_upgrade_facility | club.rs:10 | upgrade_facility | shared | |
| club_request_board_support | club.rs:78 | request_board_support | shared | |
| club_request_marketing | club.rs:96 | request_marketing_campaign | shared | |
| club_request_sponsor_pitch | club.rs:114 | request_sponsor_pitch | shared | |
| staff_hire | club.rs:46 | hire_staff | shared | |
| staff_release | club.rs:62 | release_staff | shared | |
| staff_get | club.rs:26 | get_staff | delegates | formatter over `game.staff` |
| scout_youth_start | scouting.rs:89 | start_youth_scouting | duplicated | findings 9, 10 |
| scout_youth_cancel | scouting.rs:151 | cancel_youth_scouting | duplicated | finding 10 |
| scout_youth_reassign | scouting.rs:169 | reassign_youth_scouting | duplicated | finding 10 |
| scout_get_reports | scouting.rs:40 | get_messages_page / get_staff | delegates | formatter |
| scout_send | scouting.rs:10 | send_scout | duplicated | finding 10 |
| season_check_complete | season.rs:78 | check_season_complete | duplicated (known) | |
| season_advance | season.rs:98 | advance_to_next_season | duplicated (known) | |
| season_get_awards | season.rs:127 | get_season_awards | duplicated | finding 6 |
| game_new | game.rs:53 | start_new_game + select_team | duplicated | finding 3 |
| game_select_team | game.rs:95 | select_team | duplicated | finding 3 |
| game_load_save | game.rs:132 | load_game | duplicated (known) | |
| game_save | game.rs:31 | save_game | duplicated | finding 11 |
| game_exit | game.rs:158 | exit_to_menu | duplicated | finding 11 |
| game_export_world | game.rs:185 | export_world_database | shared | |
| game_list_saves | game.rs:11 | get_saves | delegates | inline lock map (18) |
| game_delete_save | game.rs:209 | delete_save | duplicated | finding 13 |
| game_list_world_databases | game.rs:230 | list_world_databases | shared | calls the command fn itself |
| game_is_finished | info.rs:242 | check_season_complete | duplicated | third copy of the known body (8) |
| match_start | live_match.rs:22 | start_live_match | shared | but defaults ET to true (4) |
| match_step | live_match.rs:53 | step_live_match | shared | |
| match_command | live_match.rs:93 | apply_match_command | shared | |
| match_snapshot | live_match.rs:120 | get_match_snapshot | shared | |
| match_finish | live_match.rs:136 | finish_live_match | shared | |
| match_team_talk | live_match.rs:164 | apply_team_talk | duplicated | finding 10 (get→set around the shared fn) |
| match_press_conference | live_match.rs:206 | submit_press_conference | duplicated (known) | |
| jobs_available | season.rs:166 | get_available_jobs | delegates | same `ofm_core` call |
| jobs_apply | season.rs:188 | apply_for_job | duplicated | finding 10 |
| help_find_tool | help.rs | — | delegates | no counterpart |
| help_list_categories | help.rs | — | delegates | no counterpart |

Totals: 89 tools — 49 shared, 14 delegates, 26 duplicated (4 of them known). Competition mode
disables five (`config.rs:64-75`: `game_new`, `game_select_team`, `game_export_world`, `game_exit`,
`game_load_save`), so four of the 26 duplicated tools are unreachable in competition mode; the
other 22 are live in both modes.

---

## What one fix would retire the most findings

1. `db`: replace the 21 hand `parse_*` tables with serde/`FromStr` from `domain` and make unknown
   strings an `Err` (finding 1). One PR, one round-trip test per enum that saves every variant.
2. `commands`: give `select_team`, `save_game`, `exit_to_menu`, `apply_for_job`, `apply_team_talk`,
   `get_season_awards` and the four scouting commands an `_internal`, then make the MCP tools call
   them (findings 3, 6, 10, 11). Delete `bootstrap_game_for_mcp`'s second copy of the body.
3. MCP time tools call `advance_one_day_internal` / `skip_to_match_day_internal` /
   `compute_blocking_actions` and format their JSON (finding 2); `info_*` call `query_schedule` /
   `query_competitions` / `League::sorted_standings` (findings 5, 8).
4. `translate_error` resolves against `en.json` with the frontend's `?param=` rule, or is gated by
   a test that diffs its arms against every `be.error.*` in the tree (finding 12).
