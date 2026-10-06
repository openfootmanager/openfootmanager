# CLAUDE.md — OpenFoot Manager

Guidance for Claude Code and other AI coding agents working in this repository.

This file is an **index and a rulebook**, not a second architecture document. Anything already
explained in `docs/` is linked from here, never restated, so the two cannot drift apart.

---

## What this project is

A desktop football management simulation: **Tauri v2** shell, **Rust** backend (4 library crates
plus a CLI), **React 19 + TypeScript + Tailwind v4** frontend. GPLv3.

Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) before your first non-trivial change.

---

## Commands

### Frontend

```bash
npm install                 # once
npm test                    # vitest run — the full frontend suite (~150 test files)
npm exec --no -- vitest run <path>   # one file or directory (after npm ci — see below)
npm run build               # tsc && vite build — type errors fail here
npm run tauri dev           # run the real app (Vite + Tauri together)
npm run audit:i18n          # advisory hardcoded-string report (see caveat below)
```

### Backend

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace     # everything
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets
cargo fmt --manifest-path src-tauri/Cargo.toml --all            # format before committing
```

Run backend commands from `src-tauri/` if you prefer; the `--manifest-path` form works from the
repository root.

### Two gotchas worth memorising

- **Tauri command tests live in the `openfootmanager_lib` lib target.** Use
  `cargo test --manifest-path src-tauri/Cargo.toml --lib` (or plain `cargo test --lib` from
  inside `src-tauri/` — there is no manifest at the repository root).
  `cargo test --bin` matches **zero** tests — the `[lib] name` is deliberately suffixed
  (`src-tauri/Cargo.toml`), so the binary target contains almost nothing.
- **`npm run audit:i18n` never fails.** It is a heuristic reporter that prints candidates and
  exits 0 (`scripts/audit-i18n.mjs`). Read its output; do not treat a clean run as a pass. The
  real i18n gate is `npm exec --no -- vitest run src/i18n`.

### `npm ci` first, then `npm run <script>` — never `npx <tool>`

There is an unrelated package on npm literally called `biome`, last published at 0.3.3. `npx
biome` finds *that*, prints nothing, and exits 0 — a lint or format run that looks clean because
it never happened.

`npm exec --no` is a weaker guard against it than it looks. `--no` refuses to *fetch*, but it will
still run whatever `~/.npm/_npx` already holds, and a single earlier `npx biome` in any checkout
on the machine is enough to put the impostor there. In a fresh worktree with no `node_modules`,
`npm exec --no -- biome --version` prints `0.3.3` and exits 0; `npm run format:check` in that same
tree fails loudly with `biome: command not found`. On a tree that has not been installed the two
forms fail in **opposite** directions — one silent and green, the other loud and red.

That is not hypothetical either: it was sprung here on 14 Sep 2026, on a formatter run reported as
a pass that had formatted nothing.

So: `npm ci` before anything else in a new worktree, and prefer `npm run <script>`, which puts
`node_modules/.bin` first on `PATH`. `npm exec --no -- <tool>` is for ad-hoc flags a script does
not expose, and is only as trustworthy as the install underneath it. CI asserts Biome's version
before trusting it.

---

## Non-negotiables

Six rules. CI checks some mechanically; the review requirements and evidence are stated below.

1. **TDD.** Write the failing test first, then the code. Rust unit tests go in a `#[cfg(test)]`
   module in the same file; frontend tests are co-located as `*.test.ts(x)`. A PR that adds
   behaviour without a test that would have caught its absence is incomplete.

