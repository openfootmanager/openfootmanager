# Tactics, Substitutions, and Squad UX Plan

## Goal

Use the referenced tactics screen as inspiration for a stronger Openfoot Manager workflow without copying it directly. The aim is to make tactics, substitutions, and squad management feel more expressive, more readable, and more match-oriented while staying appropriate for OFM's current scope.

## What The Reference Gets Right

The reference screen is useful because it combines tactical setup, player assignment, and visual formation feedback in one place.

Strong ideas worth adapting:

- Multiple saved tactics with quick switching.
- A clear distinction between predefined tactical templates and custom tactics.
- Tactical behavior split by phase: with the ball, without the ball, offensive transition, defensive transition.
- A visible "modified" state when a tactic no longer matches its base preset.
- A synchronized table + pitch view, so role changes and selection changes are instantly visible.
- Player portraits on the pitch, which improve recognition and attachment.
- Tactical roles per player, not just raw positions.
- Set pieces integrated into the same broader tactical workspace.
- Collapsible sections with one-line summaries when collapsed.
- A layout selector, implying that different managers may prefer different information density.

## What OFM Already Has

OFM already has a good base that should be evolved rather than replaced.

Current strengths in the codebase:

- Dedicated tactics screen with pitch, compare panel, formation changes, play style, drag-and-drop lineup, and separate roles tab.
- Separate squad roster view with strong player-table filtering and contextual actions.
- Pre-match lineup flow with formation, play style, set pieces, and swapping starters with bench players.
- Live substitution modal with player-off selection, bench comparison, and substitution history.
- Existing player face media support, which makes pitch portraits feasible.

Important current model limits:

- Persistent team tactics currently store `formation`, `play_style`, `starting_xi_ids`, and `match_roles`.
- There is no deeper saved tactic object yet for phase-based behavior, player roles, transitions, or multiple tactic slots.
- The engine currently understands broad play styles, but not a richer tactical instruction tree.

This means some improvements are mostly UI work, while others need new backend and simulation support.

## Opportunities The Reference Suggests Beyond The Obvious

Ideas that go beyond "add more tactic options":

### 1. Tactics As Plans, Not Just Settings

OFM currently feels closer to "pick a formation and play style." The reference points toward "build a match plan."

What to add:

- `Primary tactic`
- `Alternative tactic`
- `Protect lead`
- `Chase goal`
- `Late game chaos`

This does not need full automation on day one. Even manual quick-switch plans would already improve the UX.

### 2. Progressive Disclosure

The reference exposes depth without forcing everything to be open at once.

OFM should adopt:

- collapsed sections with summaries
- a compact layout and a detailed layout
- beginner-safe presets first, advanced tuning second

This is especially important because OFM should not become intimidating for new players.

### 3. Stronger Identity For Players On The Pitch

The pitch should feel like managing people, not tokens.

Good OFM adaptations:

- use face images when available
- otherwise fall back to initials/avatar blocks
- show OVR, fitness, and role at a glance
- show warnings directly on the pitch for low fitness, wrong position, suspension risk, or injury risk

### 4. Tactical Cause And Effect

The reference hints at what each choice means. OFM can go further by explaining consequences clearly.

Examples:

- "Short build-up: safer progression, slower territory gain."
- "High press: more recoveries high up, faster fatigue loss."
- "Counter: lower midfield control, stronger direct attacks."

This is valuable both for UX and for teaching players the simulation.

### 5. Substitutions As Scenario Management

The current sub flow is functional, but it can become much more managerial.

Potential additions:

- recommended substitutions based on fitness/form/match state
- quick sub motives: `fresh legs`, `protect yellow card`, `defensive closeout`, `need goal`
- drag swap directly from bench to pitch in live match
- small impact preview before confirming
- saved emergency patterns like "replace winger with winger" or "add striker for defender"

### 6. Tighter Squad-Tactics Connection

The squad page and tactics page should reinforce each other.

Examples:

- squad table badges for `starter`, `bench`, `tactical fit`, `out of role`
- tactical-fit view in squad management
- "best roles" and "best system fit" inside player row or profile
- one-click actions from squad to tactics: `make starter`, `set as backup`, `add to bench unit`, `mark as set-piece option`

## Recommended OFM Product Direction

The best direction is not to copy the reference screen one-to-one. OFM should lean into three traits:

- clearer than the reference
- more simulation-explanatory than the reference
- more flexible in layout than the reference

Recommended product principles:

- Keep the pitch as the emotional center.
- Keep table/detail panels for decision support.
- Prefer presets + summaries over dozens of exposed controls at once.
- Make every advanced choice explain its tradeoff.
- Let players choose their working mode: `Compact`, `Balanced`, `Detailed`.

## Proposed Information Architecture

### A. Tactics Workspace

Top bar:

- tactic slot switcher
- tactic name
- preset/custom badge
- dirty state badge
- save / save as new / reset to preset
- quick switch menu for match plans

Left rail:

- Formation
- Team style
- With ball
- Without ball
- Offensive transition
- Defensive transition
- Set pieces

