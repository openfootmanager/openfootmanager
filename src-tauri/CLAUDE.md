# Backend guidance — `src-tauri/`

Rust: four library crates, a CLI, a benchmark harness, and the Tauri command layer.
Read [`../CLAUDE.md`](../CLAUDE.md) first for the project-wide rules, and
[`../docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md) for the full picture.

---

## 1. Crate boundaries — the rule that matters most

```text
  Tauri commands   src/commands/, src/application/, src/mcp_server/
                   → domain, engine, ofm_core, db

  db               SQLite persistence        → domain, ofm_core
  ofm_core         game logic, state, turn   → domain, engine
  engine           match simulation          → nothing in this workspace
  domain           pure data types           → nothing in this workspace

  ofm-cli          standalone CLI binary     → ofm_core
  sim-bench        balance benchmark harness → engine
```

- **`domain`** — structs and enums only. No game logic. Everything else may depend on it; it
  depends on nothing here.
- **`engine`** — the match simulation. It **does not depend on `domain`**. It defines its own
  mirror types (`PlayerData`, `TeamData`, `Position`, `PlayStyle`) so it can be tested with
  synthetic data and evolved independently. `ofm_core/turn/` performs the conversion, and it is
  the *only* place that conversion is allowed to live.
- **`ofm_core`** — game logic and the `StateManager`. Depends on `domain` and `engine`.
- **`db`** — SQLite persistence. Depends on `domain` **and `ofm_core`** — persistence sits *above*
  game logic in this workspace, which is a real layering inversion rather than the intended design.

> This section used to say `ofm_core` depends on `db`, and drew `engine` sitting on `domain`. Both
> were wrong, and survived because the only thing checking the crate graph was a reviewer reading
> this file. `src-tauri/tests/architecture.rs` now asserts the two leaf boundaries mechanically:
> `engine` and `domain` declare no workspace dependency of any kind, so the edge that matters most
> can no longer be added quietly. That is the whole of what the test defends — every other edge
> above is still prose, and an edit that misstates `db → ofm_core` will fail nothing.

Adding `domain = { path = "../domain" }` to `crates/engine/Cargo.toml` "to avoid duplication"
looks like a cleanup and is actually the single most damaging change you can make to this
codebase. If the engine needs a new field, add it to the engine's own type and extend the bridge
in `ofm_core/turn/`.

The `ofm-cli` binary statically links `ofm_core`, which is why CLI package validation is
guaranteed to match in-game validation — same code, no duplicate validator.

---

## 2. Persistence: `serde(default)` is necessary but not sufficient

Old save files must keep loading. Two separate obligations:

**a) Serde.** Every new field on a serialized type gets `#[serde(default)]`, or a
`#[serde(default = "…")]` function when zero/empty is the wrong default. This is what lets a save
written before the field existed deserialize at all.

**b) SQLite.** The repositories in `crates/db/src/repositories/` use **positional** SQL — column
lists and `row.get(N)` indices written out by hand. Serde defaults do nothing here: a field that
is not in the SQL is silently dropped on every save/load round trip.

Adding one field to `Team` means editing `repositories/team_repo.rs` in **five** places:

1. the `INSERT OR REPLACE INTO teams (...)` column list in `save_team`
2. its `VALUES (?1, …, ?N)` placeholder run — the count must match
3. the `params![…]` list, in the same order
4. `row_to_team`'s positional `row.get(N)?` mapping — every index after the insertion point shifts
5. the `SELECT` column lists in **both** `load_all_teams` and `load_team` (use
   `COALESCE(col, default)` for columns added by a migration, as the existing JSON columns do)

Plus a migration: add `crates/db/src/sql/vNNN_<description>.sql`, register it in
`crates/db/src/migrations.rs`, and bump `MIGRATION_COUNT`.

Then prove it: a save/load round-trip test asserting the new field survives. `crates/db/` already
has these — extend the nearest one rather than starting fresh.

The same five-site pattern applies to `player_repo.rs`, `staff_repo.rs`, and the rest. Use
`/add-domain-field`, which walks the whole sequence.

---

## 3. State and locking

`StateManager` (`crates/ofm_core/src/state.rs`) holds the active game, stats, live match session,
and save id behind `Mutex<Option<…>>`. It is shared between the Tauri command thread pool and —
when built with `--features mcp` — the axum/tokio pool.

- **To mutate, use `update_game`.** It takes the lock once and hands you `&mut Game`, so the
  read-modify-write is atomic. Its doc comment says exactly why: a `get_game` → mutate clone →
  `set_game` sequence loses updates when the GUI and an MCP agent act concurrently. There is
  regression history here (`fix/lost-update-races`).
- **To read, use `get_game`** with a closure that extracts and clones only what you need. Don't
  clone the whole `Game` to read one field.
- **Never hold a lock across an `.await` or an IPC boundary.**
- Multi-step operations that must not interleave (advance-a-day, finish-a-live-match) belong
  inside a single `update_game` closure, not spread across several calls.

---

## 4. Commands, and sharing them with MCP

Tauri commands live in `src/commands/` (one module per area) and are registered in the builder in
`src/lib.rs`. Multi-step orchestration sits in `src/application/`.

The MCP server (`src/mcp_server/`, behind the `mcp` Cargo feature) exposes the same behaviour to
AI agents. It calls the same `_internal` functions the Tauri commands call — **never** a second
copy of the logic. When you add a command that an agent should be able to use, factor the body
into an `_internal` function and have both call it.

Adding an MCP tool has its own checklist, in `src/mcp_server/tools.rs` at `tool_catalog()` and in
[`../docs/MCP_SERVER.md`](../docs/MCP_SERVER.md). Use `/add-mcp-tool`.

---

## 5. Tests

```bash
cargo test --manifest-path Cargo.toml --workspace    # everything
cargo test --lib                                     # Tauri command tests (see below)
cargo test -p ofm_core                               # one crate
```

**The `--bin` trap.** `[lib] name = "openfootmanager_lib"` in `Cargo.toml` (the comment there
explains the Windows name-collision reason). Command tests therefore live in the **lib** target:
`cargo test --lib` runs them, `cargo test --bin` matches zero tests and exits successfully,
which looks like a pass.

Conventions:

- Unit tests in a `#[cfg(test)]` module in the same file — the Rust norm and what
  `CONTRIBUTING.md` asks for.
- Cross-crate and end-to-end tests in `crates/<crate>/tests/`.
- Write the failing test first.
- Simulation changes: `crates/sim-bench/` exists to check that balance changes do what you think
  across many matches. A tuning change without bench output is a guess.

---

## 6. Style

- `cargo clippy --workspace --all-targets` before every PR. Address warnings; don't `#[allow]`
  them without a comment saying why.
- `cargo fmt` your own files. (A repo-wide format sweep is pending — see the note in
  `.github/workflows/build-check.yml`; keep your diff to code you actually touched.)
- User-facing text is a **translation key**, never English prose. The frontend resolves keys via
  `src/utils/backendI18n.ts`; `scripts/audit-i18n.mjs` scans `src-tauri/` for literals that
  escaped. Adding a key means adding it to all 11 locale files — use `/add-ui-string`.
- Public API is a promise. Keep helpers private until a second caller exists.
- Large files get split into a `mod.rs` shell plus submodules — `ofm_core/generator/`,
  `ofm_core/slices/`, and `ofm_core/turn/` show the shape. Before adding to a file already past
  ~1500 lines, ask whether it should be decomposed first.
- Comments explain *why*. The `time = "=0.3.51"` pin in `Cargo.toml` is the model: it names the
  breakage and the condition for removing the pin.