2. **Every string a *player* reads is translated into every locale the game ships in.** Not just
   `en.json`. The list is `SUPPORTED_LANGUAGES` in `src/i18n/index.ts`; it grows.
   → use [`/add-ui-string`](.claude/skills/add-ui-string/SKILL.md).
   → enforced by `src/i18n/localeCoverage.test.ts` and `src/i18n/frontendKeyCoverage.test.ts`.

   **"A player reads it" is the test, and it is not the same as "a human reads it".** In scope: the
   whole UI, and any backend string that reaches it — a Tauri command's error is a translation key
   (`be.error.*`), never English prose, because the player sees it.

   Out of scope, deliberately, and English is correct there:

   | Surface | Why |
   |---|---|
   | MCP tool output (`src-tauri/src/mcp_server/tools_impl/`) | The reader is an AI agent, not a player. Its errors are rendered for the agent by `tools.rs::err_result`, so a private MCP error may be plain English or a key. The exception: an MCP function that propagates an error from a shared `application::` service which also backs a Tauri command must keep the `be.error.*` key, because the UI shows it on that path. |
   | `src-tauri/crates/ofm-cli/` output and its scaffold templates | A modder at a terminal, not a player in the game. |
   | `docs/`, including `docs/modding/` | Developer and modder documentation. |
   | Code comments, log lines, panic messages | Nobody ships these to a player. |

   This is here because reviewers kept raising it as a defect on all three out-of-scope surfaces —
   see the closed threads on #479 (MCP markdown) and #437 (CLI scaffold comments). Both are working
   as intended. If a surface is genuinely ambiguous, ask rather than translating on spec: a key that
   no player will ever see still costs thirteen translations and a row in every locale file.

3. **`engine` never imports `domain`.** The match engine defines its own mirror types on purpose
   so it can be tested and evolved independently; `ofm_core/turn/` is the only bridge. This is
   the project's central architectural decision — see `docs/ARCHITECTURE.md` §"Engine Isolation".
   → checked by the `ofm-architecture-reviewer` agent.

4. **Every new serialized field gets `#[serde(default)]`.** Old save files must keep loading.
   For fields that also hit SQLite, `serde(default)` alone is not enough — see
   [`src-tauri/CLAUDE.md`](src-tauri/CLAUDE.md).

5. **No `any` in TypeScript. Tailwind utilities, not raw CSS.** Design tokens live in
   `src/App.css` under `@theme`; use the token classes, never hex literals.

6. **Conventional commits, branched from and merged into `develop`.** Match the existing history:
   `fix(ui):`, `feat(world-cup):`, `test(training):`, `refactor(review):`, `chore(...)`. Never
   commit directly to `develop`.

<!-- ofm-code-quality:start -->
## Code quality

These are merge requirements. A green linter does not prove architecture or test quality: run
`/preflight` and the reviewers named below, and report what you ran and what you did not. Never
describe an unrun check as passing. **CI** = a required job goes red. **test** = a named test in the
suite goes red. **review** = a read-only agent reports it and a human decides.

### Size and shape (new and changed production code; limits are ceilings, not targets)

| Rule | Rust | Frontend | Enforced by |
|---|---|---|---|
| Function length | <= 100 lines (`clippy::too_many_lines`) | <= 200 lines (`complexity/noExcessiveLinesPerFunction`); <= 100 in `src/{lib,utils,services,store}` | review until PR 9 (Rust), PR 7 (frontend) |
| Cognitive complexity | <= 25 (`clippy::cognitive_complexity`) | <= 25; <= 15 in `src/{lib,utils,services,store}` | review until PR 9 (Rust), PR 7 (frontend) |
| Nesting | <= 5 (`clippy::excessive_nesting`) | covered by complexity | review until PR 9 (Rust), review (frontend) |
| Parameters | <= 7 (`clippy::too_many_arguments`) | <= 7 (`complexity/useMaxParams`) | CI (Rust, existing suppressions reviewed until PR 3); review until PR 7 (frontend) |
| File length | <= 1,500 lines up to the first `#[cfg(test)]` | <= 1,000 lines, test files excluded | review until PR 5 (Rust), PR 8 (frontend) |
| Tauri command | <= 50 lines, delegates to an `_internal` fn that the MCP tool shares | - | review until PR 4 |
| Test files | integration test file <= 3,000 lines | a vitest file <= 1,000 | review until PR 5 (Rust), PR 8 (frontend) |

Extract cohesive operations, not arbitrary chunks that only satisfy a line limit. Do not shorten
names, delete useful comments, or weaken tests to meet a limit. A limit that is not yet machine-
enforced for a crate or directory is still a review requirement for the units you change.

The PR numbers above and below refer to the enforcement programme sequence, not GitHub PR IDs.
The starting limits will be tightened in PR 11 (Rust cognitive 15, nesting 4, files 1,000;
frontend files 500).

### Suppressions
- `#[allow(..)]` is banned (`clippy::allow_attributes`, `allow_attributes_without_reason`): review until PR 3. Use
  `#[expect(lint, reason = "<constraint>; owner=#NNN")]`; it fails the build when the lint stops
  firing. `biome-ignore` carries a reason after the colon. Never a broad or file-wide suppression.
