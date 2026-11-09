// Refuses to run the app against anything but a throwaway home on a display the suite started.
//
// The app reads and writes the player's saves and settings under XDG_DATA_HOME and
// XDG_CONFIG_HOME, and opens its window on DISPLAY. A flow that starts a new game and plays on
// would overwrite a real career and take over the player's screen, so the suite starts only from
// run.sh, which makes a scratch HOME, points every XDG directory into it, and starts an Xvfb of its
// own on a display number it chose.
//
// Variables alone prove nothing (anyone can set them), so what is checked is what they point at:
// real paths, after symlinks, and a live Xvfb process serving exactly the DISPLAY in use.
import { readFileSync, realpathSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const XDG_DIRECTORIES = ["XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_STATE_HOME"];

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

const procfs = {
  readCmdline: (pid) => readFileSync(`/proc/${pid}/cmdline`, "utf8"),
};

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
  const program = path.basename(argv[0] ?? "");
  if (program !== "Xvfb" || !argv.includes(display)) {
    throw new Error(
      `Refusing to start: DISPLAY ${display} is not served by an Xvfb of the suite's own ` +
        `(process ${pid} is ${argv.slice(0, 2).join(" ") || "unknown"}).`,
    );
  }
}

export function assertIsolated(env = process.env, processes = procfs) {
  if (env.OFM_E2E_ISOLATED !== "1") {
    throw new Error("Refusing to start: run the suite through e2e/ui/run.sh, which isolates it.");
  }
  const tmp = resolved(tmpdir());
  const home = env.HOME ? resolved(env.HOME) : null;
  if (!tmp || !home || !inside(tmp, home)) {
    throw new Error(`Refusing to start: HOME (${env.HOME}) is not a scratch directory.`);
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
