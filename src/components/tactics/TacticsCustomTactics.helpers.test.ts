import { describe, expect, it } from "vitest";

import type { CustomTactic } from "../../services/customTacticsService";
import {
  importLegacyTactics,
  toCustomTactic,
  toLibraryEntries,
} from "./TacticsCustomTactics.helpers";

const MANAGER = "mgr1";
const START = "2026-07-01";

function fakeStorage(initial: Record<string, string> = {}) {
  const data = new Map(Object.entries(initial));
  return {
    data,
    get length() {
      return data.size;
    },
    key: (index: number) => [...data.keys()][index] ?? null,
    getItem: (key: string) => data.get(key) ?? null,
    removeItem: (key: string) => {
      data.delete(key);
    },
  };
}

function legacy(id: string, name = id) {
  return {
    id,
    type: "custom",
    name,
    description: "",
    formation: "4-4-2",
    playStyle: "Balanced",
    sourcePresetName: null,
  };
}

const keyFor = (team: string) => `ofm:tactics:custom:${MANAGER}:${START}:${team}`;

/** A save that keeps tactics in the order made and replaces by id, like the backend. */
function fakeSave(initial: CustomTactic[] = []) {
  const list = [...initial];
  const calls: string[] = [];
  return {
    list,
    calls,
    save: async (tactic: CustomTactic) => {
      calls.push(tactic.id);
      const at = list.findIndex((existing) => existing.id === tactic.id);
      if (at === -1) list.push(tactic);
      else list[at] = tactic;
      return [...list];
    },
  };
}

describe("importLegacyTactics", () => {
  /**
   * Given a browser library of three tactics, newest first
   * When it is imported, and then imported again
   * Then the save holds the three in the order they were made, the old key is gone, and the
   *      second import saves nothing.
   */
  it("imports an old library once, oldest first, and clears the key", async () => {
    const storage = fakeStorage({
      [keyFor("team1")]: JSON.stringify([legacy("c"), legacy("b"), legacy("a")]),
    });
    const backend = fakeSave();

    const first = await importLegacyTactics(MANAGER, START, [], backend.save, storage);
    const second = await importLegacyTactics(MANAGER, START, first, backend.save, storage);

    expect(backend.calls).toEqual(["a", "b", "c"]);
    expect(first.map((tactic) => tactic.id)).toEqual(["a", "b", "c"]);
    expect(second).toEqual(first);
    expect(storage.data.size).toBe(0);
  });

  /**
   * Given a library saved while at one club and another saved at the next
   * When they are imported
   * Then both end up in the save: a change of club no longer hides a library.
   */
  it("imports the libraries of every club the career has been at", async () => {
    const storage = fakeStorage({
      [keyFor("team1")]: JSON.stringify([legacy("a")]),
      [keyFor("team2")]: JSON.stringify([legacy("b")]),
      [`ofm:tactics:custom:other-manager:${START}:team1`]: JSON.stringify([legacy("z")]),
    });
    const backend = fakeSave();

    await importLegacyTactics(MANAGER, START, [], backend.save, storage);

    expect(backend.list.map((tactic) => tactic.id).sort()).toEqual(["a", "b"]);
    // Another career's library is not this career's to import, nor to clear.
    expect([...storage.data.keys()]).toEqual([`ofm:tactics:custom:other-manager:${START}:team1`]);
  });

  /**
   * Given a tactic the save already holds
   * When the old library is imported
   * Then it is not saved again.
   */
  it("skips a tactic the save already holds, whatever padding its old id had", async () => {
    const storage = fakeStorage({ [keyFor("team1")]: JSON.stringify([legacy("a "), legacy("b")]) });
    const held: CustomTactic[] = [
      { id: "a", name: "kept", description: "", formation: "4-4-2", playStyle: "Balanced" },
    ];
    const backend = fakeSave(held);

    await importLegacyTactics(MANAGER, START, held, backend.save, storage);

    expect(backend.calls).toEqual(["b"]);
    expect(backend.list.find((tactic) => tactic.id === "a")?.name).toBe("kept");
  });

  /**
   * Given a save that fails part way through
   * When the old library is imported, and later imported again once the save works
   * Then the key survives the failure, and the retry saves only what was not yet saved.
   */
  it("keeps the old key when the save fails, and a retry does not import twice", async () => {
    const storage = fakeStorage({
      [keyFor("team1")]: JSON.stringify([legacy("c"), legacy("b"), legacy("a")]),
    });
    const backend = fakeSave();
    let failFrom = 1;
    const flaky = async (tactic: CustomTactic) => {
      if (failFrom-- <= 0) throw new Error("no session");
      return backend.save(tactic);
    };

    const afterFailure = await importLegacyTactics(MANAGER, START, [], flaky, storage);
    expect(afterFailure.map((tactic) => tactic.id)).toEqual(["a"]);
    expect(storage.data.size).toBe(1);

    const retry = await importLegacyTactics(MANAGER, START, afterFailure, backend.save, storage);

    expect(retry.map((tactic) => tactic.id)).toEqual(["a", "b", "c"]);
    expect(backend.calls).toEqual(["a", "b", "c"]);
    expect(storage.data.size).toBe(0);
  });

  /**
   * Given an old tactic the game refuses (a formation it cannot lay out)
   * When the library is imported
   * Then the rest are still saved and the key is cleared: a refusal never turns into acceptance.
   */
  it("does not let a refused tactic keep the old key alive", async () => {
    const storage = fakeStorage({
      [keyFor("team1")]: JSON.stringify([legacy("good"), legacy("bad")]),
    });
    const backend = fakeSave();
    const picky = async (tactic: CustomTactic) => {
      if (tactic.id === "bad") throw "be.error.customTactic.invalidFormation";
      return backend.save(tactic);
    };

    const result = await importLegacyTactics(MANAGER, START, [], picky, storage);

    expect(result.map((tactic) => tactic.id)).toEqual(["good"]);
    expect(storage.data.size).toBe(0);
  });

  /**
   * Given no browser storage at all, and storage with nothing for this career
   * When the import runs
   * Then nothing is saved.
   */
  it("does nothing without storage or without an old library", async () => {
    const backend = fakeSave();

    await importLegacyTactics(MANAGER, START, [], backend.save, null);
    await importLegacyTactics(MANAGER, START, [], backend.save, fakeStorage());

    expect(backend.calls).toEqual([]);
  });
});

describe("tactic mapping", () => {
  /**
   * Given tactics in the order made
   * When they are shown
   * Then the newest is first, and an id with padding goes back to the save trimmed.
   */
  it("shows the newest first and sends ids trimmed", () => {
    const made: CustomTactic[] = [
      { id: "a", name: "A", description: "", formation: "4-4-2", playStyle: "Balanced" },
      { id: "b", name: "B", description: "", formation: "3-5-2", playStyle: "Counter" },
    ];

    const shown = toLibraryEntries(made);

    expect(shown.map((entry) => entry.id)).toEqual(["b", "a"]);
    expect(toCustomTactic({ ...shown[0], id: "  b  " }).id).toBe("b");
  });
});
