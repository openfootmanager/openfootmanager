# Match Simulation

This document describes how OpenFoot Manager simulates football matches. The simulation has two modes: **instant** (used for AI-vs-AI matches during day advancement) and **live** (step-by-step, used when the player watches or controls a match).

Both modes share the same core resolution logic, but the live system adds interactivity — substitutions, formation changes, halftime talks — and per-minute stamina depletion.

## Historical Context

The original OFM simulation (documented in `docs/legacy/simulation.rst`) used a detailed event model with 15 pitch zones, transition matrices, and event chains (pass → intercept → foul → free kick). That system tracked possession through a fine-grained state machine.

The current implementation simplifies this to a **5-zone, action-based** model. Rather than modelling individual ball movements across 15 regions with transition matrices, the engine resolves 1–3 **actions per minute** in the current zone, with zone progression driven by action outcomes. This keeps the simulation fast enough for instant mode (a full match in ~2ms) while still producing realistic statistics and event feeds.

---

## Architecture

The simulation lives in the `engine` crate (`src-tauri/crates/engine/`), which is deliberately **decoupled from the domain crate**. It defines its own mirror types (`Position`, `PlayStyle`, `PlayerData`, `TeamData`) so that the engine can be tested and evolved independently.

```
engine/
├── types.rs      — PlayerData, TeamData, MatchConfig, Zone, Side, PlayStyle
├── event.rs      — MatchEvent struct + EventType enum (22 event variants)
├── engine.rs     — Core instant simulation (simulate / simulate_with_rng)
├── report.rs     — MatchReport, TeamStats, PlayerMatchStats, GoalDetail
├── live_match.rs — LiveMatchState for step-by-step simulation
├── ai.rs         — AI manager decision engine
└── lib.rs        — Re-exports
```

The `ofm_core` crate bridges domain ↔ engine:
- `turn.rs` — converts domain types to engine types, runs simulations, applies results back
- `live_match_manager.rs` — wraps `LiveMatchState` with session management, RNG, AI profiles

---

## The Zone Model

The pitch is divided into 5 logical zones, viewed from a neutral perspective:

```
┌──────────┬──────────────┬───────────┬──────────────┬──────────┐
│ HomeBox  │ HomeDefense  │ Midfield  │ AwayDefense  │ AwayBox  │
└──────────┴──────────────┴───────────┴──────────────┴──────────┘
  ← Away attacks this way          Home attacks this way →
```

- **HomeBox / AwayBox** — the penalty areas. When the attacking team reaches the opposing box, a **shot** is resolved.
- **HomeDefense / AwayDefense** — the defensive thirds. Attacking here involves dribbles and tackles against defenders.
- **Midfield** — the contested middle. Possession battles, passes, interceptions happen here.

Zone progression is directional: the ball advances toward the attacking team's target box one zone at a time on successful actions, and retreats on failures.

---

## Minute-by-Minute Flow

Each simulated minute follows this sequence:

1. **Possession tick** — the possessing side's counter increments (used for possession % stats).
2. **1–3 actions** — a random number of actions are resolved in the current zone.
3. **Midfield contest** — after actions, a possession contest occurs. The possessing side's midfield rating is compared against the defending side's. If the defender wins, possession flips and the ball resets to midfield.

### Action Resolution by Zone

| Current Zone | Resolution Function | Key Players | Outcome on Success | Outcome on Failure |
|---|---|---|---|---|
| Own defense (buildup) | `resolve_buildup` | Defenders pass | Ball → Midfield | Interception, possession flips |
| Midfield | `resolve_midfield` | Midfielder vs Midfielder | Ball → Attacking third | Tackle/Interception, may trigger foul |
| Attacking third | `resolve_attacking_third` | Forward vs Defender | Ball → Attacking box | Tackle/Clearance, possible corner (25%), may trigger foul |
| Attacking box | `resolve_shot` | Forward shoots vs GK | Goal or save | Shot off/blocked/saved, ball resets to midfield |

### Shot Resolution

When the ball reaches the attacking box, a shot is taken:

