import { act, renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { type GameStateData, useGameStore } from "../../store/gameStore";
import { useTacticsLibrary } from "./useTacticsLibrary";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", () => {
  const translation = {
    t: (key: string, fallback?: unknown) => (typeof fallback === "string" ? fallback : key),
    i18n: { language: "en" },
  };
  return {
    initReactI18next: { type: "3rdParty", init: () => {} },
    useTranslation: () => translation,
  };
});

const mockedInvoke = vi.mocked(invoke);

function deferred<T>() {
  let resolve: (value: T) => void = () => {};
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

const career = (managerId: string, careerToken: number) =>
  ({
    clock: { current_date: "2026-08-01", start_date: "2026-08-01" },
    career_token: careerToken,
    revision: 0,
    teams: [],
    players: [],
    competitions: [],
    messages: [],
    staff: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
    manager: { id: managerId, team_id: null },
  }) as unknown as GameStateData;

const wire = (id: string) => ({
  id,
  name: id,
  description: "",
  formation: "4-4-2",
  playStyle: "Balanced",
});

describe("the library when another career is installed from outside the screens (#1438)", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    useGameStore.getState().clearGame();
  });

  /**
   * Given career A's library load still out, and then another career installed the way the MCP
   *       `game_load_save` tool does it: a `game-state-changed` refetch hands the store a snapshot of
   *       a later career, with no setGameActive
   * When A's late list arrives
   * Then it does not show in the picker: it is the previous career's, whatever the screen opened
   *      for (the career epoch moves with the installed career).
   */
  it("drops a previous career's late list after a career is installed over the held one", async () => {
    const oldList = deferred<unknown[]>();
    const newList = deferred<unknown[]>();
    let lists = 0;
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_custom_tactics") {
        lists += 1;
        return lists === 1 ? oldList.promise : newList.promise;
      }
      if (command === "legacy_tactics_import_pending") return false;
      return { modified: false };
    });
    const careerA = career("mgr1", 1);
    act(() => useGameStore.getState().setGameState(careerA));
    const { result, rerender } = renderHook(
      ({ game }: { game: GameStateData }) =>
        useTacticsLibrary({
          gameState: game,
          formation: "4-4-2",
          activePlayStyle: "Balanced",
          initialPreset: null,
          onApplyPreset: vi.fn(),
          onKeepInstructions: vi.fn(),
          onGameUpdate: vi.fn(),
          onWriteError: vi.fn(),
          onWriteSaved: vi.fn(),
        }),
      { initialProps: { game: careerA } },
    );
    await waitFor(() =>
      expect(mockedInvoke.mock.calls.filter(([c]) => c === "list_custom_tactics")).toHaveLength(1),
    );

    const careerB = career("mgr2", 2);
    act(() => useGameStore.getState().setGameState(careerB));
    rerender({ game: careerB });
    await act(async () => oldList.resolve([wire("from-a")]));

    const customIds = () =>
      result.current.tacticLibrary.filter((e) => e.type === "custom").map((e) => e.id);
    expect(customIds()).toEqual([]);
    await act(async () => newList.resolve([wire("from-b")]));
    await waitFor(() => expect(customIds()).toEqual(["from-b"]));
  });
});
