# Feature spec: Match engine calculation model

- **Status:** Draft for review
- **Target:** implement on a fresh branch off `develop`. This spec is written against the
  current engine in `src-tauri/crates/engine` (`engine/` instant simulator + `live_match/`
  interactive simulator, sharing `shared.rs`).
- **Anchors:** issue **#347** ("PlayerRole modifiers are unreachable for GK, and dead or
  trap-inverted for ~46% of outfield roles") is both a motivating symptom and an acceptance
  criterion — see [§10](#10-open-issues) and [§9](#9-validation--balancing).

---

## 1. Motivation

The match engine resolves a game as a **zone walk**: the ball lives in a zone
(`own third → midfield → attacking third → box`) and each action is a probability contest
that advances it, loses it, or ends in a shot. The contest maths is sound in outline but has
three structural problems that this spec addresses:

1. **Aggregate/individual split is unprincipled.** Some phases are player-vs-player duels
   (midfield, attacking third, shot); others are player-vs-team-average (buildup press, the
   end-of-minute possession contest). The split is incidental, not designed, so individual
   quality and tactical intent leak out in places they shouldn't.

2. **Inputs the UI advertises don't all reach the simulation.** Issue #347 is the proof:
   `role_attribute_modifier` is only dispatched in four `(Position, Phase)` combinations, so
   **9 of 26 assignable roles are purely cosmetic and 3 are trap-inverted** (only their
   penalty half fires). Player attributes have the same risk (`leadership`, `aggression` are
   likely dead), and several tactics dials are near-neutral. **The worst outcome is a UI that
   promises effects the engine never produces.**

3. **Progression and possession are the same roll.** Advancing the ball and keeping the ball
   are decided together, which both double-counts turnovers (per-action *and* the
   per-minute possession contest) and prevents modelling "keep the ball but don't progress"
   (recycling).

This spec defines a single coherent model that fixes all three, plus a validation strategy
whose explicit goal is **no dead inputs**.

## 2. Goals / non-goals

**Goals**

1. One **contest shape** used everywhere: a concrete 1v1 core, modulated by small,
   bounded team terms, with a defensive collective weight that **tapers by pitch position**.
2. A clean **two-calculation** per-minute model: *progression* (advance/stay, never loses the
   ball) is separate from *possession* (the single turnover channel, once per minute).
3. Every **player attribute, tactic setting, and player role** has a measurable,
   correctly-signed effect on the simulation — enforced by tests, not hope.
4. **Tactics are decisions the manager makes; the engine executes them faithfully and never
   auto-optimises the choice.**
5. A small set of **fundamental, trainable attributes**, with contextual skills derived as
   pairings — no specialised attributes.

**Non-goals**

- Positional (x/y coordinate) simulation. The model stays **zone-based**; player involvement
  is expressed through weighted selection, not coordinates.
- Rewriting the two engines into one. `engine/` (instant) and `live_match/` (interactive)
  stay separate but must share this model (as they share `shared.rs` today).
- Set-piece depth beyond a single dangerous-free-kick resolution and corners.

## 3. Locked design principles (from the design discussion)

- **Tactics decide, engine executes, no auto-optimisation.** A setting selects *what* the team
  does; attributes decide *how well*. The engine must never inspect outcomes to pick the
  "better" route for the player — that would make Mixed/Normal strictly dominant and hollow
  out the manager's decisions.
- **Concrete primary player on each side**, always — for the matchup (attack their weak side)
  and for narration ("X played it, Y won it back").
- **Defensive collective weight tapers toward goal.** Deep defending is a coordinated unit
  act; the decisive act near goal is individual. So the "collective coat" is heavy in
  buildup and minimal in the box.
- **Team terms are counts / receivers / weighted sums — never means over a unit.** A mean
  collapses variance and dilutes individuals; a weighted sum preserves both.
- **Shared inputs are allowed.** A factor may feed more than one calculation (pressing feeds
  both progression and turnover).
- **Fundamentals, not specialised attributes.** Everything trainable; contextual skills are
  pairings of two fundamentals.

## 4. The per-minute loop

Each simulated minute, for the team that **starts** the minute in possession:

1. **Deplete condition** once (per player in the live engine; see [§8](#8-condition--fatigue)).
2. **Tempo sets the number of progression attempts** (calc 1), drawn stochastically around
   the tempo knob:
   - Patient → ~1–2 (mean ~1.5)
   - Balanced → 1–3 (mean 2, today's behaviour)
   - Direct → ~2–4 (mean ~3)
3. Run that many **calc-1** attempts. Each advances the ball one zone or leaves it put. **A
   calc-1 attempt never loses the ball.** Possession is therefore **fixed within the minute**.
4. If no terminal/transition event fired, run **calc 2 once** at the end: keep or lose,
   with a **tempo penalty** (more attempts ⇒ more exposure ⇒ higher turnover chance).

**Possession only changes via:** calc 2 (once/minute), a **shot** (terminal — self-resolves),
or a **counter** (transition event). There are no other mid-minute flips.

### The contest shape (used by both calcs)

```
E_side = base(pairing) x trait x play_style x role x home x tactics
P      = E_att / (E_att + E_def)          # advance / keep if rand[0,1) < P
```

- `base(pairing)` — mean of the two fundamental attributes relevant to the action (see §7).
- `role` — the role modifier, now dispatched in **every** phase (fixes #347).
- The **defender side** is always `E_def = R_primary_defender x C(lambda)`, where `C` is the
  collective coat and `lambda` is the pitch-position weight below.

### The defensive gradient

| Phase | Individual / collective (indicative) |
|---|---|
| Buildup (deepest) | ~30 / 70 |
| Midfield | ~50 / 50 |
| Attacking third | ~70 / 30 |
| Box / shot | ~minimal collective; individual keeper + last-ditch block |

## 5. Phases

### Phase 1 — Buildup (own third)

Deepest phase, **heavy collective coat**.

- **Calc 1 (advance out of own third into midfield):**
  - **Ball-player** from a duty-weighted pool (keeper / centre-back / deep midfielder).
  - **Receiver** = the out-ball target, chosen by **build-up style**: Short → a midfielder
    (leans passer vision + midfielder positioning); Long → a forward (leans forward pace +
    positioning + ball control). Long **bypasses the press vertically**.
  - **Width** = horizontal press-relief: Wide adds flank outlets (play *around* a press);
    Narrow is press-vulnerable but tight in space. Wide leaves **space in behind** (counter
    risk).
  - **More men committed → fewer forward outlets → lower advance chance.**
- **Calc 2 (retention, if the minute ends here):** more men committed → **higher retention**;
  teamwork-weighted; opponent press → higher turnover; tempo penalty. (The commitment
  trade-off is intrinsic: the same lever that makes you safe makes you toothless.)
- **Defender:** one concrete presser + a **heavy** collective press coat.
- **Buildup does not trigger a counter** — losing it here is a *high press win* for the
  opponent (near your goal), not a break into space.

### Phase 2 — Midfield

**Medium coat.**

- **Calc 1 (advance into the attacking third):** a **carrier** (midfielder) vs a concrete
  midfield defender + medium coat. **Receiver** = the attacking-third outlet: **Narrow** →
  central attacker; **Wide** → winger / overlapping full-back.
- **Calc 2:** medium coat, teamwork-weighted, press + tempo penalty.
- **Home of the counter** (see §6): winning the ball here springs transitions.

### Phase 3 — Attacking third

**Light coat (~70/30 individual/collective).** Job: create a shooting position in the box.
Three calc-1 routes, chosen by tactic (never auto-optimised):

1. **Carry in** (Narrow-leaning): a forward beats his marker (dribbling/pace/agility/
   composure vs the concrete defender).
2. **Cross in** (Wide-leaning): a wide player crosses → **aerial/header sub-contest** in the
   box (target aerial + positioning vs defender aerial). Win → shooting position; lose →
   cleared.
3. **Distance shot** (shoot-on-sight tendency): a low-quality terminal shot from range — the
   answer to a deep compact block. See §7 for the precise/power profiles. Blocked/parried
   long shots bias toward **rebounds** in the box.

Who shoots from distance: **whoever is on the ball** (weighted pool — usually a midfielder or
forward, not "forwards only"), gated by the team's shoot-on-sight tendency and the player's
own `shooting + decisions`.

### Phase 4 — Box / shot (terminal event)

**Minimal coat.** Not a calc-1/calc-2 phase — it self-resolves possession immediately.

- **Gate A — on target:** finishing = `shooting + composure`, reduced by a **last-defender
  block** (individual) and keeper `positioning`.
- **Gate B — beat the keeper:** finishing vs keeper `reflexes + positioning`. Keeper
  `handling` decides the **save type**: high → catch (collects); low vs a powerful strike →
  **parry** (rebound).
- **Chance quality is inherited from Phase 3:** a clean carry-in is a high-quality chance; a
  contested header is medium; a rebound/scramble is scrappy; a distance shot carries its
  power/precise profile. Chance quality sets the base; finishing modulates it.
- **Four outcomes & minute handling:**

| Outcome | Effect |
|---|---|
| **Goal** | ends the minute (kickoff reset) |
| **Free kick / penalty** | resolved **inline**; if it yields a goal → minute ends, else play continues |
| **Keeper collects** | clean turnover → **prime counter launch** for the defending side |
| **Keeper drops / parries** | inline **rebound roll** → follow-up shot / clearance / corner |

- **Penalty:** a separate taker-vs-keeper mini-calc.
- **Only a goal resets the clock.** Every other event flows on within the minute.

## 6. Special events

### Counter (transition event)

Triggered at a turnover **in midfield, attacking third, or box** (never buildup). Modelled as
a **free zone advance** rather than through the two-calc contest, because a counter's essence
is that *there is no organised defence to beat*.

- **Transition mini-game (two dials):** the side that just lost the ball may **counter-press**
  it straight back (`counter_press_duration`), killing the counter; if it doesn't, the side
  that won it may **break** (`break_speed`).
- **Trigger is a conjunction** (rare by construction, deadly when it fires):
  1. break speed is Fast,
  2. counter-press failed,
  3. the opponent was **over-committed** (high line / many men forward / high tempo) — and the
     **deeper** the turnover, the larger the space,
  4. **pace mismatch**: the receiving forward is faster than the recovering centre-backs.
- **Own finish** (does *not* use the normal shot): forward-vs-keeper with the **defensive coat
  stripped** (the CBs are beaten by definition), **higher base conversion**, keyed on the pace
  margin. Still saveable/missable, so it isn't automatic.
- Emergent payoff: "sit deep, stay compact, punish over-commitment on the break" — the
  aggressor's own high line / high tempo / slow CBs *supply* the trigger criteria.

### Dangerous free kick

- Position-gated: only free kicks in the attacking third are a scoring threat; deeper ones
  just resume possession (≈ today's behaviour).
- Resolution: **direct shot** (shooting pairings vs keeper) **or delivery** into the box
  (→ aerial/header sub-contest). Outcome is **goal or take-over**.
- **At most one free kick per minute**; a second stoppage rolls into the next minute.

### Out of play

- **Corner** is the *only* out-of-play state with real mechanics (a retained set-piece chance).
- Everything else that "leaves the field" (throw-in, goal kick) is a **take-over with a
  narration label** — no out-of-bounds physics needed. There are no throw-ins today, and none
  are required.

## 7. Player attributes

**Philosophy: keep a small set of fundamental, trainable attributes; derive every contextual
skill as a pairing of two. Never add a specialised attribute a pairing can express** (so it
stays trainable — you train fundamentals, derived skills follow).

Confirmed pairings / renames:

| Skill in the sim | Pairing |
|---|---|
| dribbling (beat a man) | **ball control** + agility |
| first touch / receiving | **ball control** + composure (or positioning) |
| long ball | passing + vision |
| short combination | passing + decisions |
| distance shot — **precise** | shooting + composure — high accuracy, moderate conversion |
| distance shot — **power** | shooting + strength — low accuracy, high conversion (parry→rebound) |
| box finish | shooting + composure |
| keeper shot-stopping | reflexes + positioning (handling = save type) |

- **Rename `dribbling` → `ball control`** (the fundamental); dribbling/first-touch/close-control
  become derived pairings. This closes the "no ball control attribute" gap while keeping it
  trainable.
- A player's distance-shot **style** (precise vs power) is his higher of `composure` /
  `strength` (a trait may override). This characterises the *player*; it is **not**
  auto-optimising a *tactical* choice.
- **Audit for composites:** `defending` looks like `tackling + positioning + decisions` and
  should probably not be a stored fundamental. The §9 coverage test must prove every
  fundamental is in ≥1 pairing.

## 8. Tactics & selection

### Weighted selection pool (the backbone)

Replace hard position-filtering with a **weighted pool**: each player has a selection *weight*
per phase, and one is drawn in proportion. The weight combines **position fit + role/duty +
tactic (e.g. width)**. This is the single mechanism that:

- lets a forward with a **defending duty** be selected in defensive phases,
- pulls **full-backs** into the wide-outlet pool under Wide,
- dispatches **role modifiers in every phase** (including GK, buildup, press, box) — the
  direct fix for #347.

It reproduces today's behaviour as a special case (weight 1 for the preferred position, 0
otherwise), stays **zone-level (no coordinates)**, and its cost is a per-phase weight table
(bounded, interpretable).

### Route emphasis (fixed blends, tunable)

No engine adaptation. The setting sets an **emphasis**, not an absolute:

| Setting | primary | secondary |
|---|---:|---:|
| Short / Narrow | 75% | 25% |
| Mixed / Normal | 50% | 50% |
| Long / Wide | 25% | 75% |

75/25 keeps every side two-dimensional (un-exploitable) while the decision still clearly
matters. Starting default; tunable.

### Dial roles

- **Build-up style** (vertical), **width** (horizontal / press-relief), **tempo** (attempt
  frequency) are **orthogonal** buildup axes.
- **Pressing** feeds *both* calcs (fewer outlets in calc 1, more turnover in calc 2) and costs
  condition — countered by the long-ball (vertical) and Wide (horizontal) escapes and by the
  counter it exposes.
- **Break speed** / **counter-press duration** drive the counter transition (§6).

### Commitment pricing

A player committed to a phase is priced **twice**: removed from the pool of the phase he left
(opportunity cost) **and** drained a little faster (see §8 condition). Commitment bonuses are
**concave (saturating)** and **bounded** (target ≤ ~±10%), so the duel always dominates.

## 8b. Condition / fatigue

- **Team-level pressing tax is fine** (aggressive press drains the whole side faster).
- **Issue #3 — add individual workload drain:** a player given a defensive duty who tracks
  back every action should tire faster than an uninvolved team-mate. Condition then feeds
  both calcs (execution in calc 1, errors in calc 2), so an over-worked or high-tempo side
  **fades late** — enabling "absorb and outlast" as a real counter-strategy.

## 9. Validation & balancing

All tests run on **seeded Monte-Carlo batches** (e.g. seeds `0..N`) so the aggregate statistic
is **deterministic and non-flaky** while still sampling the real distribution. Tolerance guide:
the standard error of a rate is ≈ `sqrt(p(1-p)/n)`, so `n = 10_000` gives ≈ ±0.01; assert
ranges at ~±0.02, or snapshot the seeded aggregate exactly for regression.

Home: the **`sim-bench`** crate; the engine already exposes `simulate_with_rng`. `proptest`
for spread inputs.

### Tier 1 — Curated smoke tests (CI, reduced N)

Directional/metamorphic checks that the engine "makes sense": change one input, assert the
aggregate moves the right way by a meaningful margin.

1. **Overall quality** — a uniformly better team wins more (anchor).
2. **Condition** — the fresher team wins more.
3. **Home advantage** — same matchup wins more at home than away.
4. **Finishing** — higher shooting → more goals.
5. **Pressing** — aggressive press → more opponent turnovers *and* worse own late-game.
6. **High line** — → more goals *and* more counters conceded.
7. **Pace mismatch** — fast forwards vs slow CBs → more counter goals (validates §6).
8. **Tempo** — higher tempo → more shots but more turnovers and a late-game dip.

### Tier 2 — Coverage guarantee ("no dead inputs" / anti-UI-lie)

Systematic: **one sensitivity sweep (OFAT) per attribute, per tactic dial, and per player
role**, each asserting a **non-zero, correctly-signed** effect. A flat sweep = a dead input =
a failing test. This is the guarantee that the UI never advertises an effect the engine
ignores. Runs in `sim-bench` on demand / pre-release (≈ attributes + dials + 26 roles).

### Tier 3 — Targeted acceptance tests

- **#347:** every assignable `PlayerRole` (including the two GK roles) produces its **intended,
  correctly-signed** effect — no cosmetic roles, no trap-inverted roles. Concretely: a
  role's sweep must move the metric its table entry promises, in the promised direction. The
  weighted pool + full-phase role dispatch (§8) is the fix; Tier 2's role coverage is the
  regression lock.

### Caveat

These verify **properties** — reproducibility, direction, plausible ranges. They cannot
judge *fun* or fix *target numbers* (e.g. goals-per-game). Workflow: pick realistic targets by
hand → tune dials via the sweeps until aggregates hit them → lock with regression snapshots.

## 10. Open issues

Internal design issues surfaced during the discussion, and the GitHub anchor:

| # | Issue | Resolution in this spec |
|---|---|---|
| 1 | Selection pool ignores tactics/duties (position-only) | Weighted selection pool (§8) |
| 2 | Possession contest is midfield-aggregate & ball-blind | Becomes calc 2 — context-aware, once/minute (§4, §5) |
| 3 | Condition drain isn't workload-based | Individual duty drain (§8b) |
| 4 | No ball-control attribute | Rename `dribbling`→`ball control`, pairings (§7) |
| 5 | No short/long passing split | Pairings: passing+vision / passing+decisions (§7) |
| 6 | No long-shots attribute | Pairing: shooting+(strength\|composure) (§7) |
| **#347** | Role modifiers unreachable/dead/trap-inverted (9 cosmetic, 3 trap) | Weighted pool + role dispatch in every phase + Tier-2/3 coverage (§8, §9) |

Also flagged for later: audit `defending` and other composite attributes. The §9 Tier-2
coverage sweep identifies any dead inputs empirically.

## 11. Implementation note

Both `engine/resolution.rs` (instant) and `live_match/zone_resolution.rs` (interactive)
reimplement the zone logic; every change here must land in **both** to keep quick-sim and
watched-match results consistent, with `shared.rs` holding the unified modifier/pairing tables.

## 12. Fouls, cards & injuries

Re-walked under the two-calc model. There are **two foul types**, mapped by their consequence
(who ends up with the ball), not by who is nominally attacking:

### Fouls

- **Defensive foul → calc-1, a "stay" subtype.** The defender stops the attacker illegally →
  the attacker **retains** the ball + a set piece (free kick / penalty). No possession change,
  so it sits cleanly inside calc-1 (which never loses the ball). Keyed on the **defender's
  aggression + tackling**, tactics (pressing × marking), and foul traits (HotHead/CoolHead).
  This is a **re-home** of today's foul (currently gated on "lost duel → tackle branch") onto
  the calc-1 "stay" outcome — same trigger, cleaner place.
- **Offensive foul → calc-2, a "lose" subtype.** When calc-2 resolves as a turnover, a
  fraction of losses are the attacker's *own* foul (charge, backing-in, aerial arm, barging
  the keeper, handball) → the opponent gets a **free kick**, **whistle-stopped so it yields no
  counter**. Keyed on the **attacker's aggression + strength**; clusters in physical/aerial
  contexts. **New — the engine has no offensive fouls today** (the fouler is always the
  tackling defender).

So calc-1 "stay" branches into *clean stop* vs *defensive foul*; calc-2 "lose" branches into
*clean loss* (open play → possible counter) vs *offensive foul* (whistle → free kick, no
counter). The invariant is unchanged: **calc-1 never loses the ball; all turnovers flow
through calc-2; the shot is the only terminal exception.**

- **Box foul** unified onto the same foul machinery (drop today's flat outlier of 3.6% that
  ignores aggression/traits/tactics).
- **Cards and injuries follow the fouler / fouled regardless of side** — an attacker can be
  booked (elbow → yellow/red); a defender can be injured by an offensive foul.

### Cards

Current structure is mostly sound — keep it: conditional on a foul, chance modulated by the
fouler's aggression, two-stage (card? → red vs yellow), and yellow accumulation → second
yellow → sent off (removed from the selection pool). One fix: **red-card likelihood should key
on the foul's context** (last-man / denial-of-a-clear-chance / cynical), not a flat sub-roll of
any carded foul.

### Injuries — greenfield

Confirmed across **both** engines (`engine/fouls.rs`, `live_match/zone_resolution.rs`):
injuries are generated **only** in the foul path, as a flat `injury_probability` (0.03) roll on
the fouled player — no severity, no duration, no attribute input, no non-foul source. To
design:

- **Non-foul sources** — fatigue/workload (ties to §8b) is the obvious first; also hard-but-
  clean challenges and aerial landings.
- **Severity & duration** on the injury itself (a knock → a few minutes; serious → out for
  weeks).
- **Robustness / injury-proneness** expressed as a **pairing** (per §7), not a new specialised
  attribute.
