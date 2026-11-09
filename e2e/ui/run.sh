#!/usr/bin/env bash
# Runs the real-UI suite against a built app, in a throwaway home on a display of its own.
#
#   OFM_E2E_APP=/path/to/openfootmanager  bash e2e/ui/run.sh [wdio args]
#
# Build the app first with its frontend embedded (`npm run tauri build -- --debug --no-bundle`
# from the repository root); a plain `cargo build` binary opens the dev server's URL instead.
# Needs Xvfb (the xvfb package), WebKitWebDriver (webkit2gtk-driver, or webkitgtk-webdriver on
# newer Ubuntu) and tauri-driver (`cargo install tauri-driver --locked`).
#
# The display is started here, on a number this script picks, and its process id is handed to
# the suite (OFM_E2E_XVFB_PID), which checks that it really is an Xvfb serving that display. The
# player's own display is never used.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
app="$(realpath -m "${OFM_E2E_APP:-$here/../../src-tauri/target/debug/openfootmanager}")"
[ -x "$app" ] || { echo "run.sh: no app at $app (set OFM_E2E_APP)" >&2; exit 2; }
command -v Xvfb >/dev/null || { echo "run.sh: Xvfb is required (the xvfb package)" >&2; exit 2; }

# Never the real home: everything the app writes lands in this directory, which is removed after.
# Always under /tmp, never TMPDIR: the guard accepts only /tmp and /dev/shm as scratch roots.
scratch="$(mktemp -d /tmp/ofm-ui-e2e.XXXXXX)"
xvfb_pid=""
cleanup() {
  if [ -n "$xvfb_pid" ]; then kill "$xvfb_pid" 2>/dev/null || true; fi
  rm -rf "${scratch:?}"
}
trap cleanup EXIT
mkdir -p "$scratch/data" "$scratch/config" "$scratch/cache" "$scratch/state"

# A display number nothing else is using. :0 to :99 are left to real sessions.
display=""
for number in $(seq 100 199); do
  if [ -e "/tmp/.X11-unix/X$number" ] || [ -e "/tmp/.X$number-lock" ]; then continue; fi
  Xvfb ":$number" -screen 0 1280x800x24 -nolisten tcp >"$scratch/xvfb.log" 2>&1 &
  xvfb_pid=$!
  for _ in $(seq 1 50); do
    [ -S "/tmp/.X11-unix/X$number" ] && break
    kill -0 "$xvfb_pid" 2>/dev/null || break
    sleep 0.1
  done
  if kill -0 "$xvfb_pid" 2>/dev/null && [ -S "/tmp/.X11-unix/X$number" ]; then
    display=":$number"
    break
  fi
  # This one never came up (or came up without its socket): stop it before trying another number,
  # or nothing would be left holding its pid to stop it at the end.
  kill "$xvfb_pid" 2>/dev/null || true
  wait "$xvfb_pid" 2>/dev/null || true
  xvfb_pid=""
done
[ -n "$display" ] || { echo "run.sh: could not start an Xvfb" >&2; cat "$scratch/xvfb.log" >&2; exit 2; }

# Resolved here, before the cd below: a relative path means relative to where the caller stood.
artifacts="$(realpath -m "${OFM_E2E_ARTIFACTS:-$here/artifacts}")"
mkdir -p "$artifacts"

cd "$here"
env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u XAUTHORITY \
  OFM_E2E_ISOLATED=1 \
  OFM_E2E_XVFB_PID="$xvfb_pid" \
  OFM_E2E_APP="$app" \
  OFM_E2E_ARTIFACTS="$artifacts" \
  DISPLAY="$display" \
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
  npx --no-install wdio run wdio.conf.mjs "$@"
