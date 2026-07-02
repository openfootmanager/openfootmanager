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

## Running

```
npm run test:e2e
```

Each session runs in a temp `XDG_DATA_HOME` (created in `onPrepare`,
deleted in `onComplete`), so tests never touch your real save directory
at `~/.local/share/com.sturdyrobot.openfootmanager/`.

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
