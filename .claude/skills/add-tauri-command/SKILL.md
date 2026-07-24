---
name: add-tauri-command
description: Expose new backend behaviour to the frontend over Tauri IPC. Covers the _internal function split so MCP tools can share the logic, atomic mutation via mutate_active_game, translation-key errors, registration in the invoke handler, the typed service wrapper, store update, tests on both sides, and the docs/ARCHITECTURE.md command table.
when_to_use: Adding a new Tauri command, changing an existing command's signature, or wiring a new backend action into the UI.
argument-hint: "[command name and what it does]"
allowed-tools: Read, Edit, Write, Grep, Glob, Bash(cargo test*), Bash(cargo clippy*), Bash(npx vitest run src/services), Bash(npx tsc --noEmit)
---

# Adding a Tauri command

Every frontend↔backend interaction goes through `invoke()`. The layering is strict, and the
`_internal` split is what keeps the MCP server from becoming a second copy of the game.

---

## 1. Write the command in `src-tauri/src/commands/`

One module per area — `squad.rs`, `transfers.rs`, `contracts.rs`, `club.rs`, `time.rs`, … Put it
where its siblings live rather than starting a new module.

**Split it in two.** The `#[tauri::command]` wrapper does IPC plumbing only; the real work lives
in a plain `*_internal` function that takes `&StateManager`:

```rust
pub fn set_play_style_internal(state: &StateManager, play_style: &str) -> Result<Game, String> {
    mutate_active_game(state, |game| {
        // validate first — see below
        // then mutate
        Ok(())
    })
}

#[tauri::command]
pub fn set_play_style(state: State<Arc<StateManager>>, play_style: String) -> Result<Game, String> {
    set_play_style_internal(&state, &play_style)
}
```

This is not ceremony. The MCP server (`src-tauri/src/mcp_server/tools_impl/`) calls the same
`_internal` function, so an agent playing the game and a human clicking a button run identical
logic. A command whose body lives inside the `#[tauri::command]` fn cannot be reached from MCP
without duplicating it.

### Mutate atomically

Use `mutate_active_game` from `src-tauri/src/commands/util.rs`. It borrows the game under the
lock, mutates in place, and clones **once** for the response — the old
`get_game(clone)` → mutate → `set_game` pattern deep-cloned the whole world twice per command and
lost updates when the GUI and an MCP agent acted concurrently.

**Validate before you mutate.** The closure operates on the live game, so anything changed before
an early `Err` return persists. Check preconditions up front, then mutate.

### Errors are translation keys

Return `"be.error.noTeamAssigned"`, not `"No team assigned"`. The frontend resolves these through
`src/utils/backendI18n.ts`. A new error key must be added to all 11 locale files — use
`/add-ui-string`.

## 2. Register it

Add the command to the `tauri::generate_handler![…]` list in `src-tauri/src/lib.rs`. If you
created a new module, add `pub mod` and the `pub use` re-export in `src-tauri/src/commands/mod.rs`.

Forgetting registration compiles fine and fails at runtime with an unhelpful IPC error.

## 3. Wrap it in a service

Components never call `invoke()`. Add a typed wrapper to the matching
`src/services/<area>Service.ts`:

```ts
export async function setPlayStyle(playStyle: PlayStyle): Promise<GameStateData> {
  return invoke<GameStateData>("set_play_style", { playStyle });
}
```

**Argument names are camelCase on the TypeScript side** and snake_case in Rust — Tauri converts
between them. Getting this wrong produces a runtime "missing required key" error, not a compile
error, so mirror an existing wrapper exactly.

Types come from `src/store/types.ts` or `src/store/gameStore.ts`. No `any`.

## 4. Update the store

Commands that return game state feed `gameStore`. Follow the existing call sites: call the
service, then set store state. Never mutate store state in place.

## 5. Test both sides

Backend, in a `#[cfg(test)]` module in the same file — test the `_internal` function, which needs
no Tauri runtime:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib      # command tests live in the lib target
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```

`cargo test --bin` matches **zero** tests and exits 0. It is not a check.

Frontend: `src/services/<area>Service.test.ts`, mocking `@tauri-apps/api/core`. Existing service
tests show the shape. Cover the failure path — a command that returns `Err` must surface a usable
message.

Write the failing test first.

## 6. Update the docs

`docs/ARCHITECTURE.md` has command tables (Game Lifecycle, Match, Team Management, Settings). Add
your row. An out-of-date table is worse than none.

If the command should also be available to AI agents, continue with `/add-mcp-tool`.

---

## Checklist

- [ ] Logic in a `*_internal(&StateManager, …)` function; `#[tauri::command]` is a thin wrapper
- [ ] Mutations go through `mutate_active_game`, with validation before mutation
- [ ] Errors are translation keys, added to all 11 locales
- [ ] Registered in `generate_handler![…]` in `src-tauri/src/lib.rs`
- [ ] New modules re-exported from `commands/mod.rs`
- [ ] Typed wrapper in `src/services/`, camelCase args, no `any`
- [ ] Store updated without in-place mutation
- [ ] Backend test on the `_internal` fn (`cargo test --lib`), written first
- [ ] Frontend service test including the error path
- [ ] `docs/ARCHITECTURE.md` command table updated
- [ ] `cargo clippy --workspace --all-targets` and `npx tsc --noEmit` green
