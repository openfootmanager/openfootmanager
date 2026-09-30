# Duplication audit — `src-tauri/crates/ofm_core/src`

**Sweep A — 22–24 September 2026, at `c8840375`.**

Scope: everything under `ofm_core/src`, about 49.6k lines at that revision. Read and search only;
nothing compiled. Line numbers are from that revision and have moved since — `transfers/` was then
`mod.rs` + `execution.rs`, so a site now in `transfers/market.rs` appears here as
`transfers/mod.rs:292`. Every `file:line` was re-checked against the source before being recorded.

Findings already known when this sweep ran are not repeated. Where one of them has a newly found
neighbour, it is in §D.

Ranking: §A is verified divergence a player or package author can observe today. §B is
divergence that may be deliberate — the maintainer should decide. §C is in-sync copies that can
drift. §D is adjacent to the known list. §E is what was checked and found genuinely single-source.

---

## A. Drifted — observable today

### A1. Who pays a loaned player's wage: three rules, three answers

**Concept.** The share of a loaned player's wage that each club carries.

**Locations.**
- `finances.rs:901-920` — `process_weekly_finances`, the only code that debits money. Line 915:
  `wage_by_team[player.team_id] += player.wage / 52`. `player.team_id` is the **borrowing** club
  during a loan (`transfers/execution.rs:90`). The parent is charged nothing.
  `wage_contribution_pct` is stored (`transfers/execution.rs:94-100`) and never read by any
  ledger code; `grep -i wage transfers/execution.rs` finds only the field.
- `finances.rs:215-231` — `player_annual_wage_for_team`: borrower pays
  `wage * pct / 100`, parent pays the rest. Feeds `calc_wages` (`:181`) and
  `calc_annual_wages` (`:198`), which feed the finance snapshot (`:272-273`), the weekly
  finance warnings (`:1032-1033`), the loan-affordability check (`transfers/mod.rs:62`) and the
  transfer-bid projection (`transfers/mod.rs:1244`).
- `contract_wage_policy.rs:22-44` — `annual_team_wage_bill` uses `contract_owner_team_id`, so
  the **parent** carries 100% and the borrower 0%. Feeds `renewal_wage_policy_allows` and
  `project_contract_offer_financial_impact` (`:55-80`).

**Drifted.** Take a 520k/yr player loaned at 30%. The Loan Desk affordability check and the
borrower's finance page say +3,000/wk. Every Monday the ledger takes 10,000/wk from the borrower
and 0 from the parent. The parent's finance page shows a 7,000/wk expense that is never charged.
The renewal wage-policy cap counts the full 10,000 against the parent and nothing against the
borrower. Three screens, three numbers, one of which moves the money.

**Source of truth.** `player_annual_wage_for_team` — it is private and in the same file as the
weekly pass, so the fix is local: the weekly loop should sum it (or a shared
`weekly_wage_share(player, team_id)`) instead of `player.team_id`. `contract_wage_policy` should
call `calc_annual_wages`; note the design tension — a renewal concerns the owner's contract, but
the budget cap should count what the club actually pays — so that is a decision, not a mechanical
change.

**Severity.** Wrong result today; it is money.

### A2. `OPENING_SHORT_CONTRACT_END` works for exactly one opening year

**Concept.** "A contract that expires at the end of the opening season."

**Locations.**
- `generator/mod.rs:45` — `const OPENING_SHORT_CONTRACT_END: &str = "2027-06-30"`.
- `generator/mod.rs:83-101` — `normalize_opening_contracts` caps expiring contracts at
  `MAX_OPENING_EXPIRING_CONTRACTS` (2) by filtering on that literal (`:87`).
- `generator/generation.rs:326` and `:711` — generated ends are
  `format!("{}-06-30", opening_year + contract_years)`.
- `generator/mod.rs:110,1027-1030` — the opening year is author/config supplied, clamped to
  `MIN_OPENING_YEAR` (1900)..`MAX_OPENING_YEAR`.
- `generator/mod.rs:1847` — the only test, at `TEST_OPENING_YEAR` 2026.

**Drifted.** The literal is `opening_year + 1` evaluated for 2026 only. For any other opening
year the filter matches nothing and the cap is a no-op. `generation.rs:315-323` gives 40% of
players aged 32+ a one-year deal, so a career opened in 2027 (three months from now) or a 1962
historical package starts with an uncapped pile of expiring contracts and a first-season inbox
full of expiry warnings.

