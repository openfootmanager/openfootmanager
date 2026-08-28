<!--
Thanks for contributing to OpenFoot Manager!

Target branch must be `develop`. Give the PR a conventional-commit title, e.g.
  fix(ui): stop the Select menu being clipped by overflow ancestors
  feat(world-cup): multi-season qualifying
  test(training): cover the peaked-player regression
-->

## What and why

<!-- What changes, and what problem it solves. The "why" matters more than the "what" —
     the diff already says what. -->

Closes #

## How to verify

<!-- The steps a reviewer should take to see this working, or the tests that prove it. -->

---

## Checklist

- [ ] Targets `develop`, branched from an up-to-date `develop`
- [ ] Conventional-commit title
- [ ] Linked to an issue (open one first for new features — see [CONTRIBUTING.md](../CONTRIBUTING.md))

**Tests**

- [ ] New behaviour has a test that would have failed before this change
- [ ] `npm test` passes
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --workspace` passes
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings` is clean

**If this changes user-facing text**

- [ ] Added to `src/i18n/locales/en.json`
- [ ] Translated into **every** other locale — `cs de es fr id it pt pt-BR ru tr zh-CN`
- [ ] `npx vitest run src/i18n` passes
- [ ] Backend strings emit translation **keys**, not English prose

**If this changes the UI**

- [ ] Uses design tokens from `src/App.css` — no hex literals, no arbitrary values
- [ ] Checked in **both** light and dark themes
- [ ] Keyboard reachable, visible focus, icon-only controls have a translated `aria-label`

**If this changes persisted data**

- [ ] New serialized fields have `#[serde(default)]`
- [ ] Repository columns, `params!`, positional `row.get`, and **both** SELECT lists updated
- [ ] Migration added and `MIGRATION_COUNT` bumped
- [ ] Round-trip test proves the field survives save/load

**If this changes docs-worthy behaviour**

- [ ] `docs/ARCHITECTURE.md` command tables updated
- [ ] `docs/MCP_SERVER.md` tool tables and counts updated

---

## AI assistance

<!-- This is a GPLv3 project, so provenance matters. Just be straightforward. -->

- [ ] This change was written or assisted by an AI coding agent
- [ ] I have read the diff myself and I stand behind it

<!-- Contributors using AI agents: see CLAUDE.md and AGENTS.md at the repository root
     for the project's conventions, and the /preflight skill for the full check list. -->
