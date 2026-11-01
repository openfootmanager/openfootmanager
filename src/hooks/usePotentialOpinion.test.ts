import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { PotentialOpinion } from "../store/generated/PotentialOpinion";
import { usePotentialOpinion } from "./usePotentialOpinion";

vi.mock("../services/scoutingService", () => ({ getPotentialOpinion: vi.fn() }));

const { getPotentialOpinion } = await import("../services/scoutingService");
const mockedOpinion = vi.mocked(getPotentialOpinion);

const coachesOpinion: PotentialOpinion = {
  low: 76,
  high: 84,
  band: "A",
  certainty: "fairly_sure",
  source: { kind: "coaches" },
};

beforeEach(() => {
  mockedOpinion.mockReset();
});

describe("usePotentialOpinion", () => {
  /**
   * Given the backend holds the coaches' opinion of a player, 76-84
   * When the hook mounts
   * Then it reads no opinion until the answer arrives, then the backend's
   */
  it("reads no opinion until the backend answers, then the backend's", async () => {
    mockedOpinion.mockResolvedValue(coachesOpinion);

    const { result } = renderHook(() => usePotentialOpinion("p1", "rev-1"));

    expect(result.current).toBeNull();
    await waitFor(() => expect(result.current).toEqual(coachesOpinion));
    expect(mockedOpinion).toHaveBeenCalledWith("p1");
  });

  /**
   * Given the profile shows one player's opinion
   * When it moves on to another player whose answer has not arrived yet
   * Then it reads no opinion meanwhile, never the previous player's range
   */
  it("never shows the previous player's opinion while the next one's is asked", async () => {
    mockedOpinion.mockResolvedValueOnce(coachesOpinion);
    mockedOpinion.mockReturnValueOnce(new Promise<never>(() => undefined));
    const { result, rerender } = renderHook(
      ({ playerId }) => usePotentialOpinion(playerId, "rev-1"),
      { initialProps: { playerId: "own-player" } },
    );
    await waitFor(() => expect(result.current).toEqual(coachesOpinion));

    rerender({ playerId: "rival" });

    expect(result.current).toBeNull();
  });

  /**
   * Given an opinion on screen
   * When the game moves on (a club change, say) and the next answer is on its way, and then fails
   * Then it reads no opinion meanwhile and after the failure, never the last answer's range
   */
  it("reads no opinion, not the last answer, until the game's next answer arrives", async () => {
    mockedOpinion.mockResolvedValueOnce(coachesOpinion);
    mockedOpinion.mockRejectedValueOnce("be.error.noActiveGameSession");
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    const { result, rerender } = renderHook(({ revision }) => usePotentialOpinion("p1", revision), {
      initialProps: { revision: "rev-1" },
    });
    await waitFor(() => expect(result.current).toEqual(coachesOpinion));

    rerender({ revision: "rev-2" });

    expect(result.current).toBeNull();
    await waitFor(() => expect(mockedOpinion).toHaveBeenCalledTimes(2));
    expect(result.current).toBeNull();
  });
});
