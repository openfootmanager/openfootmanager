---
name: new-ui-surface
description: Build a new frontend component, panel, dashboard tab, or screen that matches the Matchday design language, works in light and dark, is keyboard and screen-reader accessible, reuses existing primitives, and ships with a Testing Library test.
when_to_use: Creating any new React component, adding a dashboard tab, building a modal or panel, redesigning an existing screen, or when a UI change needs to look and behave like the rest of the app.
argument-hint: "[what you are building]"
allowed-tools: Read, Edit, Write, Grep, Glob, Bash(npx vitest run src/components*), Bash(npx tsc --noEmit)
---

# Building a new UI surface

The app has one visual voice — "Matchday", a broadcast-graphics look. New surfaces should be
indistinguishable from existing ones. Reuse first, then compose, then style.

## 1. Reuse before you build

Read `src/components/ui/index.ts` before writing markup. It already exports:

`Card` / `CardHeader` / `CardBody` · `Button` · `Badge` · `ProgressBar` · `Select` · `Checkbox` ·
`DatePicker` · `CountryFlag` · `TeamLocation` · `PlayerAvatar` · `TeamLogo` · `InjuryBadge` ·
`JerseyIcon` · `ThemeToggle` · `PitchToken` · `AssetImage` · `GeneratedAvatar` · `GeneratedCrest` ·
`CountryCombobox`

Also check before writing a helper:

- `src/lib/` — formatting and domain helpers (`finance.ts`, `playerSquad.ts`, `playerRoles.ts`,
  `countries.ts`, `dateFormatting.ts`, `seasonContext.ts`, `playerOvr.ts`, …)
- `src/hooks/` — `useAdvanceTime`, `useFetchedSquad`, `useUndoRedo`, `useAssetDataUrl`, …
- `src/services/` — every backend call. Components never call `invoke()` directly.

A new component belongs in `src/components/ui/` **only** if a second feature will use it.
Otherwise put it in the feature folder (`src/components/squad/`, `src/components/transfers/`, …).

## 2. Design tokens

Colours and fonts come from the `@theme` block in `src/App.css`. Nothing else defines them.

| Use | Token |
|---|---|
| Primary action, positive state | `primary-500…900` (emerald) |
| Highlight, award, emphasis | `accent-400` (`#ffd60a` gold), `accent-500…900` |
| Success | `success-400/500/600` |
| Dark surfaces | `navy-900` (deepest) → `navy-800` → `navy-700` → `navy-600` |
| Headings | `font-heading` (Barlow Condensed), usually `uppercase tracking-wider` |
| Body | `font-sans` (Inter) |

- **Never** a hex literal or an arbitrary value (`bg-[#10b981]`) in a component.
- **Every** colour class needs its `dark:` partner. Light and dark both ship.
- Light surfaces are `white` / `gray-100`; their dark counterparts are `navy-800` / `navy-900`.

`src/components/ui/Button.tsx` is the reference for how variants, sizes, and states are composed —
read it before inventing a new pattern.

## 3. Accessibility floor

Not optional, and cheap if done while writing rather than after.

- **Focus is visible.** Every interactive element: `focus:outline-none focus:ring-2
  focus:ring-offset-2 focus:ring-<token>` plus `dark:focus:ring-offset-navy-800`. Removing an
  outline without replacing it is a bug.
- **Semantic elements.** `<button>` for actions, `<a>` for navigation, `<table>` for tabular data,
  headings in order. A `div` with `onClick` is not keyboard reachable.
- **Icon-only controls get an accessible name** — `aria-label={t("…")}`, a translated string.
- **Modals** trap focus, close on `Escape`, restore focus to the trigger, and are labelled
  (`role="dialog"` + `aria-labelledby`).
- **Lists and tables** that sort or filter announce their state (`aria-sort`, `aria-live` for
  result counts).
- **Colour is never the only signal.** Pair it with a label, icon, or shape — this matters for the
  form/condition/morale indicators especially.
- **Contrast holds in both themes.** Gold on white and mid-emerald on navy are the usual failures.
- **Respect `prefers-reduced-motion`** for anything that animates on its own.

## 4. Strings

Every visible string — and every `aria-label`, `title`, `placeholder`, and `alt` — is a
translation key in all 11 locales. Use `/add-ui-string`; don't hand-roll it.

## 5. State

- Zustand stores in `src/store/`. **Never mutate store state from a component** — copy, then set.
  (`fix/hometab-store-mutation` is the regression that made this a rule.)
- Backend data flows through `src/services/*Service.ts`.
- Derive, don't duplicate. If a value can be computed from the store, compute it.

## 6. Test it

Co-locate `Foo.test.tsx` next to `Foo.tsx`. Write it first.

```ts
// Query by role and accessible name. If this line fails, the component
// is not accessible — the test is doing double duty.
const save = screen.getByRole("button", { name: /save/i });
```

- Query by role/label, never by class or test id.
- Pure logic goes in a `*.helpers.ts` file and gets unit-tested directly —
  `src/components/squad/SquadTab.helpers.ts` is the pattern.
- Cover the keyboard path, not just the click path, for anything interactive.

```bash
npx vitest run src/components/<area>
npx tsc --noEmit
```

## 7. Invariants, if you touch squad, tactics, or the pitch

- **XI slot alignment** — starting-XI array entry `i` *is* formation slot `i`. Substitutions
  insert at the **vacated index**; never append or re-sort. Slot geometry comes from
  `buildPitchRows(formation)` in `src/components/squad/SquadTab.helpers.ts`, and `PitchToken` is
  the shared token component.
- **Deployed vs natural position** — `player.position` is the natural position and is never
  mutated. Where a player is currently deployed comes from `getDeployedPosition(team, slot)`.
  Role pickers and validators use the deployed slot; player descriptions use the natural position.

## Checklist

- [ ] Searched `src/components/ui/index.ts`, `src/lib/`, `src/hooks/` before writing anything new
- [ ] Token classes only — no hex literals, no arbitrary values
- [ ] Every colour class has a `dark:` partner; checked in both themes
- [ ] Visible focus ring on every interactive element
- [ ] Semantic elements; icon-only controls have translated `aria-label`s
- [ ] All strings routed through `/add-ui-string` (all 11 locales)
- [ ] No store mutation from a component
- [ ] Co-located test querying by role, written before the component
- [ ] `npx vitest run src/components/<area>` and `npx tsc --noEmit` green
