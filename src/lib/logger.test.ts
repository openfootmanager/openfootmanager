import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import { logError } from "./logger";

/** Lets a `.catch()` attached inside the logger run before a test asserts on its effect. */
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("logger", () => {
  beforeEach(() => {
    invokeMock.mockReset().mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  /**
   * Given the app's log plugin
   * When an error is logged
   * Then it is sent to the plugin's `log` command at the error level (5), in the file the Rust side
   *      writes (tauri-plugin-log, targets stdout and the log directory).
   */
  it("sends an error to the log plugin at the error level", async () => {
    logError("render exploded");
    await flush();

    expect(invokeMock).toHaveBeenCalledWith("plugin:log|log", {
      level: 5,
      message: "render exploded",
    });
  });

  /**
   * Given a log plugin that is not there (outside Tauri, or a missing capability), rejecting or
   *       throwing on the call
   * When an error is logged
   * Then the logger does not throw and no rejection escapes: the logger failing must never be
   *      worse than the bug it reports.
   */
  it("does not throw or reject when the plugin is unavailable", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    invokeMock.mockRejectedValueOnce(new Error("no plugin"));
    expect(() => logError("one")).not.toThrow();
    invokeMock.mockImplementationOnce(() => {
      throw new Error("no ipc");
    });
    expect(() => logError("two")).not.toThrow();
    await flush();

    expect(warn).toHaveBeenCalled();
  });
});
