// The guard is what keeps a stray `wdio run` from launching the app against the developer's own
// saves and settings, so it is tested on its own, with plain environments.
import assert from "node:assert/strict";
import { mkdtempSync, realpathSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";

import { assertIsolated } from "./isolation.mjs";

const scratch = realpathSync(mkdtempSync(path.join(tmpdir(), "ofm-e2e-guard-")));

function isolatedEnv(overrides = {}) {
  return {
    OFM_E2E_ISOLATED: "1",
    HOME: scratch,
    XDG_DATA_HOME: path.join(scratch, "data"),
    XDG_CONFIG_HOME: path.join(scratch, "config"),
    XDG_CACHE_HOME: path.join(scratch, "cache"),
    XDG_STATE_HOME: path.join(scratch, "state"),
    GDK_BACKEND: "x11",
    DISPLAY: ":99",
    ...overrides,
  };
}

test("an environment run.sh made is accepted", () => {
  assert.doesNotThrow(() => assertIsolated(isolatedEnv()));
});

test("a run that did not come from run.sh is refused", () => {
  assert.throws(() => assertIsolated(isolatedEnv({ OFM_E2E_ISOLATED: undefined })), /run\.sh/);
});

test("a HOME outside the temporary directory is refused", () => {
  assert.throws(() => assertIsolated(isolatedEnv({ HOME: "/home/someone" })), /HOME/);
});

test("an XDG directory pointing at the real data is refused", () => {
  assert.throws(
    () => assertIsolated(isolatedEnv({ XDG_DATA_HOME: "/home/someone/.local/share" })),
    /XDG_DATA_HOME/,
  );
  assert.throws(
    () => assertIsolated(isolatedEnv({ XDG_STATE_HOME: undefined })),
    /XDG_STATE_HOME/,
  );
});

test("a Wayland or non-X11 session is refused: the window would open on the desktop", () => {
  assert.throws(() => assertIsolated(isolatedEnv({ WAYLAND_DISPLAY: "wayland-0" })), /Wayland/);
  assert.throws(() => assertIsolated(isolatedEnv({ GDK_BACKEND: "wayland" })), /GDK_BACKEND/);
});

test("a missing display is refused", () => {
  assert.throws(() => assertIsolated(isolatedEnv({ DISPLAY: undefined })), /DISPLAY/);
});
