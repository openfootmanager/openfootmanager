# Tactics Workspace Redesign

## Why We Are Resetting

The current tactics screen improved some capabilities, but it still feels like:

- a roster screen with a pitch attached
- a dense strategy panel next to an overloaded canvas
- a prototype for interactions rather than a coach-friendly workflow

The main pain points in the current direction:

- The player cards on the pitch are too large, too repetitive, and too hard to scan.
- Portraits are not adding value yet. They read as low-quality decoration, not identity.
- The pitch is trying to support selection, comparison, drag-and-drop, fit labels, and tactical explanation all at once.
- Preset tactics are presented as cards, which takes too much space and makes switching slower than it should be.
- Formation, play style, and tactical intent are still too tightly coupled.
- There is no strong concept of a user-owned tactic, only a current setup that happens to resemble a preset.
- The current "analysis" experience is not clearly separated from the "arrange my team" experience.

This redesign should treat tactics as a first-class workspace, not a supporting tab.

## Product Goal

Create a tactics workspace that helps the player do three jobs quickly:

1. Choose or build a tactic.
2. Arrange players and roles clearly on the pitch.
3. Understand how the team behaves in each phase of play.

The screen should feel:

- faster to read
- easier to manipulate
- more football-native
- less visually noisy
- more compatible with future simulation depth

## Design Principles

### 1. The Pitch Is The Center, Not The Whole Product

The pitch should be the emotional center of the screen, but not the only place where decisions happen.

The pitch should answer:

- who is playing
- where they are
- what role they have
- whether anything is wrong

It should not try to contain every tactical explanation in the player token itself.

### 2. Default To Coaching, Not Analysis

The primary mode should help the player coach:

- pick a tactic
- move players
- assign roles
- review warnings

Analysis should exist, but as a separate mode or panel, not as permanent clutter.

### 3. Separate Interactions By Intent

The same surface should not behave as:

- drag canvas
- compare tool
- role editor
- phase editor
- warning legend

all at the same time.

We need explicit modes or sub-states.

### 4. Presets Should Be Fast, Custom Tactics Should Be First-Class

Presets should be a quick starting point.

Custom tactics should support:

- create
- duplicate
- rename
- save
- reset to base preset
- mark as modified

The user should feel they own a library of tactics, not a single mutable setup.

### 5. Roles And Phases Must Be Visible In The Model

The redesign should plan for:

- base shape
- in-possession shape
- out-of-possession shape
- attacking transition
- defensive transition
- player role per phase where relevant

Even if the engine does not simulate all of this immediately, the UI and data model should stop assuming tactics are only `formation + play_style`.

## Current UI Diagnosis

## What To Keep

- Dedicated tactics workspace
- Existing formation support
- Existing play style support as a temporary bridge
- Existing starting XI persistence
- Existing player face media pipeline as optional asset support
- Existing branch work on SVG pitch and pointer-driven interaction

## What To Replace

- Large pitch cards
- Permanent compare-first layout
- Preset cards as the main selection method
- Fit labels repeated on every player token
- Current layout mode framing
- Current coupling of tactics setup and player arrangement

## What To Reframe

- The interactive pitch branch should become a guided shape editor, not a freeform toy.
- Portraits should become optional identity accents, not the main payload of the token.
- Role fit should become contextual feedback, not permanent visual spam.

## North Star Information Architecture

### Top Bar

The top bar becomes the tactic command bar.

Content:

- tactic combobox
- create new tactic
- duplicate tactic
- rename tactic
- save
- save as new
- reset to preset
- dirty state badge
- preset/custom badge

Combobox behavior:

- searchable
- grouped into `Presets` and `My tactics`
- supports quick switching
- selecting a preset creates a working copy unless the user explicitly edits an existing saved tactic

Example entries:

- `Preset: Balanced Control`
- `Preset: High Press`
- `My tactic: 4-3-3 Home Press`
- `My tactic: Protect Lead`

### Phase Navigation

Directly under the top bar, add phase tabs:

- Shape
- With Ball
- Without Ball
- Transition To Attack
- Transition To Defend
- Set Pieces & Roles

These phases should not all expose the same controls.

### Main Workspace Layout

Default desktop layout:

