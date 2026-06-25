import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";

import type { GameStateData, PlayerData } from "../store/gameStore";
import {
  getRuntimeGeneratedPlayerPortrait,
  getBackgroundPortraitPrewarmKey,
  prewarmPlayerPortraits,
  queueBackgroundPortraitPrewarm,
  selectBackgroundPortraitPlayers,
  selectManagerSquadPortraitPlayers,
} from "./portraitService";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const invokeMock = vi.mocked(invoke);
const isTauriMock = vi.mocked(isTauri);

function player(id: string, teamId: string | null): PlayerData {
  return {
    id,
    full_name: `Player ${id}`,
    match_name: id,
    nationality: "PT",
    date_of_birth: "2000-01-01",
    team_id: teamId,
    retired: false,
  } as PlayerData;
}

function gameState(players: PlayerData[]): GameStateData {
  return {
    clock: { current_date: "2026-08-01", start_date: "2026-08-01" },
    manager: { team_id: "team-a" },
    players,
    league: {
      fixtures: [
        {
          id: "f-1",
          matchday: 1,
          date: "2026-08-02",
          home_team_id: "team-a",
          away_team_id: "team-b",
          competition: "League",
          status: "Scheduled",
          result: null,
        },
      ],
    },
  } as GameStateData;
}

describe("portraitService prewarm planning", () => {
  beforeEach(() => {
    vi.useRealTimers();
    invokeMock.mockReset();
    isTauriMock.mockReturnValue(true);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("selects only manager squad players for blocking prewarm", () => {
    const state = gameState([
      player("a-1", "team-a"),
      player("a-2", "team-a"),
      player("b-1", "team-b"),
      player("free-1", null),
    ]);

    expect(selectManagerSquadPortraitPlayers(state).map((p) => p.id)).toEqual([
      "a-1",
      "a-2",
    ]);
  });

  it("skips players with imported face media during prewarm planning", () => {
    const importedFacePlayer = player("a-1", "team-a");
    importedFacePlayer.media = {
      face: "assets/worlds/test-world/players/a-1.png",
    };
    const state = gameState([
      importedFacePlayer,
      player("a-2", "team-a"),
      player("b-1", "team-b"),
    ]);

    expect(selectManagerSquadPortraitPlayers(state).map((p) => p.id)).toEqual([
      "a-2",
    ]);
    expect(selectBackgroundPortraitPlayers(state).map((p) => p.id)).toEqual([
      "b-1",
    ]);
  });

  it("prioritizes next opponent players before other world players in background", () => {
    const state = gameState([
      player("a-1", "team-a"),
      player("b-1", "team-b"),
      player("b-2", "team-b"),
      player("c-1", "team-c"),
    ]);

    expect(selectBackgroundPortraitPlayers(state).map((p) => p.id)).toEqual([
      "b-1",
      "b-2",
      "c-1",
    ]);
  });

  it("caps default background prewarm to a small relevant window", () => {
    const state = gameState([
      player("a-1", "team-a"),
      ...Array.from({ length: 24 }, (_, index) =>
        player(`b-${index + 1}`, "team-b"),
      ),
      ...Array.from({ length: 60 }, (_, index) =>
        player(`c-${index + 1}`, "team-c"),
      ),
    ]);

    const selected = selectBackgroundPortraitPlayers(state);

    expect(selected).toHaveLength(48);
    expect(selected.slice(0, 24).map((p) => p.id)).toEqual(
      Array.from({ length: 24 }, (_, index) => `b-${index + 1}`),
    );
    expect(selected[selected.length - 1]?.id).toBe("c-24");
  });

  it("keeps the background prewarm key stable across equivalent game state objects", () => {
    const state = gameState([
      player("a-1", "team-a"),
      player("b-1", "team-b"),
      player("b-2", "team-b"),
      player("c-1", "team-c"),
    ]);
    state.manager.id = "manager-stable-key";
    const refreshedState = {
      ...state,
      clock: { ...state.clock },
      manager: { ...state.manager },
      players: state.players.map((p) => ({ ...p })),
      league: state.league
        ? {
            ...state.league,
            fixtures: state.league.fixtures.map((fixture) => ({ ...fixture })),
          }
        : null,
    } as GameStateData;

    expect(getBackgroundPortraitPrewarmKey(refreshedState)).toBe(
      getBackgroundPortraitPrewarmKey(state),
    );
  });

  it("invokes the batch prewarm command without requesting image payloads", async () => {
    invokeMock.mockResolvedValue({
      generator: "runtime-component-recipe-rust-v1",
      requestedCount: 2,
      generatedCount: 2,
      cachedCount: 0,
      failedCount: 0,
      renderMs: 8.5,
      elapsedMs: 12.1,
      records: [],
    });

    await prewarmPlayerPortraits([player("a-1", "team-a"), player("a-2", "team-a")]);

    expect(invokeMock).toHaveBeenCalledWith("prewarm_player_portraits", {
      requests: [
        {
          playerId: "a-1",
          fullName: "Player a-1",
          matchName: "a-1",
          nationality: "PT",
          dateOfBirth: "2000-01-01",
        },
        {
          playerId: "a-2",
          fullName: "Player a-2",
          matchName: "a-2",
          nationality: "PT",
          dateOfBirth: "2000-01-01",
        },
      ],
    });
  });

  it("splits background prewarm into small default batches", async () => {
    vi.useFakeTimers();
    invokeMock.mockResolvedValue({
      generator: "runtime-component-recipe-rust-v1",
      requestedCount: 4,
      generatedCount: 4,
      cachedCount: 0,
      failedCount: 0,
      renderMs: 8.5,
      elapsedMs: 12.1,
      records: [],
    });
    const state = gameState([
      player("a-1", "team-a"),
      player("b-1", "team-b"),
      player("b-2", "team-b"),
      player("b-3", "team-b"),
      player("b-4", "team-b"),
      player("b-5", "team-b"),
      player("c-1", "team-c"),
      player("c-2", "team-c"),
      player("c-3", "team-c"),
    ]);

    queueBackgroundPortraitPrewarm(state, { delayMs: 1 });

    await vi.advanceTimersByTimeAsync(1);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock.mock.calls[0][1]).toMatchObject({
      requests: expect.arrayContaining([
        expect.objectContaining({ playerId: "b-1" }),
        expect.objectContaining({ playerId: "b-4" }),
      ]),
    });
    expect(
      (invokeMock.mock.calls[0][1] as { requests: unknown[] }).requests,
    ).toHaveLength(4);

    await vi.advanceTimersByTimeAsync(150);
    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(
      (invokeMock.mock.calls[1][1] as { requests: unknown[] }).requests,
    ).toHaveLength(4);
  });

  it("cancels pending background prewarm batches", async () => {
    vi.useFakeTimers();
    invokeMock.mockResolvedValue({
      generator: "runtime-component-recipe-rust-v1",
      requestedCount: 4,
      generatedCount: 4,
      cachedCount: 0,
      failedCount: 0,
      renderMs: 8.5,
      elapsedMs: 12.1,
      records: [],
    });
    const state = gameState([
      player("a-1", "team-a"),
      player("b-1", "team-b"),
      player("b-2", "team-b"),
      player("b-3", "team-b"),
      player("b-4", "team-b"),
      player("b-5", "team-b"),
      player("c-1", "team-c"),
      player("c-2", "team-c"),
      player("c-3", "team-c"),
    ]);
    state.manager.id = "manager-cancel";

    const cancel = queueBackgroundPortraitPrewarm(state, { delayMs: 1 });

    await vi.advanceTimersByTimeAsync(1);
    expect(invokeMock).toHaveBeenCalledTimes(1);

    cancel();
    await vi.advanceTimersByTimeAsync(150);

    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("stops background prewarm and releases the queue key when a batch fails", async () => {
    vi.useFakeTimers();
    invokeMock.mockRejectedValueOnce(new Error("backend unavailable"));
    const state = gameState([
      player("a-fail-1", "team-a"),
      player("b-fail-1", "team-b"),
      player("b-fail-2", "team-b"),
      player("b-fail-3", "team-b"),
      player("b-fail-4", "team-b"),
      player("b-fail-5", "team-b"),
      player("c-fail-1", "team-c"),
    ]);
    state.manager.id = "manager-failed-batch";

    queueBackgroundPortraitPrewarm(state, { delayMs: 1 });

    await vi.advanceTimersByTimeAsync(1);
    expect(invokeMock).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(150);
    expect(invokeMock).toHaveBeenCalledTimes(1);

    invokeMock.mockResolvedValueOnce({
      generator: "runtime-component-recipe-rust-v1",
      requestedCount: 4,
      generatedCount: 0,
      cachedCount: 4,
      failedCount: 0,
      renderMs: 0,
      elapsedMs: 1.2,
      records: [],
    });

    queueBackgroundPortraitPrewarm(state, { delayMs: 1 });
    await vi.advanceTimersByTimeAsync(1);

    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it("dedupes only in-flight single portrait requests", async () => {
    invokeMock.mockResolvedValue({
      generator: "runtime-component-recipe-rust-v1",
      cacheKey: "cache-a",
      sourceId: "source-a",
      cachePath: "/tmp/cache-a.webp",
      dataUrl: null,
      generated: true,
      renderMs: 4.1,
      elapsedMs: 5.2,
      width: 384,
      height: 384,
    });
    const subject = player("single-cache-1", "team-a");

    const first = getRuntimeGeneratedPlayerPortrait(subject);
    const second = getRuntimeGeneratedPlayerPortrait(subject);

    expect(first).toBe(second);
    expect(invokeMock).toHaveBeenCalledTimes(1);

    await first;
    await getRuntimeGeneratedPlayerPortrait(subject);

    expect(invokeMock).toHaveBeenCalledTimes(2);
  });
});
