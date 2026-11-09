// The guard is what keeps a stray `wdio run` from launching the app against the developer's own
// saves, settings or screen, so it is tested on its own, with plain environments and a stand-in
// for /proc.
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, symlinkSync } from "node:fs";
import path from "node:path";
import { after, test } from "node:test";

import { assertIsolated } from "./isolation.mjs";

const XVFB_PID = "4242";

// The guard accepts scratch homes under /tmp and /dev/shm only, and not wherever TMPDIR says: the
// variable is the caller's, so it can be made to name a real home.
const SCRATCH_ROOT = "/tmp";

// Everything a test creates is removed afterwards, whether or not the test passed: a persistent
// machine would otherwise collect scratch homes, links and /dev/shm directories run after run.
const created = [];
after(() => {
  for (const entry of created) rmSync(entry, { recursive: true, force: true });
});

/** Registers `entry` for removal and hands it back. */
function track(entry) {
  created.push(entry);
  return entry;
}

/** A scratch home with its four XDG directories, the way run.sh makes one. */
function scratchHome() {
  const home = track(realpathSync(mkdtempSync(path.join(SCRATCH_ROOT, "ofm-e2e-guard-"))));
  for (const name of ["data", "config", "cache", "state"]) mkdirSync(path.join(home, name));
  return home;
}

function isolatedEnv(home, overrides = {}) {
  return {
    OFM_E2E_ISOLATED: "1",
    HOME: home,
    XDG_DATA_HOME: path.join(home, "data"),
    XDG_CONFIG_HOME: path.join(home, "config"),
    XDG_CACHE_HOME: path.join(home, "cache"),
    XDG_STATE_HOME: path.join(home, "state"),
    GDK_BACKEND: "x11",
    DISPLAY: ":107",
    OFM_E2E_XVFB_PID: XVFB_PID,
    ...overrides,
  };
}

/** What /proc/<pid>/cmdline says for each process the test pretends exists. */
function processes(table) {
  return {
    readCmdline(pid) {
      if (!(pid in table)) throw new Error(`no such process ${pid}`);
      return table[pid];
    },
  };
}

/** A command line as /proc shows it: each argument ends in a NUL. */
const cmdline = (...argv) => `${argv.join("\u0000")}\u0000`;

const xvfbOn = (display) =>
  cmdline("Xvfb", display, "-screen", "0", "1280x800x24", "-nolisten", "tcp");

const ourXvfb = processes({ [XVFB_PID]: xvfbOn(":107") });

test("an environment run.sh made is accepted", () => {
  assert.doesNotThrow(() => assertIsolated(isolatedEnv(scratchHome()), ourXvfb));
});

test("a run that did not come from run.sh is refused", () => {
  assert.throws(
    () => assertIsolated(isolatedEnv(scratchHome(), { OFM_E2E_ISOLATED: undefined }), ourXvfb),
    /run\.sh/,
  );
});

test("a HOME outside the temporary directory is refused", () => {
  assert.throws(
    () => assertIsolated(isolatedEnv(scratchHome(), { HOME: "/home/someone" }), ourXvfb),
    /HOME/,
  );
});

test("an XDG directory pointing at the real data is refused", () => {
  const home = scratchHome();
  assert.throws(
    () =>
      assertIsolated(isolatedEnv(home, { XDG_DATA_HOME: "/home/someone/.local/share" }), ourXvfb),
    /XDG_DATA_HOME/,
  );
  assert.throws(
    () => assertIsolated(isolatedEnv(home, { XDG_STATE_HOME: undefined }), ourXvfb),
    /XDG_STATE_HOME/,
  );
});

test("an XDG directory that is a symlink out of the scratch home is refused", () => {
  // The path reads as inside the scratch home; what it opens is the player's real data.
  const home = scratchHome();
  const realData = track(realpathSync(mkdtempSync(path.join(SCRATCH_ROOT, "ofm-e2e-real-data-"))));
  const link = path.join(home, "escape");
  symlinkSync(realData, link);
  assert.throws(
    () => assertIsolated(isolatedEnv(home, { XDG_DATA_HOME: link }), ourXvfb),
    /XDG_DATA_HOME/,
  );
});

