# Architecture

OpenFoot Manager is a desktop football management simulation built with **Tauri** (Rust backend) and **React** (TypeScript frontend). This document describes the project structure, key architectural decisions, and how the pieces fit together.

---

## Technology Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **Desktop shell** | Tauri v2 | Native window, IPC, file system access |
| **Backend** | Rust | Game logic, simulation, persistence |
| **Frontend** | React + TypeScript | UI rendering, user interaction |
| **Styling** | Tailwind CSS | Utility-first CSS framework |
| **State (frontend)** | Zustand | Lightweight stores for game and settings |
| **i18n** | i18next + react-i18next | Internationalization (7 locales) |
| **Build** | Vite | Frontend bundler and dev server |

---

## Project Structure

```
openfootmanager/
├── src/                          # Frontend (React + TypeScript)
│   ├── components/               # Reusable UI components
│   │   ├── match/                # Match day sub-components
│   │   └── ui/                   # Design system primitives (Badge, ThemeToggle)
│   ├── context/                  # React contexts (ThemeContext)
│   ├── i18n/                     # Internationalization config + locale files
│   │   └── locales/              # en.json, de.json, es.json, fr.json, it.json, pt.json, pt-BR.json
│   ├── lib/                      # Shared utilities (helpers.ts, countries.ts)
│   ├── pages/                    # Route-level pages
│   ├── services/                 # Frontend Tauri/invoke wrappers
│   ├── store/                    # Zustand stores (gameStore, settingsStore)
│   ├── utils/                    # Frontend helpers and i18n adapters
│   ├── App.tsx                   # Router setup
│   └── main.tsx                  # Entry point
├── src-tauri/                    # Backend (Rust + Tauri)
│   ├── src/
│   │   └── lib.rs                # Tauri commands, app setup, settings
│   ├── crates/
│   │   ├── domain/               # Pure data types (no logic)
│   │   ├── engine/               # Match simulation engine
│   │   ├── ofm_core/             # Game logic, state, turn processing
│   │   └── db/                   # Save/load persistence
│   ├── data/                     # External definition files (names, teams JSON)
│   └── databases/                # Bundled world database files
├── docs/                         # Documentation
├── public/                       # Static assets
└── images/                       # Branding assets
```

---

## Crate Architecture

The Rust backend is organized into 4 crates with clear dependency boundaries:

```
                    ┌──────────┐
                    │  Tauri   │  src-tauri/src/lib.rs
                    │ Commands │  (IPC boundary)
                    └────┬─────┘
                         │
                    ┌────┴─────┐
                    │ ofm_core │  Game logic, turn processing, state
                    └──┬───┬───┘
                       │   │
              ┌────────┘   └────────┐
         ┌────┴───┐           ┌─────┴────┐
         │ engine │           │    db    │
         │        │           │          │
         └────────┘           └──────────┘
              │
         ┌────┴───┐
         │ domain │  Pure data types (shared by all)
         └────────┘
```

### `domain` — Pure Data Types

Contains only structs and enums with no game logic. All other crates depend on it.

- **`player.rs`** — `Player`, `PlayerAttributes` (18 attributes), `Position`, `PlayerTrait` (20 traits), `PlayerSeasonStats`, `Injury`, `TransferOffer`
- **`team.rs`** — `Team`, `PlayStyle`, `TrainingFocus`, `TrainingIntensity`, `TrainingSchedule`, `TeamColors`
- **`staff.rs`** — `Staff`, `StaffRole` (4 roles), `CoachingSpecialization` (7 specializations), `StaffAttributes`
- **`manager.rs`** — `Manager`, `ManagerCareerStats`, `ManagerCareerEntry`
- **`league.rs`** — `League`, `Fixture`, `StandingEntry`, `MatchResult`, `GoalEvent`
- **`message.rs`** — `InboxMessage`, `MessageCategory` (15 categories), `MessagePriority`, `MessageAction`, `ActionType`
- **`news.rs`** — `NewsArticle`, `NewsCategory` (8 categories), `NewsMatchScore`

