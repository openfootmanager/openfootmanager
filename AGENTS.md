# AGENTS.md

Conventions for AI coding agents contributing to OpenFoot Manager. Tool-agnostic — Claude Code,
Cursor, Copilot, Codex, Aider, or anything else.

Claude Code users get more: see [`CLAUDE.md`](CLAUDE.md) for the full command reference plus the
project's skills (`/add-ui-string`, `/preflight`, …) and review agents in `.claude/`.

## Build and test

```bash
npm install
npm test                                                        # frontend suite
npm run build                                                   # tsc && vite build
cargo test --manifest-path src-tauri/Cargo.toml --workspace      # backend suite
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets
cargo fmt --manifest-path src-tauri/Cargo.toml --all
```

Tauri command tests live in the `openfootmanager_lib` lib target — use
`cargo test --manifest-path src-tauri/Cargo.toml --lib`. `cargo test --bin` matches zero tests.

## The six rules

1. **TDD.** Failing test first. Rust tests in a `#[cfg(test)]` module in the same file; frontend
   tests co-located as `*.test.ts(x)`.
2. **Every locale.** Every string a player can read is translated into all of
   `SUPPORTED_LANGUAGES` (`src/i18n/index.ts`, one file each in `src/i18n/locales/`).
   English-only changes fail
   `src/i18n/localeCoverage.test.ts`. Backend text a player sees (a Tauri command's error) is a translation *key*, never English prose. MCP tool output, `src-tauri/crates/ofm-cli/` and `docs/` are read by agents and modders, not players, and stay English.
3. **`engine` never imports `domain`.** The match engine keeps its own mirror types on purpose;
   `ofm_core/turn/` is the only bridge. See `docs/ARCHITECTURE.md` §"Engine Isolation".
4. **`#[serde(default)]` on every new serialized field.** Old saves must keep loading. Fields
   that reach SQLite need repository and migration edits too — see `src-tauri/CLAUDE.md`.
5. **No `any` in TypeScript; Tailwind utilities, not raw CSS.** Design tokens are defined in
   `src/App.css` under `@theme`; use token classes, never hex literals.
6. **Conventional commits on a branch off `develop`**, e.g. `fix(ui):`, `feat(world-cup):`,
   `test(training):`. PRs target `develop`. Never commit to `develop` directly.

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

## Before opening a PR

Run the frontend suite, the backend suite, `npm run build`, and `cargo clippy`. Disclose that the
change was AI-assisted in the PR description — this is a GPLv3 project and provenance matters.
See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Shared procedures and reviewers

Use the same [.claude/skills](.claude/skills/) procedures and
[.claude/agents](.claude/agents/) reviewers indexed in [CLAUDE.md](CLAUDE.md).
Their instructions are shared across tools; do not maintain another copy.