Center:

- starting XI / bench list
- optional detailed table
- role assignment controls

Right:

- pitch
- player cards on pitch
- role labels
- warnings

Layout modes:

- `Pitch Focus`
- `Split View`
- `Table Focus`

### B. Squad Management

Add tactical context into the squad screen:

- tactical-fit filter
- role suitability column
- "starter / bench / reserve / youth" views
- out-of-position and low-fitness warnings tied to current tactic
- quick action to assign player into XI or bench plan

### C. Live Match / Substitutions

Upgrade the substitutions workspace into a faster decision surface:

- pitch + bench in one glance
- impact comparison
- quick reasons/actions
- auto-suggested changes
- faster formation change while substituting
- clearer sub history and remaining subs

## Suggested Feature Set By Phase

## Phase 1: High-Value UI Wins

Mostly frontend work, minimal simulation risk.

- Add tactic presets UI, even if initially mapped to current `formation + play_style`.
- Add tactic slot switcher and `save as` flow.
- Add dirty-state badge when current tactic differs from selected preset.
- Add player portraits on pitch using existing face media support.
- Add layout selector for `Pitch Focus`, `Split View`, and `Table Focus`.
- Move set pieces into the main tactics workspace instead of making them feel separate.
- Add collapsible tactical sections with summary text.
- Improve bench grouping and visual hierarchy in tactics and pre-match screens.
- Add stronger visual warnings for low fitness and out-of-position starters.

## Phase 2: Better Tactical Semantics

Requires new persisted frontend/backend model, but can still be mostly descriptive at first.

- Introduce a `TacticPreset` or `TacticPlan` model.
- Support multiple saved tactics per team.
- Add per-phase instruction groups:
  - with ball
  - without ball
  - offensive transition
  - defensive transition
- Add per-player tactical role assignments.
- Add "recommended role" hints per player based on attributes.
- Add `preset`, `custom`, and `modified from preset` states.

Suggested first model shape:

- `id`
- `name`
- `base_preset_id`
- `formation`
- `play_style`
- `phase_instructions`
- `player_roles`
- `set_piece_assignments`
- `bench_priority`

## Phase 3: Matchday Decision Layer

Bridges tactics and substitutions.

- Pre-match quick plans: `standard`, `protect lead`, `need goal`.
- Suggested bench order and role coverage warnings.
- Auto-generated substitution recommendations during match.
- One-click tactical switches in live match.
- Substitution preview showing likely gains/losses:
  - energy
  - width
  - defense
  - creativity
  - aerial threat

## Phase 4: Simulation Integration

This is the deepest layer and should come last.

- Teach the engine phase-based tactical behaviors.
- Map role choices to attribute emphasis in simulation.
- Let transition settings influence pace, territory gain, and risk.
- Let defensive block/pressing settings meaningfully change fatigue, recoveries, and chance prevention.
- Expose post-match feedback showing whether the tactic worked as intended.

## Concrete UX Improvements By Area

### Tactics

- Replace the current formation/play-style-only panel with a layered tactics builder.
- Keep beginner presets first, advanced phase editing optional.
- Show a compact summary sentence for each collapsed section.
- Let users duplicate an existing tactic before editing it.
- Add undo/reset behavior inside the tactic editor.

### Squad Management

- Add tactical-fit and role-fit columns.
- Add filters for `Best XI`, `Bench`, `Not in plans`, `Can cover role`.
- Show "best used as" in player hover cards or profile summary.
- Add clearer separation between squad planning and contract/admin actions.

### Substitutions

- Reduce modal friction with quicker pitch interaction.
- Add recommended subs list above the bench.
- Surface yellow-card and fatigue risk more strongly.
- Allow quick role or shape tweak at the same time as the change.
- Preserve a post-sub summary so the user can understand what changed.

## Implementation Notes For OFM

Existing files that are strong starting points:

- `src/components/tactics/TacticsTab.tsx`
- `src/components/tactics/TacticsSetupPanel.tsx`
- `src/components/tactics/TacticsPitch.tsx`
- `src/components/tactics/TacticsRolesPanel.tsx`
- `src/components/squad/SquadRosterView.tsx`
- `src/components/match/PreMatchSetup.tsx`
- `src/components/match/PreMatchLineup.tsx`
- `src/components/match/SubPanel.tsx`

Existing model and command touchpoints:

- `src/store/types.ts`
- `src-tauri/src/commands/squad.rs`
- `src-tauri/crates/domain/src/team.rs`

Useful existing asset support:

- `src/components/ui/PlayerAvatar.tsx`
- `src/store/types.ts` player media face support

## Interactive Pitch Branch Assessment

Branch reviewed:

- `copilot/investigate-interactive-football-pitch`

Relevant pitch commits at the top of that branch:

- `c2221ae` - refactor tactics pitch to SVG interactions
- `a05d020` - review fixes
- `bd2d7d0` - drag bookkeeping fixes
- `8542475` - shape handle polish

### What That Branch Introduced

The branch is best understood as an evolution of the current tactics pitch rather than a separate subsystem.

