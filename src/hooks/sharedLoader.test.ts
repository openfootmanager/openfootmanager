import { act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createSharedLoader } from "./sharedLoader";

beforeEach(() => vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] }));
afterEach(() => vi.useRealTimers());

describe("a loader with more than one key", () => {
  /**
   * Given a key that loaded, then another key that loaded in its place (the squad changed)
   * When a screen subscribes to the first key again (the squad is back as it was)
   * Then it is shown that key's value, not left waiting for a request that already finished.
   */
  it("shows a key again after another key took its place", async () => {
    const fetchValue = vi.fn(async (key: string) => `value of ${key}`);
    const loader = createSharedLoader(fetchValue, "test value");
    const first = loader.subscribe("a", () => {});
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    first();
    const second = loader.subscribe("b", () => {});
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    second();
    const shown = vi.fn();

    loader.subscribe("a", shown);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(shown).toHaveBeenCalledWith("value of a");
  });

  /**
   * Given a key whose request failed and another key mounted at the same time that succeeds
   * When the retry delay passes
   * Then the failed key is asked again: the other key's success does not cancel its retry.
   */
  it("keeps retrying a failed key while another key succeeds", async () => {
    const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
    const fetchValue = vi
      .fn<(key: string) => Promise<string>>()
      .mockRejectedValueOnce(new Error("offline"))
      .mockImplementation(async (key) => {
        // Later than a's failure, so the success is what meets a waiting retry timer.
        await new Promise((resolve) => setTimeout(resolve, 10));
        return `value of ${key}`;
      });
    const loader = createSharedLoader(fetchValue, "test value");
    const shownA = vi.fn();
    loader.subscribe("a", shownA);
    loader.subscribe("b", () => {});
    await act(async () => {
      await vi.advanceTimersByTimeAsync(20);
    });
    expect(shownA).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_100);
    });

    expect(shownA).toHaveBeenCalledWith("value of a");
    quiet.mockRestore();
  });

  /**
   * Given a key whose request failed after its screen had moved on to another key, and the new
   *       key's own first request failing later
   * When the new key's failure is a second after it happened
   * Then it retries then, not earlier: the old key's failure neither raised the delay nor armed
   *      the timer.
   */
  it("counts only the failures of keys a screen still mounts", async () => {
    const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
    const later = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
    const fetchValue = vi.fn<(key: string) => Promise<string>>(async (key) => {
      if (key === "old") {
        await later(5);
        throw new Error("offline");
      }
      if (fetchValue.mock.calls.filter(([asked]) => asked === "new").length === 1) {
        await later(800);
        throw new Error("offline");
      }
      return "value of new";
    });
    const loader = createSharedLoader(fetchValue, "test value");
    loader.subscribe("old", () => {})();
    const shown = vi.fn();
    loader.subscribe("new", shown);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(fetchValue.mock.calls.filter(([asked]) => asked === "new")).toHaveLength(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(999);
    });
    expect(fetchValue.mock.calls.filter(([asked]) => asked === "new")).toHaveLength(1);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });

    expect(shown).toHaveBeenCalledWith("value of new");
    quiet.mockRestore();
  });
});
