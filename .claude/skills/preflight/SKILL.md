---
name: preflight
description: Run the full local verification gauntlet before opening a pull request — type check, frontend tests, build, backend tests, clippy, and the i18n audit — in cheapest-first order, and confirm the PR hygiene items (branch, conventional commit, linked issue, AI disclosure).
when_to_use: Before opening or updating a pull request, before asking for review, or any time you want to know whether the change is actually ready.
allowed-tools: Read, Grep, Glob, Bash(npm test), Bash(npx vitest run*), Bash(npm run build), Bash(npm run lint), Bash(npm run audit:i18n), Bash(npx tsc --noEmit), Bash(cargo test*), Bash(cargo build*), Bash(cargo clippy*), Bash(cargo fmt*), Bash(git status), Bash(git diff*), Bash(git log*), Bash(git branch*)
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
cargo test --locked --manifest-path src-tauri/Cargo.toml --workspace
```

`--locked` is what CI passes, so a lockfile you forgot to commit fails here rather than twenty
minutes into a CI run. If it stops with *cannot update the lock file*, re-run without the flag
and commit the resulting `src-tauri/Cargo.lock` with your manifest change.

If you changed a Tauri command, also run the lib target explicitly:

```bash
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
```

**`cargo test --bin` matches zero tests and exits 0.** It looks like a pass and checks nothing.

Touched MCP server code? That is behind a feature flag and is not compiled by default:

```bash
cargo build --locked --manifest-path src-tauri/Cargo.toml --features mcp
```

## 6. Clippy

```bash
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
```

Clippy must be clean before a PR (`CONTRIBUTING.md` has always asked for this; check
`.github/workflows/build-check.yml` for whether CI enforces it yet). Fix warnings rather than
adding `#[allow]`; if an `#[allow]` is genuinely right — a Tauri command whose long argument list
*is* the IPC signature, say — put a comment above it explaining why.

**The toolchain already matches CI.** `rust-toolchain.toml` at the repository root names the
same version `.github/workflows/build-check.yml` installs, and rustup reads it for any cargo run
inside the checkout — you do not have to do anything. **Never write `cargo +<toolchain>`**: it
overrides the file, which is the one thing the pin cannot defend against, and
`scripts/check-toolchain-pin.sh` rejects it outright in a workflow.

Touched MCP code? CI lints it separately, because the feature isn't on by default:

```bash
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --workspace --all-targets --features mcp -- -D warnings
```

## 7. Formatting

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all
```

`cargo fmt --check` **is** a CI gate (the `format` job). The repo-wide sweep has been done, so
running the formatter now touches only what you touched — there is no unrelated churn to avoid
any more, and the old advice to format by hand is retired.

## 8. Lint and the quality ratchet

```bash
npm run lint          # biome, --error-on-warnings: warnings fail, same as CI
npm run quality:check # nothing in quality-baseline.json may rise
```

Both **are** CI gates now. `npm run lint` is the same command CI runs, strictness included, so a
green run here is a green run there.

`quality:check` compares the repo against `quality-baseline.json`: file sizes, suppression counts,
named anti-patterns, dead exports, and the lint rules that sit at `info` while their backlog is
worked down. Numbers may fall, never rise. If yours fall, run `npm run quality:baseline` and
commit the regenerated file with the change that earned it.

**The whole frontend gauntlet is one command**, in cheapest-first order:

```bash
npm run preflight
```

That is deliberately the *only* definition of the frontend gate set — `package.json` holds it,
CI runs its parts, and this skill points at it, so the three cannot drift apart.

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
- [ ] Every locale updated if any user-facing text changed
- [ ] AI-assisted work disclosed in the PR description — this is a GPLv3 project and provenance
      matters

## Consider a reviewer agent

For anything non-trivial, run the relevant read-only reviewer over your diff before a human sees
it: `ofm-architecture-reviewer` (crate boundaries, layering, SOLID), `i18n-auditor` (untranslated
strings), `ui-accessibility-reviewer` (contrast, focus, keyboard, labelling).
