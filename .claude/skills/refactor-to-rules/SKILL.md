---
name: refactor-to-rules
description: Decompose OpenFoot Manager code by responsibility when a quality gate is red, a reviewer requests a split or a refactor is authorized. Characterize behaviour first, preserve contracts and report scoped before/after diagnostics without raising limits or reformatting unrelated code.
when_to_use: A quality gate or reviewer requests a responsibility split, or a behaviour-preserving refactor is authorized.
argument-hint: "[file or unit to refactor and the rule it violates]"
allowed-tools: Read, Edit, Write, Grep, Glob, Bash
---

<!-- Adapted for OFM from nathankim0/clean-architecture-skills (MIT), revision
725cbeeb12e3f731445e7688dee3bbfe5e149c8e:
https://github.com/nathankim0/clean-architecture-skills/blob/725cbeeb12e3f731445e7688dee3bbfe5e149c8e/plugins/clean-architecture/skills/clean-architecture/SKILL.md
Copyright (c) 2026; and btseee/clean-code-skills (MIT), revision
49b1354be3ec9d0158c99b1c1c9bc1f833133640:
https://github.com/btseee/clean-code-skills/blob/49b1354be3ec9d0158c99b1c1c9bc1f833133640/skills/clean-code/SKILL.md
Copyright (c) 2026 Battseren Badral. Full notices: ../../THIRD_PARTY_NOTICES.md.
Adapted to OFM's approved crate graph, football vocabulary and shared application seams. -->

# Refactor to rules

Read the root Code quality rules and the unit's callers/tests. A numeric ceiling triggers an
inspection; meeting it by splitting arbitrary chunks is not the purpose. Preserve documented
inputs, outputs, errors, ordering, stored names and save compatibility. No wider redesign or
`.clean/` state store is part of this procedure.

## 1. Characterize the current boundary

Name the unit's one reason to change and list the responsibilities currently mixed into it.
Record the failing command/diagnostic and the relevant limits from the root section; avoid a
second threshold configuration here. Search `ofm_core`, `src/lib`, `src/utils`, `src/services` and
`src/components/ui/index.ts` for an existing owner before extracting another copy.

Use [`/write-tests`](../write-tests/SKILL.md) to pin weakly covered scenarios **before** moving
code. For pure decomposition, observe existing and characterization tests green on the unchanged
code; do not invent a red failure for unchanged behaviour. A new behaviour/fix gets its own red
scenario and separate change. Cover the public state/persistence seam when wiring can regress.

## 2. Extract by responsibility

- Rust: keep a `mod.rs` orchestration shell and cohesive submodules, following
  `ofm_core/src/generator/` and `ofm_core/src/turn/`. Engine mirror conversions stay only in `turn/`.
- Frontend: a cohesive `*.helpers.ts`, hook or child component, with football names; respect
  library/service/UI/feature/page directions. Components call services; policy stays in Rust.
- Boolean selectors for distinct operations become two named functions or a meaningful enum.
  Preserve a documented wire selector at its adapter if changing it would break callers.
- More than seven parameters become a request struct only if they describe a real concept,
  with explicit units and IDs. Never introduce a catch-all `Context` containing the whole world.
- Long Tauri command bodies move into a shared `*_internal`/application function; both command
  and MCP wrappers delegate there. Application modules do not import either adapter.
- Helpers start private or `pub(crate)` until a second caller needs them. A trait belongs at a
  consuming boundary only for real I/O with a second implementation/test double; no abstract
  scaffolding to satisfy SOLID. The approved `db -> ofm_core` edge is preserved.

Check single responsibility, extension seams, replacement contracts, small input interfaces and
real dependency inversion. Do not rename stored fields or IPC keys unless an authorized migration
ships; pure refactoring preserves serialized data. Keep validation before live mutation, atomic
writes and error propagation. An intentional outcome change is a separate tested fix.

## 3. Verify each small extraction

Rerun the named scenario tests and the scoped failing gate after each coherent move. For Rust,
check default and `mcp` feature builds/tests if the seam is shared. For frontend, verify stateful
hosts and component/service wiring as well as helper outputs. Read persisted state through a
fresh reader when writes move. Use `ofm-architecture-reviewer`, `ofm-dedup-reviewer` and
`ofm-test-reviewer` for their respective evidence, then [`/preflight`](../preflight/SKILL.md).

Never raise a threshold, add a broad suppression, shorten meaningful names, remove useful comments,
move tests to evade metrics or weaken tests to turn a gate green. Format only changed code; the
final format check is read-only. Report an out-of-scope duplicate in the PR body for #589, do not
file an issue or copy it again. A blocker does not authorize unrelated fixes.

## 4. Report the result

Give before/after diagnostics, responsibilities and their new homes, the public contract preserved,
scenario/test commands and results, reviewer findings, and checks not run. Explain the split in
plain words so the reviewer can assess its purpose. Do not claim a green linter proves SOLID or
behaviour, and do not include an unmeasured complexity number.
