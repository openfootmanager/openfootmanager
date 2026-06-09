# Match Commentary — Design Spec

**Date:** 2026-06-09
**Status:** Approved (design), pending implementation plan
**Author:** Brainstormed with Claude

## Goal

Give the live match simulation Bygfoot-style narrative commentary. Instead of a
terse structured event line, key moments read like a live blog:

- `GOOAL! — Haaland scored another goal!`
- `IT'S A FOUL — Marquinhos went hard on Mbappé! Foul for PSG.`
- `THAT WAS CLOSE! — Neymar's effort flashed just past the post. The keeper could only watch.`

The feature should feel alive and varied (no obvious repetition), stay truthful
to what the engine actually simulated, and remain fully localized across the
project's 9 locales.

## Scope decisions (from brainstorming)

- **Generation: Hybrid.** The engine emits a small, truthful detail field; the
  frontend turns it into prose with phrasing variety.
- **Coverage: key moments only.** Goals, penalties, big near-misses (saved /
  off-target / blocked shots), fouls, cards, injuries, substitutions, and
  structural events (kickoff / half-time / full-time). Build-up play (passes,
  dribbles, interceptions) keeps today's terse rendering or stays hidden.
- **Localization: full i18n across all 9 locales.** Variants live under a new
  `match.commentary.*` subtree; a machine-translated first pass ships so the
  enforced locale-coverage test passes. Community refines later.
- **Display: headline + sentence, keep structure.** Retain the minute / icon /
  team chrome; key-moment events gain a punchy headline plus the generated
  sentence. Non-key events render exactly as they do today.

## Non-goals

- No new physically-simulated geometry (real shot distance, post side, etc.).
  Decorative details that aren't simulated are avoided; we only narrate what the
  engine truthfully knows. Flavor comes from phrasing variety, not invented facts.
- No change to post-match reports / news copy in this pass (the `detail` field is
  available to them later, but wiring that is out of scope here).
- No audio / TTS.

## Architecture

### Data-model seam (the "hybrid" boundary)

`MatchEvent` gains a single optional, serializable `detail` field. This is purely
additive: existing consumers (`report.rs`, engine tests, the frontend `MatchEvent`
type) keep working because the field is optional / nullable.

```rust
// engine/src/event.rs
pub struct MatchEvent {
    pub minute: u8,
    pub event_type: EventType,
    pub side: Side,
    pub zone: Zone,
    pub player_id: Option<String>,
    pub secondary_player_id: Option<String>,
    pub detail: Option<EventDetail>,   // NEW
}

pub enum EventDetail {
    Shot { danger: DangerBand },     // Speculative | Decent | BigChance
    Save { quality: SaveQuality },   // Routine | Strong | WorldClass
    Foul { severity: FoulSeverity }, // Soft | Hard | Reckless
    Goal { context: GoalContext },   // Opener | Equaliser | Extends | Consolation
}
```

The enum carries only qualifiers the engine already computes, so it never lies:

- **`DangerBand`** — derived from `shoot_rating` vs `gk_rating` /
  `conversion` at the shot-resolution site (`live_match/zone_resolution.rs`
  ~lines 232–263).
- **`SaveQuality`** — derived from the same margin on the `ShotSaved` branch.
- **`FoulSeverity`** — from the foul-resolution path in
  `engine/fouls.rs` (e.g. whether it escalates to a card / injury).
- **`GoalContext`** — from the score delta at the moment of the goal
  (opener / equaliser / extends lead / consolation).

All other event types leave `detail = None`.

### Commentary module (frontend)

New file `src/components/match/commentary.ts`:

- `getCommentary(evt, snapshot, t) -> { headline: string; line: string } | null`
  — returns `null` for non-key events (caller falls back to today's terse row).
- Template registry keyed by `event_type`, refined by the `detail` variant where
  present (e.g. `Goal` × `GoalContext`, `ShotSaved` × `SaveQuality`).
- Each key maps to an i18n key whose value is an **array of 3–5 phrasing
  variants** plus a `headline`.
- **Deterministic variant selection:** `index = hash(minute, event_type,
  player_id) % variantCount`. A pure hash (no RNG, no stored state) means the
  same event always renders the same sentence — stable across re-renders,
  scroll-back, and snapshot rebuilds.
- **Derived context (no engine change needed):**
  - Scorer's running goal tally — count prior `Goal`/`PenaltyGoal` events for
    that `player_id` in the snapshot → drives "another goal", hat-trick lines.
  - Team name / opponent name from the snapshot.
  - Victim name from `secondary_player_id` (fouls, assists).

### i18n

New `match.commentary.*` subtree. Shape per key:

```json
"match": {
  "commentary": {
    "Goal": {
      "headline": "GOOAL!",
      "lines": [
        "{{player}} smashes it home for {{team}}!",
        "{{player}} scores another goal!",
        "What a finish from {{player}}!"
      ]
    },
    "Foul": {
      "headline": "IT'S A FOUL",
      "lines": [
        "{{player}} went hard on {{victim}}! Foul for {{team}}.",
        "{{player}} brings down {{victim}}. Free kick."
      ]
    }
  }
}
```

Interpolation tokens: `{{player}}`, `{{victim}}`, `{{team}}`, `{{opponent}}`,
`{{count}}`. When an event carries a `detail` variant, the module looks up the
refined sub-key first (e.g. `Goal.equaliser.lines`) and falls back to the base
key (`Goal.lines`) if no sub-key exists — so detail refinement is optional per
event type and we only author the variants that matter. Added to **all 9
locales**, machine-translated first pass so the coverage test passes.

### Display (`MatchPanels.tsx` `EventFeed`)

- Keep minute + icon + team chrome.
- If `getCommentary(evt, …)` returns non-null, render:
  - **Headline** — bold, uppercase, colored by severity/side.
  - **Sentence** — beneath, in body text, replacing the bare label+name row.
- Otherwise render today's terse label+name row unchanged.

## Testing

**Rust (engine):**
- `resolve_shot` emits the correct `DangerBand` / `SaveQuality` at rating
  extremes (low shooter vs strong keeper → `Speculative`/`WorldClass`, etc.).
- `GoalContext` reflects the score state at goal time.
- `MatchEvent` with `detail` round-trips through serde (serialize/deserialize).
- Existing engine tests still pass (additive field).

**TypeScript (frontend):**
- `commentary.test.ts`:
  - Determinism — same event object → same `{headline, line}` on repeated calls.
  - Goal-tally context — a second `Goal` for the same player yields the
    "another goal" variant family.
  - Every key event type returns a non-empty `headline` and `line`.
  - No unresolved interpolation tokens leak into output (no literal `{{player}}`).
- Existing `localeCoverage.test.ts` guards 9-language key sync.

## Risks / open questions

- **Translation quality** of the machine-translated first pass is approximate;
  acceptable per the localization decision (community refines later).
- **Variant fatigue** — 3–5 variants per type may still repeat in long matches.
  Mitigated by keying the hash on `minute` so different minutes vary; can grow
  the variant pools later without code change.
- **Goal-tally accuracy** depends on counting events present in the snapshot;
  if the snapshot ever prunes old events this would undercount. Verify the
  snapshot retains full event history (it appears to via `snapshot.events`).
