import { describe, expect, it } from "vitest";

import capabilities from "../src-tauri/capabilities/default.json";

/**
 * Tauri commands the frontend calls are only permitted if the capability file
 * grants them. A missing grant fails at runtime, inside a `catch`, so it shows
 * up as a console error on a user's machine and nowhere in CI — which is how
 * the window title silently kept its build-time value for so long.
 */
describe("main window capabilities", () => {
  it("grants the window permissions the app actually uses", () => {
    // App.tsx calls getCurrentWindow().setTitle() on mount to append the
    // version; without this grant the call throws and the title never updates.
    expect(capabilities.permissions).toContain("core:window:allow-set-title");
  });

  it("keeps the grants the shell already depends on", () => {
    for (const permission of [
      "core:default",
      "core:window:allow-destroy",
      "core:window:allow-close",
      "dialog:allow-open",
      "dialog:allow-save",
    ]) {
      expect(capabilities.permissions).toContain(permission);
    }
  });

  it("lets the frontend write to the log file", () => {
    // src/lib/logger.ts forwards console.error and every uncaught error to the shared log file.
    // Without this grant those calls reject, the logger falls back to the console, and a bug
    // report's logs go back to describing only the Rust half — silently, since the fallback is
    // exactly what makes the failure invisible.
    expect(capabilities.permissions).toContain("log:default");
  });
});
