import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { AutoSetPieces } from "../services/squadService";
import { useAutoSetPieces } from "./useAutoSetPieces";

vi.mock("../services/squadService", () => ({ autoSelectSetPieces: vi.fn() }));

const { autoSelectSetPieces } = await import("../services/squadService");
const mockedAuto = vi.mocked(autoSelectSetPieces);

const picks = (captain: string): AutoSetPieces => ({
  captain,
  penalty_taker: "pen",
  free_kick_taker: "fk",
  corner_taker: "cor",
});

beforeEach(() => {
  mockedAuto.mockReset();
});

describe("useAutoSetPieces", () => {
  /**
   * Given a starting eleven
   * When the hook mounts
   * Then it asks the backend for the takers it would use and exposes them.
   */
  it("asks the backend for the takers of the eleven it is given", async () => {
    mockedAuto.mockResolvedValue(picks("c1"));

    const { result } = renderHook(() => useAutoSetPieces(["a", "b"]));

    await waitFor(() => expect(result.current?.captain).toBe("c1"));
    expect(mockedAuto).toHaveBeenCalledWith(["a", "b"]);
  });

  /**
   * Given no one is in the eleven yet
   * When the hook mounts
   * Then nothing is asked.
   */
  it("does not ask for an empty eleven", () => {
    renderHook(() => useAutoSetPieces([]));

    expect(mockedAuto).not.toHaveBeenCalled();
  });

  /**
   * Given the same eleven in a new array
   * When the component renders again
   * Then the backend is not asked again; a changed eleven asks once more.
   */
  it("asks again only when the eleven changes", async () => {
    mockedAuto.mockResolvedValue(picks("c1"));
    const { rerender } = renderHook(({ ids }) => useAutoSetPieces(ids), {
      initialProps: { ids: ["a", "b"] },
    });
    await waitFor(() => expect(mockedAuto).toHaveBeenCalledTimes(1));

    rerender({ ids: ["a", "b"] });
    expect(mockedAuto).toHaveBeenCalledTimes(1);

    rerender({ ids: ["a", "c"] });
    await waitFor(() => expect(mockedAuto).toHaveBeenCalledTimes(2));
  });

  /**
   * Given the same eleven but a newer squad revision (a day passed, attributes moved)
   * When the hook renders with the new revision
   * Then the backend is asked again, since it ranks on attributes, not on who is in the eleven.
   */
  it("asks again when the squad revision changes with the same eleven", async () => {
    mockedAuto.mockResolvedValue(picks("c1"));
    const { rerender } = renderHook(({ revision }) => useAutoSetPieces(["a", "b"], revision), {
      initialProps: { revision: 1 },
    });
    await waitFor(() => expect(mockedAuto).toHaveBeenCalledTimes(1));

    rerender({ revision: 1 });
    expect(mockedAuto).toHaveBeenCalledTimes(1);

    rerender({ revision: 2 });
    await waitFor(() => expect(mockedAuto).toHaveBeenCalledTimes(2));
  });

  /**
   * Given two requests in flight, the first answered last
   * When both have answered
   * Then the takers shown are the newer eleven's.
   */
  it("ignores a late answer for an older eleven", async () => {
    let answerFirst!: (value: AutoSetPieces) => void;
    mockedAuto
      .mockImplementationOnce(
        () => new Promise<AutoSetPieces>((resolve) => (answerFirst = resolve)),
      )
      .mockResolvedValueOnce(picks("newer"));
    const { result, rerender } = renderHook(({ ids }) => useAutoSetPieces(ids), {
      initialProps: { ids: ["a"] },
    });

    rerender({ ids: ["b"] });
    await waitFor(() => expect(result.current?.captain).toBe("newer"));
    await act(async () => answerFirst(picks("older")));

    expect(result.current?.captain).toBe("newer");
  });
});
