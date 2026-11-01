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
});
