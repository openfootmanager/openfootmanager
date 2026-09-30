import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const pluginMessage = vi.fn();
const logError = vi.fn();

vi.mock("@tauri-apps/plugin-dialog", () => ({
  message: (text: string, options?: unknown) => pluginMessage(text, options),
}));
vi.mock("./logger", () => ({
  logError: (text: string) => logError(text),
}));

import { showError } from "./errorDialog";

describe("showError", () => {
  beforeEach(() => {
    pluginMessage.mockReset().mockResolvedValue(undefined);
    logError.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("shows the detail in a native error dialog", async () => {
    await showError("Could not start your career", "The world package is missing.");

    expect(pluginMessage).toHaveBeenCalledTimes(1);
    const [text, options] = pluginMessage.mock.calls[0];
    expect(text).toBe("The world package is missing.");
    expect(options).toMatchObject({
      title: "Could not start your career",
      kind: "error",
    });
  });

  it("writes the error to the log file as well as showing it", async () => {
    await showError("Could not start your career", "The world package is missing.");

    expect(logError).toHaveBeenCalledTimes(1);
    const logged = logError.mock.calls[0][0] as string;
    expect(logged).toContain("Could not start your career");
    expect(logged).toContain("The world package is missing.");
  });

  it("logs before showing, so a dialog that never appears still leaves a trace", async () => {
    // `window.alert` was unreliable in the WebKitGTK webview, which is half the reason this
    // exists. If the replacement is unreliable too, the log line has to survive it — so the
    // ordering here is the actual guarantee, not an implementation detail.
    const order: string[] = [];
    logError.mockImplementation(() => order.push("log"));
    pluginMessage.mockImplementation(() => {
      order.push("dialog");
      return Promise.resolve();
    });

    await showError("Title", "Detail");

    expect(order).toEqual(["log", "dialog"]);
  });

  it("does not throw when the dialog plugin is unavailable", async () => {
    pluginMessage.mockRejectedValue(new Error("not permitted"));

    await expect(showError("Title", "Detail")).resolves.toBeUndefined();
    expect(logError).toHaveBeenCalled();
  });

  it("does not throw when the dialog plugin throws synchronously", async () => {
    pluginMessage.mockImplementation(() => {
      throw new Error("no plugin");
    });

    await expect(showError("Title", "Detail")).resolves.toBeUndefined();
  });
});
