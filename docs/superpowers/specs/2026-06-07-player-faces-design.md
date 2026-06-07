# Player Profile Faces — Design

**Date:** 2026-06-07
**Status:** Approved (pending spec review)
**Author:** Jeremy + Claude

## Summary

Give every player a distinctive cartoon profile picture, generated deterministically
from the player's id using [faces.js](https://github.com/zengm-games/facesjs) (the same
library ZenGM uses for Basketball GM / Football GM). Faces are computed on the fly in the
React layer — **no images are stored, no save-format change, and the Rust engine is
untouched.** A face is a pure derived view of `player.id`, exactly like the existing
`ovr` rating.

## Goals

- Each player has a stable, recognizable face that never changes across sessions.
- Works for every existing save instantly (faces derive from `player.id`, which already exists).
- Zero binary assets shipped per player; minimal bundle cost.
- Faces appear on the player profile, the players list, and the squad roster.

## Non-Goals (YAGNI)

- Stored, editable, or custom/imported faces; a "regenerate face" button.
- Nationality- or age-based realism (ethnicity/greying/balding/facial-hair biasing).
- Faces for staff, managers, or youth-specific styling.
- Faces on transfer, scouting, match-lineup, or other secondary screens.

The seed-based approach leaves the door open to add an optional stored-override face field
later (`storedFace ?? generateFromSeed(id)`) with no rework, if editability is ever wanted.

## Key Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Art style | faces.js cartoon SVG | Built for sports sims; polished; face-as-object (tiny data, computed image). |
| Determinism | Seed from `player.id` | No storage, no DB migration, works on all existing saves, engine untouched. Fits the codebase's "everything derives from a seed" philosophy. |
| Biasing | `gender: "male"` only | Fully deterministic; no nationality→race mapping table to maintain; avoids baking ethnic assumptions into the game. |
| Scope | Profile hero + players list + squad roster | Immersive where players are browsed, without touching every surface. |
| Library version | Pin `facesjs@5.0.3` exactly | Generated faces must never silently shift on a dependency upgrade. |

## Architecture

A single new module owns everything face-related. The rest of the app only renders a React
component and never imports faces.js directly.

```
player.id ──► getFace(id)  [src/lib/playerFace.ts]
                  │  seeded PRNG ⟶ faces.js generate({ gender: "male" })
                  │  memoized in Map<string, FaceConfig>
                  ▼
            FaceConfig (plain object)
                  │
                  ▼
        <PlayerFace playerId size />  [src/components/PlayerFace.tsx]
                  │  faces.js <Face face={...} lazy />  ⟶ SVG
                  ▼
   Hero card · Players list row · Squad roster row
```

### Why faces.js is NOT seed-aware, and how we handle it

faces.js `generate()` uses `Math.random` internally and exposes no seed parameter. To make
a face a pure function of `player.id`, `getFace` temporarily swaps the global `Math.random`
for a seeded PRNG, calls `generate`, and restores the real `Math.random` in a `finally`
block. This is safe because `generate` is synchronous and JavaScript is single-threaded —
no other code runs between the swap and the restore.

This RNG override is the one "hack" in the design and is fully contained in `playerFace.ts`.

## Components

### `src/lib/playerFace.ts` (new)

The only module that imports faces.js generation. Responsibilities:

- `getFace(playerId: string): FaceConfig`
  1. Hash `playerId` to a 32-bit seed (e.g. `cyrb53` → `mulberry32`).
  2. Check the in-module memo `Map<string, FaceConfig>`; return cached if present.
  3. Save `Math.random`; assign the seeded PRNG to `Math.random`.
  4. `const face = generate(undefined, { gender: "male" })`.
  5. Restore `Math.random` in `finally`.
  6. Store in the memo and return.
- A small, self-contained seeded-PRNG helper (no new dependency).

**Interface contract:** input = a player id string; output = a faces.js `FaceConfig`
object; depends only on faces.js and a local hash/PRNG. Same id always yields a deep-equal
object; the global `Math.random` is unchanged after the call (even on throw).

### `src/components/PlayerFace.tsx` (new)

Thin presentational wrapper. Props: `{ playerId: string; size: number; className?: string }`.
Calls `getFace(playerId)` and renders faces.js `<Face face={face} lazy />` inside a
sized, `rounded`/`rounded-full` container. Wrapped in `React.memo` keyed on `playerId` + `size`.

### `PlayerProfileHeroCard.tsx` (modified)

Add a `w-24 h-24` `<PlayerFace>` portrait to the left of the existing OVR box
(`src/components/playerProfile/PlayerProfileHeroCard.tsx:67`). The OVR remains, shown as a
badge beside/over the portrait rather than as the standalone large block.

### `PlayersListTab.tsx` (modified)

Add a new leading avatar `<td>` (~32px `<PlayerFace>`) before the name column
(`src/components/players/PlayersListTab.tsx:465`), plus a matching header cell in the table
head row.

### `SquadRosterView.tsx` (modified)

Add the same ~32px `<PlayerFace>` thumbnail to each player row.

## Performance

- **Memoization:** `getFace`'s module-level `Map` ensures each id is generated at most once
  per session; subsequent renders reuse the cached `FaceConfig`.
- **Lazy rendering:** faces.js `<Face lazy>` defers rendering off-screen SVGs in long lists.
- **Component memo:** `PlayerFace` is `React.memo`'d so unrelated parent re-renders don't
  re-render faces.

A players list or squad can show dozens of faces at once; these three measures keep that
cheap. If profiling later shows the SVG DOM is still heavy in very long lists, a follow-up
could rasterize list thumbnails — out of scope for this pass.

## Dependencies & Licensing

- Add `facesjs@5.0.3` (pinned, exact). License: **Apache-2.0**, which is one-way compatible
  with OpenFoot's **GPLv3** — it may be included, with no in-UI attribution requirement.
- No new runtime dependency for hashing/PRNG (small local helper).

## Testing

### `src/lib/playerFace.test.ts` (new)
- **Determinism:** `getFace(id)` called twice returns deep-equal objects.
- **Distinctness:** different ids produce different faces (spot-check several).
- **RNG restoration:** `Math.random` is the original reference after `getFace`, including
  when `generate` throws (mock it to throw and assert restoration in `finally`).

### Component tests
- `PlayerFace.test.tsx` (new): renders an `<svg>` for a given id; same id → stable output.
- `PlayerProfileHeroCard` / `PlayersListTab` existing tests: updated to expect the new
  avatar element alongside name/OVR.

## Rollout

Single PR (or a small stack): add dependency + `playerFace.ts` + tests first, then
`PlayerFace.tsx`, then the three UI integrations. No migration, no feature flag — purely
additive and cosmetic.

## Open Questions

None blocking. Future optional extension: stored-override faces for editability.
