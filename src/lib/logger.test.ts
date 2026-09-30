import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const pluginError = vi.fn();
const pluginWarn = vi.fn();
const pluginInfo = vi.fn();

vi.mock("@tauri-apps/plugin-log", () => ({
  error: (message: string) => pluginError(message),
  warn: (message: string) => pluginWarn(message),
  info: (message: string) => pluginInfo(message),
}));

import { installConsoleForwarding, logError, logInfo } from "./logger";

/** Lets a `.catch()` attached inside the logger run before we assert on it. */
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("logger", () => {
  beforeEach(() => {
    pluginError.mockReset().mockResolvedValue(undefined);
    pluginWarn.mockReset().mockResolvedValue(undefined);
    pluginInfo.mockReset().mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("sends messages to the log file", async () => {
    logError("boom");
    logInfo("hello");
    await flush();

    expect(pluginError).toHaveBeenCalledWith("boom");
    expect(pluginInfo).toHaveBeenCalledWith("hello");
  });

  it("does not throw when the log plugin is unavailable", async () => {
    // Outside Tauri — a browser, a test, a webview whose plugin failed to register — the
    // plugin's functions are `invoke()` calls that reject. The logger has to absorb that.
    pluginError.mockRejectedValue(new Error("plugin missing"));
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});

    expect(() => logError("boom")).not.toThrow();
    await flush();

    // Reaching the console fallback is the observable proof that the rejection was caught, and
    // so did not surface as an unhandled rejection.
    expect(consoleError).toHaveBeenCalled();
  });

  it("does not throw when the log plugin throws synchronously", () => {
    pluginError.mockImplementation(() => {
      throw new Error("sync boom");
    });
    vi.spyOn(console, "error").mockImplementation(() => {});

    expect(() => logError("boom")).not.toThrow();
  });
});

describe("console forwarding", () => {
  let restore: (() => void) | undefined;

  beforeEach(() => {
    pluginError.mockReset().mockResolvedValue(undefined);
    pluginWarn.mockReset().mockResolvedValue(undefined);
  });

  afterEach(() => {
    restore?.();
    restore = undefined;
    vi.restoreAllMocks();
  });

  it("sends existing console.error calls to the log file", async () => {
    const original = vi.spyOn(console, "error").mockImplementation(() => {});
    restore = installConsoleForwarding();

    console.error("something broke", { detail: 1 });
    await flush();

    expect(pluginError).toHaveBeenCalledTimes(1);
    expect(pluginError.mock.calls[0][0]).toContain("something broke");
    // The original console behaviour has to survive, or devtools goes quiet.
    expect(original).toHaveBeenCalled();
  });

  it("forwards console.warn too", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    restore = installConsoleForwarding();

    console.warn("hmm");
    await flush();

    expect(pluginWarn).toHaveBeenCalledTimes(1);
  });

  it("does not recurse when the log plugin itself writes to the console", async () => {
    // The plugin failing is the dangerous case: its rejection reaches the console fallback, which
    // is now forwarded, which calls the plugin again. Without a guard this is an infinite loop
    // that hangs the webview rather than reporting anything.
    vi.spyOn(console, "error").mockImplementation(() => {});
    pluginError.mockRejectedValue(new Error("plugin missing"));
    restore = installConsoleForwarding();

    console.error("first");
    await flush();
    await flush();

    expect(pluginError).toHaveBeenCalledTimes(1);
  });

  it("never throws at the call site, whatever it is handed", async () => {
    // A null-prototype cyclic object defeats both renderers: `JSON.stringify` refuses the cycle,
    // and `String()` has no `toString` to convert through, so it throws too. That throw came out
    // of the patched `console.error` at whatever call site was reporting the original problem —
    // losing the error being logged and replacing it with a different one. The `unhandledrejection`
    // handler reaches here with a rejection reason that can be any value at all.
    const hostile: Record<string, unknown> = Object.create(null);
    hostile.self = hostile;
    expect(() => JSON.stringify(hostile)).toThrow();
    expect(() => String(hostile)).toThrow();

    vi.spyOn(console, "error").mockImplementation(() => {});
    restore = installConsoleForwarding();

    expect(() => console.error("while loading", hostile)).not.toThrow();
    await flush();

    expect(pluginError).toHaveBeenCalledWith("while loading [unprintable]");
  });

  it("restores the original console when uninstalled", async () => {
    const original = console.error;
    restore = installConsoleForwarding();
    expect(console.error).not.toBe(original);

    restore();
    restore = undefined;
    expect(console.error).toBe(original);
  });
});
