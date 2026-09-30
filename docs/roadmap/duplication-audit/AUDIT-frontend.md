# Duplication audit — frontend (`src/`)

**Sweep B — 22–24 September 2026, at `6e812670`.**

Scope: `src/components/`, `src/lib/`, `src/utils/`, `src/services/`, `src/store/`, `src/pages/`,
`src/hooks/`. Read only.

Findings already known when this sweep ran (pyramid.ts, the Dashboard season decision,
PostMatchHelpers ratings, finance.ts runway, the frozen `asOfDate`, the eleven/twelve locale
lists, the six standings comparators, `buildPitchRows`, the transfer tension gauge and January
window, the English error literal, the two exhausted thresholds) are not repeated below except
where a distinct adjacent site was found.

Every claim below was checked against the source at that revision, and `file:line` numbers are from
it. "Wrong today" means a player can observe the divergence now; "latent" means the copies
agree today but nothing keeps them so.

**Ranked by what a player sees today**

1. A1 — cup/continental fixtures labelled "Friendly" on Home and the Next Match card (four label
   copies, three rules).
2. A2 — eleven readers of the deprecated `gameState.league` mirror; Team Profile shows "—" for
   every club outside the manager's division.
3. A6 — scouting search paints every outfield player red (coarse-bucket badge copy, PR #416
   shape).
4. A7 — live/post-match "Shots" excludes missed penalties; the saved report includes them.
5. A5 — finance cash-flow chart ignores the currency setting the rest of the tab honours.
6. A3 — Tactics tab's fallback set-piece takers are not the ones the engine picks.
7. A4 — career-creation age check uses July 1 where the backend starts World Cup summers on
   June 1; third age algorithm in `src/`.
8. F1 — no `Modal` primitive; the de facto one has no dialog semantics; no focus trap anywhere.
9. B1/B2 — season phase, transfer window and "season complete" are fully recomputed in TS while a
   backend field/command already exists (and `check_season_complete` is never invoked).
10. B3 — `inferRegionId` mirrors the generator's import fallback (no Africa), not the runtime
    catalog.

---

## A. Wrong today — verified divergence

### A1. Cup and continental fixtures are labelled "Friendly" on the Home tab and Next Match card

**Concept.** What to call a fixture: matchday N, pre-season, friendly, or the competition.

**Locations.**
- `src/lib/fixtures.ts:4` `getFixtureDisplayLabel` — Preseason → "Pre-season", Friendly →
  "Friendly", **everything else → "Matchday N"** (used by `DashboardMatchConfirmModal.tsx:50`,
  `PreMatchSetup.tsx:231`).
- `src/components/NextMatchDisplay.tsx:49-53` — League → "Matchday N", Preseason → "Pre-season",
  **everything else → "Friendly"**.
- `src/components/home/HomeNextOpponentCard.tsx:39-45` — same rule as NextMatchDisplay, copied
  inline.
- `src/components/schedule/ScheduleTab.tsx:437` `groupLabel` — a fourth, four-way rule (League
  with matchday, Preseason, Friendly, else competition name).
- `src/pages/MatchSimulation.tsx:411` `isLeagueFixture` — re-implements
  `isCompetitiveFixture` (`src/lib/fixtures.ts:19`) inline.
- Rust: `src-tauri/crates/domain/src/league.rs:224` `FixtureCompetition` has eight variants
  (League, Cup, ContinentalClub, InternationalClub, InternationalNation, Friendly, FriendlyCup,
  PreseasonTournament).

**Drifted.** Four copies, three different rules. `getUserNextFixture` (`fixtures.ts:182`)
deliberately spans every competition the club is in, so when the next fixture is a domestic cup
tie or a continental match, the Home tab's "Next opponent" card and the dashboard's
`NextMatchDisplay` both print **"Friendly"**, while the pre-match screen for the same fixture
prints "Matchday N" (also wrong for a cup, but differently). The Schedule tab is the only one that
names the competition.

**Owner.** One frontend helper — extend `getFixtureDisplayLabel` to the four-way rule
`ScheduleTab.groupLabel` already has (it needs the competition name, so give it
`competitionDisplayName` from `src/lib/competitionName.ts`) and delete the three inline copies.
The backend already tells the frontend the competition through `fixture.competition_id`
(`store/types.ts:517`); nothing needs a Rust change.

**Severity.** Wrong today, cosmetic but on the most-visited screen.

### A2. Eleven readers of the deprecated `gameState.league` mirror; Team Profile shows "—" for every club outside the manager's division

**Concept.** Which competition a piece of UI is talking about.

**Locations.**
- Rust contract: `src-tauri/crates/ofm_core/src/game.rs:90-97` — `league` is "DEPRECATED …
  Do not add new readers", and `game.rs:188` `sync_legacy_league` makes it a copy of the
  **user's** competition.
- `src/components/teamProfile/TeamProfile.viewModel.ts:36,40,70` — sorts `gameState.league`
  standings and looks the profiled team up in them.
- `src/components/EndOfSeasonScreen.tsx:43` — re-derives champion and user position from
  `gameState.league` (the backend's `EndOfSeasonSummary` at
  `src-tauri/crates/ofm_core/src/end_of_season.rs:934` already carries `champion_id`).
- `src/components/match/helpers.tsx:220` `resolveMatchFixture` and
  `src/pages/MatchSimulation.tsx:154` — index `gameState.league.fixtures[fixtureIndex]`.
- `src/components/match/RoundDigestScreen.tsx:121,140` — "other results this round" from
  `gameState.league.fixtures` only.
