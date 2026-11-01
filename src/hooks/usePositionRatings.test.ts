import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { getPositionRatings } from "../services/positionService";
import type { PositionRatings } from "../store/generated/PositionRatings";
import { usePositionRatings } from "./usePositionRatings";

vi.mock("../services/positionService", () => ({ getPositionRatings: vi.fn() }));
const mockedRatings = vi.mocked(getPositionRatings);

const ratings = (rating: number): PositionRatings => ({
  source: { kind: "live" },
  rows: [
    {
      position: "CentralMidfielder",
      rating,
      familiarity: "natural",
      deployed: false,
      key_attributes: [],
    },
  ],
});

beforeEach(() => {
  mockedRatings.mockReset();
});

describe("usePositionRatings", () => {
  /**
   * Given the backend rates a player 77 at his natural position
   * When the hook mounts
   * Then it reads none until the answer arrives, then the backend's, asked for him
   */
  it("reads no ratings until the backend answers, then the backend's", async () => {
    mockedRatings.mockResolvedValue(ratings(77));

    const { result } = renderHook(() => usePositionRatings("p1", "r1"));

    expect(result.current).toBeNull();
    await waitFor(() => expect(result.current).toEqual(ratings(77)));
    expect(mockedRatings).toHaveBeenCalledWith("p1");
  });

  /**
   * Given one player's ratings on screen
   * When another player's profile opens, whose answer has not arrived
   * Then the first player's ratings are not shown for him
   */
  it("never shows the previous player's ratings", async () => {
    mockedRatings.mockResolvedValueOnce(ratings(77));
    const { result, rerender } = renderHook(({ id }) => usePositionRatings(id, "r1"), {
      initialProps: { id: "p1" },
    });
    await waitFor(() => expect(result.current).toEqual(ratings(77)));
    mockedRatings.mockReturnValueOnce(new Promise(() => {}));

    rerender({ id: "p2" });

    expect(result.current).toBeNull();
  });

  /**
   * Given a player's ratings on screen
   * When the game moves on (a new revision) and the backend's next answer fails
   * Then no ratings are read, not the last answer
   */
  it("reads no ratings, not the last answer, until the game's next answer arrives", async () => {
    mockedRatings.mockResolvedValueOnce(ratings(77));
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});
    const { result, rerender } = renderHook(({ revision }) => usePositionRatings("p1", revision), {
      initialProps: { revision: "r1" },
    });
    await waitFor(() => expect(result.current).toEqual(ratings(77)));
    mockedRatings.mockRejectedValueOnce("be.error.noActiveGameSession");

    rerender({ revision: "r2" });

    await waitFor(() => expect(errors).toHaveBeenCalled());
    expect(result.current).toBeNull();
    errors.mockRestore();
  });
});