- left rail: tactic settings for the active phase
- center: pitch and bench tray
- right panel: player or tactic inspector

Mobile/tablet layout:

- pitch first
- bench as bottom drawer
- settings and inspector as side sheets / stacked panels

## Recommended Modes

The workspace should support three clear modes.

### 1. Coach View

Default mode.

Best for:

- picking tactic
- swapping players
- assigning roles
- checking warnings

Visible:

- pitch
- bench tray
- compact settings rail
- inspector only when something is selected

### 2. Build View

Best for deeper setup.

Visible:

- phase controls expanded
- role and instruction editing
- pitch remains central

Use this when editing:

- role behavior
- phase instructions
- shape adjustments

### 3. Analysis View

Best for explanation and validation.

Visible:

- tactical summary
- squad fit
- weak spots
- role coverage
- style fit
- likely tradeoffs

This should not be the default.

## Pitch Redesign

## Core Change

Stop treating pitch players as mini profile cards.

Replace them with compact tactical tokens.

## New Player Token Structure

Default token:

- shirt circle or clean marker
- short name
- role abbreviation
- tiny status line or bar

Optional secondary info:

- OVR
- condition
- warning dot

Portrait behavior:

- only use portraits if they are high quality and readable at small sizes
- otherwise use shirt, initials, or a team-style silhouette
- portraits should be a theme layer, not required for clarity

Recommended token states:

- default
- selected
- compare target
- dragged
- warning
- out of role

Recommended warning indicators:

- amber corner dot for adapted fit
- red corner dot for out of role
- heart / stamina strip for low condition
- injury / suspension icon when relevant

Do not display large fit badges on every token by default.

## Pitch Interaction Model

### Default Interaction

Single click:

- select player
- open inspector on the right

Double click:

- quick swap mode or role picker

Drag:

- move player to another slot
- swap if slot occupied
- highlight valid targets

### Bench Interaction

Bench should live in a horizontal tray below the pitch.

Each bench token should be compact and sortable by:

- role
- fitness
- OVR
- tactical fit

Drag from bench to pitch:

- replaces occupant
- previews the tactical consequence before drop confirm if useful

### Edit Modes On Pitch

The pitch needs explicit edit modes:

- `Lineup`: swap players only
- `Roles`: assign role behavior within slots
- `Shape`: adjust line height, width, compactness, support lanes
- `Phase`: preview how shape changes with ball / without ball

This avoids the current overloaded interaction model.

## Shape Editing

The interactive pitch branch should be reused carefully here.

### Keep

- SVG pitch rendering
- slot coordinate model
- pointer-driven dragging
- clamping rules

### Change

- no unrestricted freeform dragging
- no session-only experimental behavior
- no unlabeled shape handles

### New Shape Model

Instead of moving every player freely, expose guided shape controls:

- defensive line height
- attacking line height
- team width
- compactness
- fullback aggression
- winger width
- striker support distance

Advanced mode can expose position anchors, but only within bounded lanes.

## Roles System

We should explicitly separate:

- position
- role
- duty or emphasis

Example:

- position: `ST`
- role: `False 9`
- duty: `Support`

Or:

- position: `RB`
- role: `Inverted Fullback`
- duty: `Defend`

## Recommended Role Framework

For the first full redesign, support role families by line:

Goalkeeper:

- Goalkeeper
- Sweeper Keeper

Defenders:

- Central Defender
- Ball Playing Defender
- No Nonsense Defender
- Fullback
- Wingback
- Inverted Fullback

Midfielders:

- Anchor
- Deep Lying Playmaker
- Box To Box Midfielder
- Advanced Playmaker
- Ball Winner
- Mezzala
- Wide Midfielder

Attackers:

- Winger
- Inverted Winger
- Inside Forward
- Target Forward
- Advanced Forward
- Poacher
- False 9
- Pressing Forward

The UI does not need every role on day one, but the system should support them cleanly.

## Instructions Model

We should move from one broad play style to layered instructions.

### Base Identity

Keep a high-level identity as the tactic summary:

- Balanced
- Possession
- Counter
- High Press
- Defensive
- Attacking

### Under That, Add Phase Instructions

With Ball:

- build-up style
- width
- tempo
- final-third focus
- overlap / underlap emphasis

Without Ball:

- block height
- pressing intensity
- compactness
- trigger preference

Transition To Attack:

- counter quickly
- hold shape
- target flanks
- target striker feet / space

Transition To Defend:

- counter-press
- regroup
- force wide
- protect center

These should create the tactic's identity more than formation alone.

## Recommended Component Architecture

The redesign should likely split the current tactics surface into:

- `TacticsWorkspaceShell`
- `TacticSelectorBar`
- `TacticPhaseTabs`
- `TacticsSettingsRail`
- `TacticsPitchCanvas`
- `PlayerToken`
- `BenchTray`
- `TacticInspectorPanel`
- `RolePicker`
- `ShapeEditorOverlay`
- `TacticSummaryStrip`

Current files that can feed this:

- `src/components/tactics/TacticsTab.tsx`
- `src/components/tactics/TacticsPitch.tsx`
- `src/components/tactics/TacticsSetupPanel.tsx`
- `src/components/tactics/TacticsRolesPanel.tsx`
- `src/components/tactics/TacticsTab.helpers.ts`

## Data Model Direction

We need a true tactic object.

Suggested shape:

- `id`
- `name`
- `source_preset_id`
- `is_custom`
- `formation`
- `base_shape`
- `in_possession_shape`
- `out_of_possession_shape`
- `play_style_summary`
- `phase_instructions`
- `player_role_assignments`
- `set_piece_assignments`
- `bench_preferences`
- `last_modified_at`

Recommended sub-models:

- `shape_anchor_by_slot`
- `role_by_slot`
- `role_by_phase` where needed later
- `dirty_from_source`

## UX Rules For Presets And Custom Tactics

### Presets

Presets are templates.

User can:

- preview
- apply
- duplicate into custom tactic

Editing a preset directly should not be the default mental model.

### Custom Tactics

Custom tactics must support:

- saved name
- editable shape
- editable phase instructions
- editable roles
- visible "based on" preset reference

### Dirty State

If the user changes anything after loading a saved tactic:

- show unsaved state
- allow save
- allow save as copy
- allow revert

## Recommended Build Phases

## Phase 0: UX Spec And Wireframe

Before more implementation:

- settle information architecture
- settle token design
- settle interaction rules
- settle tactic object model

Deliverables:

- low-fidelity layout spec
- interaction flow notes
- state model notes

## Phase 1: Visual And Workflow Reset

Frontend-heavy, minimal simulation change.

- replace preset cards with combobox
- add tactic actions: new, duplicate, save, rename
- replace pitch cards with compact tokens
- move bench into dedicated tray
- simplify right panel into inspector
- separate Coach / Build / Analysis modes

## Phase 2: Guided Pitch Interaction

- adopt SVG pitch and pointer interaction from the branch
- add robust swap and drag behavior
- add explicit lineup / roles / shape modes
- add guided shape controls instead of freeform chaos

## Phase 3: Real Tactic Model

- persist custom tactics
- persist base and phase settings
- persist role assignments
- persist shape anchors

## Phase 4: Matchday Integration

- use tactic selection in pre-match
- quick switch during live match
- substitution suggestions tied to tactic intent
- bench ordering by tactical coverage

## Phase 5: Simulation Integration

- connect phase instructions to match engine behavior
- connect role assignments to attribute weighting
- expose post-match explanation of tactical outcomes

## Immediate Recommendations

If we restart the tactics redesign now, the best near-term decisions are:

1. Stop investing further in the current large pitch-card presentation.
2. Replace preset tactic cards with a searchable combobox plus saved-tactic actions.
3. Redesign the pitch around compact tactical tokens instead of profile cards.
4. Reuse the interactive pitch branch only for SVG, coordinates, and pointer drag.
5. Introduce explicit modes for `Lineup`, `Roles`, and `Shape`.
6. Plan the data model around saved custom tactics before adding more surface controls.

## Proposed First Implementation Slice

The first slice of the redesign should be:

- new tactic selector bar
- custom tactic CRUD scaffolding
- new pitch token design
- bench tray redesign
- simplified right-side inspector

This slice will immediately improve usability and visual quality without forcing the full phase model on day one.