Main ideas introduced there:

- an SVG-based pitch surface instead of the current box-layout pitch
- pointer-driven dragging instead of native HTML drag-and-drop
- a `pitch slot` coordinate model with explicit `x/y` placement per tactical slot
- a frontend-only `shape editor` that lets the user move slot markers around
- clamped slot adjustments so players cannot be dragged outside sane pitch bounds
- a pointer ghost while dragging for clearer interaction feedback
- stronger fit-state signaling such as `Natural`, `Adapted`, and `Out of position`

### What We Can Reuse

The branch contains several genuinely useful foundations.

- The `pitch slot` coordinate model is valuable.
  It creates a better long-term base for portraits, role labels, live substitution overlays, and future tactical-shape tools.

- The SVG pitch rendering is worth keeping in mind.
  It gives OFM more control over line quality, scaling, and future overlays than the current decorative block layout.

- Pointer-driven interactions are promising.
  They are better aligned with a more custom pitch experience than native drag-and-drop, especially if we later support touch and more precise slot manipulation.

- Slot adjustment clamping is good product thinking.
  It avoids chaotic layouts and would help any future custom-shape or tactical-role visualization.

- The branch proves that OFM can support a more dynamic pitch without changing the entire tactics screen architecture.

### What Needs Improvement

The current branch direction should not be merged as-is.

- The shape editor is disconnected from meaningful tactical decisions.
  It lets users move dots around, but those movements are not yet clearly tied to tactical concepts, saved plans, or simulation outcomes.

- The feature is session-only and frontend-only.
  That makes it feel experimental rather than managerial, because the user invests effort into a shape that is not a real team plan.

- The interaction model overloads the pitch.
  Selection, comparison, dragging, and slot editing all compete in the same space, which hurts clarity.

- The fit labels are informative but visually noisy.
  Showing `Natural`, `Adapted`, and `Out of position` on every player can quickly clutter the pitch.

- It does not yet leverage OFM's player identity strengths.
  The branch still relies on rating circles and text blocks rather than properly integrating player portraits and richer status cues.

- It risks becoming a toy if the editor allows free movement without football logic.
  The value should come from tactical intent, not just from manually reshaping a formation graphic.

### Recommended Reframe

The branch should be reframed from:

- `freeform interactive pitch editor`

into:

- `guided tactical shape adjustment inside saved tactics`

That means:

- custom shape should belong to a tactic plan, not a temporary session widget
- editing should happen in a dedicated mode
- changes should be bounded by football logic and clear labels
- the shape should support tactical phases and match plans later

### How To Combine This With The Main Plan

Best integration path:

#### Phase 1A

- Adopt the SVG pitch direction if it improves clarity and responsiveness.
- Reuse the coordinate-slot model behind the scenes.
- Keep current lineup interactions simple while the rest of the tactics UX evolves.

#### Phase 1B

- Add player portraits, role labels, and clearer status overlays on top of that slot model.
- Use the new pitch as the visual center of the tactics workspace.

#### Phase 2

- Convert `shape editor` into a guided `team shape` tool attached to saved tactic plans.
- Make it explicit that shape changes affect:
  - line height
  - team width
  - compactness
  - support distances

#### Phase 3

- Let different match plans use different saved shapes:
  - standard
  - protect lead
  - chase goal
  - high press

#### Phase 4

- Only then connect those shapes to actual simulation behavior.

### Specific Recommendations For That Branch

- Keep:
  - SVG pitch rendering direction
  - slot coordinate model
  - pointer-drag foundation
  - slot clamping logic

- Redesign:
  - freeform shape editing UX
  - visual density on player markers
  - relationship between editing and player selection
  - persistence model

- Add before reuse:
  - portraits
  - clearer edit/view mode separation
  - saveable tactic-plan integration
  - tactical explanation text
  - mobile and accessibility review

### Final Verdict

That branch is a useful prototype, not a merge-ready solution.

Its real value is that it already explored the hardest presentational problem:

- how to make the tactics pitch feel dynamic rather than static

We should absolutely build on that idea, but channel it into:

- `saved tactic plans`
- `guided shape adjustment`
- `portrait-led pitch visualization`
- `faster substitution and lineup workflows`

## Recommended Build Order

1. Create a UX spec for the new tactics workspace.
2. Ship Phase 1 visual and workflow improvements without changing simulation depth.
3. Add a persistent tactics model with presets and multiple saved plans.
4. Integrate those plans into pre-match and live-match flows.
5. Expand engine behavior only after the UI language and data model are stable.

## Risks To Avoid

- Adding too many tactical knobs before the simulation can justify them.
- Copying another game's layout too literally instead of fitting OFM's identity.
- Overloading beginners with expert controls immediately.
- Building complex tactic persistence before deciding how it appears in live match.
- Hiding key tactical consequences behind vague football jargon.

## Best First Slice

If OFM wants one strong first move, it should be:

`Saved tactic presets + phase sections + pitch portraits + layout modes`

That bundle would make the screen feel dramatically more modern and strategic without requiring the full simulation overhaul yet.
