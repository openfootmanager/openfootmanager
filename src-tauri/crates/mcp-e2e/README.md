# mcp-e2e

Drives the built app the way an agent does: one real process per scenario, spoken to over MCP,
results read back as the structs in `mcp-results`.

## What it needs

- **A binary built with `--features mcp`.** The shipped nightly build does not have the feature, so
  build it yourself: `cargo build --features mcp --bin openfootmanager` from `src-tauri/`. The
  fixture looks for `target/debug/openfootmanager`, or `$OFM_E2E_BIN`.
- **A virtual display: always run under `xvfb-run -a`.** The app builds a window even with
  `--no-gui`, so it cannot start without a display. The fixture removes `WAYLAND_DISPLAY` and
  `WAYLAND_SOCKET` from the app's environment and sets `GDK_BACKEND=x11`, so the app can only use
  the X display in `DISPLAY`. Under `xvfb-run` that is the virtual one. **Without it, `DISPLAY` is
  your own session's (XWayland, for example, at `:0`), and the window opens on your desktop.** The
  fixture refuses to start unless the display is evidently virtual (`XAUTHORITY` made by
  `xvfb-run`, or `OFM_E2E_ALLOW_DISPLAY=1` to vouch for your own Xvfb or Xephyr; an XWayland
  display is always refused), and never skips.

## Running

Every scenario that starts the app is `#[ignore]`d, so `cargo test --workspace` and the pull-request
path never launch it. The nightly runs them:

```bash
OFM_E2E_SEED=7 xvfb-run -a cargo test -p mcp-e2e -- --ignored --test-threads=2
```

A failure prints the binary, port, launch arguments, the last 50 log lines and the directory it kept,
and the repro line: `OFM_E2E_SEED=<seed> xvfb-run cargo test -p mcp-e2e --test <story> <scenario> -- --ignored --nocapture`.

## A stale binary is refused

The app reports the sources it was built from in the MCP `initialize` result (`serverInfo.version`
is `0.3.0+<revision>`). The revision is a digest of the content of the files the app is compiled from (git-listed, so
ignored files are out), and nothing else: committing or staging identical files, or changing docs,
tests or this crate, leaves it alone, so editing a test never needs an app rebuild.
The fixture computes the same revision for the working tree and refuses a binary that differs,
naming the rebuild command: `cargo build --features mcp --bin openfootmanager` from `src-tauri`.
`OFM_E2E_ALLOW_STALE=1` skips the check for a binary you know is different. It covers `src-tauri` and the crates the app links (their `src`, `data` and manifest), not their tests or the crates it does not link, and **not the frontend** a release build embeds from `../dist`.

## Writing a scenario

- `App::launch(&AppConfig::default())` starts a sandbox on a free port with private
  `XDG_DATA_HOME` and `XDG_CONFIG_HOME`; dropping it stops the process and frees the port.
- `app.client().<tool>(...)` is one typed method per tool (`tools.rs`). A refusal is
  `Err(CallError::Refused(..))` with the `be.error.*` key. Assert on keys and fields, never on text.
- `assert_refusal_changes_nothing(&app, key, || ...)` proves a refused call left the game and every
  file under the saves directory as it was.
- A known bug is carried as one plain assert that it still happens, naming its issue and failing with
  "#123 is fixed: delete this exemption". There is no helper for it.
- The client never retries. A call that timed out may have run: read the state back instead.

## Keeping the typed client honest

`tools.rs` is one method per tool, with `CATALOG` listing each tool's parameters. The ignored test
`catalog_matches_the_live_server` compares it with the running server's `tools/list`, so a tool
added, removed or given another parameter fails there by name until `tools.rs` is updated.
