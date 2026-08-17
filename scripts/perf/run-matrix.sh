#!/usr/bin/env bash
# Run the UI benchmark once per WebKitGTK rendering configuration.
#
# Each row sets a different combination of environment variables, launches the app, waits for the
# benchmark to POST its result to the collector, and moves on. The output feeds report.mjs, which
# builds the table in docs/LINUX_GRAPHICS.md.
#
#   scripts/perf/run-matrix.sh [row ...]
#
# With no arguments, runs every row. Pass row names to run a subset, e.g.
#
#   scripts/perf/run-matrix.sh baseline shipped
#
# The rows are ordered so everything above `shipped` keeps hardware acceleration. Tauri's own
# guidance (https://v2.tauri.app/develop/debug/linux-graphics/) ranks the workarounds the same way:
# try the cheap ones first, because the DMABuf disable costs the fast path.

set -euo pipefail

# Job control, so every background job below lands in its own process group. That is what lets
# cleanup kill a whole `npm -> cargo -> app` tree by group id instead of by name: matching on
# names (`pkill -f vite`) would also take down a dev server you happened to be running in another
# terminal.
set -m

# Linux-only by construction: every row sets WebKitGTK or GPU-driver environment variables, and
# the hardware probe reads /sys/class/drm. Windows renders through WebView2 and macOS through
# WKWebView, so there is no equivalent ladder to walk there.
if [ "$(uname -s)" != "Linux" ]; then
  echo "run-matrix.sh measures the WebKitGTK rendering path and only applies to Linux." >&2
  exit 1
fi

cd "$(dirname "$0")/../.."

RESULTS_DIR="${RESULTS_DIR:-scripts/perf/results}"
TIMEOUT_SECS="${TIMEOUT_SECS:-90}"

# row name | environment assignments (space separated, KEY=VALUE)
#
# OFM_GPU_PROFILE=off stops lib.rs setting anything of its own, so a row's variables are the only
# ones in play. Without it, row 0 is unreachable — which was the whole reason for that flag.
declare -A ROWS=(
  [baseline]="OFM_GPU_PROFILE=off"
  [auto]="OFM_GPU_PROFILE=auto"
  [explicit-sync]="OFM_GPU_PROFILE=off __NV_DISABLE_EXPLICIT_SYNC=1"
  [render-intel]="OFM_GPU_PROFILE=off WEBKIT_WEB_RENDER_DEVICE_FILE=/dev/dri/renderD128"
  [render-nvidia]="OFM_GPU_PROFILE=off WEBKIT_WEB_RENDER_DEVICE_FILE=/dev/dri/renderD129"
  [disable-gbm]="OFM_GPU_PROFILE=off WEBKIT_DMABUF_RENDERER_DISABLE_GBM=1"
  [force-shm]="OFM_GPU_PROFILE=off WEBKIT_DMABUF_RENDERER_FORCE_SHM=1"
  [shipped]="OFM_GPU_PROFILE=safe"
  [no-compositing]="OFM_GPU_PROFILE=off WEBKIT_DISABLE_COMPOSITING_MODE=1"
  [xwayland]="OFM_GPU_PROFILE=off GDK_BACKEND=x11"
)

ROW_ORDER=(baseline auto explicit-sync render-intel render-nvidia disable-gbm force-shm shipped no-compositing xwayland)

# Only the `auto` rows record a startup attempt, and each row is killed as soon as its result
# lands — sooner than the app's startup grace period — so an `auto` row leaves its failure count
# behind. Left in place it would push the next `auto` row onto the conservative fallback and
# silently measure the wrong configuration.
#
# Globbed rather than named exactly: this file lives in Rust (`startup_state_path` in
# src-tauri/src/platform/linux_graphics.rs) and the two drifted apart once already when it was
# renamed, which turned this cleanup into a silent no-op.
STATE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/openfootmanager"

