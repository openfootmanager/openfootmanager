# AGENTS.md

Conventions for AI coding agents contributing to OpenFoot Manager. Tool-agnostic — Claude Code,
Cursor, Copilot, Codex, Aider, or anything else.

Claude Code users get more: see [`CLAUDE.md`](CLAUDE.md) for the full command reference plus the
project's skills (`/add-ui-string`, `/preflight`, …) and review agents in `.claude/`.

## Build and test

```bash
npm install
npm test                                                        # frontend suite
npm run build                                                   # tsc && vite build
cargo test --manifest-path src-tauri/Cargo.toml --workspace      # backend suite
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets
cargo fmt --manifest-path src-tauri/Cargo.toml --all
```

Tauri command tests live in the `openfootmanager_lib` lib target — use
`cargo test --manifest-path src-tauri/Cargo.toml --lib`. `cargo test --bin` matches zero tests.

## The six rules

1. **TDD.** Failing test first. Rust tests in a `#[cfg(test)]` module in the same file; frontend
   tests co-located as `*.test.ts(x)`.
2. **Every locale.** Every user-facing string is translated into all of
   `SUPPORTED_LANGUAGES` (`src/i18n/index.ts`, one file each in `src/i18n/locales/`).
   English-only changes fail
   `src/i18n/localeCoverage.test.ts`. Rust emits translation *keys*, never English prose.
3. **`engine` never imports `domain`.** The match engine keeps its own mirror types on purpose;
   `ofm_core/turn/` is the only bridge. See `docs/ARCHITECTURE.md` §"Engine Isolation".
4. **`#[serde(default)]` on every new serialized field.** Old saves must keep loading. Fields
   that reach SQLite need repository and migration edits too — see `src-tauri/CLAUDE.md`.
5. **No `any` in TypeScript; Tailwind utilities, not raw CSS.** Design tokens are defined in
   `src/App.css` under `@theme`; use token classes, never hex literals.
6. **Conventional commits on a branch off `develop`**, e.g. `fix(ui):`, `feat(world-cup):`,
   `test(training):`. PRs target `develop`. Never commit to `develop` directly.

## Before opening a PR

Run the frontend suite, the backend suite, `npm run build`, and `cargo clippy`. Disclose that the
change was AI-assisted in the PR description — this is a GPLv3 project and provenance matters.
See [`CONTRIBUTING.md`](CONTRIBUTING.md).