1. **Accuracy check** — base accuracy (configurable, default 45%) adjusted by the shooter's `shooting + composure + decisions` rating. Clamped to 15%–85%.
   - **Miss**: 40% chance the shot is blocked, 60% chance off target.
2. **Conversion check** — base conversion (default 30%) adjusted by `(shooter_rating - goalkeeper_rating) / 150`. Clamped to 10%–70%.
   - **Score**: Goal event emitted with scorer + assister.
   - **Save**: Shot saved event.

### Foul & Discipline

Fouls can occur after tackles in midfield and the attacking third:

1. **Foul probability** — base 12%, modified by the fouler's `aggression` attribute and the `HotHead` / `CoolHead` traits.
2. **If foul occurs in the box** → 8% chance of a **penalty** being awarded.
3. **Card probability** — 30% chance of a yellow, modified by aggression. 4% chance a card is upgraded to red.
4. **Second yellow** → automatic red card and sending off.
5. **Injury** — 3% chance per foul that the fouled player is injured.

Sent-off players are excluded from all future player selection for their team.

---

## Player Attributes

The engine uses 18 player attributes, grouped into categories:

**Physical**: pace, stamina, strength, agility
**Technical**: passing, shooting, tackling, dribbling, defending
**Mental**: positioning, vision, decisions, composure, aggression, teamwork, leadership
**Goalkeeper**: handling, reflexes, aerial

### Overall Rating

A player's overall rating (OVR) is the mean of the 11 core outfield attributes (pace, stamina, strength, passing, shooting, tackling, dribbling, defending, positioning, vision, decisions).

### Effective Rating

The effective rating accounts for **condition** (stamina/fitness, 0–100):

```
effective_overall = overall × (condition / 100)
```

A tired player (condition 50) performs at half their potential.

---

## Composite Team Ratings

The engine computes team-level ratings from player attributes by position:

| Rating | Source Players | Attributes Used | Weighting |
|---|---|---|---|
| **Defense** | Defenders (70%) + GK (30%) | defending, tackling, positioning, strength | Position average |
| **Midfield** | Midfielders | passing, vision, decisions, stamina | Position average |
| **Attack** | Forwards (75%) + Midfielders (25%) | shooting, dribbling, pace, positioning | Blended average |
| **Goalkeeper** | Goalkeepers | positioning, decisions, pace, strength | Position average |

---

## Play Styles

Each team has a play style that applies multiplicative modifiers to different phases of play:

| Play Style | Midfield | Attack | Defense | Press |
|---|---|---|---|---|
| **Balanced** | 1.00 | 1.00 | 1.00 | 1.00 |
| **Attacking** | 1.00 | **1.12** | 0.93 | 1.00 |
| **Defensive** | 1.00 | 0.93 | **1.12** | 1.00 |
| **Possession** | **1.15** | 0.97 | 1.00 | 1.00 |
| **Counter** | 0.92 | **1.18** | 1.00 | 1.00 |
| **High Press** | 1.00 | 1.00 | 0.95 | **1.20** |

These modifiers are applied to the relevant team rating during action resolution. For example, a Counter team gets an 18% boost to attack rating but an 8% penalty to midfield control.

---

## Phase Blueprint (TacticsConfig)

On top of the play style, each team carries a `TacticsConfig` (mapped from the
domain `TacticsPhaseSettings`) of nine dials. Every dial defaults to a neutral
option that multiplies by ×1.0 — and the transition dials roll nothing — so a
team on its defaults simulates **byte-identically** to the pre-dial engine.

Each dial has exactly one hook and one direction (no opposing effects stacked on
a single dial). The first five are applied in their per-action zone; the rest add
the possession/transition dimension the zones don't cover, hooked into the
per-minute possession contest.

| Dial | Hook | Effect |
|---|---|---|
| build_up | build-up pass success | Short retains, Long is riskier |
| width | cross probability (attacking third) | Wide crosses more |
| def_line | shot conversion conceded | High concedes better chances in behind |
| marking | foul probability | Man-to-man fouls more |
| pressing | possession contest (def) + build-up press + **in-match stamina** | Aggressive wins the ball back more, tires faster |
| tempo | midfield progression (att) + possession contest retention (att) | Direct shoots more / Patient holds the ball |
| defensive_shape | attacking-third defence (def) | Compact concedes fewer chances |
| counter_press | possession flip: losing side may re-win the ball | Long regains possession more |
| break_speed | possession flip: winner may counter into the final third | Fast turns turnovers into chances |

