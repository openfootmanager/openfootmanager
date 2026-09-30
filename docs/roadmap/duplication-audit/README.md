# Duplication audit — consolidated index

> **A dated collection, not a status board, and not a single snapshot.** The four sweeps were run
> over 22–24 September 2026 against *different* revisions — the table below gives each one, and the
> reports state their own. So line numbers are only valid against the revision that sweep names, and
> two reports can disagree about the same file for no reason other than that. `commands/game.rs`, for
> instance, is still one 4,358-line file in the tree sweeps A–C read.
>
> Findings have been fixed since, and a few did not survive closer reading. For what is still
> outstanding, read [epic #589](https://github.com/openfootmanager/openfootmanager/issues/589) and
> its children (#590–#603) — that is the live record. This directory is the evidence they were filed
> from, kept because the reasoning behind a finding is worth more than its one-line summary, and
> because re-deriving it costs four sweeps.

Four sweeps, ~106 findings across ~2,540 lines of report. Full detail in the sibling files; this
is the map.

| Sweep | Area | Report | Read at | Findings |
|---|---|---|---|---|
| A | `ofm_core/src` internals | `AUDIT-ofm-core.md` | `c8840375` (`fix/pyramid-promotion-relegation`) | 7 verified + latent/in-sync sections |
| B | React frontend (`src/`) | `AUDIT-frontend.md` | `6e812670` (`fix/pyramid-promotion-relegation`) | 37 |
| C | commands ↔ MCP ↔ `db` ↔ `domain` | `AUDIT-seams.md` | `fix/pyramid-promotion-relegation` (no commit recorded) | 20 + a table of all 89 MCP tools |
| D | docs, `.ofm` schema, fixtures, CI | `AUDIT-docs-schema.md` | `8e01f97b` (cached `upstream/develop`, PR #559) | 42 + verification of 7 prior findings |

Earlier context and the four bugs that prompted the audit: `findings.md` — read 22 Sep 2026, and written up on the 24th after the first copy was lost.

---

## The eight root patterns

Individual findings are symptoms. These are the causes, and they are what a design plan has to
address. Instance counts are indicative, not exhaustive.

### P1 — An else-branch stands in for an enum's tail
The single most productive defect in the whole audit: a `match` or ternary handles the variants
somebody had in mind and dumps the rest into a default.

- Cup, continental and international fixtures all render as **"Friendly"** on Home and Next Match
  (`HomeNextOpponentCard.tsx:39`, `NextMatchDisplay.tsx:49`) — verified.
- `league_repo::parse_fixture_competition:139` collapses **6 of 8** `FixtureCompetition` variants
  to `League`; `stats_repo` maps all eight — verified.
- `infer_region_id` (`generator/world_io.rs:11`) has **no Africa arm** and returns `"europe"` for
  54 African codes plus most Asian and Caribbean ones.
- **21 db enum columns** are written `format!("{:?}")` and read by a hand `match` with `_ =>`, so a
  new variant is never a compile error. `MessageCategory::JobOffer` already shipped broken this
  way; it is fixed *and tested* in `message_repo` only.
- `ScoutingPlayerSearchCard.tsx:208` buckets granular positions into four and paints the rest red.

**Fix shape:** exhaustive matching, `Display`/`FromStr` on the domain type (the `kit_pattern`
precedent already in the tree), and a round-trip test per enum. Makes the next variant a compile
error instead of a silent default.

### P2 — The frontend recomputes a rule the backend owns
Every shipped bug of this class so far. The backend answer exists; TypeScript computes its own.

- Promotion zones (`pyramid.ts`), "season is over" (`Dashboard.tsx`) — both fixed on #559.
- `finance.ts:203` omits matchday income and sponsor bonus, so the dashboard fires a
  finance-crisis alert for a club the backend rates **watch** (arithmetic witness in sweep D).
- The whole season-context derivation and `seasonComplete` exist in TS although
  `check_season_complete` exists and **has zero TS callers**.
- `gameStore.ts:74` rebuilds the Rust session slice; `get_session_state` is **never invoked**.
- Player match ratings invented from a 6.0 baseline; the engine has no rating field.
- Fixture labels, region inference, ~6 copies of the standings comparator, a third age algorithm.

**Fix shape:** the backend owns the answer and ships it on the wire; the TS copy is deleted, not
kept in sync. Where a command already exists and is uncalled, call it.

### P3 — MCP reimplements a command instead of sharing `_internal`
`src-tauri/CLAUDE.md` §4 forbids this. **26 of 89 tools** are second implementations (49 share, 14
delegate).

- `match_start` defaults `allows_extra_time: true`, so a drawn **league** match goes to extra time
  and penalties.
- `game_select_team` skips `resolve_simulation_scope`, so agent careers simulate the whole world,
  plus three other steps of the real `select_team`.
- `season_check_complete` / `season_advance`: an agent cannot finish a season at all.
- Five copies of a standings sort that omits goal difference; `translate_error` is a 44-arm hand
  copy against 160 keys with 49 reachable keys unmapped.

**Fix shape:** factor each command body into `_internal`, have both callers use it, and add a gate
that fails when an MCP tool reaches past it.

### P4 — A "primary" or "first" accessor stands in for a specific thing
- `primary_competition()` is `competitions.first()`; competitions are ordered by country code, so
  for most careers it is a foreign league. Drives round-summary standings diffs.
- `game.league` is a second copy of the user's competition — one canonical writer plus five ad-hoc
  assignments, **eleven frontend readers**, and `game.rs:91` already says "do not add new readers".
- The session slice and the store both pick by first `participant_ids`, and both disagree with
  `user_competition_index`.

**Fix shape:** one named accessor for "the user's competition", used everywhere; `primary_*`
narrowed to the one persistence-metadata caller that genuinely means "first", or removed.

### P5 — One concept, several formulas
- **Loan wages, three rules.** The ledger charges the borrower 100%; the snapshot and the
  affordability check split by `wage_contribution_pct`; a third site charges the parent 100%. The
  negotiated percentage is never read by code that moves money — verified end to end. **Money bug.**
- **Age: seven helpers, three algorithms** (day-of-year, month/day, year-only). The year-only one
  drives potential bands and the wonderkid gate.
- **Five team-strength calculators** with different fallbacks and rosters; the upset detector
  judges a result with a different strength than the sim that produced it.
- Contract end dates ×3, contract-length-by-age tables ×2, money formatting (`1.2M` vs `€1.2M`),
  fee-discount ladders ×2.

**Fix shape:** one function per concept, in the lowest crate that needs it, with the callers
converted rather than copied.

### P6 — Authored schema accepted, then ignored
The `.ofm` contract is described in the Rust structs, `ofm-cli`, the Package Editor types and
`SCHEMA_REFERENCE.md` — four hand-maintained copies.

- `format.groupSize` is validated (rejects `< 2`) and then discarded: `GroupStageConfig` has no
  group-size field and `group_stage.rs` uses a hardcoded `GROUP_SIZE = 4` — verified. An author
  setting `groupSize: 2` gets groups of four.
- A player may omit `club`, validation deliberately allows it, and world construction has no
  unattached-player path — the authored player silently never enters the world.
- Two names entities in one package: last wins, the other's pool is silently dropped.
- Two world manifests: last wins, where the docs promise an error.

**Fix shape:** one core schema policy; generate the CLI surface, the editor types and the reference
from it rather than maintaining four.

### P7 — Docs and tooling asserting what the code does not do
- `ARCHITECTURE.md:178` documents 29 commands; `lib.rs:192` registers 123.
- `MCP_SERVER.md` documents `TAURI_SAVE_DIR` for per-agent save isolation; **nothing reads it**, so
  the multi-agent script runs every instance against the same save area.
- `MCP_SERVER.md:206` says `season_advance` advances through the off-season; it advances one day.
- `domain/player.rs:559` documents Wonderkid as 21/85/10; `player_rating.rs` uses 20/90/14.

**Fix shape:** generate what can be generated (command table, schema reference); delete what cannot
be kept true.

### P8 — Test infrastructure duplicating production rules, and flakiness
- Three test files hand-maintain their own locale map; **none reads `SUPPORTED_LANGUAGES`**, and
  `backendI18n.localeCoverage.test.ts` omits Indonesian, so a missing backend key passes CI.
- **65 `rand::rng()` sites** in `ofm_core`/`engine` — unseeded randomness in production paths that
  tests exercise. 39 tests loop over seeds to self-mitigate, which says the problem is known
  informally and never addressed. Also a *correctness* constraint: unseeded RNG is why fixtures
  cannot be settled early without breaking save-reload determinism.
- Vitest reds whole files under CPU load (observed 5 failures at load 15, 26 at load 25, clean in
  isolation). Not filed anywhere; currently folklore.
- One flaky test is filed (#481).
- No `Modal` primitive and **no focus trap anywhere**; four `SortHeader` implementations of which
  one is keyboard-reachable; four dead components.

---

## Corrections applied to earlier summaries

Recorded because the earlier wording was wrong and may have been repeated:

- **Batch vs live, not AI vs human.** Injured and non-XI players reach *batch* simulation; live AI
  opponents use the healthy-XI builder too. The split is by code path, not by who manages the club.
- **Batch does deplete condition**, at team level (`engine/mod.rs:192`). The confirmed divergence is
  *per-player* condition scaling and event detail, not absent fatigue.
- **Locale counts.** `i18n/localeCoverage.test.ts` has 11 non-English entries plus separately
  imported English = 12 covered. The claim that its map holds twelve entries was inaccurate; the
  divergence with the backend-key test stands.
- **`MessageCategory::JobOffer` is already fixed**, with a round-trip test. The pattern it belongs
  to is not.

## Caveats on the audit itself

- Sweep D worked against a locally cached `upstream/develop` snapshot: `git fetch` could not write
  `FETCH_HEAD` and SSH/HTTPS were both unavailable in that sandbox. "Current develop" there means
  that dated snapshot.
- Sweep A's line numbers come from its own worktree, where `transfers/` is not split the same way.
- Every finding is a source audit with bounded probes. Where a report says "source-proved" it means
  input, transformation and consumer were read — not that a runtime scenario was executed.
- Four headline claims needed correction on contact. Verify before acting on any single line here.