**Source of truth.** `normalize_generated_team` (`generator/mod.rs:325`) already receives
`opening_year` and calls `normalize_opening_contracts(players)` at `:327` without it. Pass it
and build the string. Delete the const.

**Severity.** Wrong result for every non-2026 opening year.

### A3. Country → region inference: `world_io.rs` contradicts the "single source of truth"

**Concept.** Which region/confederation a country code belongs to, and the region's display name.

**Locations.**
- `nations.rs:271-276` — `region_for_code`, whose doc comment says "Single source of truth for
  region inference across the generator, competitions, and the UI". Catalog: 54 nations in
  `africa`, 45 in `asia`, 32 in `central-america`, 12 in `oceania`.
- `generator/world_io.rs:11-22` — `infer_region_id`, a hand-typed 27-code subset with **no
  Africa arm**; everything unlisted falls to `"europe"`.
- `generator/world_io.rs:24-33` — `region_name`, six arms, also no `"africa"`.
- `generator/mod.rs:844-856` — `builtin_region_name`, seven arms including Africa; and
  `regions_from_package` (`:861-912`) which correctly uses `nations::region_for_code`.
- `generator/world_io.rs:46-70` — `infer_world_regions` calls the wrong pair; `:91`
  `normalize_world` runs it whenever `world.regions` is empty; `:170` `world_data_from_parts`
  passes `regions: vec![]`, so every randomly generated world goes through it.

**Drifted.** Nigeria, Egypt, Morocco, Senegal (all 54 African codes), Iran, Uzbekistan, Iraq,
Thailand, India, Indonesia, Vietnam, Jamaica, Cuba → `"europe"`, named "Europe". The package
path is safe (`regions_from_package` fills `regions` before `normalize_world` sees it). The
built-in random data is eight European countries (`generator/data.rs:560-665`), so the default
career is right by luck. It bites two inputs: a legacy world JSON without a `regions` section,
and a random world generated from a custom definitions directory with non-European clubs.

**Source of truth.** `nations::region_for_code` + `generator::builtin_region_name`
(make it `pub(crate)`). Delete `infer_region_id` and `region_name`.

**Severity.** Wrong result for those inputs; doc-vs-code. Strongest instance of that shape found.

### A4. When an N-year contract ends: season-aligned in the generator, anniversary everywhere else

**Concept.** The end date of a contract of `N` years.

**Locations.**
- Generator: June 30 of `opening_year + N` — `generator/generation.rs:326`, `:711`,
  `generator/mod.rs:100`.
- Renewal / free agent / delegated renewal: `current_date + 12N months` —
  `contracts/renewals.rs:296-298`, `contracts/free_agent.rs:154-156`,
  `delegated_renewals.rs:172-174`. Three byte-identical copies; the error const they share is
  itself declared twice (`contracts/consts.rs:14`, `delegated_renewals.rs:18`).

**Drifted.** A renewal agreed on 15 March for two years expires 15 March, mid-season. The
contract list then shows "expires 15 Mar 2029" beside generated "expires 30 Jun 2029", the
180/365-day risk thresholds fire mid-season, and the player walks in March. Generated contracts
can never do this.

**Source of truth.** One `contract_end_after(current_date, years)` in `contracts/helpers.rs`,
with the rule chosen once (season-aligned is what the generator and real football do).

**Severity.** Observable in the first renewal; whether it is a bug or an accepted gap is the
maintainer's call, which is why it is here and not in §B.

### A5. Contract length by age: the authored-player path has a different table

**Concept.** How many years a generated player's opening contract runs.

**Locations.**
- `generator/generation.rs:315-323` (random players): ≤21 → 3–5, ≤27 → 2–4, ≤31 → 2–3,
  else 40% one year / 60% two.
- `generator/generation.rs:706-710` (authored players): ≤27 → 2–5, else 1–3.
- `contracts/helpers.rs:120-131` — `expected_contract_years` (renewal ask): ≤28 → 3, ≤32 → 2,
  else 1.
- The surrounding age-factor / market-value / wage block is a byte copy at
  `generation.rs:303-314` and `:695-705`, with literals `200` and `500` duplicating
  `MARKET_VALUE_TO_WAGE_RATIO` and `MINIMUM_DEFAULT_WAGE` (`contracts/consts.rs:7-8`).

**Drifted.** A package author's 19-year-old can open on a two-year deal; a generated 19-year-old
never gets fewer than three. An authored 34-year-old can get three years; a generated one never
more than two.

