import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { SquadFitness } from "../services/squadService";
import { useSquadFitness } from "./useSquadFitness";

vi.mock("../services/squadService", () => ({ getSquadFitness: vi.fn() }));

const { getSquadFitness } = await import("../services/squadService");
const mockedFitness = vi.mocked(getSquadFitness);

const fitness = (average: number): SquadFitness => ({
  players: 11,
  average_condition: average,
  exhausted_count: 0,
  critical_count: 0,
});

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
});
