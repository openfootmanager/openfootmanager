# End-to-end tests

WebdriverIO + tauri-driver, running against the real compiled Tauri app.
Complements Vitest (component tests, mocked IPC) and cargo tests (engine
logic) — this layer is for user-journey assertions that need a real
WebView + real Rust backend.

## Setup

Two binaries need to be on your `PATH`:

- **`tauri-driver`** — bridges the WebDriver protocol to Tauri's WebView.
  Not published as a system package; install via
  `cargo install tauri-driver --locked`.
- **`WebKitWebDriver`** — ships with WebKitGTK. On Debian/Ubuntu install
  `libwebkit2gtk-4.1-dev` (or the equivalent for your distro).

Then run `npm install` and build the Tauri dev binary once, **with
the `mcp` Cargo feature enabled**:

```
npm run tauri -- build --debug --no-bundle --features mcp
```

- `--no-bundle` skips `.deb` / `.rpm` packaging — those need `dpkg-deb`
  / `rpmbuild` on PATH and hang on machines that don't ship them. We
  only need the raw binary at `src-tauri/target/debug/openfootmanager`.
- `--features mcp` compiles in the automation surface documented in
  `docs/MCP_SERVER.md`. Specs use `--mcp-auto-start` to boot the game
  into a known fixture state without clicking through the new-game
  menu. Without the feature, MCP CLI flags are silently ignored and
  the game boots to the main menu instead.

Override the binary location with
`TAURI_APP_BINARY=/path/to/binary npm run test:e2e`.

Alternatively, `nix-shell shell.nix` provides `tauri-driver`,
`WebKitWebDriver`, `xvfb-run`, and the GStreamer plugins already wired up
— the same environment the nightly CI job uses.

## Running

```
npm run test:e2e
```

Each session runs in a temp `XDG_DATA_HOME` (created in `onPrepare`,
deleted in `onComplete`), so tests never touch your real save directory
at `~/.local/share/com.sturdyrobot.openfootmanager/`.

### Headless

To run without a real display (as CI does), wrap the suite in a virtual
X server:

```
xvfb-run -a npm run test:e2e
```

`xvfb-run` is on `PATH` inside `nix-shell shell.nix`, or install it via
your distro (Debian/Ubuntu: `xvfb`).

## Continuous integration

The suite runs **nightly**, not per-PR — see
`.github/workflows/e2e-nightly.yml`. The debug Tauri build plus a
headless WebKitGTK WebView is too slow to gate every commit, so it runs
on a schedule (02:30 UTC) and on manual `workflow_dispatch`. The job
builds with `--features mcp` and runs under `xvfb-run`, installing
`webkit2gtk-driver` and pinning `tauri-driver` to match `shell.nix`.

## Writing specs

Specs live under `tests/e2e/*.spec.ts` and use the WebdriverIO
`@wdio/globals` API (`$`, `$$`, `browser`, `expect`). The Mocha `bdd`
UI is configured (`describe` / `it`).

For selectors, prefer `data-testid` attributes over text-matching so
copy tweaks don't break tests. Add the attributes in the component as
you write the spec.

To assert on persisted state (save DB, config), open the SQLite file
under `$XDG_DATA_HOME/com.sturdyrobot.openfootmanager/saves/` from Node
— the env var is set for the whole session.

## Troubleshooting

- **`WebKitWebDriver not found`** — see setup. Confirm `which
  WebKitWebDriver` resolves. On some distros the binary is under
  `libexec/webkit2gtk-4.1/` rather than `bin/` and needs to be put on
  PATH explicitly.
- **`tauri-driver` not found or exits immediately** — confirm `which
  tauri-driver` resolves. Exits with a Tauri version mismatch usually
  mean the installed `tauri-driver` is older than the app's Tauri
  version; bump with `cargo install tauri-driver --locked --force`.
- **Modal never renders on Wayland** — try
  `WEBKIT_DISABLE_DMABUF_RENDERER=1 npm run test:e2e`, or fall back
  to `GDK_BACKEND=x11 npm run test:e2e` (requires an XWayland-capable
  session).
- **Game boots to the main menu instead of the fixture** — the
  binary was likely built without `--features mcp`. `--mcp-auto-start`
  is silently ignored on non-MCP builds. Rebuild with the feature
  (see setup step 2).
