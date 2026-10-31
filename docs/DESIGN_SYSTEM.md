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
career; its test fails when a token in `@theme` or an exported component is missing from it. It's
for development only: it isn't in the build, and no player sees it.

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

A control that is on or chosen (a switch that is on, a segmented option filled with `strong`, a
checked box, a count badge, the highlighted option in `Select` and the chosen day in
`DatePicker`) is getting its own `selected` role, with a dark pair, in place of the brand green:
one place to change how "chosen" looks (maintainer, 22 Oct). Until that PR lands they use the
`primary-*` greens.

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
| `w-sidebar-rail` | 3.5rem | the sidebar's icon rail |
| `w-sidebar-compact` / `w-sidebar-standard` / `w-sidebar-wide` | 13 / 16 / 17.75rem | the open sidebar, by Settings › Sidebar width; `sidebarLayout.ts` reckons with the same values |
| `max-w-page` | 80rem | the page width, applied once by the workspace |
| `@split` | 36rem | the container breakpoint for two panels |

Rows are `h-row`, 40px, or 32px when compact; there is no taller row token (maintainer, 22 Oct).

A container breakpoint is in rem, so it moves with the UI scale. In a 960×640 window at normal
scale `@split` is 576px and fits the 655px workspace, so two panels show; at extra large it is
720px and they stack. Choose breakpoints by computing them at a 20px root, not by the name of
the class.

## 3. Components

Use the primitive; never rebuild one. All are exported from `src/components/ui/index.ts` and
drawn in the style guide.

| Need | Use |
|---|---|
| a page's sections, filters and one primary action | `PageToolbar`, with `Tabs` in its `start` slot and the action at its right end |
| sections of a page, or a two- or three-way switch in a panel | `Tabs` (`look="underline"` or `"segmented"`) with a `TabPanel` each, all mounted and all but one `hidden`; the arrow keys, Home and End move between tabs. Tabs that don't fit scroll inside the tablist, and the selected one is scrolled into view |
| a choice of one value from a few (a match's speed, a density) | `SegmentedControl`, a radio group with Tabs' segmented look: `emphasis="strong"` fills the chosen option for a setting the player watches, `layout="stacked"` puts an icon over each label in a 52px target |
| a card or panel | `Card`, `CardHeader`, `CardBody` |
| a button | `Button` |
| a chip or label | `Badge` |
| a modal | `Dialog` (named, focus-trapped, closes on Escape) |
| a drop-down | `Select` |
| a tick box | `Checkbox`, for a choice that is submitted later |
| an on/off setting that takes effect at once | `Switch`, a WAI-ARIA switch named by its row's label (`labelledBy`) or its own translated `label`; the state shows by more than the thumb's position |
| a date | `DatePicker` |
| a progress or condition bar | `ProgressBar` |
| a player, club or country | `PlayerAvatar`, `TeamLogo`, `CountryFlag`, `TeamLocation` |
| an injury | `InjuryBadge` |
| an unread count or a mark on a nav item (a circle for one digit, 99+ at most) | `UnreadBadge` |
| a kit | `JerseyIcon` |
| a player on the pitch | `PitchToken` |
| a list as a table (players, clubs, staff) | `DataTable`: column definitions; `aria-sort` headers that ask to sort (it never sorts the rows); rows of `h-row` whose name is the open button (Enter opens; a click on the row too, but not on its action column); a `positive` or `danger` edge accent; empty and loading states; under 48rem the `drawer` columns fold into the row's details (never the first column, which keeps the accent and, failing a primary, the name). It draws no panel: put it in a `Card` |
| a player's overall rating | `RatingChip`: the number alone in its tier's tone (the backend's tier, from `useRatingTiers`), named "Overall 79, tier A" for a screen reader and as a tooltip; `sm` for a row, `md` for a card |

More primitives arrive as the migration goes on: empty and loading states outside a table, stat
tile, search field, toggle, and player, match and club cards. Each one gets a row here and a place in the style guide in the PR that adds it.

## 4. Rated values

Any value the game rates (OVR and potential, an attribute, condition, morale, a match rating,
form) is coloured by **tone**: `top`, `good`, `fair`, `neutral` (no colour), `low`, `bad`. **The backend
decides the tone**, the way `ofm_core::player_tier` decides S, A and B, and the frontend only
paints it. A TypeScript file never compares a number against a threshold to pick a colour.

**The game rates; it doesn't advise.** There is no style-fit verdict and no "best position for
this formation" badge on a player who isn't starting (maintainer, 22 Oct): picking the team is the
manager's job. A position badge says where a player can play, not where they should.

A tone never stands alone: there is always a letter, a word or the number next to it. A value
the game doesn't know (an unscouted attribute, a match not yet rated) shows "–" in `fg-subtle`,
never a grey number that could read as a low value.

Condition will have words as well as tones, from one rule in `ofm_core` that will also drive the
staff warnings and the squad's fitness counts, so the words, the tones and the counts always
agree. The maintainer chose these bands; the rule doesn't exist yet (today's fitness status splits
only at 40 and 25), and this section is updated when it lands:

| Condition | Word | Tone |
|---|---|---|
| 75 and over | Fresh | `good` |
| 40–74 | Tiring | `neutral` |
| 25–39 | Exhausted | `low` |
| under 25 | Critical | `bad` |

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

`src/designSystem.test.ts` checks six rules in every folder listed in its `FINISHED_FOLDERS`:
role type sizes only, no colour literal (except SVG artwork whose colours are data), no raw palette
class, a full-screen overlay only through `Dialog`, no `on-brand` text on a `-solid` fill, and one
focus ring (`ring-focus`, on `focus-visible`, never plain `focus` or another colour, offset in the
surface behind the control: `page`, `panel`, `sunken` or `chrome`). Each
rule has a fixture that breaks it. A folder joins the list in the PR that brings it up to
standard, so the list only ever names finished work, never a known violation; when every folder
is in, the list goes.

Rule 7, one home per vocabulary, runs over its own list, `VOCABULARY_FOLDERS`, which grows as each
round-4 page moves onto the primitives:
- no colour map keyed by a position, a tier letter or a condition band;
- no rated value (`condition`, `ovr`, `potential`, `morale`, `reputation`) compared with a number in
  a component, because the backend picks a rated value's tone;
- no import of a helper whose primitive has landed (`RETIRED_HELPERS`).

What a test can't see (uppercase, one primary action per page, colour as
the only signal) is for review: the `ui-accessibility-reviewer` agent checks it.

