import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { getPlayerCareer } from "../services/playerCareerService";
import type { PlayerCareer } from "../store/generated/PlayerCareer";
import { usePlayerCareer } from "./usePlayerCareer";

vi.mock("../services/playerCareerService", () => ({ getPlayerCareer: vi.fn() }));
const mocked = vi.mocked(getPlayerCareer);

const career: PlayerCareer = {
  by_season: [],
  by_club: [],
  totals: { appearances: 30, goals: 4, assists: 2 },
};

beforeEach(() => {
  mocked.mockReset();
  vi.spyOn(console, "error").mockImplementation(() => undefined);
});

describe("usePlayerCareer", () => {
  /**
   * Given the backend's career for a player
   * When the hook mounts
   * Then it reads loading, then the backend's career, asked for him
   */
  it("reads loading, then the backend's career", async () => {
    mocked.mockResolvedValue(career);

    const { result } = renderHook(() => usePlayerCareer("p1", "r1"));

    expect(result.current.status).toBe("loading");
    await waitFor(() => expect(result.current).toEqual({ status: "answered", value: career }));
    expect(mocked).toHaveBeenCalledWith("p1");
  });

  /**
   * Given an id no active player has (null) and a request that fails
   * When each is asked for
   * Then the first reads answered with no career, the second reads failed with no data
   */
  it("tells a failed load from no career", async () => {
    mocked.mockResolvedValueOnce(null);
    mocked.mockRejectedValueOnce("be.error.noActiveGameSession");

    const none = renderHook(() => usePlayerCareer("ghost", "r1"));
    const failed = renderHook(() => usePlayerCareer("failing", "r1"));

    await waitFor(() => expect(none.result.current).toEqual({ status: "answered", value: null }));
    await waitFor(() => expect(failed.result.current.status).toBe("failed"));
    expect(failed.result.current).not.toHaveProperty("value");
  });

  /**
   * Given a failed load
   * When the retry is asked for and the backend answers this time
   * Then it asks again and reads the career
   */
  it("asks again on retry", async () => {
    mocked.mockRejectedValueOnce("down");
    mocked.mockResolvedValueOnce(career);
    const { result } = renderHook(() => usePlayerCareer("p1", "r1"));
    await waitFor(() => expect(result.current.status).toBe("failed"));

    act(() => {
      if (result.current.status === "failed") result.current.retry();
    });

    await waitFor(() => expect(result.current).toEqual({ status: "answered", value: career }));
    expect(mocked).toHaveBeenCalledTimes(2);
  });

  /**
   * Given one player's career on screen
   * When another player's profile opens, whose career has not arrived
   * Then it reads loading, never the previous player's career
   */
  it("never shows the previous player's career", async () => {
    mocked.mockResolvedValueOnce(career);
    mocked.mockReturnValueOnce(new Promise<never>(() => undefined));
    const { result, rerender } = renderHook(({ id }) => usePlayerCareer(id, "r1"), {
      initialProps: { id: "p1" },
    });
    await waitFor(() => expect(result.current.status).toBe("answered"));

    rerender({ id: "p2" });

    expect(result.current.status).toBe("loading");
  });

  /**
   * Given his career on screen
   * When the revision changes (a move, a new day)
   * Then it is asked for again
   */
  it("asks again when the revision changes", async () => {
    mocked.mockResolvedValue(career);
    const { rerender } = renderHook(({ revision }) => usePlayerCareer("p1", revision), {
      initialProps: { revision: "r1" },
    });
    await waitFor(() => expect(mocked).toHaveBeenCalledTimes(1));

    rerender({ revision: "r2" });

    await waitFor(() => expect(mocked).toHaveBeenCalledTimes(2));
  });
});
