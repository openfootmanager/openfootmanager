#!/usr/bin/env bash
# Runs the real-UI suite against a built app, in a throwaway home on a virtual display.
#
#   OFM_E2E_APP=/path/to/openfootmanager  bash e2e/ui/run.sh [wdio args]
#
# Build the app first with its frontend embedded (`npm run tauri build -- --debug --no-bundle`
# from the repository root); a plain `cargo build` binary opens the dev server's URL instead.
# Needs xvfb-run, WebKitWebDriver (the webkit2gtk-driver package) and tauri-driver
# (`cargo install tauri-driver --locked`).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
app="${OFM_E2E_APP:-$here/../../src-tauri/target/debug/openfootmanager}"
[ -x "$app" ] || { echo "run.sh: no app at $app (set OFM_E2E_APP)" >&2; exit 2; }
command -v xvfb-run >/dev/null || { echo "run.sh: xvfb-run is required" >&2; exit 2; }

# Never the real home: everything the app writes lands in this directory, which is removed after.
scratch="$(mktemp -d "${TMPDIR:-/tmp}/ofm-ui-e2e.XXXXXX")"
trap 'rm -rf "${scratch:?}"' EXIT
mkdir -p "$scratch/data" "$scratch/config" "$scratch/cache" "$scratch/state"

artifacts="${OFM_E2E_ARTIFACTS:-$here/artifacts}"
mkdir -p "$artifacts"

env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET \
  OFM_E2E_ISOLATED=1 \
  OFM_E2E_APP="$app" \
  OFM_E2E_ARTIFACTS="$artifacts" \
  HOME="$scratch" \
  XDG_DATA_HOME="$scratch/data" \
  XDG_CONFIG_HOME="$scratch/config" \
  XDG_CACHE_HOME="$scratch/cache" \
  XDG_STATE_HOME="$scratch/state" \
  XDG_SESSION_TYPE=x11 \
  GDK_BACKEND=x11 \
  LANG=en_US.UTF-8 \
  WEBKIT_DISABLE_DMABUF_RENDERER=1 \
  OFM_GPU_PROFILE=safe \
  xvfb-run -a -s "-screen 0 1280x800x24" \
  npx --no-install wdio run "$here/wdio.conf.mjs" "$@"
