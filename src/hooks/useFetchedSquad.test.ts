import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

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
});