**Design decision**: Domain types use `#[serde(default)]` extensively on newer fields for backward compatibility with old save files.

### `engine` — Match Simulation

Self-contained simulation engine, deliberately **decoupled from `domain`**. Defines its own mirror types (`PlayerData`, `TeamData`, `Position`, `PlayStyle`) so it can be tested and evolved independently.

See [MATCH_SIMULATION.md](MATCH_SIMULATION.md) for full details.

- **`engine.rs`** — Instant full-match simulation (`simulate()`, `simulate_with_rng()`)
- **`live_match.rs`** — Step-by-step `LiveMatchState` with phase management, commands, substitutions
- **`ai.rs`** — AI manager decisions (`AiProfile`, `ai_decide()`)
- **`types.rs`** — Engine-specific data types and `MatchConfig`
- **`event.rs`** — `MatchEvent` + `EventType` (22 variants)
- **`report.rs`** — `MatchReport`, `TeamStats`, `PlayerMatchStats`, `GoalDetail`

### `ofm_core` — Game Logic

The core game loop — ties domain, engine, and all game systems together.

- **`game.rs`** — `Game` struct (the root game state: clock, manager, teams, players, staff, messages, news, league)
- **`clock.rs`** — `GameClock` with date tracking and day advancement
- **`state.rs`** — `StateManager` with `Mutex<Option<Game>>` and `Mutex<Option<LiveMatchSession>>` for thread-safe Tauri access
- **`turn.rs`** — Day processing: match simulation, domain↔engine conversion, stats application, news generation
- **`training.rs`** — Training system: attribute gains, condition management, staff bonuses, fitness warnings
- **`schedule.rs`** — Round-robin league schedule generation (circle method)
- **`generator.rs`** — World generation: name/team definition loading, player/staff/team creation
- **`live_match_manager.rs`** — `LiveMatchSession` wrapping the engine's `LiveMatchState` with RNG, AI profiles
- **`messages.rs`** — Inbox message generation (welcome, match previews/results, board directives, etc.)
- **`job_offers.rs`** — Vacancy-backed job opportunities, direct applications, offer responses, and offer expiry
- **`firing.rs`** — Board warning/firing flow and managerial-change dismissal news
- **`ai_hiring.rs`** — AI manager seeding, vacancy aging, and delayed replacement hires
- **`board_objectives.rs`** — Board objective messages and objective tracking
- **`news.rs`** — News article generation (match reports, league roundups, standings, season previews, managerial appointments)

### `db` — Persistence

Save/load functionality:

- **`save_manager.rs`** — Save slot orchestration, load/save flows, and round-trip tests
- **`game_persistence.rs`** — `Game` serialization/deserialization between runtime state and storage records
- **`repositories/`** — SQLite repositories for saves, metadata, news, and related persistence records

---

## State Management

### Backend State

The `StateManager` (in `ofm_core/state.rs`) holds the active game and live match session behind `Mutex` locks:

```rust
pub struct StateManager {
    pub active_game: Mutex<Option<Game>>,
    pub live_match: Mutex<Option<LiveMatchSession>>,
}
```

This is registered as Tauri managed state and accessed by all commands via `State<StateManager>`.

**Key pattern**: Commands acquire the mutex, clone the `Game` for return, and release the lock. Mutations happen inside the lock scope.

### Frontend State

Two Zustand stores:

- **`gameStore`** — Active game state (`GameStateData`), manager info, `hasActiveGame` flag. Updated after every Tauri command that returns game state.
- **`settingsStore`** — `AppSettings` (theme, language, currency, match preferences). Loaded once on Settings page mount, persisted via Tauri commands.

---

## Tauri Command Interface (IPC)

All frontend↔backend communication goes through Tauri's `invoke()` mechanism. Commands are defined in `src-tauri/src/lib.rs` and registered in the Tauri builder.

