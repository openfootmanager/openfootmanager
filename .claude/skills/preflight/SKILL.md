---
name: preflight
description: Verify a pull request with the frontend scripts, default and MCP backend tests and clippy, formatting, architecture checks and review evidence. Report results and unrun checks without claiming a local pass guarantees CI.
when_to_use: Before opening or updating a pull request or asking for review.
allowed-tools: Read, Grep, Glob, Bash(npm ci), Bash(npm test*), Bash(npm run*), Bash(cargo test*), Bash(cargo clippy*), Bash(cargo fmt*), Bash(cargo mutants*), Bash(scripts/mutants-in-diff.sh*), Bash(git status*), Bash(git diff*), Bash(git log*), Bash(git branch*), Bash(git rev-parse*)
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

## 3a. Mutants on the diff (trial until 27 Oct 2026)

Coverage says a line ran; it does not say a test would notice the line being wrong. On 13 Oct 2026
`contracts/renewals.rs` had 89% line coverage and a 29% mutation score. This step mutates only the
Rust production lines your branch changes and reports which mutants your tests miss. It is a
two-week trial, from 13 to 27 Oct 2026: there is no baseline file and no CI gate. On 27 Oct the
results pasted in PR bodies decide whether it stays.

Run it on any branch that touches `src-tauri/`. Use cargo-mutants 25.3.1, the version
`mutation-tests.yml` pins (`cargo install --locked cargo-mutants@25.3.1`). From the repository
root, with everything under `src-tauri/` committed:

```bash
scripts/mutants-in-diff.sh                        # base defaults to upstream/develop
CARGO="<build-slot wrapper> cargo" scripts/mutants-in-diff.sh   # on a machine shared with other agents
```

The script prints the counts line and one line per missed or timed-out mutant; that output is what
goes in the PR body. What it does, and why:

- **It refuses every way of passing without testing anything.** cargo-mutants matches the diff's
  paths against the workspace in `src-tauri/`. A diff rooted at the repository (`src-tauri/crates/…`)
  matches nothing, and cargo-mutants prints `No mutants to filter` and exits 0. The script takes the
  diff with `--relative=src-tauri` and refuses a patch whose paths are not under `src-tauri/`.
  `scripts/mutation-outcome.sh` then refuses a run that wrote no report or tested no mutant.
- **Zero is a real answer for a branch that changes no Rust production line.** A test-only Rust PR
  (an integration test, or a `#[cfg(test)]` module), a docs PR and a frontend PR all list 0 mutants:
  cargo-mutants never mutates test code. The script then prints `Mutants on the diff: 0 mutants`
  and runs nothing. Paste that line too.
- **`--in-place` edits your working tree while it runs**, one mutant at a time, restoring each
  file afterwards. That is why the script refuses uncommitted changes under `src-tauri/`, and why
  it restores `src-tauri/` from HEAD if an interrupted run leaves a mutant behind. In-place reuses
  your build; a copied tree would rebuild every dependency from scratch.
- `--profile mutants` and `CARGO_INCREMENTAL=1` are the two settings that brought an `ofm_core`
  mutant from 115 s to 34 s in CI; `mutation-tests.yml` explains both. `--timeout 300` stops a
  mutant that loops forever. cargo-mutants reports it as a timeout, not as caught, and it costs the
  full five minutes; triage it like a survivor (usually "a loop that never ends").
  `MUTANTS_TIMEOUT` (default `180m`) caps the whole run.
- **Expect about a minute per mutant**, plus five to six minutes for the unmutated baseline
  (measured on a loaded 8-core machine with two build jobs; see the PR that added this step). The
  script prints the count before it starts; run it while you do something else.
- Each mutant is tested with its own crate's tests. Exit 2 (some missed) and exit 3 (some timed
  out) are results; the script exits 0 for both and lists the survivors. Any other failure means
  no evidence: report the step as not run.

Paste the result into the PR body, with a triage after each surviving or timed-out mutant:

```text
Mutants on the diff: 14 caught, 2 missed, 0 timed out, 1 unviable (6 min)
- [missed] crates/ofm_core/src/aging.rs:85:9: delete match arm 35 in retirement_chance: real gap, test added (`the_retirement_chance_rises_with_each_year_from_33`)
- [missed] crates/ofm_core/src/finances/journal.rs:202:15: replace < with <= in credit_opening_cash: equivalent, because 0 already returned two lines above
```

"Real gap, test added" means you wrote the test and re-ran it with the mutant applied by hand, and
it failed. "Equivalent" needs a reason a reviewer can check. A survivor you cannot place in either
category is a finding: say so rather than leaving it out.

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