**Source of truth.** One `opening_contract_years(age, rng)` in `generation.rs`, and the
value/wage block extracted once. The renewal table is a different question (what the player asks
for) and can stay separate.

**Severity.** Observable to package authors comparing squads; low impact.

### A6. "How strong is this side": five calculators, two fallbacks, three rosters

**Concept.** A scalar strength for a team, used to simulate, seed, and judge results.

**Locations.**
- `turn/round_summary.rs:343-370` `team_strength`: mean `effective_rating_for_assignment` over
  the saved XI; if no XI, mean `natural_ovr` over the **whole squad**; empty → `0.0`. Feeds
  `build_notable_upset` (`:172`).
- `catchup.rs:16-29` `club_strength`: best-11 `ovr` mean; empty → `50.0`. Feeds dormant-league
  sims (`turn/dormant.rs:39`), catch-up, and the shootout for level engine ties
  (`turn/mod.rs:502`).
- `national_team.rs:213-224` `squad_strength`: best-11 `ovr` from a squad list; empty → `50.0`.
- `world_cup.rs:94-100` `pool_strength`: top-11 `ovr`; empty → `0.0`, and callers at
  `:126/:191/:210` `unwrap_or(0.0)`.
- `slices/teams/projection.rs:19-24` `avg_ovr`: whole-roster integer mean, for the team card.

**Drifted.**
- Fallbacks: a nation with no eligible players ranks last for World Cup selection (`0.0`) but
  plays as a 50-rated side once selected (`squad_strength` → `50.0`).
- Roster: a club with no saved XI is rated on its whole squad by the round summary and on its best
  eleven by the simulator that produced the result.
- Judgement: the user's league is played by the engine (`build_engine_team`), dormant leagues by
  `club_strength`, and the upset detector judges both with a third notion. A club with a weak
  saved XI and a strong bench is the underdog to the summary and the favourite to the sim, so
  the expected result gets reported as a "notable upset".

**Source of truth.** One `club_strength(roster)` next to `natural_ovr` in `player_rating.rs`,
with one fallback; `round_summary`, `catchup`, `national_team`, `world_cup` call it.

**Severity.** Inconsistent judgement rather than a provably wrong number, except the fallbacks,
which are provable. Observable as odd upset headlines.

### A7. Money shown with and without the currency symbol in the same inbox

**Concept.** Compact money formatting.

**Locations.**
- `finances.rs:1165` `format_money` → `currency::format_compact_number` (**no symbol**), into
  params at `:1056`, `:1092`, `:1129`, `:1132`. The templates add none:
  `src/i18n/locales/en.json:2867` "The club is currently {{amount}} in debt",
  `:2871` "({{weeklyWages}}/week in wages)", `:2875` "({{annualWages}}) … ({{wageBudget}})".
- `news.rs:329` `format_transfer_fee` and `messages.rs:160,194,243,300,396` →
  `currency::format_compact_money` (**with symbol**).

**Drifted.** The board writes "1.2M in debt"; the same day's transfer note says "€1.2M".

**Source of truth.** `currency.rs:73 format_compact_money`. Drop `format_money`.

**Severity.** Cosmetic, observable.

---

## B. Drifted, possibly deliberate — maintainer decides

### B1. Two per-position attribute-weight tables

`player_rating.rs:307-410 weighted_score` (rating at a position) and
`player_identity.rs:308-400 score_position` (inferring the natural position). Same attribute
sets per position, different weights: GK handling/reflexes 28/28 vs 30/30, RB positioning 12 vs
5, CB decisions 8 vs 10, ST shooting 26 vs 30, and so on for every arm. Nothing ties them.
Failure mode: a player's inferred natural slot is not his highest-rated slot, so his displayed
OVR understates him. Not proven on a real player. If the difference is intended, a comment
saying so on both tables would stop the next reader unifying them.

### B2. Contract-days-remaining discount ladders on both sides of a transfer

`transfers/mod.rs:313-330 minimum_acceptable_fee` (≤60/≤180/≤365 → −0.25/−0.15/−0.05, morale
≤40) vs `:532-546 suggested_incoming_fee` (≤60/≤180 → −0.15/−0.10, morale ≤45), plus
`contracts/renewals.rs:436-447 build_renewal_feedback` (≤90/≤180/≤365 urgency). Seller floor vs
suggested bid vs renewal pressure are different questions, but the day thresholds are the same
concept as the known 180/365 contract-risk thresholds and drift independently.

