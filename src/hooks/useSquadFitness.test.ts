import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { SquadFitness } from "../services/squadService";
import { createSquadFitness } from "../test-utils/factories";
import { useSquadFitness } from "./useSquadFitness";

vi.mock("../services/squadService", () => ({ getSquadFitness: vi.fn() }));

const { getSquadFitness } = await import("../services/squadService");
const mockedFitness = vi.mocked(getSquadFitness);

const fitness = (average: number): SquadFitness =>
  createSquadFitness({ average_condition: average });

beforeEach(() => {
  mockedFitness.mockReset();
});

describe("useSquadFitness", () => {
  /**
   * Given the backend's summary for the squad
   * When the hook mounts
   * Then it is null until the answer, then the backend's figures as they are.
   */
  it("reads the backend's summary", async () => {
    mockedFitness.mockResolvedValue(fitness(72));

    const { result } = renderHook(() => useSquadFitness());
    expect(result.current).toBeNull();

    await waitFor(() => expect(result.current?.average_condition).toBe(72));
  });

  /**
   * Given a newer revision of the game (a day passed)
   * When the hook renders with it
   * Then the backend is asked again, and the last answer stays until the new one lands.
   */
  it("asks again when the game changes and keeps the last answer meanwhile", async () => {
    mockedFitness
      .mockResolvedValueOnce(fitness(72))
      .mockReturnValueOnce(new Promise<SquadFitness | null>(() => {}));
    const { result, rerender } = renderHook(({ revision }) => useSquadFitness(revision), {
      initialProps: { revision: 1 },
    });
    await waitFor(() => expect(result.current?.average_condition).toBe(72));

    rerender({ revision: 1 });
    expect(mockedFitness).toHaveBeenCalledTimes(1);
    rerender({ revision: 2 });

    await waitFor(() => expect(mockedFitness).toHaveBeenCalledTimes(2));
    expect(result.current?.average_condition).toBe(72);
  });

  /**
   * Given a first request still pending when the game changes and a second request answers first
   * When the first request, which belongs to the old game, answers last
   * Then its answer is dropped: the screen keeps the newer game's figures.
   */
  it("ignores a late answer to a request the game has moved past", async () => {
    let answerFirst: (value: SquadFitness) => void = () => {};
    mockedFitness
      .mockReturnValueOnce(
        new Promise<SquadFitness | null>((resolve) => {
          answerFirst = resolve;
        }),
      )
      .mockResolvedValueOnce(fitness(55));
    const { result, rerender } = renderHook(({ revision }) => useSquadFitness(revision), {
      initialProps: { revision: 1 },
    });
    rerender({ revision: 2 });
    await waitFor(() => expect(result.current?.average_condition).toBe(55));

    await act(async () => {
      answerFirst(fitness(20));
    });

    expect(result.current?.average_condition).toBe(55);
  });

  /**
   * Given an answer for one career and a request that then fails (the game is another career now)
   * When the hook renders with the new game
   * Then the old career's figures are cleared, not shown against the new one.
   */
  it("clears the answer when a request fails", async () => {
    const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
    mockedFitness.mockResolvedValueOnce(fitness(72)).mockRejectedValueOnce(new Error("offline"));
    const { result, rerender } = renderHook(({ revision }) => useSquadFitness(revision), {
      initialProps: { revision: 1 },
    });
    await waitFor(() => expect(result.current?.average_condition).toBe(72));

    rerender({ revision: 2 });

    await waitFor(() => expect(result.current).toBeNull());
    quiet.mockRestore();
  });
});
