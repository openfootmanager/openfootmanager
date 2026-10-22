import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useVisibleWage } from "./useVisibleWage";

vi.mock("../services/scoutingService", () => ({ getVisibleWage: vi.fn() }));

const { getVisibleWage } = await import("../services/scoutingService");
const mockedWage = vi.mocked(getVisibleWage);

beforeEach(() => {
  mockedWage.mockReset();
});

describe("useVisibleWage", () => {
  /**
   * Given the backend shows a scouted player's wage as about 16,000
   * When the hook mounts
   * Then it reads hidden until the answer arrives, so the exact figure never flashes, then as the
   *      backend says
   */
  it("reads the wage as hidden until the backend answers, then as it says", async () => {
    mockedWage.mockResolvedValue({ kind: "approximate", weekly: 16_000 });

    const { result } = renderHook(() => useVisibleWage("p1", "2026-09-15"));

    expect(result.current).toEqual({ kind: "hidden" });
    await waitFor(() => expect(result.current).toEqual({ kind: "approximate", weekly: 16_000 }));
  });

  /**
   * Given the profile shows the club's own player's exact wage
   * When it moves on to a rival whose answer has not arrived yet
   * Then it reads hidden meanwhile, never the previous player's figure
   */
  it("never shows the previous player's wage while the next one's is asked", async () => {
    mockedWage.mockResolvedValueOnce({ kind: "exact", weekly: 15_843 });
    mockedWage.mockReturnValueOnce(new Promise<never>(() => undefined));

    const { result, rerender } = renderHook(
      ({ playerId }) => useVisibleWage(playerId, "2026-09-15"),
      {
        initialProps: { playerId: "own-player" },
      },
    );
    await waitFor(() => expect(result.current).toEqual({ kind: "exact", weekly: 15_843 }));

    rerender({ playerId: "rival" });

    expect(result.current).toEqual({ kind: "hidden" });
  });

  /**
   * Given the hook has its answer
   * When the game moves on (a renewal, a transfer, a scout's report)
   * Then it asks the backend again
   */
  it("asks again when the game moves on", async () => {
    mockedWage.mockResolvedValue({ kind: "hidden" });

    const { rerender } = renderHook(({ revision }) => useVisibleWage("p1", revision), {
      initialProps: { revision: "2026-09-15" },
    });
    await waitFor(() => expect(mockedWage).toHaveBeenCalledTimes(1));

    rerender({ revision: "2026-09-16" });

    await waitFor(() => expect(mockedWage).toHaveBeenCalledTimes(2));
  });
});
