---
name: add-domain-field
description: Add a field to a domain type so it survives save/load. Covers serde(default) for backward compatibility, the five positional edit sites in the db repositories, the SQL migration and MIGRATION_COUNT bump, the round-trip test, and the frontend type plus translated label.
when_to_use: Adding a field to Team, Player, Staff, Manager, League, or any other type in the domain crate; or when a new field works in memory but is empty after saving and reloading.
argument-hint: "[type and field, e.g. Team.youth_budget]"
allowed-tools: Read, Edit, Write, Grep, Glob, Bash(cargo test*), Bash(cargo clippy*), Bash(cargo fmt*)
---

# Adding a domain field

The trap: `#[serde(default)]` makes old saves *load*, but it does nothing for SQLite. The
repositories in `src-tauri/crates/db/src/repositories/` use hand-written positional SQL, so a
field that isn't in the column lists is silently dropped on every save/load round trip — with no
error, no warning, and a passing test suite.

Do all six steps.

---

## 1. The domain type

`src-tauri/crates/domain/src/` — `team.rs`, `player.rs`, `staff.rs`, `manager.rs`, `league.rs`, …

```rust
pub struct Team {
    // …
    #[serde(default)]
    pub youth_budget: i64,
}
```

Use `#[serde(default = "default_youth_budget")]` with a function when zero, empty, or `None` is
the wrong value for an old save. Think about what a save written *before* this feature existed
should look like once loaded — that is what the default has to produce.

Domain types hold data, not logic. Behaviour goes in `ofm_core`.

## 2. The repository — five edit sites

Take `Team` as the worked example; `team_repo.rs` is the reference for the rest.
Open `src-tauri/crates/db/src/repositories/team_repo.rs` and edit **all five**:

1. **`save_team`'s `INSERT OR REPLACE INTO teams (…)` column list** — add the column.
2. **Its `VALUES (?1, ?2, …, ?N)` run** — add one more placeholder. The count must match the
   column list exactly or the insert fails at runtime.
3. **The `params![…]` list** — add the value, in the same position as the column.
4. **`row_to_team`'s positional mapping** — `row.get(N)?`. Appending at the end is safest;
   inserting in the middle shifts every index after it, and nothing will tell you if you miss one.
5. **Both `SELECT` column lists** — `load_all_teams` *and* `load_team`. Two separate strings that
   must stay in sync. For a column added by a migration, wrap it as
   `COALESCE(youth_budget, <default>)` so rows written before the migration read back as the
   default — the existing JSON columns (`media_json`, `player_roles_json`, …) show the pattern.

   **`<default>` must be the same value your serde default produces.** These are two independent
   fallbacks for the same field: serde covers a save loaded from JSON, `COALESCE` covers a row
   written before the migration. If step 1 used `#[serde(default = "default_youth_budget")]`
   returning `500_000`, then `COALESCE(youth_budget, 0)` makes the same save load differently
   depending on which path it came through. Either match the two, or give the column a non-null
   SQL `DEFAULT` in the migration and drop the `COALESCE`.

Other repositories in that directory (`player_repo.rs`, `staff_repo.rs`, `competition_repo.rs`, …)
follow the same five-site shape.

## 3. The migration

- Add `src-tauri/crates/db/src/sql/vNNN_<short_description>.sql`, numbered after the current
  highest file.
- Register it in `src-tauri/crates/db/src/migrations.rs` (`all_migrations()`).
- **Bump `MIGRATION_COUNT`** at the top of that file — the migration tests assert on it.

Migrations are append-only and must be idempotent; `migrations.rs` has a test that re-applies them
(`test_migrations_are_idempotent`). Use `ALTER TABLE … ADD COLUMN … DEFAULT …`; never rewrite or
renumber an existing migration, because shipped saves have already run it.

## 4. Prove it round-trips

This is the step that catches the dropped-column bug, so write it before the repository edits.

In `src-tauri/crates/db/`, extend the nearest existing round-trip test: build a value with the new
field set to something **non-default**, save it, load it back, assert the field survived. A test
that uses the default value passes even when the column is missing entirely.

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p db
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```

## 5. The rest of the backend

- Game logic for the field goes in `ofm_core`, not `domain`.
- If the field should reach the match engine, **do not** import `domain` into `engine`. Add the
  field to the engine's own mirror type and extend the conversion in `ofm_core/turn/`.
- If an AI agent should be able to read or set it, see `/add-mcp-tool`.

## 6. The frontend

- Add the field to the matching TypeScript type (`src/store/types.ts` or the relevant service
  types). Optional (`?`) if old saves may not have it.
- Any label, unit, or tooltip for it is a user-facing string → `/add-ui-string`, every locale.

---

## Checklist

- [ ] Field added to the `domain` type with `#[serde(default)]` or an explicit default fn
- [ ] Default value is correct for a save written before the field existed
- [ ] Repository INSERT column list updated
- [ ] `VALUES (?1…?N)` placeholder count updated to match
- [ ] `params![…]` updated, same order
- [ ] `row_to_*` positional `row.get(N)?` updated
- [ ] **Both** SELECT lists updated, with `COALESCE` for the migrated column
- [ ] `sql/vNNN_*.sql` added and registered in `migrations.rs`
- [ ] `MIGRATION_COUNT` bumped
- [ ] Round-trip test with a **non-default** value, written first
- [ ] `cargo test --workspace` and `cargo clippy --workspace --all-targets` green
- [ ] Frontend type updated; any new label translated into every locale
