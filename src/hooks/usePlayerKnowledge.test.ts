import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { usePlayerKnowledge } from "./usePlayerKnowledge";

vi.mock("../services/scoutingService", () => ({ getPlayerKnowledge: vi.fn() }));

const { getPlayerKnowledge } = await import("../services/scoutingService");
const mockedKnowledge = vi.mocked(getPlayerKnowledge);

beforeEach(() => {
  mockedKnowledge.mockReset();
});

describe("usePlayerKnowledge", () => {
  /**
   * Given the backend knows one player as the club's own and has a scout's report on another
   * When the hook mounts with both
   * Then each reads as unknown until the answer arrives, so nothing hidden flashes on screen, and
   *      then as the backend says
   */
  it("reads every player as unknown until the backend answers, then as it says", async () => {
    mockedKnowledge.mockResolvedValue({
      own: { kind: "own" },
      rival: { kind: "scouted", scouted_on: "2026-06-04", current: true },
    });

    const { result } = renderHook(() => usePlayerKnowledge(["own", "rival"], "2026-09-15"));

    expect(result.current("own")).toEqual({ kind: "unknown" });
    await waitFor(() => expect(result.current("own")).toEqual({ kind: "own" }));
    expect(result.current("rival")).toEqual({
      kind: "scouted",
      scouted_on: "2026-06-04",
      current: true,
    });
    expect(result.current("not-asked")).toEqual({ kind: "unknown" });
  });

  /**
   * Given the hook has its answer
   * When the game moves on (a day passes, a scout reports, a player changes club)
   * Then it asks the backend again
   */
  it("asks again when the game moves on", async () => {
    mockedKnowledge.mockResolvedValue({ rival: { kind: "unknown" } });

    const { rerender } = renderHook(({ revision }) => usePlayerKnowledge(["rival"], revision), {
      initialProps: { revision: "2026-09-15" },
    });
    await waitFor(() => expect(mockedKnowledge).toHaveBeenCalledTimes(1));

    rerender({ revision: "2026-09-16" });

    await waitFor(() => expect(mockedKnowledge).toHaveBeenCalledTimes(2));
  });

  /**
   * Given no players to ask about
   * When the hook mounts
   * Then it asks the backend nothing
   */
  it("asks nothing for nobody", () => {
    renderHook(() => usePlayerKnowledge([], "2026-09-15"));

    expect(mockedKnowledge).not.toHaveBeenCalled();
  });
});
