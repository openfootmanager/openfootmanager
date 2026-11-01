import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { FamiliarityProjection } from "../store/generated/FamiliarityProjection";
import { type GameStateData, useGameStore } from "../store/gameStore";
import { createPlayer } from "../test-utils/factories";
import { resetFamiliarityCache, useFamiliarity } from "./useFamiliarity";

vi.mock("../services/positionService", () => ({ getFamiliarity: vi.fn() }));

const { getFamiliarity } = await import("../services/positionService");
const mockedFamiliarity = vi.mocked(getFamiliarity);

const projection = (playerId = "p1"): FamiliarityProjection => ({
  players: [
    {
      player_id: playerId,
      by_position: { Striker: "natural", Goalkeeper: "out_of_position" },
    },
  ],
});

function installClub(
  options: { clubId?: string; careerToken?: number; playerIds?: string[] } = {},
) {
  const { clubId = "club", careerToken = 1, playerIds = ["p1"] } = options;
  const state = {
    career_token: careerToken,
    manager: { team_id: clubId },
    players: playerIds.map((id) => createPlayer({ id, team_id: clubId })),
  } as unknown as GameStateData;
  act(() => useGameStore.setState({ gameState: state }));
}

beforeEach(() => {
  mockedFamiliarity.mockReset();
  resetFamiliarityCache();
  installClub();
});

describe("useFamiliarity", () => {
  /**
   * Given the backend's familiarity for the manager's club
   * When a screen mounts the hook
   * Then nothing is graded and it is not loaded until the answer arrives, then the backend's word
   *      is read per player and position, and a stranger or an unnamed position is null.
   */
  it("reads the backend's grade, and nothing before it arrives", async () => {
    mockedFamiliarity.mockResolvedValue(projection());

    const { result } = renderHook(() => useFamiliarity());
    expect(result.current.loaded).toBe(false);
    expect(result.current.familiarityOf("p1", "Striker")).toBeNull();

    await waitFor(() => expect(result.current.loaded).toBe(true));
    expect(result.current.familiarityOf("p1", "Striker")).toBe("natural");
    expect(result.current.familiarityOf("p1", "Goalkeeper")).toBe("out_of_position");
    expect(result.current.familiarityOf("stranger", "Striker")).toBeNull();
    expect(result.current.familiarityOf("p1", "Unheard Of")).toBeNull();
    expect(result.current.familiarityOf("p1", "toString")).toBeNull();
    expect(result.current.familiarityOf("toString", "Striker")).toBeNull();
  });

  /**
   * Given several screens that mount the hook for the same club
   * When they all mount
   * Then the backend is asked once.
   */
  it("asks the backend once for however many mounts", async () => {
    mockedFamiliarity.mockResolvedValue(projection());

    const rows = [1, 2, 3].map(() => renderHook(() => useFamiliarity()));
    await waitFor(() => expect(rows[2].result.current.loaded).toBe(true));

    expect(mockedFamiliarity).toHaveBeenCalledTimes(1);
  });

  /**
   * Given a loaded table and then a transfer that changes the club's players
   * When the hook re-renders
   * Then it is not loaded for the new squad until the backend answers again, and it never shows
   *      the old answer as the new club's.
   */
  it("asks again when the club's players change, and is not loaded meanwhile", async () => {
    mockedFamiliarity
      .mockResolvedValueOnce(projection("p1"))
      .mockResolvedValueOnce(projection("p2"));
    const { result } = renderHook(() => useFamiliarity());
    await waitFor(() => expect(result.current.loaded).toBe(true));

    installClub({ playerIds: ["p1", "p2"] });

    expect(result.current.loaded).toBe(false);
    expect(result.current.familiarityOf("p1", "Striker")).toBeNull();
    await waitFor(() => expect(result.current.familiarityOf("p2", "Striker")).toBe("natural"));
    expect(mockedFamiliarity).toHaveBeenCalledTimes(2);
  });

  /**
   * Given a loaded table and then another career installed on the same club and squad
   * When the hook re-renders
   * Then the backend is asked again: the table is the career's, not the window's.
   */
  it("asks again when another career is installed", async () => {
    mockedFamiliarity.mockResolvedValue(projection());
    const { result } = renderHook(() => useFamiliarity());
    await waitFor(() => expect(result.current.loaded).toBe(true));

    installClub({ careerToken: 2 });

    await waitFor(() => expect(mockedFamiliarity).toHaveBeenCalledTimes(2));
  });

  /**
   * Given a player whose alternate positions change (his own id and club unchanged)
   * When the hook re-renders
   * Then the backend is asked again.
   */
  it("asks again when a player's known positions change", async () => {
    mockedFamiliarity.mockResolvedValue(projection());
    const { result } = renderHook(() => useFamiliarity());
    await waitFor(() => expect(result.current.loaded).toBe(true));

    act(() =>
      useGameStore.setState((state) => ({
        gameState: state.gameState && {
          ...state.gameState,
          players: state.gameState.players.map((player) => ({
            ...player,
            alternate_positions: ["Striker"],
          })),
        },
      })),
    );

    await waitFor(() => expect(mockedFamiliarity).toHaveBeenCalledTimes(2));
  });

  /**
   * Given no game is installed (no club)
   * When a screen mounts the hook
   * Then it is not loaded and the backend is not asked, which would only fail.
   */
  it("does not ask when there is no club", async () => {
    act(() => useGameStore.setState({ gameState: null }));

    const { result } = renderHook(() => useFamiliarity());
    await act(async () => {});

    expect(result.current.loaded).toBe(false);
    expect(mockedFamiliarity).not.toHaveBeenCalled();
  });

  describe("when the request fails and the screen stays mounted", () => {
    beforeEach(() => vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] }));
    afterEach(() => vi.useRealTimers());

    /**
     * Given a request that fails once
     * When the first retry delay passes
     * Then the hook asks again on its own and the table arrives, with no remount.
     */
    it("asks again on its own, so a screen waiting for the grade is not stuck", async () => {
      const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
      mockedFamiliarity.mockRejectedValueOnce(new Error("offline")).mockResolvedValue(projection());

      const { result } = renderHook(() => useFamiliarity());
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(result.current.loaded).toBe(false);

      await act(async () => {
        await vi.advanceTimersByTimeAsync(1_000);
      });

      expect(mockedFamiliarity).toHaveBeenCalledTimes(2);
      expect(result.current.loaded).toBe(true);
      quiet.mockRestore();
    });
  });
});
