import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { getVisibleWage } from "../services/scoutingService";
import { useVisibleWage } from "./useVisibleWage";

vi.mock("../services/scoutingService", () => ({ getVisibleWage: vi.fn() }));
const mocked = vi.mocked(getVisibleWage);

/** A scouted player's wage, about 16,000 a week. */
const answer = { kind: "approximate", weekly: 16_000 } as const;

beforeEach(() => {
  mocked.mockReset();
  vi.spyOn(console, "error").mockImplementation(() => undefined);
});

describe("useVisibleWage", () => {
  /**
   * Given the backend's answer about a player
   * When the hook mounts
   * Then it reads loading, then the backend's answer, asked for him
   */
  it("reads loading, then the backend's answer", async () => {
    mocked.mockResolvedValue(answer);

    const { result } = renderHook(() => useVisibleWage("p1", "r1"));

    expect(result.current.status).toBe("loading");
    await waitFor(() => expect(result.current).toEqual({ status: "answered", value: answer }));
    expect(mocked).toHaveBeenCalledWith("p1");
  });

  /**
   * Given a player the club knows nothing about (the backend answers null), and another whose request fails
   * When each is asked for
   * Then the first reads answered with a hidden wage, the second reads failed with no data: a
   *      failed load is never "nothing known"
   */
  it("tells a failed load from nothing known", async () => {
    mocked.mockResolvedValueOnce(null);
    mocked.mockRejectedValueOnce("be.error.noActiveGameSession");

    const unknown = renderHook(() => useVisibleWage("unknown", "r1"));
    const failed = renderHook(() => useVisibleWage("failing", "r1"));

    await waitFor(() =>
      expect(unknown.result.current).toEqual({ status: "answered", value: { kind: "hidden" } }),
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
    const { result } = renderHook(() => useVisibleWage("p1", "r1"));
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
    const { result, rerender } = renderHook(({ id }) => useVisibleWage(id, "r1"), {
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
    const { result, rerender } = renderHook(({ revision }) => useVisibleWage("p1", revision), {
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