### B3. Two sponsor-deal generators

`random_events/responses.rs:84-92` builds a `Sponsorship` with a literal 12 weeks and an
`UnbeatenRun{3, amount/4}` bonus; `finances.rs:20 SPONSOR_PITCH_DURATION_WEEKS` (12) and the
sponsor-pitch path build another. `random_events/responses.rs:23 parse_amount_param` then
re-parses the `amount` string out of the message params (with "1M"/"250K" support). Today the
builder stores raw digits (`random_events/message_builders.rs:55`), so it round-trips exactly;
the first time someone formats that param for display, accepted deals will be rounded to the
display precision.

---

## C. In sync today — latent

| # | Concept | Copies |
|---|---------|--------|
| C1 | `params(&[(&str,&str)]) -> HashMap` | `firing.rs:26`, `job_offers.rs:53`, `messages.rs:9`, `news.rs:11`, `player_events/message_builders.rs:6`, `random_events/mod.rs:15`, `scouting.rs:34` — seven identical bodies |
| C2 | `key?param=value` backend-message encoding | `club.rs:5`, `contracts/helpers.rs:53`, `contract_wage_policy.rs:11`, `generator/world_io.rs:35`, `scouting.rs:20` (the only one that does multi-param with `&`). The decoder is in the frontend, not audited here |
| C3 | Standings comparator points→GD→GF | `group_stage.rs:235-243`, `group_stage.rs:336-340`, `world_cup.rs:803-808`, `slices/teams/indices.rs:45-50` (with its own `goal_diff` at `:68`), `turn/round_summary.rs:332-340` (known). Canonical is `domain/src/league.rs:476 League::sorted_standings`; all five match it. `turn/news.rs:94` is the known drifted one |
| C4 | Injury multiplier from fitness | `random_events/mod.rs:55-66` and `player_wear.rs:43-54`, identical (3.0/2.0/1.5/0.7/1.0) |
| C5 | "Free Agent" as a team name | `end_of_season.rs:38`, `season_awards.rs:57` — `["Free","Agent"].join(" ")`, hardcoded English hidden from the string audit, in two places |
| C6 | Attribute ceiling 99 | `transfers/mod.rs:2878-2885` literal vs `aging.rs:8 MAX_ATTRIBUTE` / `:46` |
| C7 | XI size 11 | `catchup.rs:12 CATCHUP_XI`, `national_team.rs:23 MATCH_SQUAD_SIZE`, literals at `squad_safety.rs:127,135,154`, `live_match_manager/team_builder.rs:132,190,268`, `world_cup.rs:95` |
| C8 | `GROUP_SIZE` 4 / `MAX_REPUTATION` 1000 | `group_stage.rs:13` and `world_cup.rs:281`; `generator/package.rs:247` (u32) and `reputation.rs:9` (i32) |
| C9 | Error-key constants | `contracts/consts.rs:9,11` vs `squad_safety.rs:7,8` — same keys, different names; raw `"be.error.playerNotFound"` ×17, `"be.error.managedTeamNotFound"` ×15, `"be.error.noTeamAssigned"` ×13, `"be.error.teamNotFound"` ×10 across `transfers/mod.rs`, `transfers/execution.rs`, `contract_wage_policy.rs`, `scouting.rs`, `delegated_renewals.rs`, `finances.rs`; `ERR_UNABLE_TO_CALCULATE_CONTRACT_END_DATE` ×2 |
| C10 | Deterministic per-entity hashing | `aging.rs:22 seeded_value` and `history_generation.rs:19-31 deterministic_u32` (both `DefaultHasher`), `player_identity.rs:449 stable_hash` and `finances.rs:539-545` (both a 31-multiplier fold). `DefaultHasher`'s algorithm is not guaranteed across Rust versions, so ageing curves and synthesised history would both shift on a toolchain bump — together, silently |
| C11 | Wage budget 110% ceiling | `finances.rs:115` literal vs `contract_wage_policy.rs:6 WAGE_SOFT_CAP_PCT` |
| C12 | `{message, i18n_key, i18n_params}` response struct | `job_offers.rs:47 JobOfferResponseEffect`, `random_events/responses.rs:6 RandomEventResponseEffect`, `player_events/responses.rs:22 PlayerResponseEffect` — three identical shapes |
| C13 | Staff full name | `format!("{} {}", first_name, last_name)` at `training/fitness_warnings.rs:66,75`, `scouting.rs:304,379`; `Manager` has `full_name()` (`domain/src/manager.rs:120`), `Staff` has none |

