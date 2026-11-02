import { afterEach, describe, expect, it, vi, beforeEach } from "vitest";
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

    const { result } = renderHook(({ teamId, clockDate }) => useFetchedSquad(teamId, clockDate), {
      initialProps: { teamId: "team1", clockDate: "2026-08-01" },
    });

    await waitFor(() => expect(result.current[0]).not.toBeNull());
    expect(mockedGetSquad).toHaveBeenCalledWith("team1");
    expect(result.current[0]?.[0]?.id).toBe("p1");
  });

  // Regression for the stale-fitness bug: advancing a day changes the clock,
  // which must trigger a refetch even though the team is unchanged.
  it("refetches when the game clock advances", async () => {
    mockedGetSquad.mockResolvedValue(squad("p1"));

    const { rerender } = renderHook(({ teamId, clockDate }) => useFetchedSquad(teamId, clockDate), {
      initialProps: { teamId: "team1", clockDate: "2026-08-01" },
    });

    await waitFor(() => expect(mockedGetSquad).toHaveBeenCalledTimes(1));

    rerender({ teamId: "team1", clockDate: "2026-08-02" });

    await waitFor(() => expect(mockedGetSquad).toHaveBeenCalledTimes(2));
  });

  it("does not refetch when neither team nor clock changes", async () => {
    mockedGetSquad.mockResolvedValue(squad("p1"));

    const { rerender } = renderHook(({ teamId, clockDate }) => useFetchedSquad(teamId, clockDate), {
      initialProps: { teamId: "team1", clockDate: "2026-08-01" },
    });

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
  // Given a squad mutation on the same day, when game state changes, then the
  // backend projection is fetched again so healed seniors revoke youth call-ups.
  it("refreshes eligibility after a same-day squad mutation", async () => {
    mockedGetSquad.mockResolvedValue([{ id: "youth", match_day_eligible: true } as PlayerData]);
    const { result, rerender } = renderHook(
      ({ revision }) => useFetchedSquad("team1", "2026-08-01", revision),
      {
        initialProps: { revision: {} },
      },
    );
    await waitFor(() => expect(result.current[0]?.[0]?.match_day_eligible).toBe(true));
    mockedGetSquad.mockResolvedValue([{ id: "youth", match_day_eligible: false } as PlayerData]);
    rerender({ revision: {} });
    await waitFor(() => expect(result.current[0]?.[0]?.match_day_eligible).toBe(false));
  });
  // Given a cached player the backend rates tiring, when a Game mutation omits projection fields,
  // then his condition band stays until the fresh backend response replaces it.
  it("keeps the condition band through a mutation until the backend refresh replaces it", async () => {
    const tiring = { band: "Tiring", label_key: "be.condition.tiring", tone: "fair" } as const;
    mockedGetSquad.mockResolvedValue([{ id: "p1", condition_status: tiring } as PlayerData]);
    const { result } = renderHook(() => useFetchedSquad("team1", "2026-08-01", {}));
    await waitFor(() => expect(result.current[0]?.[0]?.condition_status).toEqual(tiring));

    act(() => result.current[1]([{ id: "p1", training_focus: "Recovery" } as PlayerData]));

    expect(result.current[0]?.[0]?.condition_status).toEqual(tiring);
  });

  // Given a cached called-up youth, when a Game mutation omits projection fields,
  // then he stays visible until the fresh backend response revokes his call-up.
  it("preserves call-up metadata until the backend refresh replaces it", async () => {
    mockedGetSquad.mockResolvedValue([{ id: "youth", match_day_eligible: true } as PlayerData]);
    const { result, rerender } = renderHook(
      ({ revision }) => useFetchedSquad("team1", "2026-08-01", revision),
      {
        initialProps: { revision: {} },
      },
    );
    await waitFor(() => expect(result.current[0]?.[0]?.match_day_eligible).toBe(true));
    let resolveRefresh!: (players: PlayerData[]) => void;
    mockedGetSquad.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveRefresh = resolve;
        }),
    );
    act(() => result.current[1]([{ id: "youth", training_focus: "Recovery" } as PlayerData]));
    rerender({ revision: {} });
    expect(result.current[0]?.[0]?.match_day_eligible).toBe(true);
    expect(result.current[0]?.[0]?.training_focus).toBe("Recovery");
    await act(async () =>
      resolveRefresh([{ id: "youth", match_day_eligible: false } as PlayerData]),
    );
    expect(result.current[0]?.[0]?.match_day_eligible).toBe(false);
  });

  describe("whether the squad is the backend's answer for what is asked now", () => {
    /**
     * Given a squad that has not been answered yet
     * When the hook mounts, the answer arrives, and then the revision changes
     * Then it is not answered at first, answered once the fetch resolves, and not answered again
     *      from the moment the revision changes until the newer fetch resolves, though the previous
     *      squad is still served meanwhile.
     */
    it("is answered only for the revision the resolved fetch was asked about", async () => {
      let resolveSecond: (players: PlayerData[]) => void = () => {};
      mockedGetSquad
        .mockResolvedValueOnce(squad("p1"))
        .mockReturnValueOnce(new Promise((resolve) => (resolveSecond = resolve)));

      const { result, rerender } = renderHook(
        ({ revision }) => useFetchedSquad("team1", "2026-08-01", revision),
        { initialProps: { revision: { n: 1 } } },
      );
      expect(result.current[2]).toBe(false);

      await waitFor(() => expect(result.current[2]).toBe(true));

      rerender({ revision: { n: 2 } });
      expect(result.current[2]).toBe(false);
      expect(result.current[0]?.[0]?.id).toBe("p1");

      await act(async () => resolveSecond(squad("p2")));
      expect(result.current[2]).toBe(true);
      expect(result.current[0]?.[0]?.id).toBe("p2");
    });

    /**
     * Given an answered squad and a mutation that patches it optimistically while the parent's
     *       revision moves on
     * When the newer fetch has not resolved
     * Then the patched squad is not counted as answered: it carries the old verdicts over.
     */
    it("does not count an optimistic patch as the backend's answer", async () => {
      mockedGetSquad.mockResolvedValueOnce(squad("p1")).mockReturnValueOnce(new Promise(() => {}));
      const { result, rerender } = renderHook(
        ({ revision }) => useFetchedSquad("team1", "2026-08-01", revision),
        { initialProps: { revision: { n: 1 } } },
      );
      await waitFor(() => expect(result.current[2]).toBe(true));

      rerender({ revision: { n: 2 } });
      act(() => result.current[1](squad("p1-patched")));

      expect(result.current[0]?.[0]?.id).toBe("p1-patched");
      expect(result.current[2]).toBe(false);
    });

    /**
     * Given an answered squad, then the team removed (unemployed) and then the same team back, with
     *       the same date and revision and its fetch still in flight
     * When the hook runs
     * Then it is not answered: the squad was cleared with the team, and what the caller falls back
     *      to is not the backend's answer.
     */
    it("is not answered again when the same team returns before its fetch resolves", async () => {
      mockedGetSquad.mockResolvedValueOnce(squad("p1")).mockReturnValueOnce(new Promise(() => {}));
      const revision = { n: 1 };
      const { result, rerender } = renderHook(
        ({ teamId }: { teamId: string | null }) => useFetchedSquad(teamId, "2026-08-01", revision),
        { initialProps: { teamId: "team1" as string | null } },
      );
      await waitFor(() => expect(result.current[2]).toBe(true));

      rerender({ teamId: null });
      rerender({ teamId: "team1" });

      expect(result.current[0]).toBeNull();
      expect(result.current[2]).toBe(false);
    });

    /**
     * Given no team
     * When the hook runs
     * Then it is never answered: there is nothing to answer for.
     */
    it("is not answered without a team", () => {
      const { result } = renderHook(() => useFetchedSquad(null, "2026-08-01"));

      expect(result.current[2]).toBe(false);
    });
  });

  describe("when a fetch fails and the screen stays mounted", () => {
    beforeEach(() => vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] }));
    afterEach(() => vi.useRealTimers());

    /**
     * Given a get_squad that fails once
     * When the first retry delay passes
     * Then the hook asks again on its own and the squad is answered, with no new revision.
     */
    it("asks again on its own, so a screen waiting for the squad is not stuck", async () => {
      const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
      mockedGetSquad.mockRejectedValueOnce(new Error("offline")).mockResolvedValue(squad("p1"));

      const { result } = renderHook(() => useFetchedSquad("team1", "2026-08-01"));
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(result.current[2]).toBe(false);
      expect(mockedGetSquad).toHaveBeenCalledTimes(1);

      await act(async () => {
        await vi.advanceTimersByTimeAsync(1_000);
      });

      expect(mockedGetSquad).toHaveBeenCalledTimes(2);
      expect(result.current[2]).toBe(true);
      quiet.mockRestore();
    });

    /**
     * Given a get_squad that keeps failing
     * When the delays pass
     * Then each retry waits longer than the last (1s, then 2s), so a dead backend is not hammered.
     */
    it("waits longer after each failure in a row", async () => {
      const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
      mockedGetSquad.mockRejectedValue(new Error("offline"));

      renderHook(() => useFetchedSquad("team1", "2026-08-01"));
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1_000);
      });
      expect(mockedGetSquad).toHaveBeenCalledTimes(2);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1_999);
      });
      expect(mockedGetSquad).toHaveBeenCalledTimes(2);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1);
      });
      expect(mockedGetSquad).toHaveBeenCalledTimes(3);
      quiet.mockRestore();
    });

    /**
     * Given a failing fetch and then the screen unmounting
     * When the delay passes
     * Then nothing is asked again.
     */
    it("stops asking when the screen unmounts", async () => {
      const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
      mockedGetSquad.mockRejectedValue(new Error("offline"));
      const { unmount } = renderHook(() => useFetchedSquad("team1", "2026-08-01"));
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });

      unmount();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(20_000);
      });

      expect(mockedGetSquad).toHaveBeenCalledTimes(1);
      quiet.mockRestore();
    });
  });
});
