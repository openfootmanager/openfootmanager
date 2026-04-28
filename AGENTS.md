# AGENTS.md - OpenFoot Manager Development Guide

## Project Overview
- **Desktop football manager** built with Tauri (Rust backend + React frontend)
- **Monorepo structure**: frontend in `src/`, Rust workspace in `src-tauri/crates/`
- **Save system**: SQLite-based with JSON serialization for game state
- **Match simulation**: Event-driven engine with live commentary

## Key Commands
```bash
# Development
npm run tauri dev           # Run dev server (frontend + backend)
npm test                    # Run frontend tests (Vitest + jsdom)
npm run test:watch          # Run frontend tests in watch mode

# Rust backend tests
cd src-tauri
cargo test --workspace      # Run all Rust tests
cargo test -p ofm_core      # Run core game logic tests
cargo test -p engine        # Run match simulation tests
cargo test -p db            # Run database/persistence tests

# Building
npm run build              # Build frontend only
npm run tauri build        # Build full desktop app
```

## Architecture Notes
- **Rust workspace** (`src-tauri/Cargo.toml`):
  - `domain`: Pure data types (no logic)
  - `ofm_core`: Game logic, turn processing, state management
  - `engine`: Match simulation engine
  - `db`: SQLite persistence, migrations, save/load
- **Frontend/backend IPC**: Tauri commands in `src-tauri/src/commands/`
- **Game state flow**: Managed by `ofm_core::turn` module with turn-by-turn progression

## Testing Conventions
- **Frontend**: Vitest with jsdom, test files co-located with components
- **Rust**: Unit tests in `tests/` subdirectories within each crate
- **Test data**: Mock game states in `tests/` modules, not external fixtures

## Development Workflow
1. **Frontend changes**: Run `npm run tauri dev` for hot reload
2. **Rust changes**: Requires restart of dev server (Tauri rebuilds automatically)
3. **Testing**: Run frontend and Rust tests separately
4. **Database changes**: Migration files in `src-tauri/crates/db/src/sql/`

## Gotchas & Quirks
- **Starting XI bug**: `live_match_manager/team_builder::build_team_with_bench` ignores `team.starting_xi_ids` and auto-selects best players by rating. This causes formation mismatches between pre-match selection and actual match start.
- **SQLite saves**: Game state serialized as JSON in `games` table, with separate `player_match_stats` table for historical data
- **Internationalization**: Frontend uses i18next, backend has separate message keys in `ofm_core/src/messages/`
- **Condition/fitness**: Player attributes affect match performance via `engine::live_match` condition adjustments

## Code Style
- **Rust**: Standard Rust idioms, `cargo fmt` for formatting
- **TypeScript**: Strict mode enabled, no unused locals/parameters
- **React**: Functional components with hooks, Zustand for state management
- **Tailwind**: Utility-first CSS with `@tailwindcss/vite`

## Key Files for Understanding
- `src-tauri/crates/ofm_core/src/turn/mod.rs` - Core game loop
- `src-tauri/crates/engine/src/live_match/mod.rs` - Match simulation
- `src-tauri/crates/ofm_core/src/live_match_manager/` - Match session management
- `src/store/gameStore.ts` - Frontend game state store
- `src-tauri/src/commands/live_match.rs` - Tauri commands for matches