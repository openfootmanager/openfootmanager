# Docs, package schema, fixtures and tooling duplication audit

**Sweep D — 24 September 2026, at `8e01f97b9b41e16e9e1d482a9d1f87428bed5f3d`** — the `upstream/develop` tip at the time (PR #559). A remote refresh was not possible, so “current develop” below means that dated local tip rather than a verified remote one.

This is a source audit with bounded probes. It is not a claim that the application or its full test suites pass.

Scope: the documentation listed below, `.ofm` authoring representations and their immediate consumers, fixtures and factories, and build/tooling configuration. Reads outside those areas were limited to proving a documented contract or one of the seven verifications recorded at the end. Deliberate engine/domain mirror types, the shared CLI/core validator, and the documented db→core edge are not findings. The previously known command-count, `season_advance`, position-token, `playStyle`, berth-shape and `logo` findings are excluded. The known locale-map duplication appears only in Part 2.

Severity: **P1** = accepted authored content silently lost or materially misinterpreted; **P2** = incorrect author/player/contributor contract with a concrete failure or misleading behavior; **P3** = stale reference or maintenance risk without a demonstrated runtime failure. Rank favors observable behavior over duplicated names. “Source-proved” means the relevant input, transformation and consumer were read; it does not mean an end-to-end runtime reproduction was executed.

## Part 1 — ranked findings

### F01 — Optional player club silently removes an authored player

**Concept:** A player definition may omit `club`, but world construction has no unattached-player path.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:176`; `src-tauri/crates/ofm_core/src/generator/package.rs:61`, `:1063`; `src-tauri/crates/ofm_core/src/generator/mod.rs:1144`, `:1165`, `:1209`, `:1212`; `src/components/menu/PackageEditor/types.ts:124`.

**Drifted:** The reference and Rust input default allow `club: ""`; reference validation deliberately ignores empty club references. Construction only indexes players with nonempty clubs and only materializes those attached to a team. The adjacent explicit path for unattached **staff** has no player equivalent. A valid-ID player without a club can therefore validate and remain visible as package data yet never enter the new world. This hurts package authors and players expecting an authored free agent. Source-proved; not inferred from the helper name alone.

**Source of truth:** One core policy for whether unattached authored players are supported; implement that policy at validation and world construction, and generate the field's requiredness/description from it. **Severity: P1.**

### F02 — Names collections discard all but the last entity

**Concept:** The general entity-collection contract does not match names aggregation.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:44`, `:580`; `src-tauri/crates/ofm-cli/src/main.rs:291`, `:456`, `:527`; `src-tauri/crates/ofm_core/src/generator/package.rs:510`, `:549`, `:585`, `:1568`.

**Drifted:** Files and CLI output support multiple names entities in `items`, but each entity executes `package.names = Some(def)`. Two disjoint pools in separate names entities in one package do not combine: only the last entity survives (files are sorted; items retain their order). Across separate packages, `merge_world_packages` *does* merge country-keyed pools at `:1576`. An author using the same entity/sharding convention as teams loses names silently; even the default stub and a later names file compete by filename. This is a within-package aggregation defect, not duplicate CLI validation.

**Source of truth:** Core names merge/cardinality policy, shared by classification and stack merging; generate the collection description and test a two-file/two-item example. **Severity: P1.**

### F03 — `format.groupSize` validates but never reaches the group builder

**Concept:** Authored group size versus fixed runtime group size.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:437`; `src-tauri/crates/ofm-cli/src/main.rs:265`; `src/components/menu/PackageEditor/types.ts:209`; `src-tauri/crates/ofm_core/src/generator/competition_def.rs:89`, `:562`, `:964`; `src-tauri/crates/ofm_core/src/group_stage.rs:13`, `:17`, `:68`.

**Drifted:** Rust, CLI prose, editor type and docs expose clubs-per-group. Validation rejects values below two. Construction forwards legs and qualification counts but has no group-size member in `GroupStageConfig`; `seed_groups` computes group count with fixed `GROUP_SIZE = 4`. For example, eight entrants with `groupSize: 2` still produce two groups of four, not four groups of two. Package authors and players get a different tournament than the accepted definition specifies.

**Source of truth:** Core format configuration consumed by scheduling; expose one default and either honor the field or reject unsupported values. Generate the schema/default reference. **Severity: P1.**

### F04 — Duplicate world manifests overwrite instead of failing cardinality validation

**Concept:** “At most one world entity” is only prose.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:47`, `:69`; `src-tauri/crates/ofm_core/src/generator/package.rs:128`, `:514`, `:585`, `:904`.

**Drifted:** Every world entity replaces `package.meta` and its source path. `validate_ids` covers six entity vectors, not world occurrences. Two valid manifests, including two entries in one `items` array, are accepted with last-wins metadata rather than the documented error. Authors can unknowingly change the effective package identity/default regions by adding a file.

**Source of truth:** Core loader cardinality validation; derive singleton/collection metadata for the CLI, editor and reference. **Severity: P2.**

### F05 — The documented MCP save-isolation variable is unused

**Concept:** Per-agent save-directory isolation.

**Locations:** `docs/MCP_SERVER.md:295`, `:313`; `src-tauri/src/lib.rs:45`, `:52`; `src-tauri/crates/db/src/save_manager.rs:123`; `src-tauri/crates/db/src/save_index_manager.rs:16`.

**Drifted:** The multi-agent script exports `TAURI_SAVE_DIR` and says it isolates each instance. Repository search finds no code reading it. Startup always uses Tauri's `app_data_dir()/saves`, including the shared index inside that directory. Contributors following the script run against the same save area. Shared access is source-proved; lost saves/corruption were not reproduced and are not asserted.

**Source of truth:** Implemented startup configuration. Generate documented flags/environment options from their declarations, or remove the unsupported variable and use a supported isolation mechanism. **Severity: P2.**

### F06 — Package IDs for teams, players and staff are not auto-generated at the public loader

**Concept:** Missing entity ID policy.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:131`, `:172`, `:261`; `src-tauri/crates/ofm-cli/src/main.rs:169`, `:191`, `:216`; `src-tauri/crates/ofm_core/src/generator/package.rs:667`, `:918`, `:957`; `src-tauri/crates/ofm_core/src/generator/generation.rs:779`, `:1031`; `src-tauri/crates/ofm_core/src/generator/definitions.rs:289`.

**Drifted:** The reference says optional/auto-UUID and also says generated from the name. CLI prose says required but “auto-uuid if empty.” The loader returns `missingId` for an absent/empty ID before normal import/build. UUID fallbacks exist in lower-level generation but do not rescue the public package load. Authors following the reference get a rejected package.

**Source of truth:** Public loader's ID contract, with generated requiredness documentation. Distinguish procedural/internal definitions from authored package input. **Severity: P2.**

### F07 — Documented optional nation-definition arrays make the override unusable

**Concept:** Serde defaults versus usable procedural-generation inputs.

**Locations:** `docs/DEFINITIONS.md:104`, `:107`, `:108`, `:118`; `src-tauri/crates/ofm_core/src/generator/definitions.rs:65`, `:139`, `:154`, `:168`, `:188`, `:203`, `:210`, `:259`.

**Drifted:** The table advertises optional `colorPalette` and `genericCities`, both defaulting to `[]`. Although they deserialize that way, usability validation rejects either empty list and falls through to another source/bundled data. A custom nations file following those defaults is discarded as a whole. The reference also describes only one/two tiers, while validation supports positive tiers up to 20; omitted limits include 1,000 clubs per division and 10,000 clubs overall. Authors see the shipped world instead of their override.

**Source of truth:** Core usability rules, not serde alone. Generate input constraints and document the fallback boundary. **Severity: P2.**

### F08 — Manual package-install and save-index locations name the wrong directories

**Concept:** Application storage paths.

**Locations:** `docs/modding/INSTALLING_PACKAGES.md:27`; `docs/SAVE_SYSTEM_DESIGN.md:21`, `:26`; `src-tauri/tauri.conf.json:5`; `src-tauri/src/commands/game/mod.rs:1582`; `src-tauri/src/lib.rs:52`; `src-tauri/crates/db/src/save_index_manager.rs:16`.

**Drifted:** Installation instructions use an `openfootmanager` app-data directory; the configured identifier is `com.sturdyrobot.openfootmanager` and code uses Tauri's app-data resolver. The save design uses yet another identifier, `com.openfootmanager`, and places `save_index.json` beside `saves/`; code places it *inside* `saves/`. A player manually installing a package can put it where it will never be discovered; contributors can inspect the wrong save/index. Platform roots may be redirected by the environment, so the resolver is authoritative.

**Source of truth:** Tauri identifier plus the actual path resolver. Prefer an app action that opens the directory and generate examples from identifier/path constants. **Severity: P2.**

### F09 — Ability ranges advertised as 1–99 are not the deserialization/validation contract

**Concept:** Player/staff attribute and overall bounds.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:184`, `:185`, `:270`; `src-tauri/crates/ofm-cli/src/main.rs:206`; `src-tauri/crates/ofm_core/src/generator/package.rs:72`, `:1001`, `:1078`; `src-tauri/crates/domain/src/player.rs:174`; `src-tauri/crates/domain/src/staff.rs:56`; `src-tauri/crates/ofm_core/src/generator/generation.rs:767`, `:814`, `:996`; `src/components/menu/PackageEditor/types.ts:95`, `:155`.

**Drifted:** `potential` explicitly validates 1–99; `overall` and explicit attribute values are plain `u8` with no corresponding range validation. Thus 0 or 200 are representable/accepted while negatives and >255 fail deserialization. Generated attributes from `overall` are clamped/jittered, while explicit player/staff attributes are cloned unchanged. Authors receive inconsistent treatment of the same advertised rating range; players can receive out-of-range explicit ability.

**Source of truth:** Core bounded ability types/validation, shared across all authored ability modes; generate numeric bounds for forms and reference. **Severity: P2.**

### F10 — Team CSV export omits `foundedYear`

**Concept:** The CSV projection is a separate, incomplete copy of the team schema.

**Locations:** `docs/modding/PACKAGE_EDITOR.md:244`; `src-tauri/crates/ofm_core/src/generator/definitions.rs:317`; `src/components/menu/PackageEditor/TeamForm.tsx:187`; `src-tauri/src/commands/package_csv.rs:42`, `:206`, `:232`; `src-tauri/src/commands/package_csv.rs:528`.

**Drifted:** Rust and the editor expose the authored founding year, but neither the CSV header nor row exports it. The guide proposes CSV as a plain-text backup/diff. Two otherwise identical clubs with different founding years export identically. Header-membership tests only compare the CSV lists with one another, so they cannot detect the omitted entity field. No CSV importer currently promises a full round trip; the demonstrated loss is in export/backup/diff coverage.

**Source of truth:** Core entity schema plus an explicit CSV projection mapping and documented exclusions; generate headers and a field-coverage check from that mapping. **Severity: P2.**

### F11 — CLI competition priority explains the opposite ordering

**Concept:** Division/prestige priority.

**Locations:** `src-tauri/crates/ofm-cli/src/main.rs:260`; `docs/modding/SCHEMA_REFERENCE.md:398`; `src-tauri/crates/ofm_core/src/end_of_season/berths.rs:84`, `:105`; `src-tauri/crates/ofm_core/src/generator/competition_def.rs:55`, `:992`.

**Drifted:** CLI says “scheduling priority (higher = scheduled first).” The reference correctly says lower is the higher division, and rollover sorts ascending. An author applying the CLI explanation to tier ranks can invert a domestic ladder. The observed ranking behavior is the ladder; no claim is made that all scheduling consumers use it.

**Source of truth:** Core ranking semantics; share field documentation between CLI and Markdown rather than a second annotated literal. **Severity: P2.**

### F12 — Competition scope requirements are not enforced

**Concept:** `countryId`/`regionId` requiredness depends on scope in one copy only.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:395`, `:396`, `:419`; `src-tauri/crates/ofm-cli/src/main.rs:257`; `src-tauri/crates/ofm_core/src/generator/competition_def.rs:47`, `:487`, `:621`, `:988`; `src-tauri/crates/ofm_core/src/end_of_season/berths.rs:89`.

**Drifted:** Docs require country for Domestic and region for Regional/Continental. Rust permits `None`; validation checks membership only when a value exists. A Domestic league with valid explicit teams and no country survives without joining a country's promotion ladder. The selector's own country requirement is separate and does not supply `competition.country_id`.

**Source of truth:** Core scope validation, or explicitly document optional affiliation and its consequences. Generate conditional constraints. **Severity: P2.**

### F13 — Embedded competition definitions conflate two world formats

**Concept:** An entity manifest is not a `WorldData` snapshot.

**Locations:** `docs/DEFINITIONS.md:211`; `src-tauri/crates/ofm_core/src/generator/definitions.rs:440`, `:451`; `src-tauri/crates/ofm_core/src/generator/package.rs:131`, `:514`; `src-tauri/crates/ofm_core/src/generator/mod.rs:1244`.

**Drifted:** Documentation tells authors to put `competitionDefinitions` in the world “manifest/package.” The field exists in flat `WorldData`, but not in the `WorldMetaDef` parsed for `schema: world`; unknown serde fields are ignored. In an entity package, competitions come from `schema: competition` entities. An author using the documented embedding location in `package.json` gets none of those definitions.

**Source of truth:** Separate versioned schemas for snapshot worlds and authored packages. Generate examples for each and test them through the matching public loader. **Severity: P2.**

### F15 — MCP mutation guidance mandates the pattern the state API was changed to prevent

**Concept:** Atomic mutation versus clone/release/write-back.

**Locations:** `docs/MCP_SERVER.md:400`; `src-tauri/CLAUDE.md:85`; `src-tauri/crates/ofm_core/src/state.rs:42`, `:83`; `src-tauri/src/commands/squad.rs:30`.

**Drifted:** MCP docs mandate cloning under a mutex, releasing it, processing, then reacquiring to write back. Current guidance/API use `update_game`, holding the mutex through read-modify-write specifically to prevent concurrent GUI/MCP lost updates. A contributor following the MCP instructions can reintroduce that race. This reports contradictory guidance, not a newly reproduced race in current callers.

**Source of truth:** `StateManager` API contract, linked from MCP documentation; an architecture check could forbid the old write-back pattern. **Severity: P2.**

### F16 — Six documented match defaults are stale

**Concept:** `MatchConfig::default` is copied into two prose tables/explanations.

**Locations:** `docs/MATCH_SIMULATION.md:76`, `:78`, `:86`, `:87`, `:88`, `:214`; `src-tauri/crates/engine/src/types.rs:359`; `src-tauri/crates/ofm_core/src/turn/mod.rs:515`; `src-tauri/crates/ofm_core/src/live_match_manager.rs:274`.

**Drifted:** Docs → code: shot accuracy **0.45→0.35**, conversion **0.30→0.36**, fatigue/minute **0.15→0.20**, foul probability **0.12→0.40**, yellow probability **0.30→0.11**, box-foul penalty probability **0.08→0.50**. Home advantage 1.08, red probability .04, stoppage max 4 and injury probability .03 still match. Players/contributors calibrating expected outcomes from the guide use obsolete probabilities.

**Source of truth:** `MatchConfig::default`; generate the defaults table from a serialized default, keeping probability interpretation as reviewed prose. **Severity: P2.**

### F17 — Recovery documentation overstates rest/recovery gains

**Concept:** Training recovery constants and modifiers.

**Locations:** `docs/GAME_SYSTEMS.md:95`, `:120`, `:121`, `:122`, `:126`; `src-tauri/crates/ofm_core/src/training.rs:228`, `:240`, `:246`, `:255`, `:303`.

**Drifted:** Rest base is **7**, not 10; Recovery focus base is **9**, not 12. Actual recovery also incorporates medical facilities, age, morale, condition and fitness; the documented stamina-only formula is incomplete. Injured players use a distinct half-base calculation and fitness decay. Players planning recovery from the table and contributors tuning it get the wrong result.

**Source of truth:** Named core recovery parameters and calculation; generate base values and document which modifiers apply to each branch. **Severity: P2.**

### F18 — `wage_budget` is documented in the wrong time unit

**Concept:** Annual budget versus weekly allowance.

**Locations:** `docs/GAME_SYSTEMS.md:458`; `src-tauri/crates/ofm_core/src/finances/mod.rs:188`, `:205`, `:281`, `:295`; `src/lib/finance.ts:200`.

**Drifted:** The field table calls it weekly. Both implementations treat it as annual, dividing by 52 for weekly budget and comparing it with annual wages for usage. Players/package contributors interpreting raw amounts as weekly are off by a factor of 52. This is separate from the projection-income omission in Part 2.

**Source of truth:** Core finance type with documented units, generated into the field reference. **Severity: P2.**

### F19 — Formation documentation still promises stat-ranked position rewriting

**Concept:** Formation change versus natural/deployed position.

**Locations:** `docs/MCP_SERVER.md:125`; `src-tauri/src/commands/squad.rs:30`; `src-tauri/crates/ofm_core/src/player_rating.rs:17`.

**Drifted:** MCP says outfield positions are reassigned by defending ability. The command deliberately preserves natural `player.position`, changes the formation, and reconciles roles. Deployed position is derived from formation plus XI order. An agent following the guide expects an automatic selection/reordering the command does not perform.

**Source of truth:** Command contract and deployed-position helper; generate tool descriptions from the same reviewed API documentation. **Severity: P2.**

### F20 — CLI and editor generate different IDs from the same name

**Concept:** Entity slug generation.

**Locations:** `src-tauri/crates/ofm_core/src/generator/scaffold.rs:99`; `src-tauri/crates/ofm-cli/src/main.rs:494`; `src/components/menu/PackageEditor/helpers.ts:237`; `src/components/menu/PackageEditor/TeamForm.tsx:102`; `src/components/menu/PackageEditor/PlayerForm.tsx:91`; `src/components/menu/PackageEditor/StaffForm.tsx:55`; `src/components/menu/PackageEditor/CompetitionForm.tsx:121`; `src/components/menu/PackageEditor/ConfederationForm.tsx:54`.

**Drifted:** Rust replaces every non-ASCII-alphanumeric character with a separator and has no 64-character cap; TS removes punctuation and then truncates to 64. `FC/A` becomes `fc-a` versus `fca`; `São Paulo` becomes `s-o-paulo` versus `so-paulo`. This creates different cross-reference/install identities when authors recreate an entity in another tool. Inputs consisting solely of discarded characters can also become blank in both; that is not a divergence.

**Source of truth:** Core slug policy exposed to the editor or generated from one specification with shared vectors. **Severity: P2.**

### F21 — The shared frontend factory builds impossible training enum values

**Concept:** Test-team default training state.

**Locations:** `src/test-utils/factories.ts:3`, `:21`; `src-tauri/crates/domain/src/team.rs:258`, `:269`; `src/store/types.ts:144` (both fields are strings); `docs/GAME_SYSTEMS.md:68`, `:82`.

**Drifted:** `createTeam()` uses focus `General` and intensity `Balanced`. Production enums use Physical/Technical/Tactical/Defending/Attacking/Recovery and Low/Medium/High, with Physical/Medium defaults. `Balanced` belongs to the schedule enum. Tests using the shared fixture can render states production serialization never emits and fail to exercise real defaults.

**Source of truth:** Generated frontend enum types plus a default fixture derived from a serialized core/domain team. Keep scenario-specific numbers explicit. **Severity: P2.**

### F22 — String-backed enum-like fields silently accept invalid values

**Concept:** `kitPattern` and `footedness` schema constraints are only enforced by form choices.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:143`, `:179`; `src-tauri/crates/ofm-cli/src/main.rs:183`, `:201`; `src-tauri/crates/ofm_core/src/generator/definitions.rs:314`; `src-tauri/crates/ofm_core/src/generator/package.rs:87`; `src-tauri/crates/ofm_core/src/generator/mod.rs:707`; `src-tauri/crates/ofm_core/src/generator/generation.rs:1059`; `src-tauri/crates/domain/src/team.rs:352`; `src/components/menu/PackageEditor/TeamForm.tsx:18`; `src/components/menu/PackageEditor/PlayerForm.tsx:225`.

**Drifted:** The authoring surface presents finite vocabularies, but Rust definition fields accept arbitrary strings. Unrecognized kit patterns fail a later parse whose error is discarded, leaving Solid; unrecognized footedness becomes Right. Shared validation does not reject these values. A package author's typo survives validation with a different kit/foot. All *listed valid* tokens agree; the mismatch is enforcement, not token spelling. The excluded `playStyle` finding is not repeated here.

**Source of truth:** Typed core authoring enums or a shared validating conversion, with choices generated for the editor/CLI/docs. **Severity: P2.**

### F26 — Country/region catalog examples are stale

**Concept:** Built-in naming/region IDs.

**Locations:** `docs/modding/README.md:41`; `docs/modding/PACKAGE_EDITOR.md:77`; `docs/modding/SCHEMA_REFERENCE.md:323`; `src-tauri/crates/ofm-cli/src/main.rs:242`, `:303`; `src-tauri/crates/ofm_core/src/nations.rs:1156`; `src-tauri/crates/ofm_core/src/generator/generation.rs:158`, `:174`.

**Drifted:** README/editor guide omit the seventh built-in region, `central-america`; the current schema reference and CLI country description include it. CLI names example uses `BRA`, while the catalog's Brazil identity is `BR`. A Brazilian generated player's exact pool lookup will not select the BRA pool; that pool can only be borrowed as an arbitrary fallback. Authors get misleading supported-region/name-pool guidance.

**Source of truth:** Core nation catalog and region registry; generate lists and examples from declared codes. **Severity: P2.**

### F27 — Definition docs still make name pools the nationality population model

**Concept:** Procedural nationality distribution.

**Locations:** `docs/DEFINITIONS.md:137`; `docs/GAME_SYSTEMS.md:427`; `src-tauri/crates/ofm_core/src/generator/generation.rs:234`, `:269`, `:306`, `:352`.

**Drifted:** Docs say the foreign draw is from available name pools. Runtime builds a weighted distribution from the nation catalogs plus declared package countries; name-pool selection is a later naming decision, with same-region fallback. Adding a name pool does not add equivalent nationality population weight. Authors tuning populations via names data are changing the wrong input.

**Source of truth:** Core nationality-distribution policy and catalog; generate population-rule parameters and keep names behavior separately documented. **Severity: P2.**

### F28 — Gameplay trait requirements and counts are stale

**Concept:** Position gates and Wonderkid thresholds.

**Locations:** `docs/GAME_SYSTEMS.md:195`, `:226`, `:227`, `:233`, `:234`; `docs/MATCH_SIMULATION.md:190`; `docs/ARCHITECTURE.md:97`; `src-tauri/crates/domain/src/player.rs:532`, `:551`, `:559`, `:563`, `:614`; `src-tauri/crates/ofm_core/src/player_rating.rs:4`, `:54`, `:70`.

**Drifted:** There are 21 trait variants, not 20. SafeHands/CatReflexes are no longer GK-only, and CompleteForward/Engine no longer require Forward/Midfielder positions: `compute_traits` deliberately ignores its position argument. Even nearby enum comments retain the obsolete position gates. The Wonderkid enum comment says ≤21 / potential≥85 / room≥10; core now uses ≤20 / ≥90 / ≥14. Players and contributors following those tables/comments predict the wrong tags.

**Source of truth:** Core trait predicates, with domain attribute-only predicates distinguished from derived age/potential predicates; generate requirement tables from declarative rules. **Severity: P2.**

### F29 — The “half skill at 50 condition” explanation is not live action scaling

**Concept:** Effective-overall helper versus actual per-action condition adjustment.

**Locations:** `docs/MATCH_SIMULATION.md:107`, `:111`, `:117`, `:257`; `src-tauri/crates/engine/src/types.rs:142`, `:158`; `src-tauri/crates/engine/src/live_match/helpers.rs:46`; `src-tauri/crates/engine/src/live_match/zone_resolution.rs:100`.

**Drifted:** The documented `overall × condition/100` correctly describes `effective_overall()`, but the accompanying performance claim does not describe live resolution. Live action skills use `0.6 + 0.4 × condition/100`, hence 50 condition retains 80% skill, not 50%. The helper is not proof of the live gameplay formula. This is a documentation mismatch; the broader batch/live divergence is verified only in Part 2.

**Source of truth:** Resolution's actual condition function. Generate examples at 0/50/100 and clearly label unused/general rating helpers. **Severity: P2.**

### F14 — Schema parity tests protect keys, not the advertised complete contract

**Concept:** “Every step fails until updated” overstates what the schema handshake verifies.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:25`; `src-tauri/crates/ofm_core/src/generator/scaffold.rs:438`, `:474`, `:648`, `:783`; `src-tauri/crates/ofm-cli/src/main.rs:1035`, `:1066`; `src/components/menu/PackageEditor/schemaParity.test.ts:33`, `:52`, `:63`; `src/components/menu/PackageEditor/schemaFields.generated.json:1`.

**Drifted:** Rust exhaustive populated literals catch added struct fields, template key parity checks both directions, and template deserialization checks enum spellings. However, the frontend fixture contains only top-level names; its test does not compare types, defaults, enum members or nested fields. CLI documentation tests only look for quoted/backticked keys. The Markdown test searches the *entire document*, so a field can be missing from one entity's section and pass because another entity documents that name. This explains why F06/F09/F11/F24 survive despite a “parity” chain. The tests themselves document several limits; the blanket guarantee in the guide is the drift.

**Source of truth:** Structured Rust-derived schema metadata, with explicit validation/runtime annotations. Generate TS, CLI reference and Markdown; retain behavioral tests for semantics that cannot be derived from serde. **Severity: P2.**

### F23 — Documented manifest lexical constraints exceed shared validation

**Concept:** Package ID, version, license and package-type contracts.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:73`, `:76`, `:78`, `:79`; `src-tauri/crates/ofm_core/src/generator/package.rs:147`, `:156`, `:162`, `:807`, `:845`; `src/components/menu/PackageEditor/helpers.ts:48`; `src/components/menu/PackageEditor/MetadataForm.tsx:12`.

**Drifted:** The reference describes lowercase slug IDs, semantic versions, SPDX licenses and a three-value package type. Validation rejects path-dangerous/reserved IDs and blank metadata, but does not implement those complete lexical/enum constraints. For example, a nonempty `version: "banana"` or `packageType: "typo"` is not rejected by `validate_manifest`; an ID with spaces/capitals is accepted if it passes the path rule. This makes “valid” weaker than authors are told. `gameMinVersion` has an additional installer semver check; that is an installation policy, not a second package validator.

**Source of truth:** Core explicitly specified validation contract. Either enforce the documented constraints or label them recommendations; generate reference constraints where enforceable. **Severity: P2.**

### F32 — Save schema prose mixes a historical proposal with current-schema claims

**Concept:** Handwritten SQL and storage design are not the migrated schema.

**Locations:** `docs/SAVE_SYSTEM_DESIGN.md:9`, `:33`, `:45`, `:245`, `:348`; `src-tauri/crates/db/src/sql/v001_initial_schema.sql:1`; `src-tauri/crates/db/src/migrations.rs:43`; `src-tauri/crates/db/src/save_index.rs:14`; `src-tauri/crates/ofm_core/src/generator/definitions.rs:65`; `docs/ARCHITECTURE.md:347`; `src-tauri/CLAUDE.md:51`.

**Drifted:** The SQL uses `game_meta.current_date`; the actual initial/migrated schema uses `game_date`. The handwritten schema omits eight current tables and many columns (full mechanically compared inventory below); even the index example omits `team_name`. The design's future strict override errors/regeneration/hardcoded `data.rs` description is not the current usable-definition fallback implementation. Separately, ARCHITECTURE says serde defaults avoid migration scripts, contradicting the current SQLite rules and 44 migrations. The design is explicitly a proposal, so merely unimplemented target items are **not** treated as production bugs; the problem is using this mixed document as a current reference.

**Source of truth:** Registered SQL migrations plus introspected schema, and implemented loader behavior. Generate a current schema appendix; freeze/date the old proposal instead of maintaining its SQL manually. **Severity: P2 for wrong current-column/migration guidance; P3 for historical proposal omissions.**

### F33 — CLI build instructions use the wrong working directory and stale toolchain minimum

**Concept:** Workspace root and Rust version.

**Locations:** `docs/modding/CLI_REFERENCE.md:9`, `:12`; `docs/modding/QUICKSTART.md:14`; `docs/modding/SCHEMA_REFERENCE.md:36`; `rust-toolchain.toml:13`; `src-tauri/Cargo.toml:19`; `src-tauri/crates/ofm_core/Cargo.toml:4`; `src-tauri/crates/ofm-cli/Cargo.toml:10`.

**Drifted:** Instructions imply the clone root (`cargo install --path src-tauri/...` is adjacent), but `cargo build -p ofm-cli`/the fixture regeneration command omit `--manifest-path src-tauri/Cargo.toml` or `cd src-tauri`; there is no root Cargo.toml. The binary path is therefore also missing `src-tauri/` for a root-relative run. “Rust ≥1.75” cannot describe dependencies authored with edition 2024; the checkout pins 1.95.0. Authors cannot follow the advertised clean-clone command literally.

**Source of truth:** Workspace manifest and toolchain file; generate/execute documentation smoke commands with an explicit cwd. **Severity: P2.**

### F35 — Tauri builds do not enforce the lockfile like direct Cargo gates

**Concept:** Consistent lockfile enforcement across verification/build entry points.

**Locations:** `CONTRIBUTING.md:194`, `:211`; `.github/workflows/build-check.yml:232`, `:267`, `:281`, `:284`, `:289`, `:294`; `.github/workflows/tauri-action.yml:28`, `:90`; `.github/workflows/nightly-tauri-action.yml:108`, `:170`.

**Drifted:** Direct Cargo verification uses `--locked`. The smoke build and both release action argument matrices do not forward it to Cargo. The documented blanket “CI runs its cargo commands with --locked” is therefore stronger than the effective entry-point configuration. A Tauri build is permitted to update a stale lockfile. These releases also depend on verification jobs, so this is **not** evidence that a bad lockfile has actually shipped; target-specific resolution and build reproducibility remain the uncovered boundary.

**Source of truth:** Shared build-command policy with a checker covering direct Cargo and Tauri forwarding arguments. Generate/reuse the release matrix. **Severity: P2.**

### F24 — Package/names default versions are zero, not one

**Concept:** Constructor/scaffold versions versus deserialization defaults.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:81`, `:586`; `src-tauri/crates/ofm-cli/src/main.rs:154`, `:295`; `src-tauri/crates/ofm_core/src/generator/package.rs:152`, `:776`; `src-tauri/crates/ofm_core/src/generator/definitions.rs:235`; `src-tauri/crates/ofm_core/src/generator/scaffold.rs:121`, `:157`; `src-tauri/crates/ofm_core/src/generator/competition_def.rs:27`.

**Drifted:** Omitted world `formatVersion` and names `version` deserialize to 0. Docs/CLI promise 1 (CLI even calls names.version required/always 1). World validation rejects only versions greater than supported 1, so 0 is accepted, not normalized. Scaffolds deliberately write 1. Standalone competition definitions separately default to 1 and must not be conflated with either field.

**Source of truth:** Each format's compatibility contract; generate a distinction between authored scaffold values and missing-field defaults. **Severity: P3.**

### F25 — CLI required-field labels disagree with serde and the schema reference

**Concept:** Optional identity/display fields are labeled required.

**Locations:** `src-tauri/crates/ofm-cli/src/main.rs:171`, `:192`, `:193`, `:194`, `:195`, `:196`, `:197`, `:217`, `:218`, `:220`, `:221`; `src-tauri/crates/ofm_core/src/generator/definitions.rs:294`; `src-tauri/crates/ofm_core/src/generator/package.rs:55`, `:105`; `docs/modding/SCHEMA_REFERENCE.md:133`, `:173`, `:262`; `src-tauri/crates/ofm_core/src/generator/generation.rs:733`.

**Drifted:** CLI marks team.shortName, player.name/firstName/lastName/club/nationality/position and staff.firstName/lastName/nationality/role required. Rust defaults those; the Markdown reference mostly describes that correctly. This makes authors fill unnecessary fields and obscures generation fallbacks. Player.club's more serious world-build consequence is F01; ID's inverse mismatch is F06.

**Source of truth:** Rust input/default metadata, augmented by post-parse requirements, generated into CLI annotations. TS required fields on *normalized loaded objects* are not themselves evidence that authored input requires them. **Severity: P3.**

### F30 — AI fatigue substitutions are not gated until minute 55

**Concept:** Earliest AI fatigue substitution.

**Locations:** `docs/MATCH_SIMULATION.md:290`; `src-tauri/crates/engine/src/ai.rs:61`, `:124`, `:155`.

**Drifted:** The guide says fatigue substitutions occur after minute 55. During any playing phase, the code uses a threshold of 35 before minute 60 and can immediately substitute an eligible tired outfielder when a replacement exists. A player can see an early substitution the guide rules out; the code's own “after minute 55+” comment is also stale.

**Source of truth:** AI policy thresholds; generate the timing table, or explicitly describe the early severe-fatigue exception. **Severity: P3.**

### F31 — Current systems are still labeled future work

**Concept:** Implemented gameplay and package capabilities versus obsolete feature-status prose.

**Locations:** `docs/GAME_SYSTEMS.md:153`, `:160`, `:303`, `:464`, `:498`; `docs/modding/INSTALLING_PACKAGES.md:65`; `src-tauri/crates/ofm_core/src/scouting.rs:84`, `:97`, `:125`, `:270`; `src-tauri/crates/ofm_core/src/finances/mod.rs:924`, `:1008`, `:1055`; `src-tauri/crates/ofm_core/src/end_of_season/mod.rs:892`; `src-tauri/crates/ofm_core/src/turn/mod.rs:185`, `:198`, `:199`; `src-tauri/src/commands/game/mod.rs:92`, `:1579`.

**Drifted:** Scout reports and scouting-attribute effects, matchday/sponsor income, prize payments and transfer resolution are implemented, despite “future” descriptions. The daily flow also invokes contracts, scouting and pending transfer/loan registrations that the abbreviated “each day follows” diagram omits. Installation docs say exactly one package and stacking is future; new-game startup accepts an ordered package-ID list and loads/merges it. Contributors can plan duplicate work and players miss existing functionality. This is a status-document audit, not a new audit of those systems' internal correctness.

**Source of truth:** Capability/API inventory linked to implemented paths and acceptance scenarios; generate feature-status lists where feasible, keeping design aspirations explicitly dated. **Severity: P3.**

### F36 — Migration instructions name the derived count instead of the required pin

**Concept:** Migration-count maintenance.

**Locations:** `src-tauri/CLAUDE.md:72`; `src-tauri/crates/db/src/migrations.rs:4`, `:18`, `:22`, `:43`.

**Drifted:** Guidance says bump `MIGRATION_COUNT`; that is now `MIGRATIONS.len()`. The deliberately independent value a new migration must bump is `EXPECTED_MIGRATION_COUNT = 44`, enforced at compile time. A contributor following the prose changes the wrong thing or still fails compilation.

**Source of truth:** Migration registration API and its diagnostic; link to that instruction instead of copying the variable name. The independent historical pin is intentional, not duplication to remove. **Severity: P3.**

### F37 — Locale counts and “lint disabled” comments contradict their sources

**Concept:** Stale inventory numbers/feature-state comments.

**Locations:** `docs/ARCHITECTURE.md:16`, `:31`; `VISION.md:68`; `src/CLAUDE.md:10`; `src/i18n/index.ts:5`; `.github/workflows/build-check.yml:188`, `:298`; `CONTRIBUTING.md:140`; `quality-baseline.json:103`.

**Drifted:** ARCHITECTURE lists seven locales; VISION says eleven; registry/frontend guidance contain twelve. This is documentation drift, distinct from the excluded duplicate test maps. CI's trailing comment still says `biome lint` is not enabled although the frontend job runs `npm run lint`. CONTRIBUTING's illustrative semantic-element debt count is nine while the baseline is ten. Contributors cannot rely on these copied inventories to know coverage/gate state.

**Source of truth:** `SUPPORTED_LANGUAGES`, active workflow steps, and the baseline. Generate counts or omit them from evergreen prose. **Severity: P3.**

### F38 — Architecture prose overrestricts the real domain/engine bridge

**Concept:** Allowed conversion location and what “domain has no logic” means.

**Locations:** `AGENTS.md:30`; `src-tauri/CLAUDE.md:24`, `:28`, `:44`; `docs/MATCH_SIMULATION.md:30`; `src-tauri/crates/ofm_core/src/live_match_manager/team_builder.rs:17`, `:416`; `src-tauri/crates/domain/src/player.rs:563`; `src-tauri/crates/domain/src/league.rs:478`; `src-tauri/crates/domain/src/team.rs:292`.

**Drifted:** Rules say only `ofm_core/turn/` may bridge types, yet live team conversion is in `live_match_manager/team_builder.rs`, as the simulation guide acknowledges. “Domain structs and enums only, no game logic” also excludes its existing trait predicates, standings sort and training-day policy. The crate dependency boundaries remain correct and deliberate; this finding concerns guidance that would lead a contributor to move legitimate code unnecessarily.

**Source of truth:** A precise permitted-layer policy grounded in current call paths; generate dependency diagrams, while explicitly documenting allowed domain-local behavior/bridge modules. **Severity: P3.**

### F39 — CLI `info` also extracts and loads the archive

**Concept:** Manifest-only inspection promise.

**Locations:** `docs/modding/CLI_REFERENCE.md:188`; `docs/ARCHITECTURE.md:461`; `src-tauri/crates/ofm-cli/src/main.rs:699`; `src-tauri/crates/ofm_core/src/generator/package.rs:1914`.

**Drifted:** `cmd_info` calls the cheap manifest reader **and** `load_world_package_from_ofm` to count entities/errors. The latter extracts into scratch storage before loading. “Without extracting it” is false for this CLI command, even though it is true of the manifest-reader helper. Authors inspecting large packages incur extraction I/O/space; no performance measurement was made.

**Source of truth:** Command's actual contract; derive CLI usage prose from it and distinguish metadata-only inspection from validated entity counts. **Severity: P3.**

### F40 — Simulation structure/test-count descriptions are stale

**Concept:** Hand-maintained file/variant/test inventory.

**Locations:** `docs/MATCH_SIMULATION.md:22`, `:23`, `:25`, `:320`, `:322`, `:333`; `docs/ARCHITECTURE.md:117`; `src-tauri/crates/engine/src/event.rs:22`; `src-tauri/crates/engine/src/engine/mod.rs:1`; `src-tauri/crates/engine/src/live_match/mod.rs:1`; `src-tauri/crates/engine/tests/simulation_tests.rs:1`; `src-tauri/crates/engine/tests/live_match_tests.rs:1`.

**Drifted:** The two former single simulation files are directories/modules. EventType has 31 variants, not 22. The two named test files contain 50 and 59 `#[test]` declarations (109 combined), not 36 and 33 (69), with additional tests elsewhere. Counts were measured statically, not by executing/collecting tests. The old world-generation flow in `docs/GAME_SYSTEMS.md:405` also says to load retired team templates; its own later definition-file section correctly names `default_nations.json`.

**Source of truth:** Source tree and compiler/test discovery; generate snapshots with a date or remove mutable counts. **Severity: P3.**

### F41 — Nested authoring fields remain outside the schema handshake

**Concept:** Nested schema completeness and normalized selector shape.

**Locations:** `docs/modding/SCHEMA_REFERENCE.md:3`, `:186`, `:475`; `src-tauri/crates/ofm-cli/src/main.rs:210`, `:271`; `src-tauri/crates/ofm_core/src/generator/competition_def.rs:126`; `src/components/menu/PackageEditor/types.ts:219`; `src/components/menu/PackageEditor/CompetitionForm.tsx:44`; `src-tauri/crates/ofm_core/src/generator/scaffold.rs:588`; `src/components/menu/PackageEditor/schemaParity.test.ts:63`.

**Drifted:** CLI's selector reference omits `excludeCompetitions`, which the Rust type and Markdown reference support. Neither CLI nor SCHEMA_REFERENCE enumerates the 19 player-attribute names and their individual required/default status, despite the latter claiming every field; the names are recoverable from Rust, editor and CSV. TS requires `selector.excludeCompetitions`, but Rust omits it when empty. Form/helper fixtures always supply `[]`, hiding that valid serialized shape. No current dereference crash was found: the type is inaccurate and the coverage gap is demonstrated, not an observed runtime failure.

**Source of truth:** Recursive schema metadata including omission rules; generate the nested reference, TS interface and fixture variants (populated **and** default/omitted). **Severity: P3.**

### F42 — The CSV debt example models a team definition the package validator rejects

**Concept:** A test fixture and prose example encode the wrong finance-range rule.

**Locations:** `docs/modding/PACKAGE_EDITOR.md:261`; `src-tauri/src/commands/package_csv.rs:31`, `:579`, `:587`; `src-tauri/crates/ofm_core/src/generator/package.rs:1137`, `:1164`; `docs/modding/SCHEMA_REFERENCE.md:141`.

**Drifted:** The CSV test/example describes `financeRange: [-2000000, -1000000]` as a legitimate authored club in debt. Core package validation rejects either negative bound. A live team's signed balance and an authored initial finance range are different contracts. The CSV escaping test remains useful for signed-number serialization; it does not prove the fixture is an installable package. Authors are given an impossible valid-package example.

**Source of truth:** Core finance-range validation; retain the numeric-escaping test with its narrower claim and generate valid package examples through validation. **Severity: P3.**

### F34 — The repeated bare `cargo test --bin` claim is false

**Concept:** Cargo target-selection syntax.

**Locations:** `AGENTS.md:21`; `CLAUDE.md:48`; `src-tauri/CLAUDE.md:124`; `CONTRIBUTING.md:240`; `.github/workflows/build-check.yml:272`; `src-tauri/Cargo.toml:13`.

**Drifted:** Docs say bare `cargo test --bin` succeeds with zero tests. Probe `cargo test --manifest-path src-tauri/Cargo.toml --bin --offline` exited **101** with `error: "--bin" takes one argument. Available binaries: openfootmanager`. No compilation occurred. Selecting the named binary instead of `--lib` is the intended trap; the literal advertised behavior is wrong. The recommendation to use `--lib` for Tauri command tests remains correct.

**Source of truth:** Cargo invocation syntax and actual target metadata; one linked command reference rather than five copied warnings. **Severity: P3.**

## Part 2 — existing findings rechecked against the audited develop snapshot

1. **CONFIRMED** — `src-tauri/crates/ofm_core/src/turn/mod.rs:396` filters only matching team ID, maps `p.position.to_group_position()` at `:401`, and calls batch simulation at `:513`; `src-tauri/crates/ofm_core/src/live_match_manager/team_builder.rs:47` rejects injured players, `:67` selects a slot-aligned XI, and `:424` uses deployed position/natural fallback. Injured/non-XI roster members are eligible for batch selection; this is **batch versus live**, not a universal AI-versus-human distinction (live AI opponents also use the healthy-XI builder).
2. **CONFIRMED** — `src/lib/finance.ts:203` projects sponsor **base** minus wages; `src-tauri/crates/ofm_core/src/finances/mod.rs:283` adds sponsor bonus and estimated matchday income before subtracting wages. Dashboard consumes the frontend snapshot at `src/components/dashboard/dashboardHelpers.ts:93` and fires crisis for critical runway at `:156`. Concrete arithmetic witness: €520,000 annual wages/budget, €20,000 cash, zero sponsor base, one counted recent home match with 1,000 capacity gives frontend −€10,000/week and two-week/critical runway; backend estimates €15,200 gate revenue, +€5,200 net, no negative runway and 100%-budget **watch** (`finances/mod.rs:121`, `:245`, `:253`). Source/formula witness, not a played scenario.
3. **CONFIRMED** — `src/components/playerProfile/PlayerProfile.helpers.ts:63` defaults to `"2026-07-01"`; `src/components/playerProfile/PlayerProfile.tsx:74` supplies no date. `src/lib/valueFormatting.ts:37` instead uses the game clock, falling back to today's UTC date only without a game date.
4. **CONFIRMED** — ordinal comparison in `src-tauri/crates/ofm_core/src/aging.rs:10`, `contracts/helpers.rs:149`, `season_awards.rs:61`; birthday month/day comparison in `src-tauri/src/commands/squad.rs:19`, `src-tauri/src/mcp_server/tools_impl/helpers.rs:59`, `src/lib/valueFormatting.ts:43`; year-only `src-tauri/crates/ofm_core/src/player_rating.rs:92`, feeding potential at `:47` and Wonderkid at `:57`. Example: DOB 2000-03-01 on 2026-03-01 is 25 via ordinal (61 vs 60), 26 via month/day; year-only overstates before birthdays. These are the seven cited helpers, **not** an exhaustive total (the profile helper above is another).
5. **CONFIRMED, count clarified** — `src/utils/backendI18n.localeCoverage.test.ts:16` has 11 locales including English, missing Indonesian; `src/i18n/localeCoverage.test.ts:23` has **11 non-English map entries**, including Indonesian, plus separately imported English at `:12`, so 12 covered locales. Neither derives its coverage set from `src/i18n/index.ts:5`'s 12-entry `SUPPORTED_LANGUAGES`. The literal claim that the second map itself has twelve entries is inaccurate; the coverage divergence remains.
6. **CONFIRMED** — `src-tauri/crates/ofm_core/src/turn/news.rs:82` throws away goals-for in its row projection and `:94` sorts points then GD only; `src-tauri/crates/domain/src/league.rs:478` adds goals-for at `:484`. Equal points/GD with unequal goals-for can yield a different news order from the canonical table.
7. **CONFIRMED, scope clarified** — separate loops in `src-tauri/crates/engine/src/engine/mod.rs:46`/`:186` and `src-tauri/crates/engine/src/live_match/mod.rs:300`; batch uses raw per-player skills at `engine/resolution.rs:81`, live adjusts them at `live_match/zone_resolution.rs:100`; batch foul event at `engine/fouls.rs:37` has no detail, live emits `EventDetail::Foul { severity }` at `live_match/zone_resolution.rs:441` (also `:299`). Core matchdays call batch at `src-tauri/crates/ofm_core/src/turn/mod.rs:516`; sim-bench calls it at `src-tauri/crates/sim-bench/src/main.rs:213`, `:295`, `:526`; live sessions instantiate `LiveMatchState` at `src-tauri/crates/ofm_core/src/live_match_manager.rs:274`. They share helpers, and batch **does** deplete team-level condition at `engine/mod.rs:192`; the confirmed difference is per-player condition scaling and event detail, not a complete absence of batch fatigue.

## Schema-copy inventory and field-by-field comparison

This inventory distinguishes the authored entity format from legacy/full-world serialization and the runtime save schema. They share concepts but are not interchangeable JSON documents. “Present” below means a field was found in the scoped entity description, not that a parity test proves its semantics. Rust defaults are **deserialization** defaults; scaffold examples are explicit sample values. Editor types model loaded objects and therefore often require fields that raw authored JSON may omit.

| Copy / representation | Exact locations | What it independently maintains |
|---|---|---|
| Canonical entity structs | `generator/package.rs:29`, `:38`, `:52`, `:104`, `:131`, `:176`; `generator/definitions.rs:234`, `:244`, `:288`, `:325`; `generator/competition_def.rs:27`, `:43`, `:84`, `:104`, `:115`, `:138` | Eight entity kinds, nested fields, serde defaults/aliases; paths in this row are under `src-tauri/crates/ofm_core/src/`. |
| Domain nested definitions | `src-tauri/crates/domain/src/player.rs:174`; `src-tauri/crates/domain/src/staff.rs:32`, `:40`, `:56`; `src-tauri/crates/domain/src/team.rs:330`; `src-tauri/crates/domain/src/league.rs:4`, `:15`, `:24` | Attributes and typed enums used directly by authoring. Engine mirrors are deliberately excluded. |
| File parsing/classification | `src-tauri/crates/ofm_core/src/generator/file_format.rs:13`, `:23`; `generator/package.rs:528`, `:582`, `:763`, `:829`, `:904`, `:1027` | JSON/YAML parser, schema/items envelope, semantic validation. file_format.rs itself has no independent entity structs. |
| Shared scaffolds | `src-tauri/crates/ofm_core/src/generator/scaffold.rs:24`, `:38`, `:99`, `:121`, `:144`, `:157`, `:167`, `:188` | Entity kind list, paths, slug policy, explicit examples and new-package defaults; CLI/editor skeleton use this core implementation. |
| CLI adapter and prose | `src-tauri/crates/ofm-cli/src/main.rs:141`, `:166`, `:188`, `:213`, `:230`, `:237`, `:249`, `:291`, `:1035` | CLI EntityKind mapping plus eight hand-authored SCHEMA_* reference strings; not another validator. |
| Editor types and inputs | `src/components/menu/PackageEditor/types.ts:3`; `helpers.ts:27`, `:48`, `:77`, `:170`, `:237`; `TeamForm.tsx:18`; `StaffForm.tsx:11`; `CompetitionForm.tsx:44`; `MetadataForm.tsx:12` (same directory) | Independent TS types, finite choices, blank-editing objects, slug policy, form field coverage and custom license choices. |
| Frontend schema fixture | `src/components/menu/PackageEditor/schemaFields.generated.json:1`; `schemaParity.test.ts:33`; `src-tauri/crates/ofm_core/src/generator/scaffold.rs:474`, `:783` | Generated top-level field lists, backed by handwritten exhaustive Rust fixture instances. It is **not JSON Schema**. |
| CSV schema projection | `src-tauri/src/commands/package_csv.rs:42`, `:63`, `:77`, `:96`, `:112`, `:206`, `:237`; `docs/modding/PACKAGE_EDITOR.md:240` | Separate team/player header lists, numeric/text classification and row ordering; 19 flattened player attributes. |
| Human references | `docs/modding/SCHEMA_REFERENCE.md:1`; `CLI_REFERENCE.md:1`; `PACKAGE_EDITOR.md:1`; `QUICKSTART.md:1`; `README.md:1`; `INSTALLING_PACKAGES.md:1`; `docs/DEFINITIONS.md:1`; `docs/ARCHITECTURE.md:397` | Tables, examples, requiredness, allowed values, packaging/install descriptions. |
| Full-world formats | `src-tauri/crates/ofm_core/src/generator/definitions.rs:372`, `:399`, `:408`, `:422`, `:440`; `world_io.rs:1` (same directory) | WorldDataMetadata, regions, shard refs, WorldManifestV2 and WorldData, distinct from WorldMetaDef. |
| Package interchange/auxiliary structs | `src-tauri/crates/ofm_core/src/generator/package.rs:195`, `:225`, `:1286`; `src/components/menu/PackageEditor/types.ts:51`, `:250`; `src-tauri/src/commands/package_editor.rs:1` | PackageInfo/WorldPackage, errors/conflicts and editor transport aggregation, rather than new on-disk entity definitions. |

**JSON Schema search:** no checked-in `.ofm` JSON Schema was found. Tracked schema-named files are the frontend key fixture/test and the initial SQL migration. `$schema` in Biome/Tauri configuration describes those tools, not package entities. The CLI `schema` command prints annotated text, not a machine-validating JSON Schema.

**Example copies:** all 11 JSON files under `docs/modding/examples/` were parsed as JSON: mini-league (`package.json`, `teams/teams.json`, `competitions/northshire-premier.json`); classic-sixteen (`package.json`, `teams/teams.json`); academy-showcase (`package.json`, `teams/teams.json`, `players/players.json`, `staff/staff.json`, `names/names.json`, `competitions/harbor-league.json`). They exercise sparse definitions, not every optional/nested field. Syntax parsing is not a claim that their Rust validation or gameplay was run.

### world — every top-level field

Copies: `src-tauri/crates/ofm_core/src/generator/package.rs` (`WorldMetaDef`); `src-tauri/crates/ofm-cli/src/main.rs:141`; `src/components/menu/PackageEditor/types.ts` (`WorldMetaDef`); `docs/modding/SCHEMA_REFERENCE.md:69`. All these top-level field names also occur in the shared scaffold and generated key fixture. `schema`/`items` are loader envelope keys, not struct fields.

| Field | Rust type; omitted input | Editor TS type | Comparison / consumer |
|---|---|---|---|
| `id` | `String`; ""; nonblank checked (`:134`) | `string` | F23: path-safe ID validation is wider than documented slug grammar. |
| `name` | `String`; ""; nonblank checked (`:136`) | `string` | Aligned name/shape; no additional observed divergence. |
| `description` | `String`; "" (`:138`) | `string` | Aligned name/shape; no additional observed divergence. |
| `defaultActiveRegions` | `Vec<String>`; [] (`:140`) | `string[]` | Aligned name/shape; no additional observed divergence. |
| `defaultActiveCompetitions` | `Vec<String>`; [] (`:142`) | `string[]` | Aligned name/shape; no additional observed divergence. |
| `baseYear` | `Option<i32>`; null / absent (`:144`) | `number \| null` | Runtime also anchors generation ages/contracts if no career year is supplied (generator/mod.rs:1090); docs only mention selector display. |
| `version` | `String`; ""; nonblank checked (`:147`) | `string` | F23: nonblank is enforced; semver is only documented. |
| `author` | `String`; "" (`:150`) | `string` | Aligned name/shape; no additional observed divergence. |
| `formatVersion` | `u32`; 0 (`:153`) | `number` | F24: docs/CLI default 1; serde default 0. |
| `license` | `String`; ""; nonblank checked (`:156`) | `string` | F23: nonblank is enforced; SPDX syntax is not. |
| `gameMinVersion` | `String`; "" (`:159`) | `string` | Aligned name/shape; no additional observed divergence. |
| `packageType` | `String`; database (`:162`) | `string` | F23: String, not a validated three-member enum. |
| `logo` | `Option<String>`; null / absent (`:165`) | `string \| null` | Previously known divergence excluded from findings; field included here for inventory completeness. |
| `fallbackLeague` | `Option<FallbackLeagueConfig>`; null / absent (`:169`) | `FallbackLeagueConfig \| null` | Aligned name/shape; no additional observed divergence. |

### team — every top-level field

Copies: `src-tauri/crates/ofm_core/src/generator/definitions.rs` (`TeamDef`); `src-tauri/crates/ofm-cli/src/main.rs:166`; `src/components/menu/PackageEditor/types.ts` (`TeamDef`); `docs/modding/SCHEMA_REFERENCE.md:127`. All these top-level field names also occur in the shared scaffold and generated key fixture. `schema`/`items` are loader envelope keys, not struct fields.

| Field | Rust type; omitted input | Editor TS type | Comparison / consumer |
|---|---|---|---|
| `id` | `String`; "" (`:292`) | `string` | F06: public loader rejects absent/blank; docs promise auto-generation. |
| `name` | `String`; required (`:293`) | `string` | Aligned name/shape; no additional observed divergence. |
| `shortName` | `String`; "" (`:295`) | `string` | F25: CLI says required, serde and reference allow omission. |
| `city` | `String`; required (`:296`) | `string` | Aligned name/shape; no additional observed divergence. |
| `country` | `String`; required (`:298`) | `string` | Aligned name/shape; no additional observed divergence. |
| `colors` | `TeamColorsDef`; required (`:299`) | `TeamColorsDef` | Aligned name/shape; no additional observed divergence. |
| `playStyle` | `String`; Balanced (`:301`) | `string` | Previously known divergence excluded from findings; field included here for inventory completeness. |
| `stadiumName` | `String`; "" (`:303`) | `string` | Aligned name/shape; no additional observed divergence. |
| `reputationRange` | `Option<[u32; 2]>`; null / absent (`:305`) | `[number, number] \| null` | Aligned name/shape; no additional observed divergence. |
| `financeRange` | `Option<[i64; 2]>`; null / absent (`:307`) | `[number, number] \| null` | Core rejects negatives/reversed endpoints; F42 debt example disagrees. |
| `logo` | `Option<String>`; null / absent (`:311`) | `string \| null` | Previously known divergence excluded from findings; field included here for inventory completeness. |
| `kitPattern` | `Option<String>`; null / absent (`:314`) | `KitPattern \| null` | F22: valid tokens agree, but invalid String silently leaves Solid. |
| `foundedYear` | `Option<u32>`; null / absent (`:317`) | `number \| null` | F10: supported in core/CLI/editor/docs; absent from team CSV. |

### player — every top-level field

Copies: `src-tauri/crates/ofm_core/src/generator/package.rs` (`PlayerDef`); `src-tauri/crates/ofm-cli/src/main.rs:188`; `src/components/menu/PackageEditor/types.ts` (`PlayerDef`); `docs/modding/SCHEMA_REFERENCE.md:166`. All these top-level field names also occur in the shared scaffold and generated key fixture. `schema`/`items` are loader envelope keys, not struct fields.

| Field | Rust type; omitted input | Editor TS type | Comparison / consumer |
|---|---|---|---|
| `id` | `String`; "" (`:54`) | `string` | F06. |
| `name` | `String`; "" (`:56`) | `string` | F25: CLI says required. |
| `firstName` | `String`; "" (`:58`) | `string` | F25: CLI says required. |
| `lastName` | `String`; "" (`:60`) | `string` | F25: CLI says required. |
| `club` | `String`; "" (`:62`) | `string` | F01/F25: optional input is dropped from world without a club; CLI says required. |
| `nationality` | `String`; "" (`:64`) | `string` | F25: CLI says required. |
| `position` | `Position`; Goalkeeper (`:66`) | `Position` | Previously known divergence excluded from findings; field included here for inventory completeness. |
| `dateOfBirth` | `Option<String>`; null / absent (`:68`) | `string \| null` | String: date syntax is not checked by serde; generation preserves an explicit string and parses its year separately. |
| `age` | `Option<u32>`; null / absent (`:70`) | `number \| null` | Preserved/type-declared; editor authors DOB rather than an age control. |
| `overall` | `Option<u8>`; null / absent (`:72`) | `number \| null` | F09: u8, not enforced 1–99. |
| `potential` | `Option<u8>`; null / absent (`:79`) | `number \| null` | Aligned field; core additionally enforces 1–99 and ceiling >= effective authored ability. |
| `attributes` | `Option<PlayerAttributes>`; null / absent (`:81`) | `PlayerAttributesDef \| null` | F09/F41: 11 leaves required, 8 default 50; CLI/docs do not enumerate them. |
| `photo` | `Option<String>`; null / absent (`:84`) | `string \| null` | Aligned name/shape; no additional observed divergence. |
| `footedness` | `Option<String>`; null / absent (`:87`) | `Footedness \| null` | F22: missing/null resolves Right in runtime; invalid String also resolves Right. |
| `youth` | `bool`; false (`:90`) | `boolean` | Aligned name/shape; no additional observed divergence. |

### staff — every top-level field

Copies: `src-tauri/crates/ofm_core/src/generator/package.rs` (`StaffDef`); `src-tauri/crates/ofm-cli/src/main.rs:213`; `src/components/menu/PackageEditor/types.ts` (`StaffDef`); `docs/modding/SCHEMA_REFERENCE.md:255`. All these top-level field names also occur in the shared scaffold and generated key fixture. `schema`/`items` are loader envelope keys, not struct fields.

| Field | Rust type; omitted input | Editor TS type | Comparison / consumer |
|---|---|---|---|
| `id` | `String`; "" (`:106`) | `string` | F06. |
| `firstName` | `String`; "" (`:108`) | `string` | F25: CLI says required; generated name fallback exists. |
| `lastName` | `String`; "" (`:110`) | `string` | F25: CLI says required; generated name fallback exists. |
| `club` | `String`; "" (`:113`) | `string` | Aligned name/shape; no additional observed divergence. |
| `nationality` | `String`; "" (`:115`) | `string` | F25: CLI says required; empty runtime fallback ENG. |
| `role` | `StaffRole`; Coach (`:117`) | `StaffRole` | F25: CLI says required; serde defaults Coach. |
| `attributes` | `Option<StaffAttributes>`; null / absent (`:119`) | `StaffAttributesDef \| null` | F09: four u8 leaves, all required when block exists; no 1–99 validation. |
| `specialization` | `Option<CoachingSpecialization>`; null / absent (`:121`) | `string \| null` | Core enum vs wider TS string; form choices agree on all seven valid tokens. |
| `dateOfBirth` | `Option<String>`; null / absent (`:123`) | `string \| null` | Explicit string preserved; year fallback separate, like player DOB. |
| `age` | `Option<u32>`; null / absent (`:125`) | `number \| null` | DOB has precedence; absence uses generated age; no age editing control. |

### confederation — every top-level field

Copies: `src-tauri/crates/ofm_core/src/generator/package.rs` (`ConfederationDef`); `src-tauri/crates/ofm-cli/src/main.rs:230`; `src/components/menu/PackageEditor/types.ts` (`ConfederationDef`); `docs/modding/SCHEMA_REFERENCE.md:300`. All these top-level field names also occur in the shared scaffold and generated key fixture. `schema`/`items` are loader envelope keys, not struct fields.

| Field | Rust type; omitted input | Editor TS type | Comparison / consumer |
|---|---|---|---|
| `id` | `String`; ""; nonempty checked (`:31`) | `string` | Aligned name/shape; no additional observed divergence. |
| `name` | `String`; required (`:32`) | `string` | Aligned name/shape; no additional observed divergence. |

### country — every top-level field

Copies: `src-tauri/crates/ofm_core/src/generator/package.rs` (`CountryDef`); `src-tauri/crates/ofm-cli/src/main.rs:237`; `src/components/menu/PackageEditor/types.ts` (`CountryDef`); `docs/modding/SCHEMA_REFERENCE.md:314`. All these top-level field names also occur in the shared scaffold and generated key fixture. `schema`/`items` are loader envelope keys, not struct fields.

| Field | Rust type; omitted input | Editor TS type | Comparison / consumer |
|---|---|---|---|
| `id` | `String`; ""; nonempty checked (`:40`) | `string` | Aligned name/shape; no additional observed divergence. |
| `name` | `String`; required (`:41`) | `string` | Aligned name/shape; no additional observed divergence. |
| `confederation` | `String`; "" (`:43`) | `string` | Aligned name/shape; no additional observed divergence. |

### competition — every top-level field

Copies: `src-tauri/crates/ofm_core/src/generator/competition_def.rs` (`CompetitionDefinition`); `src-tauri/crates/ofm-cli/src/main.rs:249`; `src/components/menu/PackageEditor/types.ts` (`CompetitionDef`); `docs/modding/SCHEMA_REFERENCE.md:372`. All these top-level field names also occur in the shared scaffold and generated key fixture. `schema`/`items` are loader envelope keys, not struct fields.

| Field | Rust type; omitted input | Editor TS type | Comparison / consumer |
|---|---|---|---|
| `id` | `String`; required (`:44`) | `string` | Aligned name/shape; no additional observed divergence. |
| `name` | `String`; required (`:45`) | `string` | Aligned name/shape; no additional observed divergence. |
| `type` | `CompetitionType`; required (`:46`) | `CompetitionType` | Aligned name/shape; no additional observed divergence. |
| `scope` | `CompetitionScope`; required (`:47`) | `CompetitionScope` | Aligned name/shape; no additional observed divergence. |
| `regionId` | `Option<String>`; null / absent (`:49`) | `string` | F12: docs conditional requirement absent from validation. |
| `countryId` | `Option<String>`; null / absent (`:51`) | `string` | F12: docs conditional requirement absent from validation. |
| `requiredRegionIds` | `Vec<String>`; [] (`:53`) | `string[]` | Declared/preserved; not covered by a dedicated form control. |
| `priority` | `u32`; 0 (`:55`) | `number` | F11: CLI describes opposite rank semantics. |
| `format` | `FormatDef`; required (`:56`) | `FormatDef` | Only kind has a form control; nested settings preserved by object spread. |
| `participants` | `ParticipantSpec`; required (`:57`) | `ParticipantSpec` | Aligned name/shape; no additional observed divergence. |
| `berths` | `Vec<Berth>`; [] (`:61`) | `unknown[]` | Previously known divergence excluded from findings; field included here for inventory completeness. |
| `seasonStartMonth` | `Option<u8>`; null / absent (`:64`) | `number` | Aligned name/shape; no additional observed divergence. |
| `seasonStartDay` | `Option<u8>`; null / absent (`:67`) | `number` | Aligned name/shape; no additional observed divergence. |
| `nameKey` | `Option<String>`; null / absent (`:73`) | `string` | Declared/preserved; not covered by a dedicated form control. |
| `logo` | `Option<String>`; null / absent (`:76`) | `string \| null` | Previously known divergence excluded from findings; field included here for inventory completeness. |

### names — every top-level field

Copies: `src-tauri/crates/ofm_core/src/generator/definitions.rs` (`NamesDefinition`); `src-tauri/crates/ofm-cli/src/main.rs:291`; `src/components/menu/PackageEditor/types.ts` (`NamesDefinition`); `docs/modding/SCHEMA_REFERENCE.md:580`. All these top-level field names also occur in the shared scaffold and generated key fixture. `schema`/`items` are loader envelope keys, not struct fields.

| Field | Rust type; omitted input | Editor TS type | Comparison / consumer |
|---|---|---|---|
| `version` | `u32`; 0 (`:236`) | `number` | F24: docs/CLI say 1, serde defaults 0. |
| `description` | `String`; "" (`:238`) | `string` | Aligned name/shape; no additional observed divergence. |
| `pools` | `HashMap<String, NamePool>`; required (`:240`) | `Record<string, NamePool>` | F02/F26: one keyed document survives per package; CLI BRA example is not catalog BR. |

### Nested fields — complete leaf inventory

Nested leaf locations and defaults follow. All have TS counterparts; absence of a named CLI/doc leaf is called out rather than treated as covered by a top-level object key. Known berth shapes are intentionally excluded. Attribute values in scaffolds are illustrative, not serde defaults.

**colors:** `src-tauri/crates/ofm_core/src/generator/definitions.rs` / `src/components/menu/PackageEditor/types.ts` (`TeamColorsDef`).

| Field | Rust type; omitted input | CLI / docs / editor comparison |
|---|---|---|
| `primary` | `String`; required (`:326`) | Present with the same name/shape in CLI, reference and editor. |
| `secondary` | `String`; required (`:327`) | Present with the same name/shape in CLI, reference and editor. |

**fallback:** `src-tauri/crates/ofm_core/src/generator/package.rs` / `src/components/menu/PackageEditor/types.ts` (`FallbackLeagueConfig`).

| Field | Rust type; omitted input | CLI / docs / editor comparison |
|---|---|---|
| `name` | `Option<String>`; null / absent (`:180`) | Optional exact override; absent uses generated/localized fallback name. |
| `legs` | `Option<u8>`; null / absent (`:183`) | Default runtime 2; metadata form restricts choice to one/two; Rust uses optional u8. |
| `scope` | `Option<CompetitionScope>`; null / absent (`:186`) | Default runtime Domestic; same enum as competition scope. |

**format:** `src-tauri/crates/ofm_core/src/generator/competition_def.rs` / `src/components/menu/PackageEditor/types.ts` (`FormatDef`).

| Field | Rust type; omitted input | CLI / docs / editor comparison |
|---|---|---|
| `kind` | `CompetitionFormat`; required (`:85`) | Present with the same name/shape in CLI, reference and editor. |
| `legs` | `Option<u8>`; null / absent (`:88`) | Absent runtime default 2; core rejects zero; form only edits kind. |
| `groupSize` | `Option<u32>`; null / absent (`:91`) | F03: accepted >=2 but ignored at runtime; groups fixed at four. |
| `qualifiersPerGroup` | `Option<u32>`; null / absent (`:94`) | Absent runtime default 2; passed to group configuration. |
| `bestThirdQualifiers` | `Option<u32>`; null / absent (`:97`) | Absent runtime default 0; passed to group configuration. |

**participants:** `src-tauri/crates/ofm_core/src/generator/competition_def.rs` / `src/components/menu/PackageEditor/types.ts` (`ParticipantSpec`).

| Field | Rust type; omitted input | CLI / docs / editor comparison |
|---|---|---|
| `explicit` | `Option<Vec<String>>`; null / absent (`:107`) | Exactly one branch must be supplied (competition_def.rs:587); TS exposes two optional properties. |
| `selector` | `Option<SelectorSpec>`; null / absent (`:110`) | Exactly one branch must be supplied (competition_def.rs:587); TS exposes two optional properties. |

**selector:** `src-tauri/crates/ofm_core/src/generator/competition_def.rs` / `src/components/menu/PackageEditor/types.ts` (`SelectorSpec`).

| Field | Rust type; omitted input | CLI / docs / editor comparison |
|---|---|---|
| `kind` | `SelectorKind`; required (`:116`) | Four enum tokens agree in Rust, CLI, docs and editor. |
| `country` | `Option<String>`; null / absent (`:119`) | Required by validation for topByReputation/allInCountry. |
| `region` | `Option<String>`; null / absent (`:122`) | Required for allInRegion. |
| `count` | `Option<u32>`; null / absent (`:125`) | topByReputation requires >=2; optional elsewhere. |
| `excludeCompetitions` | `Vec<String>`; [] (`:129`) | F41: CLI omits field; TS requires it, Rust omits it when []. |
| `sourceCompetition` | `Option<String>`; null / absent (`:133`) | Required/known competition for championsOf. |

**namePool:** `src-tauri/crates/ofm_core/src/generator/definitions.rs` / `src/components/menu/PackageEditor/types.ts` (`NamePool`).

| Field | Rust type; omitted input | CLI / docs / editor comparison |
|---|---|---|
| `first_names` | `Vec<String>`; required (`:245`) | Present with the same name/shape in CLI, reference and editor. |
| `last_names` | `Vec<String>`; required (`:246`) | Present with the same name/shape in CLI, reference and editor. |

**playerAttributes:** `src-tauri/crates/domain/src/player.rs` / `src/components/menu/PackageEditor/types.ts` (`PlayerAttributesDef`).

| Field | Rust type; omitted input | CLI / docs / editor comparison |
|---|---|---|
| `pace` | `u8`; required (`:176`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `stamina` | `u8`; required (`:177`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `strength` | `u8`; required (`:178`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `agility` | `u8`; 50 (`:180`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `passing` | `u8`; required (`:183`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `shooting` | `u8`; required (`:184`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `tackling` | `u8`; required (`:185`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `dribbling` | `u8`; required (`:186`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `defending` | `u8`; required (`:187`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `positioning` | `u8`; required (`:190`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `vision` | `u8`; required (`:191`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `decisions` | `u8`; required (`:192`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `composure` | `u8`; 50 (`:194`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `aggression` | `u8`; 50 (`:196`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `teamwork` | `u8`; 50 (`:198`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `leadership` | `u8`; 50 (`:200`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `handling` | `u8`; 50 (`:204`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `reflexes` | `u8`; 50 (`:206`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |
| `aerial` | `u8`; 50 (`:208`) | Rust/editor/CSV/scaffold name agrees; individual leaf absent from CLI and schema reference (F41). Numeric bounds: F09. |

**staffAttributes:** `src-tauri/crates/domain/src/staff.rs` / `src/components/menu/PackageEditor/types.ts` (`StaffAttributesDef`).

| Field | Rust type; omitted input | CLI / docs / editor comparison |
|---|---|---|
| `coaching` | `u8`; required (`:57`) | All four leaves listed by CLI/docs and editor; missing leaf rejects the whole explicit block; bounds F09. |
| `judgingAbility` | `u8`; required (`:59`) | All four leaves listed by CLI/docs and editor; missing leaf rejects the whole explicit block; bounds F09. |
| `judgingPotential` | `u8`; required (`:61`) | All four leaves listed by CLI/docs and editor; missing leaf rejects the whole explicit block; bounds F09. |
| `physiotherapy` | `u8`; required (`:62`) | All four leaves listed by CLI/docs and editor; missing leaf rejects the whole explicit block; bounds F09. |

**Enum inventories checked:** competition types: League, Cup, ContinentalClub, InternationalClub, InternationalNation, FriendlyCup; scopes: Domestic, Regional, Continental, International; formats: LeagueTable, Knockout, GroupAndKnockout; selectors: topByReputation, allInCountry, allInRegion, championsOf; staff roles: AssistantManager, Coach, Scout, Physio; coaching specializations: Fitness, Technique, Tactics, Defending, Attacking, GoalKeeping, Youth. Valid kit-pattern and footedness choices also agree (F22 concerns enforcement). These lists are repeated in Rust, TS/helpers, CLI annotations and reference; no new token mismatch was found.

**Input aliases:** TeamDef explicitly accepts `short_name`, `play_style`, `stadium_name`, `reputation_range`, `finance_range`, `kit_pattern` alongside camelCase; it does not generally accept every snake_case spelling. Staff attributes accept `judging_ability`/`judging_potential` aliases. Names pools intentionally use `first_names`/`last_names`; this is not drift.

**Editor coverage limit:** type presence does not mean an editable control. Player/staff age alternatives, requiredRegionIds, nameKey, selector exclusions and format subsettings are represented but not all authored by controls. Competition kind changes use `...editing.format` (`CompetitionForm.tsx:240`), preserving existing nested fields. This audit does not infer deletion of unknown JS object properties merely from a missing TS declaration.

### Other world/definition shapes, field by field

These are separate formats, not missing fields to add indiscriminately to WorldMetaDef. The field lists below are exhaustive for each named struct; their default/rename declarations were read.

- `WorldDataMetadata` (`src-tauri/crates/ofm_core/src/generator/definitions.rs:374`): `format_version`, `world_id`, `kind`, `base_year`, `snapshot_date`.
- `WorldRegionDefinition` (`src-tauri/crates/ofm_core/src/generator/definitions.rs:400`): `id`, `name`, `countryCodes`.
- `WorldShardRefs` (`src-tauri/crates/ofm_core/src/generator/definitions.rs:409`): `teams`, `players`, `staff`, `managers`, `competitions`, `nationalTeams`, `news`, `stats`, `worldHistory`.
- `WorldManifestV2` (`src-tauri/crates/ofm_core/src/generator/definitions.rs:423`): `formatVersion`, `worldId`, `name`, `description`, `regions`, `defaultActiveRegions`, `defaultActiveCompetitions`, `shards`, `compatibility`.
- `WorldData` (`src-tauri/crates/ofm_core/src/generator/definitions.rs:441`): `name`, `description`, `teams`, `players`, `staff`, `managers`, `competitions`, `competitionDefinitions`, `national_teams`, `regions`, `default_active_regions`, `default_active_competitions`, `league`, `news`, `stats`, `world_history`, `metadata`, `extra_translations`, `build_notices`.
- `NationsDefinition` (`src-tauri/crates/ofm_core/src/generator/definitions.rs:261`): `version`, `description`, `clubsPerDivision`, `colorPalette`, `genericCities`, `nations`.
- `ColorPairDef` (`src-tauri/crates/ofm_core/src/generator/definitions.rs:282`): `primary`, `secondary`.

WorldData and WorldDataMetadata serialize mostly snake_case; the lists above use their actual serialized names; WorldData explicitly renames only `competitionDefinitions`. WorldManifestV2/shard refs/regions use camelCase. WorldData has struct-wide serde defaults. WorldDataMetadata has both field defaults and a distinct Rust Default implementation (format version 1 and a new UUID); these are not interchangeable default mechanisms. `CompetitionDefinitionFile` separately contains `formatVersion` (default 1) and `competitions` (default empty). `NationsDefinition` usability requirements are F07. NationGen additionally declares `code`, `cities`, `style`, `tiers`, `strength` in `src-tauri/crates/ofm_core/src/generator/clubs.rs` (`NationGen`); its finite naming-style enum matches DEFINITIONS.md.

## Save SQL: mechanically compared projection

Applied all 44 registered migration SQL scripts to an in-memory Python SQLite database, and separately executed the SQL fenced block from SAVE_SYSTEM_DESIGN.md. Compared `PRAGMA table_info` and table names. The documented columns that still exist had matching type/null/default/primary-key metadata; differences were omissions plus the date-column name. This validates SQL shape only, not Rust persistence/load behavior. Later additions missing from a historical design are listed without pretending each is a separate production defect.

Tables only in the migrated schema: `cash_journal`, `competitions`, `national_teams`, `player_match_stats`, `team_match_stats`, `transfer_log`, `transfer_rumours`, `youth_scouting_assignments`.

| Table | Columns absent from the handwritten SQL | Columns only in the handwritten SQL |
|---|---|---|
| game_meta | `active_competition_ids_json`, `active_region_ids_json`, `app_version`, `available_staff_market_last_activity_date`, `emitted_events_json`, `extra_translations_json`, `game_date`, `package_lockfile_json`, `save_format_version`, `source_world_id`, `source_world_kind`, `vacant_team_days_json`, `world_format_version`, `world_history_json` | `current_date` |
| managers | `warning_stage` | — |
| teams | `facilities`, `financial_ledger`, `kit_pattern`, `match_roles`, `media_json`, `player_roles_json`, `sponsorship`, `tactics_phase_json`, `training_groups` | — |
| players | `active_loan`, `alternate_positions`, `fitness`, `footedness`, `jersey_number`, `loan_offers`, `media_json`, `morale_core`, `movement_history`, `natural_position`, `ovr`, `potential`, `retired`, `squad_role`, `training_focus`, `weak_foot` | — |
| fixtures | `competition` | — |

The prose comments already mention world_history_json and emitted_events_json; they still are not columns in its SQL block. Full added-table column inventories, from the migrated schema:

- `cash_journal`: `id`, `club_id`, `amount`, `kind`, `date`.
- `competitions`: `id`, `name`, `kind`, `scope`, `season`, `region_id`, `country_id`, `required_region_ids_json`, `participant_ids_json`, `rules_json`, `fixtures_json`, `standings_json`, `knockout_rounds_json`, `transfer_log_json`, `transfer_rumours_json`, `priority`, `groups_json`, `berths_json`, `season_start_month`, `season_start_day`.
- `national_teams`: `id`, `name`, `football_nation`, `region_id`, `squad_player_ids_json`, `manager_name`, `reputation`, `fixtures_json`, `name_key`.
- `player_match_stats`: `fixture_id`, `season`, `matchday`, `date`, `competition`, `player_id`, `team_id`, `opponent_team_id`, `home_team_id`, `away_team_id`, `home_goals`, `away_goals`, `minutes_played`, `goals`, `assists`, `shots`, `shots_on_target`, `passes_completed`, `passes_attempted`, `tackles_won`, `interceptions`, `fouls_committed`, `yellow_cards`, `red_cards`, `rating`.
- `team_match_stats`: `fixture_id`, `season`, `matchday`, `date`, `competition`, `team_id`, `opponent_team_id`, `home_team_id`, `away_team_id`, `goals_for`, `goals_against`, `possession_pct`, `shots`, `shots_on_target`, `passes_completed`, `passes_attempted`, `tackles_won`, `interceptions`, `fouls_committed`, `yellow_cards`, `red_cards`.
- `transfer_log`: `id`, `league_id`, `date`, `from_team_id`, `to_team_id`, `player_id`, `fee`.
- `transfer_rumours`: `id`, `league_id`, `rumour_id`, `date`, `player_id`, `player_name`, `team_id`, `team_name`.
- `youth_scouting_assignments`: `id`, `scout_id`, `days_remaining`, `target_position`, `region`, `objective`.

Migration sources for those additions: `src-tauri/crates/db/src/sql/v015_match_stats_history.sql:1`, `v018_transfer_log.sql:1`, `v021_youth_scouting_assignments.sql:1`, `v024_transfer_rumours.sql:1`, `v030_competitions_and_national_teams.sql:1`, `v044_cash_journal.sql:1`; registration order is `src-tauri/crates/db/src/migrations.rs:43`. The retained legacy `league` table alongside `competitions` is migration compatibility, not an accidental duplicate reported here.


## Fixtures/factories: rule checks and repeated-helper inventory

The only file under `src/test-utils/` is `factories.ts`, exporting `createTeam`, `createPlayer`, `createStaff` at lines 3/33/92. F21 is a confirmed invalid-enum default. F42 is a confirmed difference between a CSV fixture's claimed validity and package validation. F14/F41 cover the schema fixtures' incomplete contract. The known duplicated locale maps were not counted as new findings.

The following concrete fixture contracts were compared:

| Concept | Locations | Drifted or in sync | Source of truth / generation | Severity |
|---|---|---|---|---|
| Full procedural squad composition | `src-tauri/crates/ofm_core/tests/live_match_manager_tests.rs:78`; `src-tauri/crates/ofm_core/src/generator/generation.rs:378`; `docs/DEFINITIONS.md:134` | **In sync:** 22 total, 2 GK / 7 DEF / 7 MID / 6 FWD. The test hand-maintains the distribution, which could drift. | Export a core squad-layout description for fixtures that specifically mean a generated squad; keep deliberate small scenarios separate. | P3 maintenance risk only |
| Minimal match XI | `src-tauri/crates/ofm_core/tests/turn_tests.rs:131`; `src-tauri/crates/engine/tests/simulation_tests.rs:41`; `src-tauri/crates/engine/tests/live_match_tests.rs:43`; `src-tauri/crates/engine/src/live_match/zone_resolution.rs:568`; `src/test-utils/factories.ts:19` | **In sync:** the fixture XIs encode 4-4-2 (1/4/4/2). The 11-player turn fixture versus 22-player live-manager squad is intentional scenario size, not an invalid squad-generation default. | Named engine test XI builder could share the formation layout. Do not bridge engine tests to domain to deduplicate types. | P3 maintenance risk only |
| Domain default constructor reuse | `src-tauri/crates/ofm_core/tests/turn_tests.rs:79`, `:95`; `src-tauri/crates/ofm_core/tests/training_tests.rs:40`, `:56`; `src-tauri/crates/ofm_core/tests/live_match_manager_tests.rs:51`, `:67` | **In sync at the inspected constructors:** these helpers use Player::new/Team::new and then explicit scenario overrides; they are not separate implementations of all production defaults. | Keep constructor calls; share a testkit only where scenario intent is the same. Arbitrary skills/wages are test inputs, not necessarily default drift. | No new defect |
| Nineteen attribute keys | `src/test-utils/factories.ts:44`; `src-tauri/crates/ofm_core/tests/turn_tests.rs:25`; `src-tauri/crates/ofm_core/tests/training_tests.rs:15`; `src-tauri/crates/domain/src/player.rs:174`; `src/components/menu/PackageEditor/helpers.ts:77`; `src-tauri/src/commands/package_csv.rs:112` | **In sync on field inventory:** all 19 attributes occur. Test ratings 60/30/20 are deliberate fixture choices, not replacements for serde's optional-attribute default 50. | Rust typed literals protect field presence; generated TS fixture shape can protect the cross-language copy. | P3 maintenance risk only |
| Nationality population input in an old test | `src-tauri/crates/ofm_core/src/generator/mod.rs:2276`; `src-tauri/crates/ofm_core/src/generator/generation.rs:269`; `generator/mod.rs:271` (same core directory) | **Drifted fixture premise, bounded claim:** test input still comes from name-pool keys; production now uses weighted catalog distribution. Its >30/100 domestic-bias assertion remains a valid helper test, but is not evidence that the current population distribution is covered. New catalog-specific tests also exist; no claim that the whole feature lacks tests. | Derive a production-distribution fixture when testing defaults; retain a small injected distribution when testing only the helper's domestic bias. | P3 coverage interpretation |

### Repeated Rust helper names, with every location returned by the construction-helper scan

This mechanical inventory searched integration/testkit files and inline `#[cfg(test)]` regions for player/team/staff/squad/game/attribute/manager construction helpers, excluding immediately `#[test]`-annotated cases. It found **12 repeated names at 166 sites**. It is an index for consolidation, not 166 defects or proof that functions sharing a name have identical semantics. The checked rule-bearing examples and confirmed drift are distinguished above; arbitrary fixture differences are not promoted to findings.

- **`make_team` (41 sites):** `src-tauri/crates/engine/src/live_match/zone_resolution.rs:568`; `src-tauri/crates/engine/tests/live_match_tests.rs:46`; `src-tauri/crates/engine/tests/simulation_tests.rs:41`; `src-tauri/crates/ofm_core/src/ai_hiring.rs:378`; `src-tauri/crates/ofm_core/src/ai_training.rs:318`; `src-tauri/crates/ofm_core/src/board_objectives.rs:323`; `src-tauri/crates/ofm_core/src/finances/journal.rs:261`; `src-tauri/crates/ofm_core/src/finances/mod.rs:1244`; `src-tauri/crates/ofm_core/src/history_generation.rs:606`; `src-tauri/crates/ofm_core/src/player_identity.rs:476`; `src-tauri/crates/ofm_core/src/roster.rs:88`; `src-tauri/crates/ofm_core/src/season_awards.rs:386`; `src-tauri/crates/ofm_core/src/season_context.rs:195`; `src-tauri/crates/ofm_core/src/slices/session.rs:211`; `src-tauri/crates/ofm_core/src/state.rs:219`; `src-tauri/crates/ofm_core/src/transfers/tests.rs:10`; `src-tauri/crates/ofm_core/src/turn/dormant.rs:70`; `src-tauri/crates/ofm_core/src/turn/mod.rs:265`; `src-tauri/crates/ofm_core/src/turn/news.rs:825`; `src-tauri/crates/ofm_core/tests/club_tests.rs:8`; `src-tauri/crates/ofm_core/tests/contracts_tests.rs:79`; `src-tauri/crates/ofm_core/tests/end_of_season_tests.rs:20`; `src-tauri/crates/ofm_core/tests/finances_tests.rs:19`; `src-tauri/crates/ofm_core/tests/live_match_manager_tests.rs:66`; `src-tauri/crates/ofm_core/tests/player_events_tests.rs:60`; `src-tauri/crates/ofm_core/tests/random_events_tests.rs:60`; `src-tauri/crates/ofm_core/tests/scouting_tests.rs:58`; `src-tauri/crates/ofm_core/tests/slices_players_tests.rs:33`; `src-tauri/crates/ofm_core/tests/slices_squad_tests.rs:33`; `src-tauri/crates/ofm_core/tests/slices_staff_tests.rs:34`; `src-tauri/crates/ofm_core/tests/slices_teams_tests.rs:34`; `src-tauri/crates/ofm_core/tests/training_tests.rs:55`; `src-tauri/crates/ofm_core/tests/turn_tests.rs:94`; `src-tauri/src/commands/club.rs:105`; `src-tauri/src/commands/contracts.rs:498`; `src-tauri/src/commands/finances.rs:173`; `src-tauri/src/commands/live_match.rs:385`; `src-tauri/src/commands/messages.rs:265`; `src-tauri/src/commands/squad.rs:784`; `src-tauri/src/commands/staff.rs:65`; `src-tauri/src/commands/util.rs:111`.
- **`make_game` (36 sites):** `src-tauri/crates/ofm_core/src/aging.rs:198`; `src-tauri/crates/ofm_core/src/ai_hiring.rs:437`; `src-tauri/crates/ofm_core/src/board_objectives.rs:337`; `src-tauri/crates/ofm_core/src/finances/journal.rs:275`; `src-tauri/crates/ofm_core/src/finances/mod.rs:1261`; `src-tauri/crates/ofm_core/src/firing.rs:315`; `src-tauri/crates/ofm_core/src/history_generation.rs:662`; `src-tauri/crates/ofm_core/src/job_offers.rs:685`; `src-tauri/crates/ofm_core/src/roster.rs:100`; `src-tauri/crates/ofm_core/src/season_awards.rs:420`; `src-tauri/crates/ofm_core/src/season_context.rs:243`; `src-tauri/crates/ofm_core/src/slices/schedule.rs:278`; `src-tauri/crates/ofm_core/src/squad_safety.rs:222`; `src-tauri/crates/ofm_core/src/transfers/tests.rs:51`; `src-tauri/crates/ofm_core/src/turn/news.rs:933`; `src-tauri/crates/ofm_core/tests/club_tests.rs:22`; `src-tauri/crates/ofm_core/tests/contracts_tests.rs:113`; `src-tauri/crates/ofm_core/tests/player_events_tests.rs:72`; `src-tauri/crates/ofm_core/tests/random_events_tests.rs:72`; `src-tauri/crates/ofm_core/tests/scouting_tests.rs:88`; `src-tauri/crates/ofm_core/tests/slices_players_tests.rs:109`; `src-tauri/crates/ofm_core/tests/slices_squad_tests.rs:60`; `src-tauri/crates/ofm_core/tests/slices_staff_tests.rs:86`; `src-tauri/crates/ofm_core/tests/slices_teams_tests.rs:86`; `src-tauri/crates/ofm_core/tests/training_tests.rs:86`; `src-tauri/src/commands/club.rs:120`; `src-tauri/src/commands/contracts.rs:514`; `src-tauri/src/commands/finances.rs:226`; `src-tauri/src/commands/messages.rs:338`; `src-tauri/src/commands/squad.rs:814`; `src-tauri/src/commands/staff.rs:103`; `src-tauri/src/commands/stats/tests.rs:53`; `src-tauri/src/commands/time.rs:540`; `src-tauri/src/commands/transfers.rs:604`; `src-tauri/src/commands/util.rs:94`; `src-tauri/src/commands/world.rs:580`.
- **`make_player` (36 sites):** `src-tauri/crates/engine/src/live_match/zone_resolution.rs:536`; `src-tauri/crates/engine/tests/live_match_tests.rs:14`; `src-tauri/crates/engine/tests/simulation_tests.rs:9`; `src-tauri/crates/ofm_core/src/aging.rs:164`; `src-tauri/crates/ofm_core/src/ai_training.rs:303`; `src-tauri/crates/ofm_core/src/board_objectives.rs:308`; `src-tauri/crates/ofm_core/src/history_generation.rs:645`; `src-tauri/crates/ofm_core/src/national_team.rs:481`; `src-tauri/crates/ofm_core/src/player_identity.rs:464`; `src-tauri/crates/ofm_core/src/player_rating.rs:539`; `src-tauri/crates/ofm_core/src/player_wear.rs:112`; `src-tauri/crates/ofm_core/src/roster.rs:73`; `src-tauri/crates/ofm_core/src/season_awards.rs:398`; `src-tauri/crates/ofm_core/src/state.rs:203`; `src-tauri/crates/ofm_core/src/turn/mod.rs:280`; `src-tauri/crates/ofm_core/src/turn/news.rs:903`; `src-tauri/crates/ofm_core/tests/contracts_tests.rs:44`; `src-tauri/crates/ofm_core/tests/end_of_season_tests.rs:32`; `src-tauri/crates/ofm_core/tests/finances_tests.rs:34`; `src-tauri/crates/ofm_core/tests/live_match_manager_tests.rs:49`; `src-tauri/crates/ofm_core/tests/player_events_tests.rs:44`; `src-tauri/crates/ofm_core/tests/player_rating_tests.rs:58`; `src-tauri/crates/ofm_core/tests/random_events_tests.rs:45`; `src-tauri/crates/ofm_core/tests/scouting_tests.rs:42`; `src-tauri/crates/ofm_core/tests/slices_squad_tests.rs:46`; `src-tauri/crates/ofm_core/tests/slices_teams_tests.rs:47`; `src-tauri/crates/ofm_core/tests/training_tests.rs:39`; `src-tauri/crates/ofm_core/tests/transfers_tests.rs:47`; `src-tauri/crates/ofm_core/tests/turn_tests.rs:73`; `src-tauri/src/commands/club.rs:90`; `src-tauri/src/commands/contracts.rs:452`; `src-tauri/src/commands/finances.rs:190`; `src-tauri/src/commands/live_match.rs:369`; `src-tauri/src/commands/squad.rs:796`; `src-tauri/src/commands/stats/tests.rs:38`; `src-tauri/src/commands/time.rs:526`.
- **`default_attrs` (25 sites):** `src-tauri/crates/ofm_core/src/ai_training.rs:279`; `src-tauri/crates/ofm_core/src/board_objectives.rs:284`; `src-tauri/crates/ofm_core/src/roster.rs:49`; `src-tauri/crates/ofm_core/src/season_awards.rs:362`; `src-tauri/crates/ofm_core/src/squad_safety.rs:198`; `src-tauri/crates/ofm_core/src/state.rs:168`; `src-tauri/crates/ofm_core/src/turn/news.rs:879`; `src-tauri/crates/ofm_core/tests/contracts_tests.rs:20`; `src-tauri/crates/ofm_core/tests/live_match_manager_tests.rs:15`; `src-tauri/crates/ofm_core/tests/player_events_tests.rs:20`; `src-tauri/crates/ofm_core/tests/random_events_tests.rs:21`; `src-tauri/crates/ofm_core/tests/scouting_tests.rs:18`; `src-tauri/crates/ofm_core/tests/slices_players_tests.rs:9`; `src-tauri/crates/ofm_core/tests/slices_squad_tests.rs:9`; `src-tauri/crates/ofm_core/tests/slices_teams_tests.rs:10`; `src-tauri/crates/ofm_core/tests/training_tests.rs:15`; `src-tauri/crates/ofm_core/tests/transfers_tests.rs:23`; `src-tauri/crates/ofm_core/tests/turn_tests.rs:25`; `src-tauri/src/commands/club.rs:66`; `src-tauri/src/commands/contracts.rs:428`; `src-tauri/src/commands/live_match.rs:343`; `src-tauri/src/commands/squad.rs:753`; `src-tauri/src/commands/stats/tests.rs:14`; `src-tauri/src/commands/time.rs:502`; `src-tauri/src/commands/transfers.rs:520`.
- **`make_staff` (8 sites):** `src-tauri/crates/ofm_core/src/ai_hiring.rs:390`; `src-tauri/crates/ofm_core/src/history_generation.rs:626`; `src-tauri/crates/ofm_core/src/turn/mod.rs:316`; `src-tauri/crates/ofm_core/tests/finances_tests.rs:71`; `src-tauri/crates/ofm_core/tests/slices_staff_tests.rs:47`; `src-tauri/crates/ofm_core/tests/training_tests.rs:67`; `src-tauri/crates/ofm_core/tests/turn_tests.rs:106`; `src-tauri/src/commands/staff.rs:79`.
- **`make_manager` (5 sites):** `src-tauri/crates/ofm_core/src/ai_training.rs:335`; `src-tauri/crates/ofm_core/src/player_identity.rs:488`; `src-tauri/crates/ofm_core/src/season_awards.rs:433`; `src-tauri/crates/ofm_core/src/slices/session.rs:199`; `src-tauri/crates/ofm_core/src/turn/news.rs:837`.
- **`make_squad` (4 sites):** `src-tauri/crates/ofm_core/src/state.rs:231`; `src-tauri/crates/ofm_core/tests/live_match_manager_tests.rs:79`; `src-tauri/crates/ofm_core/tests/turn_tests.rs:131`; `src-tauri/src/commands/live_match.rs:397`.
- **`sample_attrs` (3 sites):** `src-tauri/crates/ofm_core/src/football_identity.rs:206`; `src-tauri/crates/ofm_core/src/scouting.rs:944`; `src-tauri/src/commands/world.rs:556`.
- **`make_game_with_fixture` (2 sites):** `src-tauri/crates/ofm_core/src/state.rs:273`; `src-tauri/crates/ofm_core/tests/live_match_manager_tests.rs:120`.
- **`make_player_with_position` (2 sites):** `src-tauri/crates/ofm_core/tests/contracts_tests.rs:72`; `src-tauri/src/commands/contracts.rs:470`.
- **`make_squad_game` (2 sites):** `src-tauri/crates/ofm_core/tests/contracts_tests.rs:134`; `src-tauri/src/commands/contracts.rs:535`.
- **`sample_attributes` (2 sites):** `src-tauri/crates/domain/src/player.rs:699`; `src-tauri/crates/ofm_core/src/transfers/tests.rs:27`.

## Checked, genuinely in sync

- **Rust pin:** `rust-toolchain.toml:13` and workflow toolchain selections agree on **1.95.0**. Executed `bash scripts/check-toolchain-pin.sh` and `bash scripts/check-toolchain-pin.test.sh`: the real checker passed and **all 17** positive/negative cases behaved as expected, including continuations and CRLF. Committed/generated fixture pins repeat 1.95.0 deliberately as checker inputs; they do not select the production compiler.
- **App/Biome versions:** `package.json:4`, `src-tauri/Cargo.toml:3`, `src-tauri/tauri.conf.json:4` agree on app **0.3.0**. The four library crates agree on 0.3.0; CLI/sim-bench 0.1.0 are independently versioned binaries. `package.json:40`, its lockfile, and `biome.json:2` agree on **2.5.5**. CI's `Version: 2.` assertion checks executable identity, not exact-version parity; npm's exact pin supplies that.
- **Oversized-file gate:** `scripts/quality-metrics.mjs:26` uses Rust >1,000 and TS >500 lines; a read-only recount using its newline-counting/skip rules exactly matched `quality-baseline.json:2`: **65,901 Rust / 35,970 TS aggregate oversized lines**, no new/grown entries and no stale entries. This is only the file-size portion, not a run of lint/dead-code/other quality metrics. CLAUDE's approximately 1,500-line “consider splitting” guidance (`CLAUDE.md:164`, `src-tauri/CLAUDE.md:164`) is advisory, while the ratchet prevents additional debt; these are different obligations, not contradictory thresholds for one gate.
- **Release platform lists:** `.github/workflows/tauri-action.yml:27`, `.github/workflows/nightly-tauri-action.yml:107`, and `src-tauri/deny.toml:22` describe the same five targets: Linux x64/arm64, Windows x64, macOS x64/arm64. The repeated matrices can be generated from one target list; no target mismatch was found. Repeated Linux package-install lists and Rust-cache workspace paths also agree where used.
- **Dependency gates:** `.github/workflows/build-check.yml:94` and `.github/workflows/dependency-advisories.yml:43` pin cargo-deny **0.20.2**, and both use locked/workspace/all-features selection with the same manifest. License/bans/sources versus advisories are intentionally different subchecks. No dependency build or live advisory fetch was run.
- **Shared release metadata implementation:** both manifest workflows call `.github/scripts/generate-release-manifest.mjs`; stable/nightly output names are explicit configuration. Stable excludes automatic nightly events (`release-manifest.yml:18`). Different release names/triggers are policy, not duplicate release-manifest logic.
- **Mutation workflow list:** manual choices and rotating array both name engine/domain/ofm_core/db (`.github/workflows/mutation-tests.yml:29`, `:85`). The week-number rotation actually uses the four-entry array; CLI/app omission is an intentionally scoped mutation program, not an inconsistent workspace list.
- **Performance matrix:** `scripts/perf/run-matrix.sh:43`, `:56` and `scripts/perf/report.mjs:23` agree on all ten profile row names/order. They could share a data file. Collector/report output-directory defaults agree. No benchmark/app process was launched. Vite's main port **1420** matches Tauri devUrl (`vite.config.ts:161`, `src-tauri/tauri.conf.json:8`).
- **Archive limits:** `docs/ARCHITECTURE.md:459` matches `src-tauri/crates/ofm_core/src/generator/package.rs:1667`: 256 MiB compressed, 1 GiB total uncompressed, 10,000 entries (docs use MB/GB colloquially). The separate 16 MiB cap at `:1674` applies to isolated entry reads, not a competing whole-archive limit; importer uses the shared compressed-size constant (`src-tauri/src/commands/world.rs:239`).
- **Scaffolding/validation architecture:** core EntityKind, shared skeleton writer and typed templates are genuinely shared, with exhaustive template deserialization tests present. Eight top-level editor interfaces cover the generated field list. This is substantial existing protection; F14/F41 identify its exact remaining limits. CLI does not maintain an independent validation algorithm.
- **Gameplay tables checked:** training focus attribute groups, 0.5/1/1.5 intensity gain multipliers, 3/6/10 costs, age growth factors and 6/4/2-day schedules still match (`docs/GAME_SYSTEMS.md:68`, `:82`, `:91`, `:110`; `training.rs:185`, `:219`, `:269`, `:364`; `domain/team.rs:292`). The 19 engine attribute names, helper overall mean, four unchanged MatchConfig defaults, play-style multipliers and trait-bonus table also match their named helpers (`engine/types.rs:142`, `:359`; `engine/shared.rs:160`, `:260`). This does not certify every simulation path uses those helpers identically (Part 2 item 7).

## Verification record and limits

- Covered the documents listed above, the four checked-in AGENTS/CLAUDE files, all seven workflow files, package/Cargo/Biome/baseline configuration, script entry points, schema representations, and the producer/consumer paths cited. Static searches inventoried fixtures and schema occurrences; name matches are separated from verified divergences throughout.
- Read-only probes: the toolchain checker plus 17 regression cases; JSON syntax reads of 11 modding examples; an exact oversized-file recount; 44 migration SQL scripts and documented SQL applied to two **in-memory** SQLite databases; static event and test counts; Cargo argument parsing demonstrating the bare `--bin` error.
- No Rust or frontend suite was run, no MCP/game scenario was played, and no benchmark/release/advisory operation was performed. Source-proved examples are labeled as such. The complete quality gate was not claimed from its file-size subcheck.
- A final pass verified the referenced repository paths and line bounds, that all 42 finding records carry their fields, and that all seven verification verdicts are present.