- No committed debt baseline, ratchet snapshot, or advisory-level lint setting. A new mechanical
  check enters required CI at zero findings with a fixture that proves it fails.

### Naming
- Football terms, not type terms: `deployed_position`, not `pos2`. Keep natural vs deployed position,
  annual vs weekly money, IDs vs array indices distinct. Boolean names state a predicate. rustc naming
  lints are hard errors; the rest is review (`ofm-architecture-reviewer`). Renames must preserve stored
  fields and IPC contracts unless a migration ships with them.
- Tests use a plain sentence name (snake_case in Rust) plus a Given/When/Then doc comment
  naming the scenario. Review (`ofm-test-reviewer`).

### Layering and dependency direction
- Rust crate graph, all dependency kinds: `engine: []`, `domain: []`, `ofm_core: [domain, engine]`,
  `db: [domain, ofm_core]`, `ofm-cli: [ofm_core]`, `sim-bench: [engine]`, root app composes all.
  New workspace member must be classified. test for leaf isolation today
  (`src-tauri/tests/architecture.rs`); review for the full matrix until PR 4
- `engine` types (`TeamData`, `PlayerData`, `TacticsConfig`) are constructed only under
  `ofm_core/src/turn/`, except for synthetic inputs in `src-tauri/crates/sim-bench/src/builder.rs`
  and `src-tauri/src/commands/sim_lab.rs` until the engine overhaul removes the old instant engine.
  Other modules may drive the engine, never build its input. review until PR 4
- `src-tauri/src/application/` never imports `commands` or `mcp_server`. Commands and MCP tools adapt
  the same `_internal` function. review until PR 4 (imports) + review (duplication)
- `domain` holds data, constructors, `Default` and pure value semantics. A rule that decides an outcome
  from attributes, and anything with I/O, lives in `ofm_core`. review (`ofm-architecture-reviewer`)
- Keep helpers private or `pub(crate)` until a second caller needs them. review
- Frontend (`src/architecture.test.ts`): `src/lib`, `src/utils`, `src/services`, `src/store` import no
  component or page; `src/components/ui` imports no feature folder or page; features do not import
  `src/pages`; `invoke` from `@tauri-apps/api/core` appears only in `src/services/`. Type-only imports
  and re-exports obey the same rules. review until PR 8
- Frontend presents backend policy; it does not recompute a game rule Rust owns. Prefer a Rust
  projection or generated type over a second hand-written formula. review

### Errors, comments, dead code, duplication
- No `unwrap`, `expect`, `panic!` in production Rust; return `Result` with a translation-key error.
  An invariant that cannot be a `Result` gets `#[expect(.., reason)]` naming it. review until PR 9
  (`clippy::unwrap_used`, `expect_used`, `panic`, `allow-*-in-tests = true`)
- No empty `catch {}` or ignored `Result`; validate before mutating live state; use
  `mutate_active_game` for atomic writes; never hold a lock across `await`. review until PR 6 (Biome
  `noEmptyBlockStatements`) + review
- Comments explain why (invariant, unit, compatibility, removal condition), never narrate. No
  commented-out code. review
- Dead code is a build error: rustc `dead_code` under `-D warnings`, `cargo-machete`, `knip`. CI
- One authoritative rule: search `ofm_core`, `src/lib`, `src/utils`, `src/services`,
  `src/components/ui/index.ts` before adding logic. A duplicate you cannot consolidate is named in
  the PR body for epic #589. review (`ofm-dedup-reviewer`)

### SOLID, as reviewable questions (no honest tool counts these)
Single responsibility: name the one reason the unit changes. Open/closed and Liskov: a replacement
keeps documented inputs, outcomes, errors, ordering and save compatibility, and closed enums stay
exhaustive (`wildcard_enum_match_arm` once adopted). Interface segregation: accept only the state an
operation needs, not the whole `Game`. Dependency inversion: put a trait at the consuming boundary only
when it separates real I/O; a trait without a second implementation or test double is not SOLID.
review (`ofm-architecture-reviewer`)