## 8. Screenshots for a PR

A PR that changes how something looks shows before and after, in light and dark, at 1280×800 (the
window the game opens at) and 960×640 (its minimum).

- **A primitive:** run the dev server on each commit and screenshot the style guide in headless
  Chrome, for example
  `google-chrome --headless=new --window-size=1280,3300 --screenshot=out.png "http://localhost:1420/style-guide.html?theme=dark"`.
- **A page:** the page harness mounts the real dashboard on a small generated career, with the
  backend mocked, in the same dev page: `/style-guide.html?page=squad`, with the same theme,
  density, scale and contrast options. The pages are `home`, `squad`, and the world lists
  `players`, `teams`, `managers` and `staff` (`HARNESS_PAGES` in
  `src/dev/pageHarness/harnessPage.ts`). Screenshot them the same way.
  - The career is `src/dev/pageHarness/fixture.generated.ts`: one eight-club league from a fixed
    seed, begun the way the app begins one, plus each command's answer for it. A command that
    takes a query (the Players page's, the Teams directory's) is answered only for the query the
    fixture was generated for, so a search or another page shows as unanswered rather than
    getting the opening page. Regenerate it with
    `npm run fixture:pages` (the generator is `src-tauri/crates/ofm_core/examples/page_fixture.rs`).
  - Every value in it `satisfies` the frontend type its command returns, so `npm run build` fails
    when the backend's shape and the frontend's type stop agreeing.
  - A command the page asks for that the fixture doesn't answer fails, and is listed in
    `window.__pageHarness.unanswered`; `window.__pageHarness.pastEdge()` lists anything reaching
    past the window's right edge. Neither is a CI check yet.
  - `node scripts/page-layout-check.mjs [dev-server-url]` loads the guide and each page at 1280×800
    and 960×640, and in German at extra large, and fails a page that scrolls sideways or reaches
    past the edge. It needs Chrome, a running dev server, and Node 22 or later (for the built-in
    WebSocket); on an older Node it says so and exits.
  - The harness is not the real app: note the real-app check you did, or didn't do, in the PR.
- **The real app:** run a debug build with `--features mcp` under Xvfb through `tauri-driver`,
  start a career over the MCP port (`game_new`), and drive the UI over WebDriver. Give the app
  private `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME` and `XDG_STATE_HOME` directories, so
  it never reads or writes your real saves and settings, and `GDK_BACKEND=x11` with
  `XDG_SESSION_TYPE=x11`, so it opens on the Xvfb display rather than your Wayland desktop.
  `src-tauri/crates/mcp-e2e/src/app.rs` launches it this way. Under Xvfb the webview runs at device
  pixel ratio 2. If screenshots stop returning while JavaScript still answers, the webview has
  stopped painting: restart the whole stack, including orphaned
  `WebKitWebProcess` processes.
