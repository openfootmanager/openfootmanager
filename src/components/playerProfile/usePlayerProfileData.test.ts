import { act, renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { createPlayer } from "../../test-utils/factories";
import type { PlayerRecentMatchEntry } from "./PlayerProfile.helpers";
import { usePlayerProfileData } from "./usePlayerProfileData";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

function overviewCalls(): number {
  return mockedInvoke.mock.calls.filter(([command]) => command === "get_player_stats_overview")
    .length;
}

function recentMatch(fixtureId: string): PlayerRecentMatchEntry {
  return {
    fixture_id: fixtureId,
    date: "2026-08-10",
    competition: "Friendly",
    matchday: 1,
    opponent_team_id: "team-2",
    opponent_name: "Rival FC",
    team_goals: 1,
    opponent_goals: 0,
    minutes_played: 90,
    goals: 0,
    assists: 0,
    shots: 1,
    shots_on_target: 0,
    rating: 6.5,
    rated: true,
  };
}

/** The history command answers each call with the next list; the others answer nothing. */
function historyAnswers(...answers: Array<PlayerRecentMatchEntry[] | Error>): void {
  let next = 0;
  mockedInvoke.mockImplementation((async (command: string) => {
    if (command !== "get_player_match_history") {
      return [];
    }
    const answer = answers[Math.min(next, answers.length - 1)];
    next += 1;
    if (answer instanceof Error) {
      throw answer;
    }
    return answer;
  }) as typeof invoke);
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
    const friendly = recentMatch("friendly");
    historyAnswers([friendly]);

    const { result } = renderHook((args) => usePlayerProfileData(args), {
      initialProps: { player, currentDate: "2026-08-10" },
    });

    await waitFor(() => expect(result.current.recentMatches).toEqual([friendly]));
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
    const first = recentMatch("first");
    const second = recentMatch("second");
    historyAnswers([first], [second]);
    const { result, rerender } = renderHook((args) => usePlayerProfileData(args), {
      initialProps: { player, currentDate: "2026-08-10" },
    });
    await waitFor(() => expect(result.current.recentMatches).toEqual([first]));

    rerender({ player, currentDate: "2026-08-11" });

    await waitFor(() => expect(result.current.recentMatches).toEqual([second]));
    expect(historyCalls()).toBe(2);
  });

  /**
   * Given a profile showing a player's recent matches
   * When the date moves and the refresh fails
   * Then the matches already shown stay: a failed refresh is not an empty history.
   */
  it("keeps the recent matches shown when a refresh for the same player fails", async () => {
    const player = createPlayer();
    const shown = recentMatch("shown");
    historyAnswers([shown], new Error("the backend is busy"));
    const { result, rerender } = renderHook((args) => usePlayerProfileData(args), {
      initialProps: { player, currentDate: "2026-08-10" },
    });
    await waitFor(() => expect(result.current.recentMatches).toEqual([shown]));

    rerender({ player, currentDate: "2026-08-11" });
    await waitFor(() => expect(historyCalls()).toBe(2));
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(result.current.recentMatches).toEqual([shown]);
  });

  /**
   * Given a profile showing one player's recent matches
   * When the manager moves to another player and his list has not arrived yet
   * Then nothing is shown: the old list would sit under the wrong name.
   */
  it("clears the recent matches when the player changes", async () => {
    const first = createPlayer({ id: "player-1" });
    const second = createPlayer({ id: "player-2" });
    historyAnswers([recentMatch("first-players")]);
    const { result, rerender } = renderHook((args) => usePlayerProfileData(args), {
      initialProps: { player: first, currentDate: "2026-08-10" },
    });
    await waitFor(() => expect(result.current.recentMatches).toHaveLength(1));
    mockedInvoke.mockImplementation(() => new Promise(() => undefined));

    rerender({ player: second, currentDate: "2026-08-10" });

    await waitFor(() => expect(result.current.recentMatches).toEqual([]));
  });

  /**
   * Given a profile that has just opened, whose match history has not answered
   * When it is read
   * Then the history is "loading", not an empty list that reads as "no matches"
   */
  it("reports the match history as loading until it answers", async () => {
    historyAnswers([]);
    const { result } = renderHook(() =>
      usePlayerProfileData({ player: createPlayer({ id: "p-1" }), currentDate: "2026-08-01" }),
    );

    expect(result.current.recentMatchesStatus).toBe("loading");
    await waitFor(() => expect(result.current.recentMatchesStatus).toBe("answered"));
    expect(result.current.recentMatches).toEqual([]);
  });

  /**
   * Given a history request that fails on the first try
   * When the profile opens, and the manager retries
   * Then it is "failed" (never an empty "answered"), and after the retry it holds the matches
   */
  it("reports a failed match history, and answers after a retry", async () => {
    historyAnswers(new Error("ipc"), [recentMatch("f-1")]);
    const { result } = renderHook(() =>
      usePlayerProfileData({ player: createPlayer({ id: "p-1" }), currentDate: "2026-08-01" }),
    );

    await waitFor(() => expect(result.current.recentMatchesStatus).toBe("failed"));
    expect(result.current.recentMatches).toEqual([]);

    act(() => result.current.retryRecentMatches());

    await waitFor(() => expect(result.current.recentMatchesStatus).toBe("answered"));
    expect(result.current.recentMatches.map((match) => match.fixture_id)).toEqual(["f-1"]);
  });

  /**
   * Given a list already shown for a player
   * When a refresh of it fails
   * Then the list stays and is still "answered": a failed refresh is not a failed history
   */
  it("keeps an answered history when a refresh fails", async () => {
    historyAnswers([recentMatch("f-1")], new Error("ipc"));
    const { result, rerender } = renderHook(
      ({ date }) =>
        usePlayerProfileData({ player: createPlayer({ id: "p-1" }), currentDate: date }),
      { initialProps: { date: "2026-08-01" } },
    );
    await waitFor(() => expect(result.current.recentMatchesStatus).toBe("answered"));

    rerender({ date: "2026-08-02" });
    await waitFor(() => expect(historyCalls()).toBe(2));

    expect(result.current.recentMatchesStatus).toBe("answered");
    expect(result.current.recentMatches).toHaveLength(1);
  });
});
