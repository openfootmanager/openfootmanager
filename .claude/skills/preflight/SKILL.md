---
name: preflight
description: Verify a pull request with the frontend scripts, default and MCP backend tests and clippy, formatting, architecture checks and review evidence. Report results and unrun checks without claiming a local pass guarantees CI.
when_to_use: Before opening or updating a pull request or asking for review.
allowed-tools: Read, Grep, Glob, Bash(npm ci), Bash(npm test*), Bash(npm run*), Bash(cargo test*), Bash(cargo clippy*), Bash(cargo fmt*), Bash(git status), Bash(git diff*), Bash(git log*), Bash(git branch*)
---

# Preflight

Read the root Code quality section first. It names the limits and which gates are live. Verify
without mutating formatting or weakening tests. Stop on a failure, fix within scope, then resume
from that step; disclose unrelated blockers. Never report an unrun check as passing.

## 1. Scope and install

```bash
git branch --show-current
git status --short
git diff --stat upstream/develop...HEAD
npm ci
```

Work on a branch off current `upstream/develop`, targeting `develop`. Check for unrelated files,
exported worlds, local settings and formatting churn. `npm ci` precedes frontend commands in a
fresh checkout. Use `npm run <script>`, never `npx`: a cached namesake can silently run the wrong tool.

## 2. Frontend gates

```bash
npm run preflight
```

`package.json` defines this frontend sequence: format check, lint, knip, build, tests. `npm run
build` already does `tsc && vite build`; do not repeat a separate type check. For a scoped test
while iterating, use `npm test -- <path> -t '<scenario>'` after installing dependencies; the full
suite is still required before the PR. Locale and literal-key coverage tests are part of it.
A local pass is evidence for these commands, not a guarantee that every CI job passes.

## 3. Backend gates, default and MCP

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --locked --manifest-path src-tauri/Cargo.toml --workspace
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib --features mcp
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --workspace --all-targets --features mcp -- -D warnings
```

Default workspace tests include command lib tests. The second test command actually executes MCP
lib tests; building the feature alone does not. **`cargo test --bin` may run zero tests.** Verify
that the expected named tests ran. `--locked` matches CI; if a dependency edit needs a lockfile
update, resolve it deliberately and commit the resulting lockfile alongside the manifest.

The pinned `rust-toolchain.toml` is authoritative. Do not override it with `cargo +<toolchain>`.
No new `#[allow]`: use the root constraint/owner suppression policy. Do not raise a threshold to
fit code. `fmt --check` reports changes without writing them.

The workspace run includes `src-tauri/tests/architecture.rs` and, as the programme lands, its
crate-matrix, application-import, engine-input and command-size checks, plus file-size and
instruction-sync integration tests. Frontend architecture/file-size tests join `npm test` in
programme PR 8. Until each exists, inspect those rules manually and mark them **review-only**;
do not run a missing target or count a zero-test selection as a pass.

## 4. Review evidence

Use `ofm-architecture-reviewer` for architecture and SOLID, `ofm-dedup-reviewer` for authoritative
homes and reachable reuse, and `ofm-test-reviewer` for any test change and every bug fix.
Map each named GWT scenario to one independently reported test; quote observed red/fix-removal
failure and the green command. Use a disposable copy for fix removal.
Do not claim "would fail" as observed evidence. Identify uncovered routes and exceptions.

`i18n-auditor` and `ui-accessibility-reviewer` are mandatory when their surfaces change. Read the
heuristic report if text changed:

```bash
npm run audit:i18n
```

It always exits 0; it is advisory and cannot replace locale tests or review. New mechanical
quality checks must fail required CI at zero findings; this existing heuristic is not such a gate.

Record a compact evidence table in the PR body:

| Check / scenario | Command / test name | Observed result | Limit or exception |
|---|---|---|---|

Include the canonical rule owner and layer, reviewer results, and every unrun check. Documentation
changes have no runtime red/green claim; verify links, identical instruction blocks and scope.

## 5. Merge hygiene and collisions

- Conventional commit, fresh develop base, PR target `develop`, linked issue where applicable.
- AI-assisted disclosure box completed per `CONTRIBUTING.md`.
- Search every other open PR diff for uses of public items/fields/variants removed or changed here,
  and for removals of items this branch uses. Agree merge order with the owner on a collision.
  The second PR stays draft with `⛔ Merge after #<first>` as its body's first line. Once the first
  merges, merge `upstream/develop`, adapt, rerun gates, then mark ready.
- Recheck when a merge touches your surfaces. The develop merge queue reruns required checks;
  local evidence does not replace the queue. Do not alter repository settings.