---

## D. Adjacent to the known list

- **Age helpers (known: seven).** Four more: `training.rs:406 estimate_age` and
  `player_rating.rs:95 player_age` (year-only), `generator/generation.rs:353,686` (inline
  year-only), `generator/mod.rs:147 opening_player_age` (anchored to 1 July). A December-born
  20-year-old is 21 to the year-only ones.
- **`team_name` (known: three).** A fourth, `transfers/mod.rs:1189 team_name_or_id`. And the
  fallbacks differ: `turn/news.rs:33` falls back to `""` (via `team_name_or`), the other three
  fall back to the id — an unknown team is blank in news and an id everywhere else.
- **`build_engine_team` vs `team_builder` (known).** The `p.position` field has two
  contradictory comments: `player_identity.rs:43` "intentionally stays as a coarse legacy
  bucket" vs `live_match_manager/team_builder.rs:418` "on legacy saves can still hold a stale
  coarse bucket". `team_builder.rs:307` and `:697` read `player.position.to_group_position()`
  despite the warning at `:418`; so do `turn/mod.rs:383`, `history_generation.rs:303`,
  `turn/post_match.rs:436`, `player_events/mod.rs:227`, and `season_awards.rs:315` (GK via
  `position` only, where `squad_safety.rs:176 is_goalkeeper` accepts either field).
- **`game.league` mirror (known).** In-area consumers that make it a behaviour, not just a
  copy: `ai_training.rs:148` (fixture-congestion logic counts league fixtures only — cups are
  invisible to AI rotation), `live_match_manager.rs:200` (a live match can only address a
  `game.league` fixture), `finances.rs:860-866,934-946` (sponsor `LeaguePosition` bonuses and
  matchday income keyed off the mirror), `season_awards.rs:156` (`division.or(game.league)`),
  and 23 uses in `turn/news.rs`. A further `competitions.first()` at
  `generator/world_io.rs:107` and `game.rs:293 first_mut()`.
- **`fixture_competition_for` (known).** Confirmed present at `group_stage.rs:52` and
  `generator/competition_def.rs:647` at that revision.

---

## E. Checked, genuinely in sync

- **Money formatting** all routes through `currency.rs:61 format_compact_number` /
  `:73 format_compact_money`; only the symbol choice differs (A7).
- **`season_has_started`** — `turn/news.rs:137` delegates to `end_of_season.rs:45`.
- **`award_entry`** — `news.rs:814` is a test helper; `season_awards.rs:128` is production.
- **`ovr`** has one production writer, `player_rating.rs:64`.
- **Generator ↔ rating round-trip** — `generation.rs:560 attributes_for_overall` against
  `player_rating.rs:147 ovr_from_attributes` is pinned by the test at `generator/mod.rs:1493`.
- **`formation_slots`** (`player_rating.rs:10`) and **`deployed_position`** (`:25`) — single.
- **International windows** — single source `national_team.rs:29`; `world_cup.rs:797` delegates.
- **Transfer window status** — derived once in `season_context.rs:73`;
  `transfers/mod.rs:804` reads it.
- **Next-season start** — `generator/competition_def.rs:216-243` only;
  `end_of_season.rs` calls it.
- **Rounding helpers** — `contracts/helpers.rs:167` (to 1k, wages) and
  `transfers/execution.rs:9` (to 50k, fees) are different concepts.
- **Reputation-tier literals** — `generator/mod.rs:67` (wage usage 750/550),
  `board_objectives.rs:17-19` (objectives 800/650/400), `live_match_manager.rs:335,346`
  (persona 700/800), `team_builder.rs:216` (linear 300–900) are different concepts, not copies.
- **`expected_fixture_count`** (`end_of_season.rs:14`, n(n−1)) matches the double round robin in
  `schedule.rs`.
- **`confederation_of_region`**, **`is_split_season_country`** — single (`nations.rs:283,310`).
- **Average-condition helpers** — `training/fitness_warnings.rs:28` and `ai_training.rs:150`
  do the same arithmetic for different purposes with different empty fallbacks (none vs 100.0);
  acceptable as is.
- **Response-effect builders** — `job_offers.rs:60` and `random_events/responses.rs:12`
  differ in signature on purpose; the shared shape is C12.
- **Injury duration tables** (`player_wear.rs:74`, `random_events/mod.rs:168`) — not compared;
  out of time, flagged so nobody assumes they were.
