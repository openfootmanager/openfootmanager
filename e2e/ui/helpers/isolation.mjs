// Refuses to run the app against anything but a throwaway home on a virtual display.
//
// The app reads and writes the player's saves and settings under XDG_DATA_HOME and
// XDG_CONFIG_HOME. A flow that starts a new game and plays on would overwrite a real career, so
// the suite starts only from run.sh, which makes a scratch HOME and points every XDG directory
// into it, and only on an X11 display (a Wayland session would open the window on the desktop).
import { realpathSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const XDG_DIRECTORIES = ["XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_STATE_HOME"];

function inside(parent, child) {
  const relative = path.relative(parent, child);
  return relative !== "" && !relative.startsWith("..") && !path.isAbsolute(relative);
}

function realTmpdir() {
  return realpathSync(tmpdir());
}

export function assertIsolated(env = process.env) {
  if (env.OFM_E2E_ISOLATED !== "1") {
    throw new Error("Refusing to start: run the suite through e2e/ui/run.sh, which isolates it.");
  }
  const home = env.HOME ? path.resolve(env.HOME) : "";
  if (!home || !inside(realTmpdir(), home)) {
    throw new Error(`Refusing to start: HOME (${env.HOME}) is not a scratch directory.`);
  }
  for (const name of XDG_DIRECTORIES) {
    const value = env[name];
    if (!value || !inside(home, path.resolve(value))) {
      throw new Error(`Refusing to start: ${name} (${value}) is not inside the scratch HOME.`);
    }
  }
  if (env.WAYLAND_DISPLAY || env.WAYLAND_SOCKET) {
    throw new Error("Refusing to start: a Wayland session is visible; the window would open on it.");
  }
  if (!env.DISPLAY) {
    throw new Error("Refusing to start: no DISPLAY; run under xvfb-run.");
  }
}
