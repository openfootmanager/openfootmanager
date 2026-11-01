import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { PotentialOpinion } from "../store/generated/PotentialOpinion";
import { getPotentialOpinion } from "../services/scoutingService";
import { usePotentialOpinion } from "./usePotentialOpinion";

vi.mock("../services/scoutingService", () => ({ getPotentialOpinion: vi.fn() }));
const mocked = vi.mocked(getPotentialOpinion);

/** The coaches' opinion of a player, 76-84. */
const answer = {
  low: 76,
  high: 84,
  band: "A",
  certainty: "fairly_sure",
  source: { kind: "coaches" },
  wonderkid: false,
} satisfies PotentialOpinion;

beforeEach(() => {
  mocked.mockReset();
  vi.spyOn(console, "error").mockImplementation(() => undefined);
});

describe("usePotentialOpinion", () => {
  /**
   * Given the backend's answer about a player
   * When the hook mounts
   * Then it reads loading, then the backend's answer, asked for him
   */
  it("reads loading, then the backend's answer", async () => {
    mocked.mockResolvedValue(answer);

    const { result } = renderHook(() => usePotentialOpinion("p1", "r1"));

    expect(result.current.status).toBe("loading");
    await waitFor(() => expect(result.current).toEqual({ status: "answered", value: answer }));
    expect(mocked).toHaveBeenCalledWith("p1");
  });

  /**
   * Given a player the club knows nothing about (no opinion: the backend answers null), and another whose request fails
   * When each is asked for
   * Then the first reads answered with no opinion, the second reads failed with no data: a
   *      failed load is never "nothing known"
   */
  it("tells a failed load from nothing known", async () => {
    mocked.mockResolvedValueOnce(null);
    mocked.mockRejectedValueOnce("be.error.noActiveGameSession");

    const unknown = renderHook(() => usePotentialOpinion("unknown", "r1"));
    const failed = renderHook(() => usePotentialOpinion("failing", "r1"));

    await waitFor(() =>
      expect(unknown.result.current).toEqual({ status: "answered", value: null }),
    );
    await waitFor(() => expect(failed.result.current.status).toBe("failed"));
    expect(failed.result.current).not.toHaveProperty("value");
  });

  /**
   * Given a failed load
   * When the retry is asked for and the backend answers this time
   * Then it asks again and reads the answer
   */
  it("asks again on retry and reads the answer", async () => {
    mocked.mockRejectedValueOnce("down");
    mocked.mockResolvedValueOnce(answer);
    const { result } = renderHook(() => usePotentialOpinion("p1", "r1"));
    await waitFor(() => expect(result.current.status).toBe("failed"));

    act(() => {
      if (result.current.status === "failed") result.current.retry();
    });

    await waitFor(() => expect(result.current).toEqual({ status: "answered", value: answer }));
    expect(mocked).toHaveBeenCalledTimes(2);
  });

  /**
   * Given one player's answer on screen
   * When another player's profile opens, whose answer has not arrived
   * Then it reads loading, never the previous player's answer
   */
  it("never shows the previous player's answer", async () => {
    mocked.mockResolvedValueOnce(answer);
    mocked.mockReturnValueOnce(new Promise<never>(() => undefined));
    const { result, rerender } = renderHook(({ id }) => usePotentialOpinion(id, "r1"), {
      initialProps: { id: "p1" },
    });
    await waitFor(() => expect(result.current.status).toBe("answered"));

    rerender({ id: "p2" });

    expect(result.current.status).toBe("loading");
  });

  /**
   * Given an answer on screen
   * When the game moves on (a new revision) and the next request fails
   * Then it reads loading, then failed: never the last answer
   */
  it("carries no data after the game moves on and the next request fails", async () => {
    mocked.mockResolvedValueOnce(answer);
    mocked.mockRejectedValueOnce("be.error.noActiveGameSession");
    const { result, rerender } = renderHook(({ revision }) => usePotentialOpinion("p1", revision), {
      initialProps: { revision: "r1" },
    });
    await waitFor(() => expect(result.current.status).toBe("answered"));

    rerender({ revision: "r2" });

    expect(result.current.status).toBe("loading");
    await waitFor(() => expect(result.current.status).toBe("failed"));
    expect(result.current).not.toHaveProperty("value");
    expect(mocked).toHaveBeenCalledTimes(2);
  });
});