### Tests
One independently reported test per Given/When/Then scenario (happy, edge, failure, user-and-AI, save/
load), written first and seen failing for the predicted reason. After a regression fix, remove only the
fix in a disposable copy and watch the named test fail, then restore. A zero-test selection, compile
error or timeout is not evidence. Use faithful mocks, controlled hosts that feed values back,
deterministic RNG, non-default values at the persistence boundary, and a fresh reader for write-back
bugs. review (`ofm-test-reviewer`, `/write-tests`)

`i18n-auditor` and `ui-accessibility-reviewer` remain mandatory when their surfaces change.
<!-- ofm-code-quality:end -->

---

## Skills

Repeatable procedures. Invoke with the slash command, or let Claude pick one up automatically.

| Skill | Use it when |
|-------|-------------|
| `/add-ui-string` | Adding or changing **any** text a player can see, frontend or backend |
| `/new-ui-surface` | Building a new component, panel, tab, or screen |
| `/add-domain-field` | Adding a field to a `domain` type that must survive save/load |
| `/add-tauri-command` | Exposing new backend behaviour to the frontend over IPC |
| `/add-mcp-tool` | Adding a tool to the MCP server used by AI agents playing the game |
| `/preflight` | Before opening a PR — the full local verification gauntlet |

## Agents

Read-only reviewers. Point them at your diff before you open a PR.

| Agent | What it looks for |
|-------|-------------------|
| `ofm-architecture-reviewer` | Crate-boundary violations, layering inversions, SOLID and encapsulation smells, files that should be decomposed |
| `i18n-auditor` | Untranslated user-facing strings, missing locale keys, `INTENTIONAL_SAME.json` misuse |
| `ui-accessibility-reviewer` | Hardcoded colours, missing `dark:` pairs, missing focus rings, unlabelled controls, keyboard traps |
| `ofm-dedup-reviewer` | A helper reimplemented under a new name, a second copy of an ordering or mapping, a modal shell rebuilt from scratch |

---

## Where to read more

| Document | Covers |
|----------|--------|
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Fork & pull workflow, licensing, code conventions, AI-assisted contributions |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Crate graph, state management, the full Tauri command table, data flow, key decisions |
| [`docs/GAME_SYSTEMS.md`](docs/GAME_SYSTEMS.md) | Training, staff, traits, schedule generation, inbox, news, finances, transfers |
| [`docs/MATCH_SIMULATION.md`](docs/MATCH_SIMULATION.md) | The engine: zone model, action resolution, attributes, live match phases, AI |
| [`docs/SAVE_SYSTEM_DESIGN.md`](docs/SAVE_SYSTEM_DESIGN.md) | Save format and persistence design |
| [`docs/MCP_SERVER.md`](docs/MCP_SERVER.md) | The MCP server: 89 tools, competition mode, transport, adding a tool |
| [`docs/modding/`](docs/modding/) | `.ofm` packages, the CLI, the Package Editor, the entity schema reference |
| [`docs/DEFINITIONS.md`](docs/DEFINITIONS.md) | World-generator definition file formats |

Scoped guidance loads automatically when you work in these trees:

- [`src/CLAUDE.md`](src/CLAUDE.md) — frontend: i18n, design tokens, accessibility, stores, invariants
- [`src-tauri/CLAUDE.md`](src-tauri/CLAUDE.md) — backend: crate boundaries, persistence, locking, tests

---

## House style

Beyond the six rules, this codebase has a consistent voice. Match it.

- **Readability over cleverness.** Name things for what they mean in football terms, not in
  abstract type terms. `deployed_position` beats `pos2`.
- **Encapsulation.** Keep helpers private until a second caller genuinely needs them. A `pub fn`
  is a promise.
- **Small, honest units.** Large Rust files get split into a `mod.rs` shell plus submodules —
  `ofm_core/generator/`, `ofm_core/slices/`, and `ofm_core/turn/` are the worked examples. Don't
  add production code beyond the Code quality ceiling; extract cohesive responsibilities first.
- **Comments explain *why*.** The `time = "=0.3.51"` pin in `src-tauri/Cargo.toml` is the model:
  it says what broke, and what would let us remove the pin.
- **Don't reinvent what exists.** Search `src/components/ui/index.ts`, `src/lib/`,
  `src/utils/`, and `src/services/` before writing a helper. The same goes for `ofm_core` — most game logic already
  has a home.
