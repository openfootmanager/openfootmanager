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
npx vitest run <path>        # a single file or directory, e.g. npx vitest run src/i18n
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

- **Tauri command tests live in the `openfootmanager_lib` lib target.** Use `cargo test --lib`.
  `cargo test --bin` matches **zero** tests — the `[lib] name` is deliberately suffixed
  (`src-tauri/Cargo.toml`), so the binary target contains almost nothing.
- **`npm run audit:i18n` never fails.** It is a heuristic reporter that prints candidates and
  exits 0 (`scripts/audit-i18n.mjs`). Read its output; do not treat a clean run as a pass. The
  real i18n gate is `npx vitest run src/i18n`.

---

## Non-negotiables

Six rules. Each one has something that enforces it — if you break one, something goes red.

1. **TDD.** Write the failing test first, then the code. Rust unit tests go in a `#[cfg(test)]`
   module in the same file; frontend tests are co-located as `*.test.ts(x)`. A PR that adds
   behaviour without a test that would have caught its absence is incomplete.

2. **Every user-facing string is translated into all 11 locales.** Not just `en.json`.
   → use [`/add-ui-string`](.claude/skills/add-ui-string/SKILL.md).
   → enforced by `src/i18n/localeCoverage.test.ts` and `src/i18n/frontendKeyCoverage.test.ts`.

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
  add to a file that is already past ~1500 lines without asking whether it should be split first.
- **Comments explain *why*.** The `time = "=0.3.51"` pin in `src-tauri/Cargo.toml` is the model:
  it says what broke, and what would let us remove the pin.
- **Don't reinvent what exists.** Search `src/components/ui/index.ts`, `src/lib/`, and
  `src/utils/` before writing a helper. The same goes for `ofm_core` — most game logic already
  has a home.