requested=("$@")
if [ ${#requested[@]} -eq 0 ]; then
  requested=("${ROW_ORDER[@]}")
fi

mkdir -p "$RESULTS_DIR"

echo "== environment =="
echo "session:  ${XDG_SESSION_TYPE:-unknown} / ${XDG_CURRENT_DESKTOP:-unknown}"
echo "webkit:   $(pkg-config --modversion webkit2gtk-4.1 2>/dev/null || echo unknown)"
echo "gpus:     $(for d in /sys/class/drm/card[0-9]; do
                    [ -e "$d/device/uevent" ] && grep -m1 '^DRIVER=' "$d/device/uevent" | cut -d= -f2
                  done | paste -sd, -)"
echo

for row in "${requested[@]}"; do
  env_spec="${ROWS[$row]:-}"
  if [ -z "$env_spec" ]; then
    echo "!! unknown row: $row" >&2
    exit 1
  fi

  echo "== row: $row =="
  echo "   env: $env_spec"

  rm -f "$STATE_DIR"/startup-*

  # Drop this row's previous result before running it. Without this, a row that fails to produce
  # a measurement — collector cannot bind, app crashes, row times out — still passes the
  # existence check below against a file from an earlier run, and report.mjs presents that stale
  # measurement as this configuration's evidence.
  rm -f "$RESULTS_DIR/$row.json"

  # `tauri dev` starts its own Vite via beforeDevCommand and aborts the whole row if 1420 is
  # taken, so wait for the previous row's server to let go. Nothing is killed by name here: if
  # something outside this script is holding the port, that is worth failing loudly for rather
  # than silently terminating someone's work.
  for _ in $(seq 15); do
    ss -ltn 2>/dev/null | grep -q ':1420 ' || break
    sleep 1
  done
  if ss -ltn 2>/dev/null | grep -q ':1420 '; then
    echo "   !! port 1420 is still in use — stop whatever is holding it, then re-run" >&2
    exit 1
  fi

  # One collector per row, so a stale result cannot be mistaken for this row's.
  node scripts/perf/collector.mjs --out "$RESULTS_DIR" --expect 1 --label "$row" &
  collector_pid=$!
  sleep 1

  log="$RESULTS_DIR/$row.log"
  # shellcheck disable=SC2086
  # stdin from /dev/null, not inherited: `set -m` means a background job that reads the terminal
  # is sent SIGTTIN and *stopped* rather than killed. npm, cargo and tauri dev can all touch
  # stdin, and a stopped row looks exactly like a hang — a normal startup log, then nothing.
  env $env_spec VITE_OFM_BENCH="$row" npm run tauri dev >"$log" 2>&1 </dev/null &
  app_pid=$!

  # Wait for the collector to exit, which it does as soon as the result lands.
  waited=0
  while kill -0 "$collector_pid" 2>/dev/null; do
    if [ "$waited" -ge "$TIMEOUT_SECS" ]; then
      echo "   !! timed out after ${TIMEOUT_SECS}s — see $log" >&2
      kill "$collector_pid" 2>/dev/null || true
      break
    fi
    sleep 2
    waited=$((waited + 2))
  done

  # `npm run tauri dev` spawns cargo, which spawns vite and the app. `set -m` put them all in one
  # process group led by $app_pid, so a negative pid takes the whole tree down — and nothing else.
  kill -TERM -- -"$app_pid" 2>/dev/null || kill -TERM "$app_pid" 2>/dev/null || true
  wait "$app_pid" 2>/dev/null || true

  if [ ! -f "$RESULTS_DIR/$row.json" ]; then
    echo "   !! no result recorded for $row — see $log" >&2
  fi

  # Anything WebKit said about DMABuf or EGL is evidence in its own right.
  grep -iE 'dmabuf|egl|AcceleratedSurface|wayland' "$log" | sort -u | head -5 || true
  echo
  sleep 2
done

echo "Done. Results in $RESULTS_DIR — render the table with:"
echo "  node scripts/perf/report.mjs"
