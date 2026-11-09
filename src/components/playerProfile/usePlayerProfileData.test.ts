import { renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { createPlayer } from "../../test-utils/factories";
import { usePlayerProfileData } from "./usePlayerProfileData";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

function overviewCalls(): number {
  return mockedInvoke.mock.calls.filter(([command]) => command === "get_player_stats_overview")
    .length;
}

function historyCalls(): number {
  return mockedInvoke.mock.calls.filter(([command]) => command === "get_player_match_history")
    .length;
}

describe("usePlayerProfileData", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedInvoke.mockResolvedValue([]);
  });

  /**
   * Given a profile kept open
   * When the player moves to another club
   * Then the overview is fetched again, because the league peer group changed with the club.
   */
  it("refetches the overview when the player changes club", async () => {
    const player = createPlayer({ team_id: "team-1" });
    const { rerender } = renderHook((args) => usePlayerProfileData(args), {
      initialProps: { player, currentDate: "2026-08-10" },
    });
    await waitFor(() => expect(overviewCalls()).toBe(1));

    rerender({ player: { ...player, team_id: "team-2" }, currentDate: "2026-08-10" });

    await waitFor(() => expect(overviewCalls()).toBe(2));
  });

  /**
   * Given a profile kept open
   * When the game date moves (a new matchday, or a split season's second half)
   * Then the overview is fetched again.
   */
  it("refetches the overview when the game date moves", async () => {
    const player = createPlayer();
    const { rerender } = renderHook((args) => usePlayerProfileData(args), {
      initialProps: { player, currentDate: "2026-08-10" },
    });
    await waitFor(() => expect(overviewCalls()).toBe(1));

    rerender({ player, currentDate: "2026-08-11" });

    await waitFor(() => expect(overviewCalls()).toBe(2));
  });
  /**
   * Given a player whose only matches so far were friendlies (no competitive appearance)
   * When his profile opens
   * Then his recent matches are still fetched: the list shows every match he played, and a
   * friendly is not part of his season's appearances.
   */
  it("fetches the recent matches of a player with no competitive appearances", async () => {
    const base = createPlayer();
    const player = { ...base, stats: { ...base.stats, appearances: 0 } };

    renderHook((args) => usePlayerProfileData(args), {
      initialProps: { player, currentDate: "2026-08-10" },
    });

    await waitFor(() => expect(historyCalls()).toBe(1));
  });

  /**
   * Given the same player, after a friendly
   * When the game date moves on
   * Then the recent matches are fetched again: his appearances did not change, so they cannot be
   * what says a match was played.
   */
  it("refetches the recent matches when the game date moves", async () => {
    const base = createPlayer();
    const player = { ...base, stats: { ...base.stats, appearances: 0 } };
    const { rerender } = renderHook((args) => usePlayerProfileData(args), {
      initialProps: { player, currentDate: "2026-08-10" },
    });
    await waitFor(() => expect(historyCalls()).toBe(1));

    rerender({ player, currentDate: "2026-08-11" });

    await waitFor(() => expect(historyCalls()).toBe(2));
  });
});
