// Refuses to run the app against anything but a throwaway home on a display the suite started.
//
// The app reads and writes the player's saves and settings under XDG_DATA_HOME and
// XDG_CONFIG_HOME, and opens its window on DISPLAY. A flow that starts a new game and plays on
// would overwrite a real career and take over the player's screen, so the suite starts only from
// run.sh, which makes a scratch HOME, points every XDG directory into it, and starts an Xvfb of its
// own on a display number it chose.
//
// Variables alone prove nothing (anyone can set them, TMPDIR included), so what is checked is what
// they point at: real paths, after symlinks, under scratch roots fixed in this file; and a live
// Xvfb process started on exactly the DISPLAY in use.
import { existsSync, readFileSync, realpathSync } from "node:fs";
import path from "node:path";

const XDG_DIRECTORIES = ["XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_STATE_HOME"];

// Where a scratch home may live. Fixed here, never read from TMPDIR: that variable is the caller's
// to set, and a boundary taken from it moves to wherever it points, a real home included.
const SCRATCH_ROOTS = ["/tmp", "/dev/shm"];

function inside(parent, child) {
  const relative = path.relative(parent, child);
  return relative !== "" && !relative.startsWith("..") && !path.isAbsolute(relative);
}

/** The path as the filesystem sees it, symlinks followed; null when it does not exist. */
function resolved(candidate) {
  try {
    return realpathSync(candidate);
  } catch {
    return null;
  }
}

function scratchRoots() {
  return SCRATCH_ROOTS.filter((root) => existsSync(root)).map((root) => realpathSync(root));
}

const procfs = {
  readCmdline: (pid) => readFileSync(`/proc/${pid}/cmdline`, "utf8"),
};

/**
 * The process named by OFM_E2E_XVFB_PID must be an Xvfb started on exactly `display`. Xvfb takes
 * its display as the first argument (run.sh starts it as `Xvfb :N ...`), so that argument is what
 * is compared: ":0" appearing later, as an option's value, serves nothing.
 */
function assertOwnXvfbServes(display, pid, processes) {
  if (!pid || !/^[0-9]+$/.test(pid)) {
    throw new Error("Refusing to start: OFM_E2E_XVFB_PID is not set; run.sh starts the display.");
  }
  let argv;
  try {
    argv = processes.readCmdline(pid).split("\0").filter(Boolean);
  } catch {
    throw new Error(`Refusing to start: no Xvfb is running as process ${pid}.`);
  }
  if (path.basename(argv[0] ?? "") !== "Xvfb" || argv[1] !== display) {
    throw new Error(
      `Refusing to start: DISPLAY ${display} is not served by an Xvfb started on it ` +
        `(process ${pid} is ${argv.slice(0, 2).join(" ") || "unknown"}).`,
    );
  }
}

export function assertIsolated(env = process.env, processes = procfs) {
  if (env.OFM_E2E_ISOLATED !== "1") {
    throw new Error("Refusing to start: run the suite through e2e/ui/run.sh, which isolates it.");
  }
  const roots = scratchRoots();
  const home = env.HOME ? resolved(env.HOME) : null;
  if (!home || !roots.some((root) => inside(root, home))) {
    throw new Error(
      `Refusing to start: HOME (${env.HOME}) is not a scratch directory under ${SCRATCH_ROOTS.join(" or ")}.`,
    );
  }
  for (const name of XDG_DIRECTORIES) {
    const directory = env[name] ? resolved(env[name]) : null;
    if (!directory || !inside(home, directory)) {
      throw new Error(
        `Refusing to start: ${name} (${env[name]}) is missing or does not lead into the scratch HOME.`,
      );
    }
  }
  if (env.WAYLAND_DISPLAY || env.WAYLAND_SOCKET) {
    throw new Error(
      "Refusing to start: a Wayland session is visible; the window would open on it.",
    );
  }
  if (env.GDK_BACKEND !== "x11") {
    throw new Error("Refusing to start: GDK_BACKEND must be x11.");
  }
  if (!env.DISPLAY) {
    throw new Error("Refusing to start: no DISPLAY; run.sh starts one.");
  }
  assertOwnXvfbServes(env.DISPLAY, env.OFM_E2E_XVFB_PID, processes);
}