// A directory that is none of the allowed scratch roots, standing in for a real home: the working
// directory. Where that is itself under /tmp or /dev/shm (a checkout kept there), nothing here is
// outside the roots, and the tests that need such a directory are skipped.
const SCRATCH_ROOTS = ["/tmp", "/dev/shm"].filter(existsSync).map((root) => realpathSync(root));
const workingDirectory = realpathSync(process.cwd());
const cwdIsOutsideTheRoots = SCRATCH_ROOTS.every((root) => {
  const relative = path.relative(root, workingDirectory);
  return relative.startsWith("..") || path.isAbsolute(relative);
});
const needsAHomeOutsideTheRoots = { skip: !cwdIsOutsideTheRoots };

/** A directory with the four XDG directories in it, under the working directory (a "real home"). */
function homeOutsideTheRoots() {
  const home = track(realpathSync(mkdtempSync(path.join(workingDirectory, ".ofm-guard-home-"))));
  for (const name of ["data", "config", "cache", "state"]) mkdirSync(path.join(home, name));
  return home;
}

test(
  "a scratch HOME that is itself a symlink out of the scratch roots is refused",
  needsAHomeOutsideTheRoots,
  () => {
    const elsewhere = homeOutsideTheRoots();
    const home = track(path.join(SCRATCH_ROOT, `ofm-e2e-link-${process.pid}`));
    symlinkSync(elsewhere, home);
    assert.throws(() => assertIsolated(isolatedEnv(home), ourXvfb), /HOME/);
  },
);

test("a real home is refused even when TMPDIR is forged to contain it", needsAHomeOutsideTheRoots, () => {
  // tmpdir() follows the caller's TMPDIR, so a boundary taken from it moves to wherever TMPDIR
  // points. The scratch roots are fixed, whatever the variable says.
  const home = homeOutsideTheRoots();
  const saved = process.env.TMPDIR;
  process.env.TMPDIR = path.dirname(home);
  try {
    assert.throws(() => assertIsolated(isolatedEnv(home), ourXvfb), /HOME/);
  } finally {
    if (saved === undefined) delete process.env.TMPDIR;
    else process.env.TMPDIR = saved;
  }
});

test("a Wayland or non-X11 session is refused: the window would open on the desktop", () => {
  const home = scratchHome();
  assert.throws(
    () => assertIsolated(isolatedEnv(home, { WAYLAND_DISPLAY: "wayland-0" }), ourXvfb),
    /Wayland/,
  );
  assert.throws(
    () => assertIsolated(isolatedEnv(home, { GDK_BACKEND: "wayland" }), ourXvfb),
    /GDK_BACKEND/,
  );
});

test("a missing display is refused", () => {
  assert.throws(
    () => assertIsolated(isolatedEnv(scratchHome(), { DISPLAY: undefined }), ourXvfb),
    /DISPLAY/,
  );
});

test("the developer's own X11 display is refused, even with every other variable right", () => {
  // DISPLAY=:0 is a real session's. Nothing the suite started serves it, so the process the
  // environment names is not an Xvfb for :0, however well the other variables are set.
  const real = processes({ [XVFB_PID]: xvfbOn(":107") });
  assert.throws(() => assertIsolated(isolatedEnv(scratchHome(), { DISPLAY: ":0" }), real), /Xvfb/);
  const xorg = processes({ [XVFB_PID]: cmdline("/usr/lib/Xorg", ":0", "-nolisten", "tcp") });
  assert.throws(() => assertIsolated(isolatedEnv(scratchHome(), { DISPLAY: ":0" }), xorg), /Xvfb/);
});

test("a display that only appears as another option's value is refused", () => {
  // `Xvfb :107 -auth :0` serves :107; the :0 is the path of an auth file. Finding ":0" anywhere in
  // the arguments would let DISPLAY=:0 (a real session) through, so only the display Xvfb itself
  // is started on counts.
  const sneaky = processes({ [XVFB_PID]: cmdline("Xvfb", ":107", "-auth", ":0") });
  assert.throws(
    () => assertIsolated(isolatedEnv(scratchHome(), { DISPLAY: ":0" }), sneaky),
    /Xvfb/,
  );
  const reordered = processes({ [XVFB_PID]: cmdline("Xvfb", "-auth", ":0", ":107") });
  assert.throws(
    () => assertIsolated(isolatedEnv(scratchHome(), { DISPLAY: ":0" }), reordered),
    /Xvfb/,
  );
});

test("no Xvfb of the suite's own is refused", () => {
  assert.throws(
    () => assertIsolated(isolatedEnv(scratchHome(), { OFM_E2E_XVFB_PID: undefined }), ourXvfb),
    /OFM_E2E_XVFB_PID/,
  );
  assert.throws(() => assertIsolated(isolatedEnv(scratchHome()), processes({})), /Xvfb/);
});
