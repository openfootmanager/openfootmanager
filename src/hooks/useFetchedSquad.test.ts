import { describe, expect, it, vi, beforeEach } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

import { useFetchedSquad } from "./useFetchedSquad";
import type { PlayerData } from "../store/gameStore";

vi.mock("../services/squadService", () => ({
  getSquad: vi.fn(),
}));

const { getSquad } = await import("../services/squadService");
const mockedGetSquad = vi.mocked(getSquad);

const squad = (id: string): PlayerData[] => [{ id } as PlayerData];

beforeEach(() => {
  mockedGetSquad.mockReset();
});

describe("useFetchedSquad", () => {
  it("fetches the team's squad and returns it", async () => {
    mockedGetSquad.mockResolvedValue(squad("p1"));

    const { result } = renderHook(
      ({ teamId, clockDate }) => useFetchedSquad(teamId, clockDate),
      { initialProps: { teamId: "team1", clockDate: "2026-08-01" } },
    );

    await waitFor(() => expect(result.current[0]).not.toBeNull());
    expect(mockedGetSquad).toHaveBeenCalledWith("team1");
    expect(result.current[0]?.[0]?.id).toBe("p1");
  });

  // Regression for the stale-fitness bug: advancing a day changes the clock,
  // which must trigger a refetch even though the team is unchanged.
  it("refetches when the game clock advances", async () => {
    mockedGetSquad.mockResolvedValue(squad("p1"));

    const { rerender } = renderHook(
      ({ teamId, clockDate }) => useFetchedSquad(teamId, clockDate),
      { initialProps: { teamId: "team1", clockDate: "2026-08-01" } },
    );

    await waitFor(() => expect(mockedGetSquad).toHaveBeenCalledTimes(1));

    rerender({ teamId: "team1", clockDate: "2026-08-02" });

    await waitFor(() => expect(mockedGetSquad).toHaveBeenCalledTimes(2));
  });

  it("does not refetch when neither team nor clock changes", async () => {
    mockedGetSquad.mockResolvedValue(squad("p1"));

    const { rerender } = renderHook(
      ({ teamId, clockDate }) => useFetchedSquad(teamId, clockDate),
      { initialProps: { teamId: "team1", clockDate: "2026-08-01" } },
    );

    await waitFor(() => expect(mockedGetSquad).toHaveBeenCalledTimes(1));

    rerender({ teamId: "team1", clockDate: "2026-08-01" });
    await Promise.resolve();

    expect(mockedGetSquad).toHaveBeenCalledTimes(1);
  });

  it("does not fetch without a team", () => {
    renderHook(() => useFetchedSquad(null, "2026-08-01"));
    expect(mockedGetSquad).not.toHaveBeenCalled();
  });

  it("does not expose the previous team's squad after a team switch", async () => {
    mockedGetSquad.mockImplementation((id: string) => Promise.resolve(squad(id)));

    const { result, rerender } = renderHook(
      ({ teamId, clockDate }: { teamId: string | null; clockDate: string }) =>
        useFetchedSquad(teamId, clockDate),
      { initialProps: { teamId: "team1" as string | null, clockDate: "2026-08-01" } },
    );

    await waitFor(() => expect(result.current[0]?.[0]?.id).toBe("team1"));

    rerender({ teamId: "team2", clockDate: "2026-08-01" });
    // Cache is scoped to the team — the old roster is withheld until the new
    // request for team2 resolves.
    expect(result.current[0]).toBeNull();

    await waitFor(() => expect(result.current[0]?.[0]?.id).toBe("team2"));
  });

  it("clears the cached squad when the team becomes null", async () => {
    mockedGetSquad.mockResolvedValue(squad("p1"));

    const { result, rerender } = renderHook(
      ({ teamId, clockDate }: { teamId: string | null; clockDate: string }) =>
        useFetchedSquad(teamId, clockDate),
      { initialProps: { teamId: "team1" as string | null, clockDate: "2026-08-01" } },
    );

    await waitFor(() => expect(result.current[0]).not.toBeNull());

    rerender({ teamId: null, clockDate: "2026-08-01" });

    await waitFor(() => expect(result.current[0]).toBeNull());

    // An optimistic update via the returned setter must not repopulate the cache
    // while there is no active team.
    act(() => {
      result.current[1](squad("ghost"));
    });
    expect(result.current[0]).toBeNull();
  });
});