Both the instant engine (`engine/`) and the live engine (`live_match/`) consume
the dials identically; the stamina cost of pressing applies only to the live
engine, which tracks per-minute condition. Magnitudes live in `engine::shared`
and are tuned with `cargo run -p sim-bench -- --phase-sweep`, which tabulates
each dial's effect on possession %, shots and goals against a neutral opponent.

---

## Home Advantage

The home team receives a configurable multiplier (default **1.08**, i.e. 8% boost) applied to all their ratings during action resolution. This models the effect of playing at home — crowd support, familiarity with the pitch, etc.

---

## Player Traits

Traits are computed from a player's attributes and position (see `domain::player::compute_traits()`). The engine uses trait names as strings and applies multiplicative bonuses in 7 contexts:

| Context | Relevant Traits | Bonus |
|---|---|---|
| **Shooting** | Sharpshooter (+8%), CoolHead (+4%), CompleteForward (+5%) | |
| **Dribbling** | Dribbler (+8%), Speedster (+4%), Agile (+4%) | |
| **Passing** | Playmaker (+8%), Visionary (+5%), SetPieceSpecialist (+3%) | |
| **Tackling** | BallWinner (+8%), Rock (+5%), Tank (+4%) | |
| **Goalkeeping** | SafeHands (+8%), CatReflexes (+6%), AerialDominance (+4%) | |
| **Foul** | HotHead (+25% foul chance), CoolHead (−30% foul chance) | |
| **Midfield** | Engine (+6%), TeamPlayer (+4%), Tireless (+3%) | |

Trait bonuses are multiplicative — a Sharpshooter with CoolHead gets `1.08 × 1.04 = 1.123` (12.3% boost) on shooting actions.

---

## Match Configuration

All probabilities and multipliers are tuneable via `MatchConfig`:

| Parameter | Default | Description |
|---|---|---|
| `home_advantage` | 1.08 | Multiplier for home team ratings |
| `shot_accuracy_base` | 0.45 | Base chance a shot is on target |
| `goal_conversion_base` | 0.30 | Base chance an on-target shot scores |
| `fatigue_per_minute` | 0.15 | Condition loss per minute (live mode) |
| `foul_probability` | 0.12 | Base chance a tackle results in a foul |
| `yellow_card_probability` | 0.30 | Chance a foul produces a yellow |
| `red_card_probability` | 0.04 | Chance a card is direct red |
| `penalty_probability` | 0.08 | Chance a box foul is a penalty |
| `stoppage_time_max` | 4 | Max stoppage time minutes per half |
| `injury_probability` | 0.03 | Chance of injury per foul |

---

## Match Report

After simulation, a `MatchReport` is generated from the raw event list:

- **TeamStats** — goals, shots (on/off/blocked), passes (completed/intercepted), tackles, interceptions, fouls, corners, free kicks, penalties, cards, possession ticks
- **PlayerMatchStats** — minutes played, goals, assists, shots, passes, tackles, interceptions, fouls, cards, match rating (0–10)
- **GoalDetails** — minute, scorer, assister, whether it was a penalty
- **Possession %** — computed from possession ticks: `home_ticks / (home_ticks + away_ticks)`

---

## Live Match System

The live match system (`live_match.rs`) wraps the core zone-based resolution into a **step-by-step** simulation with:

### Match Phases

```
PreKickOff → FirstHalf → HalfTime → SecondHalf → FullTime
                                                      ↓ (if drawn + extra time allowed)
                                              ExtraTimeFirstHalf → ExtraTimeHalfTime
                                                      → ExtraTimeSecondHalf → ExtraTimeEnd
                                                                                    ↓ (if still drawn)
                                                                            PenaltyShootout → Finished
```

### Commands

Users (and AI) can inject commands between minutes:

- **Substitute** — swap a player on the pitch with a bench player (max 5 subs per match)
- **ChangeFormation** — change the team's formation (e.g. "4-3-3" → "3-5-2"), which reassigns player positions
- **ChangePlayStyle** — switch between the 6 play styles
- **SetFreeKickTaker / SetCornerTaker / SetPenaltyTaker / SetCaptain** — assign set piece roles

### Stamina Depletion

In live mode, player condition depletes each minute based on `fatigue_per_minute`. A `condition_adjusted_skill()` function scales all attribute checks by the player's current condition, so fatigued players perform worse. This makes substitutions tactically meaningful.

### Penalty Shootout

If a match is drawn after extra time (when allowed), a penalty shootout is resolved:
- 5 rounds of alternating penalties
- If still tied, sudden death rounds continue until one team leads after equal attempts

### Match Snapshot

At any point, a `MatchSnapshot` can be taken — a serializable view of the entire match state including scores, teams with current conditions, events, phase, substitution records, and bench players. This is what the frontend renders.

---

## AI Manager

The AI manager (`ai.rs`) controls non-player sides during live matches. It evaluates the match state once per minute and can issue substitution and tactical commands.

### AI Profile

Each AI manager has:
- **Reputation** (0–1000) — higher = more sophisticated decisions
- **Experience** (0–100) — affects timing and quality of choices

### Decision Logic

**Substitutions:**
- After minute 55: replace most fatigued players (condition < threshold based on experience)
- After minute 65 (losing): tactical subs — bring on attacking players
- After minute 80 (winning): defensive subs — bring on fresh defenders/midfielders

**Tactical changes:**
- Losing by 2+ goals after minute 60: switch to Attacking
- Winning by 2+ goals after minute 75: switch to Defensive
- Close game: maintain current style

The experience factor scales how early and aggressively the AI makes decisions.

---

## Integration: Domain ↔ Engine

The `turn.rs` module in `ofm_core` handles the conversion:

1. **`build_engine_team()`** — converts domain `Player`/`Team` objects into engine `PlayerData`/`TeamData`, mapping positions, play styles, and all 18 attributes + traits.
2. **`simulate_matchday()`** — for each fixture on a match day, builds engine teams and calls `engine::simulate()`.
3. **`apply_match_report()`** — writes results back to the domain: fixture status, match result, standings updates, player season stats (goals, assists, cards, rating, clean sheets).
4. **`apply_player_stats()`** — updates individual `PlayerSeasonStats` from the engine's `PlayerMatchStats`.

For live matches, `live_match_manager.rs` provides:
- **`create_live_match()`** — builds a `LiveMatchSession` from the current game state, splitting players into starting XI and bench based on the team's formation
- **`LiveMatchSession`** — wraps `LiveMatchState` with an RNG, AI profiles for both sides, and metadata. Provides `step()`, `step_many()`, `run_to_completion()`, `snapshot()`, and `apply_command()` methods.

---

## Test Coverage

The simulation has **69 dedicated tests**:

**Instant simulation** (`tests/simulation_tests.rs` — 36 tests):
- Types: overall rating, condition effect, position counts, rating scaling
- Zones: attacking box, attacking third, defensive third, zone advancement
- Events: builder pattern, goal detection, chronological ordering
- Simulation: deterministic seeds, varied results, goals match scores, scorer IDs
- Balance: strong team dominance, equal teams evenness, home advantage
- Play styles: possession team has more possession
- Stats: player stats populated, shots consistent, pass accuracy in range
- Edge cases: zero stoppage time, high foul probability, JSON serialization
- Realism: average goals per game in 0.5–8.0 range

**Live match** (`tests/live_match_tests.rs` — 33 tests):
- Lifecycle, phase transitions, halftime/fulltime events
- Extra time triggering, penalty shootout resolution
- Substitution mechanics (replace, max enforced, invalid player, recorded in events)
- Tactical commands (formation, play style, set pieces)
- Stamina depletion over match duration
- AI decisions (no early action, no crash, eventual substitutions)
- Score/goals matching, strong team advantage, realistic goals
- Possession percentages, chronological events, bench management, report generation
