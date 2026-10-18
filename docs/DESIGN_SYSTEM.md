# Design system

How every screen in OpenFoot Manager looks: one set of tokens, one set of components, one set of
layout rules. This page is the reference; `src/CLAUDE.md` §2 holds the same rules in short, for
contributors writing code.

You can follow it without reading React. Wherever a rule depends on game data (a rating, a team's
colours), the Rust side decides and the frontend only paints.

---

## 1. Seeing it

Run `npm run dev` and open **http://localhost:1420/style-guide.html**. The style guide draws
every token and every component in `src/components/ui` from fixtures, with no backend and no
career. It's for development only: it isn't in the build, and no player sees it.

Its URL sets the conditions:

| Parameter | Values | Default |
|---|---|---|
| `theme` | `light`, `dark` | `light` |
| `scale` | `small`, `normal`, `large`, `xlarge`, the same root sizes as the UI-scale setting | `normal` |
| `density` | `comfortable`, `compact` | `comfortable` |
| `contrast` | `high` | off |
| `dialog` | `open` draws the dialog open | closed |

A component exported from `src/components/ui/index.ts` that the style guide doesn't draw fails
`src/dev/styleGuide/StyleGuide.test.tsx`.

## 2. Tokens

Tokens live in one place, the `@theme` block of `src/App.css`. Each one becomes a Tailwind class:
`--color-panel` gives `bg-panel`, `text-panel` and `border-panel`; `--text-caption` gives
`text-caption`.

### Colour by role

A role says what a colour is for, not what it looks like. Each role has a light value in `@theme`
and a dark value in the `.dark` block below it, so **a component that uses a role needs no
`dark:` partner**. High contrast swaps the text and line roles again, in either theme.

| Role | For |
|---|---|
| `page` | the workspace behind everything |
| `panel` | cards, tables, dialogs |
| `sunken` | insets: table headers, wells, stat tiles |
| `hover` | the surface under the pointer |
| `chrome` | the sidebar and the match band; dark in both themes |
| `overlay` | the backdrop behind a dialog |
| `fg`, `fg-muted`, `fg-subtle` | text: strong, secondary, quiet (placeholders, disabled) |
| `line`, `line-strong` | borders and dividers |
| `focus` | the focus ring |
| `on-brand` | text and icons on the brand fill (`primary-700`), on chrome, on the turf |
| `positive`, `warning`, `danger`, `info` | status text; each also has `-tint` (a background) and `-solid` (a bar or dot) |
| `tone-top`, `tone-good`, `tone-fair`, `tone-low`, `tone-bad` | the scale for rated values (§4), with `-tint` and `-solid` |

Brand palettes (`primary-*`, `accent-*`, `navy-*`) stay for brand moments: the Continue button,
the selected navigation item, a logo strip. The raw Tailwind palette (`gray-500`, `red-400`,
`white`, …) isn't used in a finished folder (§7).

**Readability is tested, not hoped for.** `src/designTokens.test.ts` checks that every text role
reaches 4.5:1 on every surface it sits on, and that the focus ring reaches 3:1, in light, dark,
high-contrast light and high-contrast dark. It composites a translucent tint over the panel
before measuring.

Two rules follow from what those numbers allow:

- **A `-solid` colour carries no text.** Solids are for bars, dots and fills; white on them falls
  under 4.5:1. Text on a fill goes on `primary-700` or `chrome`, in `on-brand`.
- **A line is never the only signal.** `line` is decorative: in high contrast light it is 2.5:1
  against the page. A border may group or separate things, but whatever it marks (a selected
  row, an error, a highlighted team) also needs text, an icon, a weight or a tint.

### Type

| Size | px at normal scale | For |
|---|---|---|
| `text-caption` | 12 | table headers, labels, meta, badges |
| `text-body` | 14 | everything else |
| `text-lead` | 16 | reading text: messages, articles |
| `text-heading` | 18 | panel and section headings |
| `text-title` | 24 | the page title |
| `text-display` | 32 | a score, the profile's OVR, one big number per page |

- **Nothing under 12px, and no pixel sizes.** Every size is in rem, so the UI-scale setting moves
  all of them; a `text-[10px]` or an inline `fontSize` is a size the setting can't touch.
- **Uppercase only** for headings at `text-heading` or larger, buttons, badges and tabs, always
  with `tracking-label`. Labels and table headers are sentence case.
- Numbers that line up in columns use `tabular-nums`.

### Layout

