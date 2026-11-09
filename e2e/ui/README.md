# Real-UI end-to-end (WebdriverIO + tauri-driver)

Drives the built app's own window, the way a player does. It sits beside `src-tauri/crates/mcp-e2e`
(which drives the app over MCP, as an agent would) and covers what that cannot: the screens.

| Flow | Spec | State |
|---|---|---|
| 1. New game, to the first match day's pre-match lineup | `specs/flow1.new-game.e2e.mjs` | here |
| 2. Tactics edit, then match; 3. substitutions; 4. end of season; 5. a package, then a career | | later |

**Nightly and advisory.** `.github/workflows/ui-e2e.yml` runs it (nightly, on demand, and on a pull
request that changes `e2e/`; it is separate from `nightly-e2e.yml`). A red run does not open the
nightly issue; it is reported in the job summary and kept as artifacts (a screenshot and the page
markup of the failing step). It becomes required once it has been green for a week.

## Running it

```bash
npm run tauri build -- --debug --no-bundle          # the app with its frontend embedded
cargo install tauri-driver --locked                 # once
sudo apt-get install -y webkit2gtk-driver xvfb      # or webkitgtk-webdriver; WebKitWebDriver and a virtual display
cd e2e/ui && npm ci
OFM_E2E_APP=../../src-tauri/target/debug/openfootmanager bash run.sh
```

`run.sh` is the only entry point. It makes a scratch `HOME` with private `XDG_*` directories, forces
`GDK_BACKEND=x11`, starts an Xvfb of its own on a free display number (100 or above, never your
session's), and deletes the scratch directory afterwards. The suite refuses to start otherwise
(`helpers/isolation.mjs`, tested by `npm run test:guard`). It checks that the setup is isolated,
not that `run.sh` made it: the scratch `HOME` and every `XDG_*` directory must lead, through
symlinks, into the temporary directory, and `DISPLAY` must be served by a live Xvfb whose process id
is given. Your own home, your own X display and a Wayland session all fail those checks; a
scratch home and an Xvfb you prepared by hand would pass them, and that is as isolated as `run.sh`'s.
The WebDriver server is the same: the suite starts its own `tauri-driver` on ports it picks, and
goes on only once that process is the one listening (`helpers/driver.mjs`), so a driver left
running on the default port is never used. A flow that starts a new game must never touch a real
career. **Do not launch the app any other way against your own home.**

## Writing a step

- Drive what the player sees: the English labels (the run sets `LANG=en_US.UTF-8`), the roles the
  app declares, and the ids and `data-testid`s it already has.
- Need a hook the markup lacks? Ask the frontend owner for a `data-testid` first; do not reach for
  a CSS class.
- One `it` per step of the path, so a red night names the step.
