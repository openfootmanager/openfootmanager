# Pull Request: Android port with responsive mobile UI

**Branch:** `feat/android-mobile-port` → base: `develop`
**Commit:** `feat(android): add Tauri Android target with responsive mobile UI`

> Note: this change was AI-assisted (developed with an AI coding agent, tested and
> reviewed by a human on a physical Android phone). Disclosed per `AGENTS.md` /
> `CONTRIBUTING.md` provenance guidance for this GPLv3 project.

## Summary

Adds Android as a first-class build target via Tauri 2's mobile support, and makes
the UI fully usable on phone-sized screens — **without changing the desktop
(Windows/Linux/macOS) UI or behavior in any way**. Every mobile adjustment is
gated behind Tailwind breakpoints (below `md`/`lg`) or an Android platform check,
so desktop rendering at `md` and up is pixel-identical to before.

No new npm or cargo dependencies. No changes to `src-tauri` source — the only
Rust-side addition is the generated `gen/android` scaffold.

## What changed

### 1. Android build scaffold (`src-tauri/gen/android/`)

- Generated with `npm run tauri android init` (Tauri CLI 2.x). Entirely additive;
  desktop builds never touch this directory.
- The existing codebase was already mobile-ready on the Rust side
  (`#[cfg_attr(mobile, tauri::mobile_entry_point)]` in `src-tauri/src/lib.rs`),
  and the whole workspace (engine, domain, `db`/SQLite, `ofm_core`)
  cross-compiles to `aarch64-linux-android` unmodified. The optional `mcp`
  cargo feature stays off for mobile builds.
- Build (requires Android SDK + NDK and Rust android targets):
  `npm run tauri android build -- --apk --debug --target aarch64`
- Verified on a physical phone (arm64): app launches, saves load, dashboard,
  match sim and navigation all work.

### 2. Responsive dashboard shell

- **`src/components/dashboard/DashboardBottomNav.tsx` (new):** mobile-only
  (`md:hidden`) bottom navigation bar rendered inside the dashboard column (not
  `position:fixed`, so it never covers content). Four primary tabs (Home, Squad,
  Inbox, Schedule) reusing the sidebar's exact item definitions, icons, i18n keys
  and badge logic (inbox unread count, schedule match-day marker), plus a "More"
  button opening a bottom sheet with all remaining tabs and the sidebar's
  Settings / Exit-to-menu entries. Tab switching flows through the same
  `onNavClick(tab)` prop as the sidebar — zero changes to navigation logic.
- **`DashboardSidebar.tsx`:** root element is now `hidden md:flex` — its only
  change; desktop rendering untouched.
- **`DashboardHeader.tsx`:** wraps gracefully (`flex-wrap`), search takes its own
  full-width row below `sm`, save/continue buttons go icon-only below `sm`.
- **`DashboardWorkspaceContent.tsx`:** workspace padding `p-3 md:p-6`.

### 3. Responsive screens & tables

- **`MatchLive.tsx`:** the fixed `w-72` controls aside becomes a slide-over
  drawer (backdrop + close button, opened from a header button) below `lg`;
  at `lg` and up it renders exactly as before.
- Added missing `overflow-x-auto` wrappers on wide tables: `ManagerTab`,
  `YouthAcademyTab`, `PlayerProfileCareerHistoryCard`,
  `TeamProfileHistoryCard`, `PostMatchScreen` team-talk table.

### 4. Android safe-area insets

- **`src/App.css`:** new `.pt-safe` / `.pb-safe` utilities mapping to
  `env(safe-area-inset-top/bottom)` — they evaluate to `0` everywhere except
  edge-to-edge mobile webviews.
- Applied to every screen-edge-flush bar: dashboard header, bottom nav + More
  sheet, match screen scoreboard bar / bottom edge / controls drawer, and the
  top bars of Team Selection, Settings, and Main Menu. Fixes the OS status bar
  and gesture bar overlaying app content on Android.
- `index.html`: viewport meta gains `viewport-fit=cover`.

### 5. Desktop-only API guards

- **`src/utils/platform.ts` (new):** `isAndroid()` via the user agent — no new
  dependencies.
- `Dashboard.tsx`: skips `getCurrentWindow().onCloseRequested(...)` and
  `.destroy()` on Android (no close-requested event exists there).
- `MainMenu.tsx`: hides the Quit button on Android and skips `window.destroy()`.
- Desktop Tauri and desktop browser behavior unchanged.

### 6. i18n

- Only two new keys: `common.more` and `match.controls`, translated into all
  11 locales (`en, es, pt, pt-BR, fr, de, it, ru, zh-CN, cs, tr`). Everything
  else reuses existing keys, so `localeCoverage` stays green.

## Tests

- New: `DashboardBottomNav.test.tsx` (11 tests: rendering, tab forwarding,
  badges, sheet open/select/close, Settings/Exit forwarding, unemployed
  filtering) and `platform.test.ts` (4 tests).
- Full suite: **1157/1158 pass** — the single failure
  (`Dashboard.test.tsx` "clears a stale active save id…") is **pre-existing** on
  `develop` (verified by stashing all changes and re-running on a clean tree).
- `npm run build` (tsc + vite) green; no new biome diagnostics introduced.
- Backend: `src-tauri` source untouched; Android workspace cross-compile and
  debug APK build verified.

## Known limitations / follow-ups

- Built and smoke-tested as an **aarch64 debug APK**; release signing and the
  other ABIs (armv7, x86_64) are just additional `--target` flags.
- Touch interactions deserve a deeper audit: right-click context menus and
  tactics-pitch drag-and-drop work but are desktop-designed interactions.
- SimLab and WorldEditor pages were left desktop-oriented (developer tooling).
- Tablets at `md`–`lg` (768–1023px) get a compressed desktop layout (sidebar +
  scrollable tables); a dedicated tablet layout could be a follow-up.
