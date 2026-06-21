import { describe, expect, it } from "vitest";
import type { GameStateData } from "../../store/gameStore";
import type { TacticsLibraryEntry } from "./TacticsCommandBar";
import {
  buildCustomTacticsStorageKey,
  loadCustomTactics,
  saveCustomTactics,
} from "./TacticsCustomTactics.helpers";

function makeGameState(
  overrides: { manager_id?: string; team_id?: string | null; start_date?: string } = {},
): GameStateData {
  return {
    manager: {
      id: overrides.manager_id ?? "mgr-1",
      team_id: "team_id" in overrides ? overrides.team_id : "team-1",
    },
    clock: { start_date: overrides.start_date ?? "2025-07-01" },
  } as unknown as GameStateData;
}

function makeEntry(overrides: Partial<TacticsLibraryEntry> = {}): TacticsLibraryEntry {
  return {
    id: "custom-1",
    name: "My Tactic",
    description: "A custom tactic",
    formation: "4-3-3",
    playStyle: "Balanced",
    type: "custom",
    ...overrides,
  };
}

describe("buildCustomTacticsStorageKey", () => {
  it("produces the expected key from manager id, start date, and team id", () => {
    expect(buildCustomTacticsStorageKey(makeGameState())).toBe(
      "ofm:tactics:custom:mgr-1:2025-07-01:team-1",
    );
  });

  it("uses no-team when manager has no team assigned", () => {
    expect(buildCustomTacticsStorageKey(makeGameState({ team_id: null }))).toContain(":no-team");
  });
});

describe("loadCustomTactics", () => {
  it("returns empty array when storage is null", () => {
    expect(loadCustomTactics(makeGameState(), null)).toEqual([]);
  });

  it("returns empty array when key is absent", () => {
    const storage = { getItem: () => null, setItem: () => {} };
    expect(loadCustomTactics(makeGameState(), storage)).toEqual([]);
  });

  it("returns valid entries from storage", () => {
    const entry = makeEntry();
    const storage = { getItem: () => JSON.stringify([entry]), setItem: () => {} };
    expect(loadCustomTactics(makeGameState(), storage)).toEqual([entry]);
  });

  it("filters out entries with missing required fields", () => {
    const invalid = { id: "x", type: "custom" };
    const storage = { getItem: () => JSON.stringify([invalid]), setItem: () => {} };
    expect(loadCustomTactics(makeGameState(), storage)).toEqual([]);
  });

  it("returns empty array when stored value is not an array", () => {
    const storage = { getItem: () => JSON.stringify({ type: "custom" }), setItem: () => {} };
    expect(loadCustomTactics(makeGameState(), storage)).toEqual([]);
  });

  it("returns empty array on invalid JSON", () => {
    const storage = { getItem: () => "not-json", setItem: () => {} };
    expect(loadCustomTactics(makeGameState(), storage)).toEqual([]);
  });
});

describe("saveCustomTactics", () => {
  it("persists only custom-type entries", () => {
    let stored: string | null = null;
    const storage = {
      getItem: () => stored,
      setItem: (_key: string, value: string) => { stored = value; },
    };
    const preset = makeEntry({ id: "preset-1", type: "preset" });
    const custom = makeEntry({ id: "custom-1" });
    saveCustomTactics(makeGameState(), [preset, custom], storage);
    expect(JSON.parse(stored!)).toEqual([custom]);
  });

  it("does nothing when storage is null", () => {
    expect(() => saveCustomTactics(makeGameState(), [makeEntry()], null)).not.toThrow();
  });
});
