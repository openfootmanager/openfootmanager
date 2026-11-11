import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PositionGroupEntry } from "../services/positionService";
import { resetPositionGroupsCache, usePositionGroups } from "./usePositionGroups";

vi.mock("../services/positionService", () => ({ getPositionGroups: vi.fn() }));

const { getPositionGroups } = await import("../services/positionService");
const mockedGroups = vi.mocked(getPositionGroups);

const table = (): PositionGroupEntry[] => [
  { position: "RightWingBack", group: "Defender", rank: 0 },
  { position: "Striker", group: "Forward", rank: 0 },
];

beforeEach(() => {
  mockedGroups.mockReset();
  resetPositionGroupsCache();
});

describe("usePositionGroups", () => {
  /**
   * Given the backend's table of position groups
   * When the hook mounts and a position is read
   * Then it has no group until the table arrives, then the table's group.
   */
  it("reads a position's group from the backend's table", async () => {
    mockedGroups.mockResolvedValue(table());

    const { result } = renderHook(() => usePositionGroups());
    expect(result.current("RightWingBack")).toBeNull();

    await waitFor(() => expect(result.current("RightWingBack")).toBe("Defender"));
    expect(result.current("Striker")).toBe("Forward");
    expect(result.current("Unheard Of")).toBeNull();
  });

  /**
   * Given several mounts of the hook, as a table of badges has
   * When they all mount
   * Then the backend is asked once.
   */
  it("asks the backend once for however many mounts", async () => {
    mockedGroups.mockResolvedValue(table());

    const rows = [1, 2, 3].map(() => renderHook(() => usePositionGroups()));
    await waitFor(() => expect(rows[2].result.current("Striker")).toBe("Forward"));

    expect(mockedGroups).toHaveBeenCalledTimes(1);
  });

  /**
   * Given a first request that fails
   * When a screen mounts the hook again
   * Then it asks again instead of keeping the failure, and the first mount gets the table too.
   */
  it("asks again after a failed request and hands the table to every mount", async () => {
    const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
    mockedGroups.mockRejectedValueOnce(new Error("offline")).mockResolvedValue(table());

    const first = renderHook(() => usePositionGroups());
    await waitFor(() => expect(quiet).toHaveBeenCalled());
    const second = renderHook(() => usePositionGroups());

    await waitFor(() => expect(second.result.current("Striker")).toBe("Forward"));
    await waitFor(() => expect(first.result.current("Striker")).toBe("Forward"));
    expect(mockedGroups).toHaveBeenCalledTimes(2);
    quiet.mockRestore();
  });

  /**
   * Given a backend answer that is not a table (a screen test's stand-in, or a broken reply)
   * When the hook reads a position
   * Then it names no group instead of throwing, so a badge still draws.
   */
  it("names no group when the answer is not a table", async () => {
    mockedGroups.mockResolvedValue({ unexpected: true } as unknown as PositionGroupEntry[]);

    const { result } = renderHook(() => usePositionGroups());
    await act(async () => {
      await mockedGroups.mock.results[0]?.value;
    });

    expect(result.current("Striker")).toBeNull();
  });

  describe("when the request fails and the screen stays mounted", () => {
    beforeEach(() => vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] }));
    afterEach(() => vi.useRealTimers());

    /**
     * Given a request that fails once and a screen that stays mounted
     * When the first retry delay passes
     * Then the hook asks again on its own and the table arrives, with no remount.
     */
    it("asks again on its own, so a waiting screen is not stuck", async () => {
      const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
      mockedGroups.mockRejectedValueOnce(new Error("offline")).mockResolvedValue(table());

      const { result } = renderHook(() => usePositionGroups());
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(result.current("Striker")).toBeNull();
      expect(mockedGroups).toHaveBeenCalledTimes(1);

      await act(async () => {
        await vi.advanceTimersByTimeAsync(1_000);
      });

      expect(mockedGroups).toHaveBeenCalledTimes(2);
      expect(result.current("Striker")).toBe("Forward");
      quiet.mockRestore();
    });

    /**
     * Given a request that keeps failing
     * When the delays pass
     * Then each retry waits longer than the last (1s, then 2s), so a dead backend is not hammered.
     */
    it("waits longer after each failure in a row", async () => {
      const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
      mockedGroups.mockRejectedValue(new Error("offline"));

      renderHook(() => usePositionGroups());
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1_000);
      });
      expect(mockedGroups).toHaveBeenCalledTimes(2);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1_999);
      });
      expect(mockedGroups).toHaveBeenCalledTimes(2);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1);
      });
      expect(mockedGroups).toHaveBeenCalledTimes(3);
      quiet.mockRestore();
    });

    /**
     * Given a failed request whose retry is pending, and its screen leaves and another mounts
     * When the new mount's own request fails too
     * Then its retry comes one second after that failure: the old timer is gone, and the old
     *      failures do not stretch the new mount's first wait.
     */
    it("starts a later mount's retries from the first delay", async () => {
      const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
      mockedGroups.mockRejectedValue(new Error("offline"));

      const first = renderHook(() => usePositionGroups());
      await act(async () => {
        await vi.advanceTimersByTimeAsync(500);
      });
      first.unmount();
      renderHook(() => usePositionGroups());
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(mockedGroups).toHaveBeenCalledTimes(2);

      // 500ms later the first mount's old deadline (1s after its failure) would have fired.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(500);
      });
      expect(mockedGroups).toHaveBeenCalledTimes(2);

      // One second after the second failure, not two.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(500);
      });
      expect(mockedGroups).toHaveBeenCalledTimes(3);
      quiet.mockRestore();
    });

    /**
     * Given a request that failed and a screen that then unmounts
     * When the retry delay passes
     * Then nothing asks again: there is no one left to wait for the table.
     */
    it("stops retrying once every screen has gone", async () => {
      const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
      mockedGroups.mockRejectedValue(new Error("offline"));

      const { unmount } = renderHook(() => usePositionGroups());
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      unmount();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(60_000);
      });

      expect(mockedGroups).toHaveBeenCalledTimes(1);
      quiet.mockRestore();
    });
  });
});