| Token | Value | For |
|---|---|---|
| `h-row` / `min-h-row` | 40px, 32px when compact | list and table rows |
| `h-panel-header` | 40px, 36px when compact | a panel's header |
| `h-toolbar` | 48px, 40px when compact | the page toolbar |
| `max-w-page` | 80rem | the page width, applied once by the workspace |
| `@split` | 36rem | the container breakpoint for two panels |

A container breakpoint is in rem, so it moves with the UI scale. In a 960×640 window at normal
scale `@split` is 576px and fits the 655px workspace, so two panels show; at extra large it is
720px and they stack. Choose breakpoints by computing them at a 20px root, not by the name of
the class.

## 3. Components

Use the primitive; never rebuild one. All are exported from `src/components/ui/index.ts` and
drawn in the style guide.

| Need | Use |
|---|---|
| a card or panel | `Card`, `CardHeader`, `CardBody` |
| a button | `Button` |
| a chip or label | `Badge` |
| a modal | `Dialog` (named, focus-trapped, closes on Escape) |
| a drop-down | `Select` |
| a tick box | `Checkbox` |
| a date | `DatePicker` |
| a progress or condition bar | `ProgressBar` |
| a player, club or country | `PlayerAvatar`, `TeamLogo`, `CountryFlag`, `TeamLocation` |
| an injury | `InjuryBadge` |
| a kit | `JerseyIcon` |
| a player on the pitch | `PitchToken` |

More primitives arrive as the migration goes on: page toolbar, tabs, empty and loading states,
stat tile, data table, search field, toggle, the rated-value chip, and player, match and club
cards. Each one gets a row here and a place in the style guide in the PR that adds it.

## 4. Rated values

Any value the game rates (OVR and potential, an attribute, condition, morale, a match rating,
form) is coloured by **tone**: `top`, `good`, `fair`, `neutral`, `low`, `bad`. **The backend
decides the tone**, the way `ofm_core::player_tier` decides S, A and B, and the frontend only
paints it. A TypeScript file never compares a number against a threshold to pick a colour.

A tone never stands alone: there is always a letter, a word or the number next to it. A value
the game doesn't know (an unscouted attribute, a match not yet rated) shows "–" in `fg-subtle`,
never a grey number that could read as a low value.

## 5. Layout rules

1. The dashboard workspace owns the scroll and applies `max-w-page` once.
2. The header shows the page's title; no page repeats it.
3. One primary action per page, at the right end of its toolbar. The header's Continue is the
   game-wide forward action.
4. Every match stage uses `MatchScreenLayout`.
5. At the 960×640 minimum window, at every UI scale: no sideways scroll and nothing clipped.
   Vertical scroll is fine.
6. At most two panels side by side, and one below `@split`.

## 6. Adding a token

1. Add it to `@theme` in `src/App.css`. If it changes with the theme, add its dark value to the
   `.dark` block, and its high-contrast values if it is text or a line.
2. Add it to `THEMED_COLOURS`, and, if it is text, to `READABLE_PAIRS` in
   `src/designTokens.test.ts`, and see the test pass.
3. Draw it in the style guide.
4. Reference it by its full name somewhere in source (`bg-hover`, or `var(--color-hover)`).
   Tailwind only emits a theme variable whose name it finds, so a name assembled from pieces at
   runtime resolves to nothing.

## 7. Enforcement

`src/designSystem.test.ts` checks five rules in every folder listed in its `FINISHED_FOLDERS`:
role type sizes only, no colour literal (except SVG artwork whose colours are data), no raw palette
class, a full-screen overlay only through `Dialog`, and no `on-brand` text on a `-solid` fill. Each
rule has a fixture that breaks it. A folder joins the list in the PR that brings it up to
standard, so the list only ever names finished work, never a known violation; when every folder
is in, the list goes. What a test can't see (uppercase, one primary action per page, colour as
the only signal) is for review: the `ui-accessibility-reviewer` agent checks it.

## 8. Screenshots for a PR

A PR that changes how something looks shows before and after, in light and dark, at 1280×800 (the
window the game opens at) and 960×640 (its minimum).

- **A primitive:** run the dev server on each commit and screenshot the style guide in headless
  Chrome, for example
  `google-chrome --headless=new --window-size=1280,3300 --screenshot=out.png "http://localhost:1420/style-guide.html?theme=dark"`.
- **A page:** run a debug build with `--features mcp` under Xvfb through `tauri-driver`, start a
  career over the MCP port (`game_new`), and drive the UI over WebDriver. Under Xvfb the webview
  runs at device pixel ratio 2. If screenshots stop returning while JavaScript still answers,
  the webview has stopped painting: restart the whole stack, including orphaned
  `WebKitWebProcess` processes.
