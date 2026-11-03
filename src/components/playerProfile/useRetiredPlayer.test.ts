import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { fetchRetiredPlayer } from "../../services/retireesService";
import type { PlayerData } from "../../store/types";
import { useRetiredPlayer } from "./useRetiredPlayer";

vi.mock("../../services/retireesService", () => ({
  fetchRetiredPlayer: vi.fn(),
}));

const mockedFetch = vi.mocked(fetchRetiredPlayer);

function retiree(id: string): PlayerData {
  return { id, full_name: `Retiree ${id}`, retired: true } as PlayerData;
}

describe("useRetiredPlayer", () => {
  beforeEach(() => {
    mockedFetch.mockReset();
  });

  /**
   * Given nobody selected, or a player who is in the roster (the caller passes null)
   * When the hook renders
   * Then nothing is fetched and nobody is returned
   */
  it("fetches nothing without an id", () => {
    const { result } = renderHook(() => useRetiredPlayer(null));

    expect(result.current).toBeNull();
    expect(mockedFetch).not.toHaveBeenCalled();
  });

  /**
   * Given a selected retiree
   * When the backend answers
   * Then he is returned
   */
  it("returns the retiree once he has loaded", async () => {
    mockedFetch.mockResolvedValue(retiree("a"));

    const { result } = renderHook(() => useRetiredPlayer("a"));

    await waitFor(() => expect(result.current?.id).toBe("a"));
  });

  /**
   * Given retiree A loaded and a different retiree B selected whose fetch is still pending
   * When the hook renders for B
   * Then A is not shown under B's id; B is returned once he arrives
   */
  it("does not show the previous retiree under a new id while the new one loads", async () => {
    let resolveSecond: (player: PlayerData) => void = () => {};
    mockedFetch.mockResolvedValueOnce(retiree("a"));
    mockedFetch.mockReturnValueOnce(
      new Promise<PlayerData>((resolve) => {
        resolveSecond = resolve;
      }),
    );
    const { result, rerender } = renderHook(({ id }) => useRetiredPlayer(id), {
      initialProps: { id: "a" },
    });
    await waitFor(() => expect(result.current?.id).toBe("a"));

    rerender({ id: "b" });

    expect(result.current).toBeNull();
    await act(async () => {
      resolveSecond(retiree("b"));
    });
    await waitFor(() => expect(result.current?.id).toBe("b"));
  });

  /**
   * Given a retiree A loaded and the selection then cleared
   * When the hook renders with no id
   * Then nobody is returned
   */
  it("returns nobody once the selection is cleared", async () => {
    mockedFetch.mockResolvedValue(retiree("a"));
    const { result, rerender } = renderHook(({ id }) => useRetiredPlayer(id), {
      initialProps: { id: "a" as string | null },
    });
    await waitFor(() => expect(result.current?.id).toBe("a"));

    rerender({ id: null });

    expect(result.current).toBeNull();
  });
});