- `src/components/transfers/TransferCentreWorldTab.tsx:46`, `src/components/news/NewsTab.tsx:133`
  — name/transfer log from the mirror (harmless while it mirrors the user's league).
- `src/services/portraitService.ts:134` — "next opponent" for portrait preloading from the
  mirror's fixtures only, unlike `getUserNextFixture` which spans every competition.
- `src/store/gameStore.ts:79` `deriveSessionState` — a third selection rule; see B0 below.
- `src/lib/fixtures.ts:86` `getPrimaryCompetition` — `competitions[0] ?? league` (the known
  Dashboard bug's helper; it has one remaining caller, `Dashboard.tsx:261`).
- Canonical TS: `src/lib/fixtures.ts:133` `getUserCompetition`, and the private
  `competitionIncludesTeam` (`fixtures.ts:119`) — **not exported**, so a "competition for team X"
  lookup is unreachable from `TeamProfile.viewModel.ts` without a change to `lib/fixtures.ts`.

**Drifted.** The mirror is the *manager's* league, so `TeamProfile.viewModel.ts:70` finds no
standing entry for any club in another division or country: the hero card prints `—` for league
position (`TeamProfileHeroCard.tsx:68,91`) and `TeamProfileLeagueStandingCard` renders nothing
(`:15`). In a generated world with several countries and tiers that is most clubs a player opens
from the Teams tab. `RoundDigestScreen` likewise omits cup results from "other matches".

The fixture-index sites are the same shape as the bug the Rust side already documents
(`game.rs:200`: the mirror "isn't reliable while the turn loop swaps competitions through it");
`MatchSimulation.tsx:158` works around it by also sending team ids, but `resolveMatchFixture`
still prefers the raw index over the team-id match.

**Owner.** Frontend: export a `getCompetitionForTeam(gameState, teamId)` from `lib/fixtures.ts`
(the `competitionIncludesTeam` predicate already exists), route the team profile, end-of-season
screen, digest and match-fixture resolution through it, and make `resolveMatchFixture` match by
team ids first. Long-term the backend intends to drop the field entirely (`game.rs:96`), at
which point every one of these readers breaks at once.

**Severity.** Wrong today (Team Profile); latent-to-wrong for the match/digest sites.

### A3. Tactics screen's fallback set-piece takers are not the ones the engine uses

**Concept.** Who takes penalties / free kicks / corners and who captains when the manager has
not assigned anyone.

**Locations.**
- `src/components/tactics/TacticsRoles.helpers.ts:16` `pickBestCandidate` +
  `src/components/match/SetPieceSelector.tsx:28` `getSetPieceStats` — TS ranking, shown as the
  "effective" roles by `useTacticsLineup.ts:198`, `TacticsRightPanel.tsx:45`,
  `TacticsRolesPanel.tsx:51`.
- Rust: `src-tauri/crates/ofm_core/src/live_match_manager/team_builder.rs:466`
  `auto_select_set_pieces`, applied at `live_match_manager.rs:234-256` whenever a saved role is
  missing.

**Drifted.** Three concrete differences: (1) Rust penalises a corner taker who is also the
free-kick taker by 5 points "to encourage variety" (`team_builder.rs:505`); TS has no such
rule, so it will name the same player for both. (2) Ties: TS breaks on name ascending
(`TacticsRoles.helpers.ts:38`), Rust `max_by_key` keeps the *last* maximum. (3) TS rounds the
averaged score (`Math.round((a.shooting + a.composure) / 2)`) so distinct sums collapse into
ties that Rust does not have. When no roles are saved (every AI-managed club, and a new save
until the manager visits Tactics), the Tactics tab displays a taker the match will not use, and
nothing writes the TS choice back (no `set_match_roles` call from those components).

**Owner.** Backend. Either expose `auto_select_set_pieces` through the existing
`auto_select_set_pieces` command (`src-tauri/src/commands/squad.rs:327`) and have the Tactics
tab read the result, or persist the auto-selection into `team.match_roles` so both sides read
one value. The TS scorer should then only *display* attribute breakdowns, not decide.

**Severity.** Wrong today for any club without saved roles; low visibility.

### A4. Career-creation age check uses July 1 in World Cup summers where the backend starts the game on June 1

**Concept.** The in-game date a new career starts on (which fixes the manager's age at creation).

**Locations.**
- `src/pages/MainMenu.tsx:197` `careerStartReferenceDate` — `Date.UTC(startYear, 6, 1)`
  (+120 days for `midSeason`).
- Rust: `src-tauri/src/commands/game.rs:160-177` `start_date_for_year` /
  `current_date_for_phase` — month 6 (June) when `world_cup::is_world_cup_summer(start_year)`,
  else July; `+ Duration::days(120)` for MidSeason.

**Drifted.** In a World Cup year the frontend validates the date of birth against a reference
one month later than the backend's actual start date. A manager born between 1 June and 1 July
passes/fails the age gate differently on the two sides (the backend re-checks with
`age_on_date`, `game.rs:181`, so the visible symptom is a translated backend error after the
form said the date was fine).

Adjacent: this is the **third** age algorithm in `src/` — `MainMenu.tsx:205`
`flooredAgeFromIsoDate` (UTC), `src/lib/valueFormatting.ts:55` `calcAgeOnDate` (UTC), and
`PlayerProfile.helpers.ts:67` `getPlayerAge` (local time, the known frozen-date one). All three
agree on the birthday rule; `calcAgeOnDate` is reachable from both other sites.

**Owner.** Backend already owns the start date; expose it (a `career_start_date(year, phase)`
query, or return it with the world-cup-summer flag the generator already computes at
`ofm_core::generator::start_date_at_game_open`, `game.rs:379`) and drop the TS copy. Replace the
two extra age functions with `calcAgeOnDate`.

**Severity.** Wrong today, one month in four years, at the DOB boundary.

### A5. Finance cash-flow chart ignores the currency setting that every other money label honours

**Concept.** Formatting an amount of money.

**Locations.**
- `src/lib/valueFormatting.ts:100` `formatVal` — converts by `exchange_rate`, prefixes the
  selected currency symbol, uses the UI locale.
- `src/components/finances/FinanceCashFlowChart.tsx:13` `formatShortAmount` — raw number,
  hard-coded `M`/`K`, no symbol, no conversion, no locale; wired to the Y-axis
  (`tickFormatter`, `:95`) and the tooltip (`formatter`, `:109`) of the chart rendered by
  `FinancesTab.tsx:3`, alongside `formatExactMoney` labels (`FinancesTab.tsx:250-260`).
- `src/components/menu/PackageEditor/TeamPreviewCard.tsx:33` `formatBudget` — same shape; and
  `src/components/menu/PackageEditor/teamRanges.helpers.ts:59` `formatCompactAmount` in the
  **same folder** already does this correctly with `Intl.NumberFormat` (its doc comment explains
  why hard-coded `K`/`M` is wrong for zh-CN and de).
- `src/utils/backendI18n.ts:175` parses the `M`/`K` suffix back out of backend strings — a
  third place that knows the suffix convention.

**Drifted.** With any currency other than the base one selected, the Finances tab's summary
cards show converted figures while the chart axis and tooltips on the same screen show
unconverted ones. `formatBudget` in the package editor is arguably fine (authoring in base units)
but sits next to the helper it duplicates and ships English suffixes to every locale.

**Owner.** Frontend: `formatVal` (or `formatCompactAmount` when a bare number is wanted); delete
both local formatters.

**Severity.** Wrong today for non-default currency; latent for the package editor.

### A6. Scouting search badges every outfield player red — the PR #416 bug, one more copy

**Concept.** Position → badge colour.

**Locations.**
- `src/lib/playerRating.ts:1` `positionBadgeVariant` — canonical, 17 granular positions.
- `src/components/scouting/ScoutingPlayerSearchCard.tsx:208-214` — inline ternary that knows
  only `Goalkeeper` / `Defender` / `Midfielder` / `Forward`, else `"danger"`.
- Rust: `Player::new` (`domain/src/player.rs:652-653`) sets both `natural_position` and
  `position` to the granular value the generator passes (`generator/generation.rs:328`), so on
  every generated world `player.position` on the wire is `CenterBack`, not `Defender`.

**Drifted.** Exactly the shape the dedup-reviewer prompt records for PR #416: on any current
save every `CenterBack`, `DefensiveMidfielder`, … in the scouting search table gets the red
forward badge, while the position *filter* on the same card goes through `normalisePosition`
(`ScoutingTab.model.ts:29`) and groups them correctly. **Owner:** `positionBadgeVariant`, which
is importable from `../../lib/helpers`. **Severity:** wrong today.

Related, in sync: `PackageEditor/PlayersTab.tsx:20` and `PlayerPreviewCard.tsx:66` index
`POSITION_COLOR` directly with a `"bg-gray-500"` fallback where `getPositionColor`
(`positionColors.ts:24`) uses `"bg-gray-600"`.

### A7. Live-match and post-match "Shots" omit missed penalties; the saved report counts them

**Concept.** How many shots a team had.

**Locations.**
- `src/components/match/MatchPanels.tsx:126-136` and `src/components/match/PostMatchScreen.tsx:163-173`
  — `Goal + PenaltyGoal + ShotSaved + ShotOffTarget + ShotBlocked`.
- Rust: `src-tauri/crates/engine/src/report.rs:235-238` — `PenaltyMiss` also increments
  `stats.shots` (and `penalties`); `:243` treats `ShotOnTarget | ShotSaved` alike.

**Drifted.** Any match with a missed penalty shows one fewer shot on the live stats panel and
the post-match screen than in the compact report the fixture stores (`FixtureData.result.report`,
`store/types.ts:562`), which the team-profile and player-profile cards read. **Owner:** backend
— the snapshot could carry `home_stats`/`away_stats` the same way the report does; the TS tally
should go. **Severity:** wrong today, one shot per missed penalty.

Also from this sweep: `event_type === "ShotOnTarget"` at `PostMatchHelpers.tsx:126` is a dead
branch — the engine never emits it (`report.rs:243` is the only Rust reference).

---

## B. Latent — a backend rule recomputed in TypeScript, in sync today

### B0. The session slice is rebuilt in the store instead of fetched, and it picks the user's competition by a rule of its own

**Locations.**
- Rust: `src-tauri/crates/ofm_core/src/slices/session.rs:102` `build_user_competition` —
  first competition whose `participant_ids` contains the team — behind the `get_session_state`
  command; `src/services/sessionService.ts:28` documents that payload.
- `src/store/gameStore.ts:74` `deriveSessionState` — re-implements the slice in TS (same
  first-`participant_ids` rule, then re-sorts and slices upcoming/recent fixtures and counts
  unread news) and runs on **every** `setGameState` (`gameStore.ts:218,228`).
  `get_session_state` is invoked nowhere in `src/` (only the comment names it).
- Contrast: `src/lib/fixtures.ts:133` `getUserCompetition` (League + Domestic first) and Rust
  `game.rs:214` `user_competition_index` (League first).

**Status.** The TS copy and the Rust slice agree with each other, and both disagree with the
league-mirror rule that the rest of the app uses: if a cup precedes the league in
`competitions`, `sessionState.user_competition.standings` is the cup's (empty) table while
Home shows the league. Today only `clock`/`manager`/`team` are read from `sessionState`
(`InboxTab.tsx:50`), so nothing visible depends on it — but it is a fully built, always-refreshed
mirror of a backend projection with a latent disagreement baked in.

**Owner.** Backend: one `user_competition` selector in `ofm_core` (`user_competition_index`)
used by the slice, the legacy mirror and — via the session payload — the frontend; delete
`deriveSessionState` and call `get_session_state`.

### B1. The whole season-phase / transfer-window derivation exists twice

**Locations.**
- `src/lib/seasonContext.ts:57-138` `deriveSeasonContext`, `deriveTransferWindowContext`, with
  `src/lib/domainConstants.ts:6` `TRANSFER_WINDOW_DAYS` (30 / 30).
- Rust: `src-tauri/crates/ofm_core/src/season_context.rs:7-8,14,73` — same constants, same
  algorithm, including the leap-day clamp (`addYearsClamped` ↔ `add_year_clamped`).

**Status.** In sync, and effectively dead: `Game.season_context` is not `Option`
(`game.rs:106`) and is refreshed on load (`db/src/game_persistence.rs:345`), so
`resolveSeasonContext` never reaches the fallback. Two divergences already exist inside the
dead branch: TS picks the league via `getUserCompetition`, Rust via `game.league`; TS declares
`PostSeason` when every league fixture is complete, Rust additionally requires a full schedule
(`is_league_complete`, `end_of_season.rs:65`). If the backend field is ever renamed, the
frontend silently switches to this copy and the transfer-window badges on Home, Schedule,
Tournaments, Transfers and the advance recap change behaviour with no error.

**Owner.** Backend. Delete the TS derivation; make `season_context` required in
`GameStateData` (`store/types.ts:778`) so a missing field is a type error, not a fallback.

### B2. "Season complete" is recomputed in TS although a backend command exists and is never called

**Locations.**
- `src/lib/fixtures.ts:57,66,76` `expectedFixtureCount` / `hasFullLeagueSchedule` /
  `isSeasonComplete` — mirrors `src-tauri/crates/ofm_core/src/end_of_season.rs:14,24,65`
  (double round-robin count, all League fixtures Completed).
- Caller beyond the known Dashboard one: `src/components/NextMatchDisplay.tsx:34` chooses
  between "Season complete" and "No upcoming opponent".
- Rust authority: `end_of_season.rs:88` `is_season_complete(game)` — exposed as the
  `check_season_complete` command (`src-tauri/src/commands/season.rs:8`). **No file in `src/`
  invokes it** (grep for `check_season_complete` in `src/` returns nothing).

**Status.** In sync line-for-line, but the TS rule is the strict `is_league_complete` variant,
which is false forever for any league that is not a full double round-robin (the split-season
Argentine divisions the known Dashboard bug tripped over, or a single round-robin lower tier), so
`NextMatchDisplay` will say "No upcoming opponent" at the end of such a season instead of
"Season complete". Same root cause as the known finding, different screen.

**Owner.** Backend: call `check_season_complete`, or ship `season_complete: bool` inside
`season_context` alongside `phase`; remove the three TS helpers.

### B3. Region inference for a country: TS mirrors the generator's *file-import* fallback, not the runtime rule

**Locations.**
- `src/lib/teamRegions.ts:24` `inferRegionId` — 30 country codes, everything else → `"europe"`.
- Rust import-time twin: `src-tauri/crates/ofm_core/src/generator/world_io.rs:11`
  `infer_region_id` — identical 30 codes.
- Rust runtime authority: `src-tauri/crates/ofm_core/src/game.rs:242` `region_for_country` →
  world data first, then `nations.rs:273` `region_for_code` over the 211-nation catalog, which
  knows `africa` and the full Asia/CONCACAF lists.
- Callers: `src/pages/TeamSelection.helpers.ts:41` (`buildFallbackRegions`, only when the world
  ships no `regions`), `src/pages/useTeamSelection.ts:169` (club filter by home region; mostly
  shadowed because a country is auto-selected at `:100`).

**Status.** In sync with the wrong twin. Morocco, Senegal, Egypt, Nigeria, Jamaica, Iran, UAE …
all infer to `"europe"` in TS, while the backend places them correctly. Observable today only
in a world without authored regions (the fallback path), where African clubs are grouped under
Europe on the team-selection screen.

**Owner.** Backend: `regions` are already in `GameStateData` (`store/types.ts:772`); add each
team's `region_id` to `TeamData` (or a `country → region` map to the state) and delete
`inferRegionId`. Also fold `world_io.rs:11` into `nations::region_for_code` (not my area; noting
the Rust twin so the other agent can pick it up).

### B4. Loan-period presets: hard-coded 30 June season end and 30–370 day bounds

**Locations.**
- `src/components/transfers/TransfersTab.helpers.ts:31-32` `MIN_LOAN_DAYS`/`MAX_LOAN_DAYS`
  = 30 / 370; Rust `src-tauri/crates/ofm_core/src/transfers/mod.rs:1291` `(30..=370)`. In sync.
- `TransfersTab.helpers.ts:79` `nextSeasonEnd` — 30 June, always. The backend has no such
  constant; season end is derived from fixtures (`season_context.rs:20`, `season_end`) and is
  already shipped in `season_context.season_end` (`store/types.ts:670`).

**Status.** Distinct from the known "January window" finding: the "End of season" preset is
wrong for every southern-hemisphere league (Argentine/Brazilian seasons end in December) and for
any authored calendar, and the preset is silently disabled (`disabledReasonKey`) when the
370-day cap trips. `season_end` is already available on the client and unused here.

**Owner.** Frontend, reading `season_context.season_end`; the 30/370 bounds should come from
the backend error rather than being pre-validated in TS.

### B5. Contract risk badge: a 2-tier projection of the backend's 4-tier warning stages

- `src/lib/contractUtils.ts:13` + `src/lib/domainConstants.ts:1` — critical ≤180 d, warning
  ≤365 d; `getDaysUntil` (`contractUtils.ts:5`) is local-time `Math.ceil`.
- Rust: `src-tauri/crates/ofm_core/src/contracts/expiry.rs:8` `contract_warning_stage` — 30 /
  90 / 180 / 365; `contracts/helpers.rs:84` and `transfers/mod.rs:324` reuse 180/365 (and 60)
  for wage and fee multipliers.

In sync on the two thresholds both sides share; the TS badge cannot show "final weeks" or
"three months" though the inbox does. Latent. Owner: backend — a `contract_stage` per player
would replace four sites that agree by coincidence.

### B6. Finance snapshot re-derived locally and dressed up as the backend's type

Beyond the known runway omission: `src/components/finances/FinancesTab.helpers.ts:97`
`mapLocalFinanceSnapshot` casts the local `TeamFinanceSnapshot` into `TeamFinanceSnapshotData`
(the service type), so the Finances tab, `dashboardHelpers.ts:104` alerts and the three
"available?" gates (`FinancesTab.helpers.ts:74-95`) run on the local copy without any type
telling a reader. Thresholds checked and in sync: wage 110/100/85 (`finance.ts:127` ↔
`finances.rs:114`), runway 4/8/12 (`finance.ts:143` ↔ `finances.rs:130`), marketing cooldown
28 d (`finance.ts:19` ↔ `finances.rs:16`), facility cost `level × 250 000`
(`FinancesTab.helpers.ts:47` ↔ `club.rs:3,26`). Latent; owner backend (it already serialises
`FinanceSnapshotData`).

### B7. Home tab morale bands off by one from the backend's

`src/components/home/HomeTab.helpers.ts:184-188` — hot `morale >= 80`, cold `morale <= 40`;
Rust `src-tauri/crates/ofm_core/src/random_events/mod.rs:320-321` — low `< 40`, high `>= 80`
(`contracts/renewals.rs:451` and `transfers/mod.rs:339` use `<= 40`). A player on exactly 40
is "cold" on Home and not "low morale" in the dressing-room report. Latent. **Owner:** backend
— Rust itself uses both `< 40` and `<= 40`; pick one constant in `ofm_core` and ship a
`morale_band` (or the thresholds) so Home, the hero card and the match screens (C5) stop
choosing their own.

### B8. Youth-academy age gate

`src/lib/playerSquad.ts:23` `canDelegateToYouthAcademy` — `age <= 21`; Rust
`generator/mod.rs:47` `OPENING_YOUTH_MAX_AGE = 21`, `training.rs:269,417`, `season_awards.rs:331`
all use 21. In sync; four Rust sites plus one TS site share a literal with no shared constant.

### B8a. Extra-time rule, restore path only

`src/pages/MatchSimulation.tsx:155` hard-codes which competition *kinds* allow extra time
(Cup, ContinentalClub, InternationalClub, InternationalNation, FriendlyCup); Rust
`src-tauri/src/application/time_advancement.rs:75` decides per *fixture* via
`league.is_knockout_fixture`. Group-stage games of a GroupAndKnockout competition diverge.
Reached only when `routeState.snapshot.allows_extra_time` is missing (a restore after
restart). Latent.

### B8b. Staff "overall" and training-focus attribute sets

`src/components/staff/StaffTab.tsx:71` `ROLE_ATTR_WEIGHTS` — AssistantManager 4/3/3 matches
`delegated_renewals.rs:256` (in sync; the file comment cites a non-existent
`assistant_quality` function); Coach/Scout/Physio weights are TS-only inventions.
`src/components/training/TrainingTab.tsx:51` `TRAINING_FOCUS_ATTRS` mirrors
`ofm_core/src/training.rs:364` `apply_focus_gains` attribute-for-attribute (in sync).
`src/components/TraitBadge.tsx:46` `TRAIT_META` matches `domain/player.rs:553` `compute_traits`
and the wonderkid rule (20 / 90 / 14) in `player_rating.rs:4` (in sync; the enum comment at
`player.rs:549` still quotes the old 21 / 85 / 10).

### B9. Scout capacity

`src/components/scouting/ScoutingTab.helpers.ts:5` `scoutMaxSlots` returns 1 and ignores
ability; Rust `scouting.rs:42` `scout_max_assignments` does the same. In sync; the TS copy is
used to grey out scouts client-side, so a backend change to "2 for elite scouts" would leave the
UI hiding scouts the backend accepts. **Owner:** backend — put `max_assignments` on the
serialised staff member (or on `ScoutingAssignment` counts) and delete `scoutMaxSlots`.

---

## C. Latent — helpers and constants with two homes inside `src/`

### C1. `getTeamName` twice, and the shared one is untranslated

- `src/lib/team.ts:3` `getTeamName` — returns the English literals `"Free Agent"` /
  `"Unknown"` / `"???"`.
- `src/components/playerProfile/PlayerProfile.helpers.ts:52` `getPlayerTeamName` — identical
  logic with translated labels passed in.
The profile copy exists *because* the lib one cannot be translated. **Owner:** frontend
(`src/lib/team.ts`) — give `getTeamName` the labels parameter (or a `t`) and delete the
profile copy. (Rust has the same English
literal at `end_of_season.rs:38` `free_agent_team_name` — not my area.)

### C2. Reputation bands and their labels, twice in one folder

`src/components/menu/PackageEditor/TeamPreviewCard.tsx:6` `REP_TIERS` (850/720/550/300 with
English labels "Elite", "Top", "Mid", "Lower", "Amateur") and
`src/components/menu/PackageEditor/teamRanges.helpers.ts:26` `REPUTATION_BANDS` (same
thresholds, i18n keys). In sync; the preview card's labels never hit `t()`.
`teamRanges.helpers.ts:13` `TRANSFER_BUDGET_SHARE = 0.15` mirrors `generator/mod.rs:632`
`finance * 0.15` — in sync. **Owner:** frontend for the bands (`REPUTATION_BANDS` is the one
with keys; the card should import it); backend for the 0.15 (expose it with the package
validation result rather than promising it in the editor).

### C3. Youth potential tier labels, twice

`src/components/inbox/InboxMessageDetailPane.tsx:553` `getProspectPotentialLabel` and
`src/components/youthAcademy/YouthAcademyTab.tsx:43` `getPotentialLabel` — byte-identical
85/75/65/55 tables and colour classes. The backend's scout reports use their own tiers
(`scouting.rs:855`, `>= 85` / `>= 70` on a fuzzed value) with separate keys, so a scouted
prospect and a youth-intake prospect with the same potential get different words. Latent.
**Owner:** backend — youth prospects already travel as `PlayerData` inside the message
(`store/types.ts:445`); attach a `potential_key` the way scout reports do and the two TS
tables go.

### C4. Manager win percentage, twice

`src/components/manager/ManagersWorldTab.tsx:33` and `src/components/manager/ManagerTab.tsx:68`
compute `wins / matches × 100` with different rounding calls (`Math.round` vs `toFixed(0)`),
and disagree on the empty case ("0%" vs "—"); the backend computes `win_rate` for
`manager_of_season` (`store/types.ts:714`). In sync. **Owner:** backend — add `win_rate` to
`ManagerCareerStats` next to `wins`/`draws`/`losses`.

### C5. Colour threshold tables for 0–99 scales — the lib helpers exist and are mostly bypassed

*Provenance for C5–C5c: the `file:line` lists below come from a delegated constant sweep of
`src/components/`; I re-verified the lib helpers, the `MatchPanels` shadowing, the `aerial`
grouping and the `Settings` currency list by hand, not every individual threshold line.*

Canonical: `src/lib/playerAttributeDisplay.ts:9` (70/40, matches `ui/ProgressBar.tsx:19`
`auto`) and `src/lib/playerConditionDisplay.ts:9` (75/50).

- Attribute colours, inline and all different from lib: `TacticsPlayerFocusPanel.tsx:46`
  (80/65/50), `PackageEditor/PlayerPreviewCard.tsx:8` (80/65/50/35), `PreMatchLineup.tsx:36`
  (75/60), `StaffTab.tsx:449` (70/50).
- OVR colours, no lib home, seven tables: 75/55 (`PlayersListTab.tsx:527`,
  `TeamProfileRosterCard.tsx:132`, `TransfersTab.tsx:1532`, `PlayerProfileHeroCard.tsx:72`,
  `TacticsTab.helpers.ts:445`), 80/55 (`SquadRosterView.tsx:912`), 80/60
  (`TacticsPlayerList.tsx:133`, `PreMatchLineup.tsx:447`), 70/50 (`PreMatchLineup.tsx:43`),
  80/65 (`PlayerPreviewCard.tsx:121`).
- Condition colours: `match/MatchPanels.tsx:245` declares a local `condColor` (70/40, yellow)
  that **shadows the lib name** while the adjacent `SubPanel` imports the lib's 75/50; further
  copies at `YouthAcademyTab.tsx:577` (70/40), `PlayerProfileHeroCard.tsx:164` (70),
  `FormationPitch.tsx:290` (50), `TrainingTab.tsx:310` (25/40), `ui/PitchToken.tsx:59` and the
  dead `TacticsPlayerTable.tsx:176` (90/75/60).
- Morale colours: `HalfTimeBreak.tsx:388` = `PostMatchScreen.tsx:547` (70/40) vs
  `PlayerProfileHeroCard.tsx:169` (70) vs `HomeTab.helpers.ts:184` (80/40, see B7).

Same value, different colour depending on which screen the player is looking at. Latent but
visible. **Owner:** frontend — one `ratingTone(value)` in `src/lib/` next to the two existing
helpers, with the OVR table added there; the sweep's list is the migration checklist.

### C5a. Attribute grouping disagrees on `aerial`

`src/components/playerProfile/PlayerProfile.attributes.ts:32` puts `aerial` under **physical**
(with a `satisfies` guard and an engine rationale); `tactics/TacticsPlayerFocusPanel.tsx:34`
and `PackageEditor/helpers.ts:81` put it under **goalkeeper**. The same attribute is shown in
a different section on the profile, the tactics focus panel and the editor. Drifted (cosmetic).
**Owner:** frontend — `ATTRIBUTE_META` is the guarded one; the other two should derive their
groups from it.

### C5b. Coarse position list and formation splitter, copied inline

`CORE_POSITIONS` (`SquadTab.helpers.ts:29`) is re-typed at `MatchPanels.tsx:212`,
`PreMatchSetup.tsx:308,383`, `PreMatchLineup.tsx:183,202`, `PlayersListTab.tsx:219`,
`TransfersTab.tsx:738`, `ScoutingPlayerSearchCard.tsx:19`, `ScoutingYouthRecruitmentCard.tsx:156`
— none import it. `parseFormationSlots` (`SquadTab.helpers.ts:195`) is re-implemented as
`parseFormationNeeds` (`PreMatchLineup.tsx:116`, same fallback) and a third splitter in
`FormationPitch.tsx:62` (different fallback). `pages/SimLab.tsx:80` carries its own
`FORMATIONS` missing `4-1-4-1` (dev page, unlinked). `transfers/TransfersTab.model.ts:7`
`SPECIFIC_POSITIONS_BY_GROUP` lists the same members as `GROUP_ROLE_PREFERENCES`
(`SquadTab.helpers.ts:171`) in a different order (checklist vs ranking — probably deliberate).
All in sync.

### C5c. Verbatim blocks duplicated across feature folders

**Owner** for all of these: frontend, the existing helper or a new one in `src/lib/`; the
`CURRENCY_OPTIONS` item is backend-owned (read `supportedCurrencies` from the store).

- Transfer-window badge variant + summary: `home/HomeTab.tsx:105-124` = `transfers/TransfersTab.tsx:633-652`.
- Youth-scouting region/objective → i18n: `ScoutingYouthRecruitmentCard.tsx:54-62` =
  `InboxMessageDetailPane.tsx:528-550` (values match Rust `game.rs:42,49`).
- Play-style icon arrays: `TacticsCommandBar.tsx:50` = `MatchLive.tsx:389`; `KitPattern` list
  ×4 (`store/types.ts:6`, `PackageEditor/types.ts:8`, `TeamForm.tsx:18`, dead
  `KitEditorCard.tsx:11`); `CompetitionScope` re-declared at `CompetitionsOverview.tsx:14`.
- Attribute abbreviations as hard-coded English (`PreMatchLineup.tsx:10`,
  `SetPieceSelector.tsx:9-60`, `SubPanel.tsx:668`), never through `t()`.
- `pages/Settings.tsx:26` `CURRENCY_OPTIONS` hard-codes EUR/GBP/USD with symbols although
  `settingsStore.ts:101` already receives `supported_currencies` from Rust
  (`ofm_core/src/currency.rs:14-24` — same three today).
- `PackageEditor/PlayerPreviewCard.tsx:44` declares a local `calcAge` (365.25-day years from
  `Date.now()`) that shadows the lib name.
- Percentage formatting: possession `.toFixed(0)` in match screens vs `.toFixed(1)` in
  `TeamProfileRecentMatchesCard.tsx:64`; `formatPercentage` in
  `PlayerProfileAdvancedStatsCard.tsx:25` (0 dp) vs `TeamProfileAdvancedStatsCard.tsx:28`
  (1 dp); player-of-the-year rating `.toFixed(1)` (`AwardsCeremonyScreen.tsx:175`) vs
  `.toFixed(2)` (`TournamentsAwardCard.tsx:128`).
- `dashboard/DashboardSimulatingModal.tsx:173` formats a date with the OS locale
  (`Intl.DateTimeFormat(undefined, …)`) instead of `getLocale(i18n.language)`.

### C6. Locale lists — two more hand-maintained copies

Adjacent to the known eleven/twelve mismatch, two further lists that nothing reads
`SUPPORTED_LANGUAGES` for: `src/lib/dateFormatting.ts:3` `LANG_LOCALE` (9 entries, keys `zh`
not `zh-CN`, `pt` → `pt-BR`, no `ru`/`pt-BR`; harmless only because of the `|| lang`
fallback) and `src/lib/countries.ts:20` `SUPPORTED_LOCALES` (10 entries, `zh` not `zh-CN`,
mapped through `getBaseLocale`). Latent. **Owner:** frontend — derive both from `SUPPORTED_LANGUAGES`
(`src/i18n/index.ts`) and add the missing gate test the memory note already calls for.

### C7. `isPendingSponsorOffer` and the player-event message-id prefixes

`src/components/finances/FinancesTab.helpers.ts:181` matches `id.startsWith("sponsor_")` +
category `Finance` + an unresolved `ChooseOption`; Rust `finances.rs:519` matches
`starts_with("sponsor_")` + any unresolved action. Both ids the backend emits
(`sponsor_pitch_…` `finances.rs:525`, `sponsor_{date}` `random_events/mod.rs:96`) satisfy both.
`src/components/inbox/inboxHelpers.tsx:69` `PLAYER_EVENT_MESSAGE_PREFIXES` = the four prefixes
in `player_events/responses.rs:112-118`. In sync; message-id string conventions are the
contract and nothing pins them. **Owner:** backend — a `kind` field on `InboxMessage`
(`message.rs:52`) would replace prefix-sniffing on both sides.

### C8. Knockout round names matched as English literals

`src/components/tournaments/TournamentsTab.helpers.ts:24` `localizedRoundName` matches
`"Final"`, `"Semifinal"`, `"Quarterfinal"`, `/^Round of (\d+)$/`. Rust emits exactly these
(`schedule.rs:252-255`, `world_cup.rs:1221,1326`, `catchup.rs:162`). In sync; a Rust rename to
"Semi-final" would fall through to the raw English string in every locale. Owner: backend
should emit a `name_key` as competitions already do (`store/types.ts:629`).

---

## D. Enum mirrors in `src/store/types.ts` — checked against `src-tauri/crates/domain/src/*.rs`

In sync, variant-for-variant and spelling-for-spelling: `KitPattern` (5), `TransactionKind`
(4), `PlayerRole` (27), the nine `tactics_phase` enums, `PlayerMovementKind` (snake_case, matches
`#[serde(rename_all = "snake_case")]`), `ContractExitIntentData` (`kind: "let_expire"`, matches
the tagged repr), `RenewalSessionStatus` (5), `PlayerSquadRole`, transfer/loan offer status (5),
`StaffRole` (4), `MessageAction.action_type` (externally tagged, 4), `FixtureData.competition`
(8), `FixtureStatus` (3), `CompetitionFormat` (3), `SeasonPhase` (3), `TransferWindowStatus` (3).
`src/components/match/types.ts:96` `MatchSnapshot` matches `engine/src/live_match/mod.rs:127`
field-for-field; all 16 `event_type` literals used in `src/components/match/` exist in
`engine/src/event.rs:20` `EventType`.

Typed as loose `string` (so a Rust rename breaks nothing visibly): `PlayerData.position` /
`natural_position` / `footedness` / `training_focus` / `traits`, `TeamData.play_style` /
`training_focus` / `training_intensity` / `training_schedule`, `StaffData.specialization`,
`MessageData.category` / `priority` / `sender_role`, `NewsArticle.category`,
`LeagueData.kind` / `scope`, `CompactMatchEventData.event_type`, `BoardObjective.objective_type`.
`RenewalSessionOutcome` is mirrored as `last_outcome?: string | null` although Rust has six
variants. No drift found in these today; they are the unguarded ones.

Every string literal compared against a backend enum in `src/components/` was checked against
its Rust enum (`event.rs`, `live_match/mod.rs`, `league.rs`, `message.rs`, `player.rs`,
`contracts/mod.rs`): no invented values. One intentional double-casing to know about:
`renewal_state.status === "Blocked"` (serde PascalCase, `PlayerProfile.tsx:297`) alongside
`session_status === "blocked"` (lower-case from `commands/contracts.rs:75`,
`PlayerProfile.tsx:513`) — documented in `store/types.ts:211`, but two spellings of one Rust
enum on one screen.

---

## E. Checked, genuinely in sync (or duplicated and fine)

- `src/lib/playerRoles.ts:9` `ROLE_OPTIONS_BY_POSITION` ↔ `src-tauri/src/commands/squad.rs:483`
  `role_valid_for_position`: identical per granular position; both sides have a pinning test.
- `SquadTab.helpers.ts:76` `POSITION_GROUPS` ↔ `domain/player.rs:134` `to_group_position`.
- `SquadTab.helpers.ts:116` `POSITION_CODES` (DEF/MID/FWD) vs MCP `tools_impl/helpers.rs:35`
  (DF/MF/FW): different surface, different audience — fine.
- `SquadTab.helpers.ts:36` `CANONICAL_POSITION_MAP` (accepts `gk`, `wingback`, `centreback` …)
  vs `db/src/repositories/player_repo.rs:143` `parse_position` (exact names, unknown →
  Midfielder): different projections (editor input vs persistence); fine.
- `src/components/match/types.ts:244` `FORMATIONS` — the only "valid formations" list in the
  repo; Rust `set_formation` (`commands/squad.rs:30`) validates nothing and
  `player_rating.rs:104` falls back to 4-4-2 for anything unparseable. Not a duplicate, but the
  rule lives on the wrong side.
- `src/utils/newsVisibility.ts:17` ↔ `ofm_core/src/slices/news.rs:30` `article_is_visible`:
  same day-prefix comparison.
- `src/lib/fixtures.ts:133` `getUserCompetition` ↔ `game.rs:214` `user_competition_index`: TS
  additionally requires `scope === "Domestic"` and also accepts fixture participation; Rust
  checks `kind == League` and standings/participants. Same answer for every generated world.
- `src/components/match/SubPanel.helpers.ts` substitution recommendations vs
  `engine/src/ai.rs:125-208`: different purpose (advice vs AI behaviour), different thresholds
  by design — fine.
- `SquadTab.helpers.ts:628` `getPlayStyleFit` attribute sets per play style: TS-only heuristic;
  the engine (`shared.rs:190`) models play styles as phase multipliers, not attribute sets. Not
  a mirror — but like the post-match rating it is a number the UI invents and labels as fit.
- `TacticsTab.helpers.ts:48` `TACTICS_PRESETS` and `engine/ai.rs:401` formation switches: the
  AI's `4-4-2 → 4-3-3 → 4-2-3-1` ladder is its own rule, not a copy.
- `HallOfFameWorldTab.model.ts:88` `derivePastChampions` treats `league_position === 1` in
  `team.history` as a title regardless of tier (`TeamSeasonRecord` carries no competition);
  the backend has no league-champion history of its own (`world_history.rs` records World Cup
  champions and awards only), so this is a data-model gap rather than a duplicate.
- `src/lib/injury.ts:12` severity bands (3/7/14 days) — display-only; no Rust classification
  of injury days exists to drift from.
- `src/services/*.ts` — thin `invoke` wrappers; nothing re-derives a backend response beyond
  `settingsStore.ts:101` indexing `supported_currencies` (which does come from Rust).

---

## F. UI shells rebuilt from scratch (the `ofm-dedup-reviewer` lens)

Findings from a full sweep of `src/components/` and `src/pages/`; the load-bearing claims
(dead components, modal semantics, sort headers, identical modal strings) were re-checked by
hand. None of these encode a game rule, so they are all latent unless marked otherwise.

### F1. There is no `Modal` primitive; the de facto one lives in `dashboard/` and has no dialog semantics

- `src/components/dashboard/DashboardModalFrame.tsx:13` — 12 consumers (all dashboard/inbox/
  menu/playerProfile confirm modals). Visual only: no `role="dialog"`, `aria-modal`,
  `aria-labelledby`, Escape handling, backdrop close or focus trap (grep confirms none of its
  consumers add them).
- Four transfers modals build their own byte-identical overlay+panel
  (`fixed inset-0 bg-black/50 flex items-center justify-center z-50`):
  `transfers/LoanOfferModal.tsx:341`, `FreeAgentContractModal.tsx:212`,
  `TransferBidModal.tsx:258`, `TransferCounterOfferModal.tsx:69`. The first three are *better*
  than the frame (role, aria-modal, aria-labelledby, backdrop close); the fourth has none.
- Other one-offs: `dashboard/FiredModal.tsx:19`, `match/SubPanel.tsx:179`,
  `match/RoundDigestScreen.tsx:515` (the only Escape handler in the app, `:49`),
  `tactics/TacticsTab.tsx:233`.
- No focus trap or focus restore exists anywhere in `src/`, against
  `.claude/skills/new-ui-surface/SKILL.md:63`.

**Owner.** Promote a `Modal` to `src/components/ui/` with `labelledBy`, `onClose` (Escape +
backdrop) and a focus trap, then move the 12 + 4 + 4 shells onto it. Moving the transfers modals
onto the frame *as it stands* would lose accessibility — the frame must grow first.
**Wrong today** in the sense that keyboard users cannot dismiss most modals.

### F2. Four `SortHeader` implementations, one keyboard-reachable

`players/PlayersListTab.tsx:660` (takes an `asc` prop it never uses),
`squad/SquadRosterView.tsx:423` (defined inside render — remounts every render),
`transfers/TransfersTab.tsx:1349` (inline), `tactics/TacticsPlayerTable.tsx:94` (dead file).
Only the Transfers one has `aria-sort`, `tabIndex` and an Enter/Space handler; the others are
`<th onClick>`. A shared `SortHeader` in `ui/` fixes the a11y gap in three tables at once.
Related: the table header row
`bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs` is pasted
12× (FinancesPayrollTable:32, ManagerTab:126, ScheduleTab:730, PlayersListTab:329,
TeamProfileRosterCard:39, TeamProfileHistoryCard:34, SquadRosterView:615, TournamentsTab:561,763,
TransfersTab:1330, YouthAcademyTab:436, TacticsPlayerTable:683) and the `<th>` class string
~54×.

### F3. Four dead components (0 import sites, tests included)

`src/components/squad/JerseyNumberInput.tsx` (126 lines), `squad/KitEditorCard.tsx` (131),
`tactics/TacticsPlayerTable.tsx` (784 — holds a SortHeader and an empty-state copy),
`tactics/TacticsRolesPanel.tsx` (183 — still imports `resolveEffectiveMatchRoles`, so it drags
finding A3 along). Already named in the dedup-reviewer prompt; still present.

### F4. `ui/Button` is missing the variants people keep rebuilding

12 filled-primary CTAs rebuilt natively without Button's focus ring (the four transfers modals
at `LoanOfferModal.tsx:321`, `FreeAgentContractModal.tsx:188`, `TransferBidModal.tsx:236`,
`TransferCounterOfferModal.tsx:168`; `EndOfSeasonScreen.tsx:191,231`;
`DashboardCloseConfirmModal.tsx:30`; `DashboardSimulatingModal.tsx:366`;
`home/JobOpportunitiesCard.tsx:223`; `season/AwardsCeremonyScreen.tsx:153`;
`tournaments/TournamentsTab.tsx:1002`; `inbox/inboxHelpers.tsx:252`). Causes: no `danger`
variant (three inbox sites override primary with `bg-red-500` on top, six sites go native), no
gradient CTA variant (~15 sites), no `flex-1`/full-width option.

### F5. Tab bars, filter chips, segmented controls — four dialects, one with roles

Only `match/PostMatchScreen.tsx:381` uses `role="tablist"/"tab"/aria-selected`. Big-pill tabs in
`StaffTab.tsx:178`, `TournamentsTab.tsx:472`, `ScheduleTab.tsx:273`, `TransfersTab.tsx:1111`
(active `primary-700` here, `primary-500` elsewhere — Transfers matches `ui/Button`, the others
don't). The filter-chip class string that `inbox/inboxHelpers.tsx:47` already hoists into
constants is pasted again in `PlayersListTab.tsx:239-279` (5×), `TransfersTab.tsx:1154,1190,
1250,1264`, `StaffTab.tsx:213,224`, `NewsTab.tsx` (2×), `ScoutingPlayerSearchCard.tsx:82`. The
CardHeader segmented control in `PlayerProfileAttributesCard.tsx:50-66` and
`TeamProfileAdvancedStatsCard.tsx:125-141` is identical markup in two folders.

### F6. Smaller shells with two or three homes

- Empty state: no primitive; ~25 hand-written copies (`NextMatchDisplay.tsx:23,32` has two in
  one file) with padding, grey shade and text size all drifting.
- `CountryCombobox`: `menu/CreateManagerNationalityField.tsx` (~270 lines) re-implements
  `ui/CountryCombobox.tsx` (same loader, cache, accent-folding filter) to add an `error` prop
  and open upward.
- PackageEditor `primitives.tsx` exports `inputClass`/`labelClass`/`LabeledInput`, yet
  `PlayerForm.tsx:59`, `CountryForm.tsx:38`, `NamesPoolForm.tsx:31` re-declare them and
  `StaffForm.tsx:187`, `TeamForm.tsx:203,227`, `ui/CountryCombobox.tsx:99` paste the literal;
  `worldEditor/WorldEditorHome.tsx:149-197` uses a third label/input style with no
  `htmlFor`/`id` association.
- `StatTile` ×3 (`HallOfFameWorldTab.tsx:282`, `ManagersWorldTab.tsx:249`,
  `TransferCentreWorldTab.tsx:309`), `InfoRow` ×2 (`TeamProfile.primitives.tsx:24` exported,
  private copy `PlayerProfileContractCard.tsx:187`), `StatBox` ×2, `QuickStat` ×2 with the same
  props, search-input-with-icon ×6 (three PackageEditor tabs identical).
- `StaffForm.tsx:160` uses a native `type="date"` while `PlayerForm.tsx:212` in the same
  folder uses `ui/DatePicker`; `LoanOfferModal.tsx:137` is the one native `<select>` outside
  the SimLab dev page.
- Doc drift: `.claude/skills/new-ui-surface/SKILL.md:18-21` claims the barrel exports
  `AssetImage`, `GeneratedAvatar`, `GeneratedCrest`, `CountryCombobox`; `ui/index.ts` does not.
