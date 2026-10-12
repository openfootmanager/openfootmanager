# mcp-e2e

Drives the built app the way an agent does: one real process per scenario, spoken to over MCP,
results read back as the structs in `mcp-results`.

## What it needs

- **A binary built with `--features mcp`.** The shipped nightly build does not have the feature, so
  build it yourself: `cargo build --features mcp --bin openfootmanager` from `src-tauri/`. The
  fixture looks for `target/debug/openfootmanager`, or `$OFM_E2E_BIN`.
- **A display.** The app opens a (hidden) window and cannot start without one, so run under
  `xvfb-run` where there is none. The fixture fails loudly when it finds neither `DISPLAY` nor
  `WAYLAND_DISPLAY`; it never skips.

## Running

Every scenario that starts the app is `#[ignore]`d, so `cargo test --workspace` and the pull-request
path never launch it. The nightly runs them:

```bash
OFM_E2E_SEED=7 xvfb-run -a cargo test -p mcp-e2e -- --ignored --test-threads=2
```

A failure prints the binary, port, launch arguments, the last 50 log lines and the directory it kept,
and the repro line: `OFM_E2E_SEED=<seed> xvfb-run cargo test -p mcp-e2e --test <story> <scenario> -- --ignored --nocapture`.

## Writing a scenario

- `App::launch(&AppConfig::default())` starts a sandbox on a free port with private
  `XDG_DATA_HOME` and `XDG_CONFIG_HOME`; dropping it stops the process and frees the port.
- `app.client().<tool>(...)` is one typed method per tool (`tools.rs`). A refusal is
  `Err(CallError::Refused(..))` with the `be.error.*` key. Assert on keys and fields, never on text.
- `assert_refusal_changes_nothing(&app, key, || ...)` proves a refused call left the game and every
  file under the saves directory as it was.
- `expect_bug!(#123, { ... })` wraps a scenario that fails while the bug reproduces, and fails with
  "remove the guard for #123" once it is fixed.
- The client never retries. A call that timed out may have run: read the state back instead.

## Keeping the typed client honest

`tools.rs` is one method per tool, with `CATALOG` listing each tool's parameters. The ignored test
`catalog_matches_the_live_server` compares it with the running server's `tools/list`, so a tool
added, removed or given another parameter fails there by name until `tools.rs` is updated.
