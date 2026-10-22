import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { RatingTier } from "../services/squadService";
import { resetRatingTiersCache, useRatingTiers } from "./useRatingTiers";

vi.mock("../services/squadService", () => ({ getRatingTiers: vi.fn() }));

const { getRatingTiers } = await import("../services/squadService");
const mockedTiers = vi.mocked(getRatingTiers);

/** A table that cuts at 70, 77 and 81, as the backend's does. */
const table = (): RatingTier[] =>
  Array.from({ length: 100 }, (_, rating) =>
    rating >= 81 ? "S" : rating >= 77 ? "A" : rating >= 70 ? "B" : "Rest",
  );

beforeEach(() => {
  mockedTiers.mockReset();
  resetRatingTiersCache();
});

describe("useRatingTiers", () => {
  /**
   * Given the backend's table of tiers
   * When the hook mounts and a rating is read on each side of both cut-offs
   * Then it reads "Rest" until the table arrives, then the table's tier for each rating.
   */
  it("reads each rating's tier from the backend's table", async () => {
    mockedTiers.mockResolvedValue(table());

    const { result } = renderHook(() => useRatingTiers());
    expect(result.current(90)).toBe("Rest");

    await waitFor(() => expect(result.current(90)).toBe("S"));
    expect(result.current(80)).toBe("A");
    expect(result.current(77)).toBe("A");
    expect(result.current(76)).toBe("B");
    expect(result.current(70)).toBe("B");
    expect(result.current(69)).toBe("Rest");
    expect(mockedTiers).toHaveBeenCalledTimes(1);
  });

  /**
   * Given a table that tiers 50 as S (nothing a TypeScript cut-off would say)
   * When the hook reads 50
   * Then it follows the table.
   */
  it("follows the table, not a threshold of its own", async () => {
    mockedTiers.mockResolvedValue(Array.from({ length: 100 }, () => "S" as const));

    const { result } = renderHook(() => useRatingTiers());

    await waitFor(() => expect(result.current(50)).toBe("S"));
  });

  /**
   * Given a rating that comes back fractional, or beyond the table
   * When the hook reads it
   * Then a fractional one is rounded to the nearest rating, and one past 99 reads as "Rest".
   */
  it("rounds a fractional rating and leaves an out-of-table one as the rest", async () => {
    mockedTiers.mockResolvedValue(table());

    const { result } = renderHook(() => useRatingTiers());
    await waitFor(() => expect(result.current(90)).toBe("S"));

    expect(result.current(76.6)).toBe("A");
    expect(result.current(69.6)).toBe("B");
    expect(result.current(120)).toBe("Rest");
  });

  /**
   * Given a screen that mounts the hook once per row
   * When three rows mount, and a later one after the table has arrived
   * Then the backend is asked once, and the later row reads the table on its first render.
   */
  it("asks the backend once for however many mounts", async () => {
    mockedTiers.mockResolvedValue(table());

    const rows = [1, 2, 3].map(() => renderHook(() => useRatingTiers()));
    await waitFor(() => expect(rows[2].result.current(90)).toBe("S"));
    const late = renderHook(() => useRatingTiers());

    expect(late.result.current(90)).toBe("S");
    expect(mockedTiers).toHaveBeenCalledTimes(1);
  });

  /**
   * Given a first request that fails
   * When a screen mounts the hook again
   * Then it asks again, instead of keeping the failure.
   */
  it("asks again after a failed request", async () => {
    mockedTiers.mockRejectedValueOnce(new Error("offline")).mockResolvedValue(table());
    const quiet = vi.spyOn(console, "error").mockImplementation(() => {});

    const first = renderHook(() => useRatingTiers());
    await waitFor(() => expect(quiet).toHaveBeenCalled());
    expect(first.result.current(90)).toBe("Rest");
    const second = renderHook(() => useRatingTiers());

    await waitFor(() => expect(second.result.current(90)).toBe("S"));
    expect(mockedTiers).toHaveBeenCalledTimes(2);
    quiet.mockRestore();
  });

  /**
   * Given a first mount whose request failed, so it reads "Rest" for everything
   * When a later mount asks again and its request succeeds
   * Then the first mount reads the table too, instead of staying on "Rest" for good.
   */
  it("gives a mount whose request failed the table a later retry fetches", async () => {
    const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
    mockedTiers.mockRejectedValueOnce(new Error("offline")).mockResolvedValue(table());

    const first = renderHook(() => useRatingTiers());
    await waitFor(() => expect(quiet).toHaveBeenCalled());
    expect(first.result.current(90)).toBe("Rest");
    const second = renderHook(() => useRatingTiers());
    await waitFor(() => expect(second.result.current(90)).toBe("S"));

    await waitFor(() => expect(first.result.current(90)).toBe("S"));
    quiet.mockRestore();
  });

  /**
   * Given a first mount whose request is still in flight
   * When a second mount renders before the table arrives, and then the table arrives
   * Then both mounts read the table, from the one request.
   */
  it("gives a mount rendered while the request is in flight the table", async () => {
    let arrive: (answer: RatingTier[]) => void = () => {};
    mockedTiers.mockReturnValue(new Promise<RatingTier[]>((resolve) => (arrive = resolve)));

    const first = renderHook(() => useRatingTiers());
    const second = renderHook(() => useRatingTiers());
    expect(second.result.current(90)).toBe("Rest");
    arrive(table());

    await waitFor(() => expect(second.result.current(90)).toBe("S"));
    expect(first.result.current(90)).toBe("S");
    expect(mockedTiers).toHaveBeenCalledTimes(1);
  });
});
