import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { RatingTier } from "../services/squadService";
import { useRatingTiers } from "./useRatingTiers";

vi.mock("../services/squadService", () => ({ getRatingTiers: vi.fn() }));

const { getRatingTiers } = await import("../services/squadService");
const mockedTiers = vi.mocked(getRatingTiers);

/** A table that cuts at 77 and 81, as the backend's does. */
const table = (): RatingTier[] =>
  Array.from({ length: 100 }, (_, rating) => (rating >= 81 ? "S" : rating >= 77 ? "A" : "Rest"));

beforeEach(() => {
  mockedTiers.mockReset();
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
    expect(result.current(76)).toBe("Rest");
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
    expect(result.current(120)).toBe("Rest");
  });
});
