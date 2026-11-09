// The guard is what keeps a stray `wdio run` from launching the app against the developer's own
// saves, settings or screen, so it is tested on its own, with plain environments and a stand-in
// for /proc.
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, realpathSync, symlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";

import { assertIsolated } from "./isolation.mjs";

const XVFB_PID = "4242";

/** A scratch home with its four XDG directories, the way run.sh makes one. */
function scratchHome() {
  const home = realpathSync(mkdtempSync(path.join(tmpdir(), "ofm-e2e-guard-")));
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
  const realData = realpathSync(mkdtempSync(path.join(tmpdir(), "ofm-e2e-real-data-")));
  const link = path.join(home, "escape");
  symlinkSync(realData, link);
  assert.throws(
    () => assertIsolated(isolatedEnv(home, { XDG_DATA_HOME: link }), ourXvfb),
    /XDG_DATA_HOME/,
  );
});

// A HOME under the temporary directory that is really a link to a directory outside it. /dev/shm
// is a writable directory outside /tmp on Linux; where it is not, there is nothing to test with.
const outsideTmp = existsSync("/dev/shm") ? "/dev/shm" : null;

test("a scratch HOME that is itself a symlink out of the temporary directory is refused", {
  skip: outsideTmp === null,
}, () => {
  const elsewhere = realpathSync(mkdtempSync(path.join(outsideTmp, "ofm-e2e-elsewhere-")));
  for (const name of ["data", "config", "cache", "state"]) mkdirSync(path.join(elsewhere, name));
  const home = path.join(realpathSync(tmpdir()), `ofm-e2e-link-${process.pid}`);
  symlinkSync(elsewhere, home);
  assert.throws(() => assertIsolated(isolatedEnv(home), ourXvfb), /HOME/);
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

test("no Xvfb of the suite's own is refused", () => {
  assert.throws(
    () => assertIsolated(isolatedEnv(scratchHome(), { OFM_E2E_XVFB_PID: undefined }), ourXvfb),
    /OFM_E2E_XVFB_PID/,
  );
  assert.throws(() => assertIsolated(isolatedEnv(scratchHome()), processes({})), /Xvfb/);
});