### Game Lifecycle Commands

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `list_world_databases` | — | `Vec<WorldDatabaseInfo>` | Scan for available world databases |
| `start_new_game` | first_name, last_name, dob, nationality, world_source? | — | Create game, generate or load world |
| `choose_team` | team_id | `Game` | Assign manager to a team |
| `get_active_game` | — | `GameStateData` | Get current game state |
| `advance_time` | — | `Game` | Advance one day |
| `advance_time_with_mode` | mode | `{action, game?, snapshot?}` | Advance with match mode preference |
| `skip_to_match_day` | — | `Game` | Fast-forward to next fixture |
| `save_game` | — | — | Persist current game |
| `load_game` | save_id | — | Load a saved game |
| `exit_to_menu` | — | — | Save and clear active game |

### Match Commands

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `start_live_match` | fixture_index, mode, allows_extra_time | `MatchSnapshot` | Initialize a live match session |
| `step_live_match` | minutes | `Vec<MinuteResult>` | Advance simulation by N minutes |
| `apply_match_command` | command | `MatchSnapshot` | Send a tactical command |
| `get_match_snapshot` | — | `MatchSnapshot` | Get current match state |
| `finish_live_match` | — | `Game` | Apply results and clean up |

### Team Management Commands

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `set_formation` | formation | `Game` | Change team formation |
| `set_play_style` | play_style | `Game` | Change play style |
| `set_training` | focus, intensity | `Game` | Set training focus and intensity |
| `set_training_schedule` | schedule | `Game` | Set weekly training schedule |
| `hire_staff` | staff_id | `Game` | Hire an unattached staff member |
| `release_staff` | staff_id | `Game` | Release a staff member |

### Settings Commands

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_settings` | — | `AppSettings` | Load settings from disk |
| `save_settings` | settings | — | Persist settings |
| `clear_all_saves` | — | — | Delete all save files |
| `export_world_database` | export_path | `String` | Export world to JSON |

---

## Data Flow

### New Game Flow

```
MainMenu → Create Manager → Choose World → Choose Team → Dashboard
    │              │               │              │
    │         form input     generate_world   choose_team
    │                        or load JSON      command
    │                             │              │
    │                        ┌────┴────┐    ┌────┴────┐
    │                        │Generator│    │  Game   │
    │                        │  +JSON  │    │ created │
    │                        └─────────┘    └─────────┘
```

### Daily Turn Flow

```
Dashboard → "Continue" button → advance_time_with_mode
                                        │
                           ┌────────────┼────────────┐
                           │            │            │
                      No match     Live match    Delegate
                           │            │            │
                    process_day()  start_live    simulate
                           │       _match()     instantly
                    ┌──────┴──────┐     │            │
                    │ Training    │  Navigate     apply
                    │ + Recovery  │  to /match    results
                    │ + Messages  │     │            │
                    │ + Clock++   │  [interactive]   │
                    └─────────────┘     │      ┌─────┴─────┐
                                   finish_live  Return to
                                   _match()     Dashboard
                                        │
                                   apply results
                                   + news + clock
```

---

## Frontend Architecture

### Routing

| Route | Component | Description |
|-------|-----------|-------------|
| `/` | `MainMenu` | Main menu with new/load/settings |
| `/team-selection` | `TeamSelection` | Choose club to manage |
| `/dashboard` | `Dashboard` | Main game interface |
| `/match` | `MatchSimulation` | Live match simulation |
| `/settings` | `Settings` | App settings |

### Dashboard Component Architecture

The Dashboard is a slim layout shell (~524 lines) with a sidebar, header, and content area. Content is rendered by tab-specific components:

```
Dashboard.tsx (layout shell)
├── Sidebar: NavItem buttons for each tab
├── Header: Back button, tab title, date, search, finances, Continue
└── Content area (conditional rendering by activeTab):
    ├── HomeTab         — Overview, next match, standings, squad summary
    ├── InboxTab        — Two-pane email client with categories
    ├── ManagerTab      — Manager profile, career stats
    ├── SquadTab        — Player roster table
    ├── TacticsTab      — Formation picker, pitch visualization
    ├── TrainingTab     — Focus/intensity/schedule selection, fitness
    ├── StaffTab        — Staff management (hire/release)
    ├── FinancesTab     — Financial overview
    ├── TransfersTab    — Transfer market with 4 views
    ├── PlayersListTab  — Full player database browser
    ├── TeamsListTab    — All teams grid
    ├── TournamentsTab  — League standings and fixtures
    ├── ScheduleTab     — Calendar of fixtures
    ├── NewsTab         — News feed
    ├── PlayerProfile   — Detailed player view (inline)
    └── TeamProfile     — Detailed team view (inline)
