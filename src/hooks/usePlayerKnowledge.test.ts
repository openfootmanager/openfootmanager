import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { PlayerKnowledge } from "../services/scoutingService";
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
   * Given the hook knows a player as the club's own
   * When the game moves on (another career installed, say) and the backend's next answer is
   *      still on its way, and then fails
   * Then he reads as unknown meanwhile and after the failure, never as the last answer's own
   */
  it("reads unknown, not the last answer, until the game's next answer arrives", async () => {
    mockedKnowledge.mockResolvedValueOnce({ p1: { kind: "own" } });
    const { result, rerender } = renderHook(
      ({ revision }) => usePlayerKnowledge(["p1"], revision),
      { initialProps: { revision: "career-1" } },
    );
    await waitFor(() => expect(result.current("p1")).toEqual({ kind: "own" }));

    mockedKnowledge.mockRejectedValueOnce(new Error("no game"));
    rerender({ revision: "career-2" });

    expect(result.current("p1")).toEqual({ kind: "unknown" });
    await waitFor(() => expect(mockedKnowledge).toHaveBeenCalledTimes(2));
    await Promise.resolve();
    expect(result.current("p1")).toEqual({ kind: "unknown" });
  });

  /**
   * Given the club's own player, from a package whose ids may hold a comma ("fc,united-9")
   * When the hook asks about him
   * Then the backend is asked about that one id, not two halves of it, and he reads as own
   */
  it("asks about an id with a comma in it as one id", async () => {
    mockedKnowledge.mockImplementation(
      async (ids): Promise<Record<string, PlayerKnowledge>> =>
        ids.includes("fc,united-9") ? { "fc,united-9": { kind: "own" } } : {},
    );

    const { result } = renderHook(() => usePlayerKnowledge(["fc,united-9"], "2026-09-15"));

    await waitFor(() => expect(mockedKnowledge).toHaveBeenCalled());
    expect(mockedKnowledge).toHaveBeenCalledWith(["fc,united-9"]);
    await waitFor(() => expect(result.current("fc,united-9")).toEqual({ kind: "own" }));
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
