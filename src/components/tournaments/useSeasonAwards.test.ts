import { describe, expect, it, vi, beforeEach } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

import { useSeasonAwards } from "./useSeasonAwards";
import type { SeasonAwardsData } from "../../store/gameStore";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const { invoke } = await import("@tauri-apps/api/core");
const mockedInvoke = vi.mocked(invoke);

const awardsFor = (season: number) =>
  ({ season }) as unknown as SeasonAwardsData;

beforeEach(() => {
  mockedInvoke.mockReset();
});

describe("useSeasonAwards", () => {
  it("does not fetch until the awards view is opened", () => {
    renderHook(() => useSeasonAwards(2026, false));

    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  it("fetches once the awards view is opened", async () => {
    mockedInvoke.mockResolvedValue(awardsFor(2026));

    const { result } = renderHook(() => useSeasonAwards(2026, true));

    await waitFor(() => expect(result.current.awards).not.toBeNull());
    expect(mockedInvoke).toHaveBeenCalledWith("get_season_awards");
    expect(result.current.awardsLoadState).toBe("idle");
  });

  // The cache is keyed by season, not by "have we fetched yet". Switching
  // competitions changes the season, and coming back to one already loaded
  // must serve it from memory rather than going to the backend again.
  it("keeps each season's awards and does not refetch one already loaded", async () => {
    mockedInvoke.mockImplementation(() => Promise.resolve(awardsFor(2026)));

    const { result, rerender } = renderHook(
      ({ season }) => useSeasonAwards(season, true),
      { initialProps: { season: 2026 } },
    );
    await waitFor(() => expect(result.current.awards).not.toBeNull());

    mockedInvoke.mockImplementation(() => Promise.resolve(awardsFor(2027)));
    rerender({ season: 2027 });
    await waitFor(() =>
      expect(result.current.awards).toEqual(awardsFor(2027)),
    );
    expect(mockedInvoke).toHaveBeenCalledTimes(2);

    // Back to the first season: still the 2026 payload, still two fetches.
    rerender({ season: 2026 });
    await waitFor(() =>
      expect(result.current.awards).toEqual(awardsFor(2026)),
    );
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
  });

  it("reports a failed fetch and retries on demand", async () => {
    mockedInvoke.mockRejectedValueOnce("boom");

    const { result } = renderHook(() => useSeasonAwards(2026, true));

    await waitFor(() => expect(result.current.awardsLoadState).toBe("error"));
    expect(result.current.awards).toBeNull();

    mockedInvoke.mockResolvedValueOnce(awardsFor(2026));
    act(() => result.current.retryAwards());

    await waitFor(() => expect(result.current.awards).not.toBeNull());
    expect(result.current.awardsLoadState).toBe("idle");
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
  });
});
