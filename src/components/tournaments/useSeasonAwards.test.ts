import { describe, expect, it, vi, beforeEach } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

import { fetchSeasonAwards } from "../../services/competitionsService";
import { useSeasonAwards } from "./useSeasonAwards";
import type { SeasonAwardsData } from "../../store/gameStore";

vi.mock("../../services/competitionsService", () => ({
  fetchSeasonAwards: vi.fn(),
}));

const mockedFetch = vi.mocked(fetchSeasonAwards);

const DAY = "2026-08-01";

const awardsFor = (season: number) =>
  ({ season }) as unknown as SeasonAwardsData;

beforeEach(() => {
  mockedFetch.mockReset();
});

describe("useSeasonAwards", () => {
  it("does not fetch until the awards view is opened", () => {
    renderHook(() => useSeasonAwards(2026, false, DAY));

    expect(mockedFetch).not.toHaveBeenCalled();
  });

  it("fetches once the awards view is opened", async () => {
    mockedFetch.mockResolvedValue(awardsFor(2026));

    const { result } = renderHook(() => useSeasonAwards(2026, true, DAY));

    await waitFor(() => expect(result.current.awards).not.toBeNull());
    expect(mockedFetch).toHaveBeenCalledTimes(1);
    expect(result.current.awardsLoadState).toBe("idle");
  });

  // The cache is keyed by season, not by "have we fetched yet". Switching
  // competitions changes the season, and coming back to one already loaded
  // must serve it from memory rather than going to the backend again.
  it("keeps each season's awards and does not refetch one already loaded", async () => {
    mockedFetch.mockImplementation(() => Promise.resolve(awardsFor(2026)));

    const { result, rerender } = renderHook(
      ({ season }) => useSeasonAwards(season, true, DAY),
      { initialProps: { season: 2026 } },
    );
    await waitFor(() => expect(result.current.awards).not.toBeNull());

    mockedFetch.mockImplementation(() => Promise.resolve(awardsFor(2027)));
    rerender({ season: 2027 });
    await waitFor(() => expect(result.current.awards).toEqual(awardsFor(2027)));
    expect(mockedFetch).toHaveBeenCalledTimes(2);

    // Back to the first season: still the 2026 payload, still two fetches.
    rerender({ season: 2026 });
    await waitFor(() => expect(result.current.awards).toEqual(awardsFor(2026)));
    expect(mockedFetch).toHaveBeenCalledTimes(2);
  });

  // Mid-season the board shows current leaders, so yesterday's copy is wrong.
  it("refetches a season it already has once the game date moves on", async () => {
    mockedFetch.mockResolvedValue(awardsFor(2026));

    const { result, rerender } = renderHook(
      ({ day }) => useSeasonAwards(2026, true, day),
      { initialProps: { day: DAY } },
    );
    await waitFor(() => expect(result.current.awards).not.toBeNull());

    rerender({ day: "2026-08-02" });

    await waitFor(() => expect(mockedFetch).toHaveBeenCalledTimes(2));
  });

  it("reports a failed fetch and retries on demand", async () => {
    mockedFetch.mockRejectedValueOnce("boom");

    const { result } = renderHook(() => useSeasonAwards(2026, true, DAY));

    await waitFor(() => expect(result.current.awardsLoadState).toBe("error"));
    expect(result.current.awards).toBeNull();

    mockedFetch.mockResolvedValueOnce(awardsFor(2026));
    act(() => result.current.retryAwards());

    await waitFor(() => expect(result.current.awards).not.toBeNull());
    expect(result.current.awardsLoadState).toBe("idle");
    expect(mockedFetch).toHaveBeenCalledTimes(2);
  });

  // One season failing must not make a different, already-loaded season look
  // broken — the awards board reads awardsLoadState to decide what to show.
  it("keeps a cached season idle after a different season fails", async () => {
    mockedFetch.mockResolvedValueOnce(awardsFor(2026));

    const { result, rerender } = renderHook(
      ({ season }) => useSeasonAwards(season, true, DAY),
      { initialProps: { season: 2026 } },
    );
    await waitFor(() => expect(result.current.awards).not.toBeNull());

    mockedFetch.mockRejectedValueOnce("boom");
    rerender({ season: 2027 });
    await waitFor(() => expect(result.current.awardsLoadState).toBe("error"));

    rerender({ season: 2026 });

    expect(result.current.awards).toEqual(awardsFor(2026));
    expect(result.current.awardsLoadState).toBe("idle");
    expect(mockedFetch).toHaveBeenCalledTimes(2);
  });
});