```

**Navigation history**: The Dashboard maintains a `navHistory` stack for back navigation when drilling into player/team profiles from any tab.

### Match Day Components

The match simulation uses a multi-stage orchestrator:

```
MatchSimulation.tsx (orchestrator)
├── PreMatchSetup    — Team sheet, formation, set pieces
├── MatchLive        — Live simulation with events, stats, controls
├── HalfTimeBreak    — Team talk, tactical changes
├── PostMatchScreen  — Result summary, scorers, team talk
└── PressConference  — Post-match press questions
```

Flow: `prematch → first_half → halftime → second_half → postmatch → press`

### Design Language

The UI follows a **"Matchday" broadcast-quality** design language:

- **Colors**: Emerald green primary (#10b981), gold accent (#ffd60a), dark navy backgrounds
- **Typography**: Barlow Condensed (headings, uppercase tracking) + Inter (body)
- **Fonts**: Bundled locally via `@fontsource` packages (no CDN)
- **Dark/Light**: Full theme support with system detection

---

## Key Architectural Decisions

### 1. Engine Isolation

The `engine` crate has **no dependency on `domain`**. It defines its own `PlayerData`, `TeamData`, etc. This means:
- The engine can be tested with synthetic data (no need to generate full game worlds)
- Engine types can evolve independently (e.g., adding engine-only fields)
- The `turn.rs` bridge in `ofm_core` handles all type conversion

### 2. `#[serde(default)]` for Backward Compatibility

Every new field added to domain types uses `#[serde(default)]` or a custom default function. This ensures old save files can be loaded without migration scripts. The trade-off is that new features gracefully degrade (e.g., old saves have no traits, empty training schedules) rather than failing.

### 3. Hardcoded Fallbacks for External Data

Generator definition files (`default_names.json`, `default_teams.json`) are loaded at runtime with fallback to hardcoded arrays compiled into the binary. This means:
- The game always works, even without external files
- Users can customize by placing definition files in the data directory
- No build-time dependency on external data files

### 4. Mutex-Based State Management

The backend uses `Mutex<Option<Game>>` rather than `RwLock` or actor patterns. This is simpler and sufficient because:
- Only one game is active at a time
- Commands are sequential from the UI perspective
- Lock contention is negligible (commands are fast)

### 5. `PlayerSnap` Pattern in Engine

The engine uses a "snapshot" pattern (`PlayerSnap`) to work around Rust's borrow checker. Before resolving an action, it clones the relevant player data into a lightweight struct, releasing the immutable borrow on `MatchContext`. This allows event emission (which needs `&mut self`) to happen in the same function.

### 6. Football Identity Codes

The game now distinguishes between general country data and football-facing identity data.
Most countries still use ISO 3166-1 alpha-2 codes (for example, `"ES"`, `"DE"`, `"BR"`), but football-specific identities can use project-owned short codes where the sport diverges from ISO country data, such as `"ENG"`, `"SCO"`, `"WAL"`, and `"NIR"`.

This enables:
- Locale-aware country and football-nation display
- Backward compatibility for legacy demonyms and `GB` saves
- Correct modeling of football nations without forcing a full non-ISO rewrite for the rest of the world

### 7. Frontend Tab Architecture

The Dashboard uses conditional rendering (not routing) for tabs. This means:
- Tab state is preserved when switching between tabs
- No URL changes for tab navigation (clean URLs)
- Profile views (player/team) overlay on top of the current tab with a back-navigation stack
