---
name: write-tests
description: Write scenario-driven tests for an OpenFoot Manager feature, bug fix or story, or characterize weakly covered behaviour before a refactor. Verify named red and green runs, fixture discrimination, state/persistence seams and regression effectiveness in a disposable copy.
allowed-tools: Read, Edit, Write, Grep, Glob, Bash
---

<!-- Adapted for OFM from obra/superpowers test-driven-development (MIT), revision
8ca22dba9a94f28898bbce59f2537ff4d87c747d:
https://github.com/obra/superpowers/blob/8ca22dba9a94f28898bbce59f2537ff4d87c747d/skills/test-driven-development/SKILL.md
Copyright (c) 2025 Jesse Vincent; and po4yka/rust-skills rust-tdd (BSD-3-Clause), revision
9f4a3a4c904c1cbc507991b65397195ff724fa70:
https://github.com/po4yka/rust-skills/blob/9f4a3a4c904c1cbc507991b65397195ff724fa70/skills/rust-tdd/SKILL.md
Copyright (c) 2026 Nikita Pochaev. Full notices: ../../THIRD_PARTY_NOTICES.md.
GWT naming, OFM fixtures and persistence seam guidance are project-specific adaptations. -->

# Write tests

Read the root Code quality rules and nearby tests first. Search canonical builders in `ofm_core`
and use `src/test-setup.ts` on the frontend. Do not replace the project's runtime or install a
whole testing framework. Tests pin the caller-visible contract; they do not mirror the implementation.

## 1. Scenarios before implementation

Write a named Given/When/Then table and carry it into the PR body:

| Scenario | Given | When | Then | Layer / test name | Predicted failure |
|---|---|---|---|---|---|

Cover happy, edge, failure/abuse, user and AI routes for shared rules, and save/load. Mark routes
inapplicable with a reason. When no written scenarios exist, state those for the touched routes
before coding. Every applicable scenario gets one independently reported test; parameterize cases
with the same outcome, split different outcomes. Preserve scope and discuss unclear acceptance
criteria rather than inventing a larger story.

Rust names: `given_<state>_when_<action>_then_<outcome>` in a same-file `#[cfg(test)]` module.
Frontend: co-located `*.test.ts(x)` with `it("given ..., when ..., then ...")`. Existing
cross-crate tests belong at the owning integration layer. Command tests are in the root lib target.

## 2. Fixtures and assertions that discriminate

Use the smallest world containing the interaction, with real existing builders and seeded RNG.
If generation/distribution is the behaviour, use the real generator rather than a hand-built
population. Distinguish competing worlds: two competitions on one matchday, different wage units,
non-default persisted values, a positive control for a negative assertion, and nonempty inputs
before assertion loops. Expected values come from literals or an independent oracle, not the code
under test. Preserve intentional engine mirrors and independent test computations.

Choose the cheapest layer that observes the claimed behaviour. Pure rules use unit tests; wiring
uses the component/service or shared `&StateManager` seam. Read live state after mutations, not a
returned clone alone. Persistence uses the real writer and a fresh DB connection/reader, with
non-default values and absent-field old-save fixtures. Failure scenarios assert no partial mutation.

Frontend mocks retain the side effects the scenario needs. A controlled host feeds changed values
back into props; a bare `onChange={vi.fn()}` proves only notification. Keep real hook behaviour for
hook-order regressions (a hook-free i18n mock hides them). Prefer role/name or label queries; cover
keyboard and focus. Service failure tests verify usable UI/error outcomes, not only a mock call.

## 3. RED, verified

Write the scenario test before production code. Name the predicted failed assertion and value,
then run only that test and confirm its name and count appear:

```bash
cargo test --locked --manifest-path src-tauri/Cargo.toml -p <crate> '<scenario>' -- --exact '<module>::tests::<scenario>'
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib '<scenario>'
# Add --features mcp to the lib command for an MCP scenario.
npm ci
npm test -- <co-located-test-file> -t '<scenario>'
```

These are templates: fill in a real target and use its full Rust path for `--exact`.

| Observed result | Next action |
|---|---|
| Predicted assertion fails | Record test count and failure line; implement the minimum fix |
| Passes unexpectedly | Check existing behaviour and fixture; make correct and broken outcomes differ, without weakening the assertion |
| Zero tests | Repair the selection; it proves nothing |
| Wrong assertion/setup panic | Repair setup or understanding, then rerun |
| Compile-invalid | Add only the necessary API/stub so the test can reach its behavioural assertion; rerun |
| Timeout/tool failure | Resolve or report the execution limit; do not call it RED |

A pure refactor's characterization tests may pass initially: name that evidence **characterization**,
not a fabricated red run. Do not delete prior work to restart TDD, commit stubs or change unrelated code.

## 4. GREEN, then effectiveness evidence

Implement the smallest production change, rerun the same scenario, then related tests. Keep each
scenario test with the implementation it drove. Do not skip, weaken or bless away an unexplained failure.

Before the PR, use either a small relevant Rust mutation run or manual fix removal. Run experiments
in a disposable directory only. For a committed revision:

```bash
git clone --no-hardlinks --no-checkout . /tmp/ofm-test-proof-<unique-id>
git -C /tmp/ofm-test-proof-<unique-id> checkout --detach <reviewed-sha>
```

For uncommitted work, make a separate filesystem copy of the exact source under review, excluding
`.git`, `node_modules`, `target` and outputs, then copy necessary untracked tests explicitly. Inspect
the source-copy diff, install dependencies there and record the snapshot identity. Never `git stash`,
`git checkout`, `git restore` or reverse-patch the caller's worktree. Do not run deletion commands on
an existing directory whose provenance you have not verified.

In the copy, run the named test green, remove **only the production fix** while retaining the test
and unrelated setup, inspect the removal diff, run the same command, quote its behavioural failure,
then restore the copied production file and run green again. A wholesale checkout of the base also
removes the test and is not proof. If the removal breaks compilation, report compile-invalid and
repair the experiment without changing the caller's files.

For Rust mutation testing, create a production diff for the reviewed revision and, after checking
installed `cargo mutants --help`, run in the copy:

```bash
cargo mutants --manifest-path src-tauri/Cargo.toml --in-diff <production-diff-file>
```

Record selection, killed/survived/timeout/compile-invalid statuses and tool exit code separately.
Empty selection is unproved; survivors need a discriminating scenario. Prefer manual fix removal
when mutation tooling is unavailable. Never describe a narrow slice as project mutation coverage.
Ask `ofm-test-reviewer` to inspect the evidence and unproved scenarios.

## 5. Property tests when an invariant earns them

Use bounded `proptest` generators for pairing uniqueness, money conservation, conversion round-trips
or lineup permutations where a small pure input suffices. Avoid full-`Game` properties or adding a
dependency for an example test. If `proptest` is absent, add it as a dev-dependency only with the
first real property test in the authorized implementation change. A shrunk failure becomes a named
GWT example with a literal input; do not commit a shared failure file or let RNG make it flaky.

## 6. Handoff

Run [`/preflight`](../preflight/SKILL.md), including default and MCP tests where applicable.
In the PR give scenario -> test -> observed red/fix-removal -> green command, selected counts,
canonical rule owner/layer, reviewer result and every unrun check. No runtime behaviour changed in
a prose-only task: use artifact validation and disclose that runtime RED does not apply.
