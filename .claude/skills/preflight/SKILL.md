---
name: preflight
description: Run the full local verification gauntlet before opening a pull request — type check, frontend tests, build, backend tests, clippy, and the i18n audit — in cheapest-first order, and confirm the PR hygiene items (branch, conventional commit, linked issue, AI disclosure).
when_to_use: Before opening or updating a pull request, before asking for review, or any time you want to know whether the change is actually ready.
allowed-tools: Read, Grep, Glob, Bash(npm test), Bash(npx vitest run*), Bash(npm run build), Bash(npm run lint), Bash(npm run audit:i18n), Bash(npx tsc --noEmit), Bash(cargo test*), Bash(cargo clippy*), Bash(cargo fmt*), Bash(git status), Bash(git diff*), Bash(git log*), Bash(git branch*)
---

# Preflight

Run these in order. Each is cheaper than the one after it, so a failure costs you the least
possible time. Stop at the first failure, fix it, restart from that step.

## 1. Scope check (seconds)

```bash
git branch --show-current
git status --short
git diff --stat develop...HEAD
```

- Not on `develop`. If you are, branch now — never commit to `develop` directly.
- No stray files: no `exported_world.json`, no `.ofm` build output, no `*.local`, no editor cruft.
- The diff is the change you meant to make. Unrelated reformatting is noise that hides the real
  edit; drop it.

## 2. Types (fast)

```bash
npx tsc --noEmit
```

## 3. Frontend tests

```bash
npm test
```

Iterate on one area first — `npx vitest run src/components/squad` — then run the full suite
before pushing. Around 150 test files; the whole run takes a few minutes.

If you touched any user-facing text, this is where `src/i18n/localeCoverage.test.ts` and
`src/i18n/frontendKeyCoverage.test.ts` catch missing locales. They run as part of `npm test`.

## 4. Frontend build

```bash
npm run build
```

`tsc && vite build`. This is the exact command CI runs, so a green local build means a green CI
frontend job.

## 5. Backend tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```

If you changed a Tauri command, also run the lib target explicitly:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

**`cargo test --bin` matches zero tests and exits 0.** It looks like a pass and checks nothing.

Touched MCP server code? That is behind a feature flag and is not compiled by default:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --features mcp
```

## 6. Clippy

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
```

CI runs this exact command and fails on any warning. Fix them rather than adding `#[allow]`; if an
`#[allow]` is genuinely right (a Tauri command that legitimately takes many parameters, say), put
a comment above it explaining why.

## 7. Formatting

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all
```

Format the files you touched. A repo-wide sweep is still outstanding, so `cargo fmt --check`
reports pre-existing diffs across the tree and is **not** a CI gate yet — don't let unrelated
formatting churn into your diff.

## 8. Lint (advisory)

```bash
npm run lint
```

Biome is installed and configured but not a CI gate: the codebase has a large pre-existing backlog.
Read the findings **for the files you touched** and fix those. Don't start the repo-wide sweep here.

## 9. i18n audit (advisory)

```bash
npm run audit:i18n
```

**Always exits 0.** It is a heuristic reporter over `src/` and `src-tauri/` that lists candidate
hardcoded strings. Read the output and check whether anything it lists came from your change. The
real gate was step 3.

---

## PR hygiene

- [ ] Branched from `develop`, PR targets `develop`
- [ ] Conventional commit subject — `fix(ui):`, `feat(world-cup):`, `test(training):`,
      `refactor(...)`, `chore(...)` — matching the existing history
- [ ] Linked to an issue, or an issue opened first if the change is a new feature
      (`CONTRIBUTING.md` asks for this)
- [ ] Commit message explains **why**, not just what
- [ ] Tests added for new behaviour, written before the code
- [ ] All 11 locales updated if any user-facing text changed
- [ ] AI-assisted work disclosed in the PR description — this is a GPLv3 project and provenance
      matters

## Consider a reviewer agent

For anything non-trivial, run the relevant read-only reviewer over your diff before a human sees
it: `ofm-architecture-reviewer` (crate boundaries, layering, SOLID), `i18n-auditor` (untranslated
strings), `ui-accessibility-reviewer` (contrast, focus, keyboard, labelling).
