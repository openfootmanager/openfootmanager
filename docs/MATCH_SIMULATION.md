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

The engine uses 19 player attributes, grouped into categories:

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

The weekly AI tactical review in `ofm_core::ai_tactics` treats at least 3.2
goals conceded per game as leaky and at most 1.4 scored as blunt. Those cutoffs
were calibrated when the generated-world probe measured about **2.29 goals per
club per game** after #605 changed the selected eleven. A scoring change needs
a fresh run of `tactical_adaptation_probe` before those cutoffs can be trusted.

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

The AI manager (`ai.rs`) controls non-player sides during live matches. It is consulted once a
minute and can issue substitution and tactical commands. It reads the match through
`AiObservation` — a borrowed, side-relative view built by `LiveMatchState::observe()`, not a
`MatchSnapshot`.

### AI Profile

Each AI manager has:
- **Reputation** (0–1000) — higher = more sophisticated decisions
- **Experience** (0–100) — decides which checkpoints he works to, how early he replaces a spent
  player, and how often he gets a close call wrong
- **Personality** — `Pragmatist` waits for the scheduled moments; `Visionary` reaches for a
  different shape before a different label and looks up when a goal goes against him; `Reactive`
  takes stock every time the score changes

### When the manager looks up

Reactions are **not** rolled for each minute. A manager takes stock at fixed moments and
immediately when the match changes under him:

| Moment | Who |
|---|---|
| Half-time, and the interval in extra time | every manager |
| 80' | every manager |
| 70' | experience ≥ 40 |
| 60' | experience ≥ 70 |
| 113' | every manager — the last look before penalties |
| A sending-off, either side | every manager, that minute |
| A goal, either way | `Reactive` always; `Visionary` when it went against him |
| Nobody left in goal | every manager, every minute, ahead of everything else |

Exhaustion is judged **every** minute rather than at a checkpoint: a spent player is not a
judgement call, and the condition economy is calibrated on that branch firing the minute a
starter crosses the line.

### What he decides

**Substitutions** (one per evaluation, in this order):
1. No available goalkeeper on the pitch — a bench keeper comes on for the most spent outfielder,
   forwards first. He keeps `Position::Goalkeeper` rather than inheriting the vacated slot.
2. Any starter below the fatigue threshold (55 − experience×10 from 75', 45 − experience×8 from
   60', otherwise 35) — replaced in kind.
3. Chasing — two goals down at any checkpoint, one goal down from 60'. A forward comes on for the
   most spent defender or midfielder, never breaking up a back four below three.
4. Protecting a lead from 80' — a defender comes on for the most spent forward, always leaving one
   up front.

Who comes off is whoever has least left to give. Two players within 10 condition points look
identical from the touchline, and a manager takes the wrong one off with probability
(1 − experience/100)/2 — the in-match twin of the lineup picker's misjudgement. It never decides
*whether* he acts.

**Tactical changes** (one per evaluation):
- A `Visionary` who is losing after 60' changes shape first: 4-4-2 → 4-3-3 → 4-2-3-1. The chain
  ends, so this fires at most twice.
- Otherwise a target play style: Attacking two goals down, or one goal down from 60' unless already
  HighPress; Defensive with a lead from 80', or pinned in one's own half for 7 of the last 10
  minutes with nothing to chase.
- If the style is already right, one dial underneath it (`ChangeTacticalDial`). Chasing: a higher
  line, then a faster break, then a harder press. Protecting: a lower line, then a compact shape,
  then a passive press — subject to the same ration on under-priced dials that `ai_tactics` applies
  between matches, at most two per side.

**Nothing here ever issues a command to undo an earlier one.** A position that has stopped calling
for a change produces no target rather than the opposite one, so a side that dropped deep under
pressure stays deep when the pressure lifts. That is the hysteresis.

All five substitutions are available to every branch; `max_subs` is the only limit.

---

## Integration: Domain ↔ Engine

The `ofm_core/turn/` bridge is the only place the conversion is allowed to live —
`engine` never imports `domain`, so every domain type is translated here.

1. **`turn/squad.rs`** — the single domain→engine squad builder, shared by both match
   paths. `build_team_with_bench()` picks eleven slot-aligned starters from the club's fit
   players and returns the bench separately, so only actual participants are handed to the
   engine. Who picks them depends on whose club it is: the user's own club goes through
   `select_starting_xi()`, which honours the saved XI and rebuilds one only when fewer than
   eight saved starters are still available; every other club goes through
   `ai_select_starting_xi()`, a reputation-driven policy that rotates for freshness. A club
   with too few fit players to fill the formation makes the shortfall up from its injured
   list rather than fielding a short side — the engine has no forfeit, and an empty side
   would crash it. The builder also maps positions, play styles, roles, the nine tactical
   dials, and all 19 attributes + traits.
2. **`simulate_matchday()`** — for each fixture on a match day, builds both squads through
   `turn/squad.rs` and calls `engine::simulate()`. The bench is discarded on this path:
   `simulate()` is one-shot with no command loop, so an instant match has no substitutions.
3. **`apply_match_report()`** — writes results back to the domain: fixture status, match result, standings updates, player season stats (goals, assists, cards, rating, clean sheets).
4. **`apply_player_stats()`** — updates individual `PlayerSeasonStats` from the engine's `PlayerMatchStats`.

For live matches, `live_match_manager.rs` provides:
- **`create_live_match()`** — builds a `LiveMatchSession` from the current game state, using the same `turn/squad.rs` builder and keeping the bench for substitutions
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
