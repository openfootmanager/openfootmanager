# Frontend guidance — `src/`

React 19 + TypeScript + Tailwind v4, talking to the Rust backend over Tauri `invoke()`.
Read [`../CLAUDE.md`](../CLAUDE.md) first for the project-wide rules.

---

## 1. Internationalisation — the rule that breaks CI most often

**Every string a player can read must exist in all 11 locales.** No exceptions, no "I'll add the
translations later."

- Source of truth for the locale list: `SUPPORTED_LANGUAGES` in `src/i18n/index.ts` —
  `en, es, pt, fr, de, it, ru, pt-BR, zh-CN, cs, tr`.
- Translation files: `src/i18n/locales/<code>.json`.
- Deliberate same-as-English terms (proper nouns, football abbreviations) go in
  `src/i18n/INTENTIONAL_SAME.json`, keyed by locale or under `global`. This is an escape hatch for
  words that genuinely don't translate — not a way to skip work.

Enforced by two tests, both run by `npm test`:

- `src/i18n/localeCoverage.test.ts` — every locale has every `en.json` key, and no locale silently
  copies the English text.
- `src/i18n/frontendKeyCoverage.test.ts` — every literal `t("…")` key used in `src/` exists in
  `en.json`. It parses the TypeScript AST, so a typo'd key is caught, not just a missing one.

Backend strings arrive as **translation keys**, not English prose. `src/utils/backendI18n.ts`
(plus `backendI18nPlayerEvents.ts` and `backendI18n.legacy.ts`) maps them to text. If you add a
message on the Rust side, you add its key to all 11 locales here.

Use `/add-ui-string` — it is the whole procedure, in order.

---

## 2. Design tokens and the "Matchday" language

Tailwind v4 keeps its config **in CSS**: the `@theme` block in `src/App.css`. That block is the
only place colours and fonts are defined.

| Token family | Meaning |
|---|---|
| `primary-50…900` | Emerald green — primary actions, positive state |
| `accent-50…900` | Gold (`accent-400` = `#ffd60a`) — highlights, awards, emphasis |
| `success-400/500/600` | Success-specific green |
| `navy-900/800/700/600` | Dark-mode surfaces, darkest to lightest |
| `font-heading` | Barlow Condensed — headings, uppercase, tracked |
| `font-sans` | Inter — body text |

Rules:

- **Use token classes** (`bg-primary-700`, `text-navy-900`). Never a hex literal in a component,
  never an arbitrary value like `bg-[#10b981]`.
- **Every colour class needs its `dark:` partner.** Both themes ship; neither is an afterthought.
- Headings use `font-heading` with `uppercase tracking-wider`, matching `Button.tsx`.
- The font stacks include CJK fallbacks for `zh-CN`. Don't strip them when editing `@theme`.

---

## 3. Reuse before you build

Check the barrel at `src/components/ui/index.ts` first. It already exports `Card`/`CardHeader`/
`CardBody`, `Button`, `Badge`, `ProgressBar`, `CountryFlag`, `TeamLocation`, `ThemeToggle`,
`DatePicker`, `Select`, `Checkbox`, `PlayerAvatar`, `TeamLogo`, `InjuryBadge`, `JerseyIcon`, and
`PitchToken`.

Shared logic lives in `src/lib/` (`playerSquad.ts`, `playerRoles.ts`, `finance.ts`, `countries.ts`,
`seasonContext.ts`, `pyramid.ts`, …), reusable hooks in `src/hooks/`, backend call wrappers in
`src/services/`. Search all three before writing a helper — most things already have a home.

A new primitive belongs in `src/components/ui/` **only** if a second feature will use it.
Otherwise keep it next to its feature.

---

## 4. Accessibility floor

`Button.tsx` is the reference implementation. Match it.

- Visible focus on every interactive element: `focus:outline-none focus:ring-2 focus:ring-offset-2`
  plus a `dark:focus:ring-offset-navy-800`. Removing an outline without replacing it is a bug.
- Semantic elements. A clickable `div` is not a button; a `<button>` is.
- Every icon-only control gets an accessible name — `aria-label` with a **translated** string.
  (`aria-label` is in the `audit-i18n` attribute allowlist precisely because it is user-facing.)
- Disabled state is communicated by more than colour (`disabled:opacity-50` *and* the `disabled`
  attribute).
- Respect `prefers-reduced-motion` for anything that moves on its own.
- Contrast has to hold in **both** themes — gold on white and emerald on navy are the usual
  offenders.

The `ui-accessibility-reviewer` agent checks all of this against a diff.

---

## 5. State

Two Zustand stores in `src/store/`: `gameStore` (active game, manager, `hasActiveGame`) and
`settingsStore` (theme, language, currency, match preferences). Shared types in `store/types.ts`.

- **Never mutate store state from a component.** Copy, then set. There is regression history here
  (`fix/hometab-store-mutation`) — a component mutated a store array in place and the UI silently
  desynced.
- Backend calls go through `src/services/*Service.ts`, never a raw `invoke()` in a component.
  The service layer is where types and error handling live.

---

## 6. Invariants that are easy to break

**XI slot alignment.** Entry `i` of the starting-XI array *is* formation slot `i`, everywhere —
frontend, backend, and match engine. Substitutions and swaps insert at the **vacated index**, they
never append or re-sort. Slot geometry comes from `buildPitchRows(formation)` in
`src/components/squad/SquadTab.helpers.ts`; `PitchToken` is the shared pitch token component.

**Deployed vs natural position.** `player.position` is the player's *natural* position and is
never mutated. Where a player is currently *deployed* is derived from the formation slot via
`getDeployedPosition(team, slot)` in `src/components/squad/SquadTab.helpers.ts`, mirroring the
backend derivation. Role pickers, validators, and profile screens use the deployed slot; anything
describing the player themselves uses the natural position.

---

## 7. Testing

Vitest + `@testing-library/react`, jsdom environment, config in `vite.config.ts`, global setup in
`src/test-setup.ts`. Tests are co-located: `Foo.tsx` → `Foo.test.tsx`.

- **Query by role and accessible name**, not by test id or class. `getByRole("button", { name })`
  fails when the accessible name is missing — so the accessibility check comes for free.
- Extract pure logic into a `*.helpers.ts` file and unit-test it directly. `SquadTab.helpers.ts`
  and `useAdvanceTime.helpers.ts` are the pattern.
- Write the failing test first. That is the point.

Run a focused file while iterating — `npx vitest run src/components/squad` — and the full suite
before you push.
