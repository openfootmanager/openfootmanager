import { describe, expect, it } from "vitest";

import type { CustomTactic } from "../../services/customTacticsService";
import type { TacticsLibraryEntry } from "./TacticPicker";
import {
  importLegacyTactics,
  layPendingChanges,
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

/** The tactics an import leaves in the save; most scenarios care about nothing else. */
const imported = async (...args: Parameters<typeof importLegacyTactics>) =>
  (await importLegacyTactics(...args)).tactics;

describe("importLegacyTactics", () => {
  /**
   * Given a browser library of three tactics, newest first
   * When it is imported, and then imported again
   * Then the save holds the three in the order they were made, the old key is left where it is,
   *      and the second import saves nothing.
   */
  it("imports an old library once, oldest first, and leaves the key alone", async () => {
    const storage = fakeStorage({
      [keyFor("team1")]: JSON.stringify([legacy("c"), legacy("b"), legacy("a")]),
    });
    const backend = fakeSave();

    const first = await imported(MANAGER, START, [], backend.save, storage);
    const second = await imported(MANAGER, START, first, backend.save, storage);

    expect(backend.calls).toEqual(["a", "b", "c"]);
    expect(first.map((tactic) => tactic.id)).toEqual(["a", "b", "c"]);
    expect(second).toEqual(first);
    expect(storage.data.size).toBe(1);
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
    });
    const backend = fakeSave();

    await imported(MANAGER, START, [], backend.save, storage);

    expect(backend.list.map((tactic) => tactic.id).sort()).toEqual(["a", "b"]);
  });

  /**
   * Given two careers begun the same year, which share a manager id and a start date and so one
   *       old key per club, each with its own save
   * When each is opened in turn
   * Then each save holds the tactics, and no key was removed: the second career's library is not
   *      taken away by the first career's import.
   */
  it("copies for every career that shares the old key and removes nothing", async () => {
    const storage = fakeStorage({
      [keyFor("team1")]: JSON.stringify([legacy("a"), legacy("b")]),
    });
    const careerOne = fakeSave();
    const careerTwo = fakeSave();

    await imported(MANAGER, START, [], careerOne.save, storage);
    await imported(MANAGER, START, [], careerTwo.save, storage);

    expect(careerOne.list.map((tactic) => tactic.id)).toEqual(["b", "a"]);
    expect(careerTwo.list.map((tactic) => tactic.id)).toEqual(["b", "a"]);
    expect(storage.data.size).toBe(1);
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

    await imported(MANAGER, START, held, backend.save, storage);

    expect(backend.calls).toEqual(["b"]);
    expect(backend.list.find((tactic) => tactic.id === "a")?.name).toBe("kept");
  });

  /**
   * Given a save that fails part way through
   * When the old library is imported, and later imported again once the save works
   * Then the key survives the failure, and the retry saves only what was not yet saved.
   */
  it("stops at a failed save, and a retry saves only what was not yet saved", async () => {
    const storage = fakeStorage({
      [keyFor("team1")]: JSON.stringify([legacy("c"), legacy("b"), legacy("a")]),
    });
    const backend = fakeSave();
    let failFrom = 1;
    const flaky = async (tactic: CustomTactic) => {
      if (failFrom-- <= 0) throw new Error("no session");
      return backend.save(tactic);
    };

    const afterFailure = await imported(MANAGER, START, [], flaky, storage);
    expect(afterFailure.map((tactic) => tactic.id)).toEqual(["a"]);

    const retry = await imported(MANAGER, START, afterFailure, backend.save, storage);

    expect(retry.map((tactic) => tactic.id)).toEqual(["a", "b", "c"]);
    expect(backend.calls).toEqual(["a", "b", "c"]);
  });

  /**
   * Given an old tactic the game refuses (a formation it cannot lay out), and a good one
   * When the library is imported
   * Then the good one is saved, the refused one is left out, and it stays in the browser's
   *      storage: a refusal never loses a tactic.
   */
  it("skips a tactic the game refuses and leaves it in storage", async () => {
    const storage = fakeStorage({
      [keyFor("team1")]: JSON.stringify([legacy("good"), legacy("bad")]),
    });
    const backend = fakeSave();
    const picky = async (tactic: CustomTactic) => {
      if (tactic.id === "bad") throw "be.error.customTactic.invalidFormation";
      return backend.save(tactic);
    };

    const result = await imported(MANAGER, START, [], picky, storage);

    expect(result.map((tactic) => tactic.id)).toEqual(["good"]);
    expect(JSON.parse(storage.data.get(keyFor("team1")) ?? "[]")).toHaveLength(2);
  });

  /**
   * Given an old tactic with a 61-character name, which the old screen allowed
   * When the library is imported
   * Then it is saved with the name shortened to the 60 the save keeps, not refused.
   */
  it("shortens a name longer than the save keeps instead of losing the tactic", async () => {
    const storage = fakeStorage({
      [keyFor("team1")]: JSON.stringify([legacy("long", "x".repeat(61))]),
    });
    const backend = fakeSave();

    await imported(MANAGER, START, [], backend.save, storage);

    expect(backend.list).toHaveLength(1);
    expect(backend.list[0].name).toBe("x".repeat(60));
  });

  /**
   * Given no browser storage at all, and storage with nothing for this career
   * When the import runs
   * Then nothing is saved.
   */
  it("does nothing without storage or without an old library", async () => {
    const backend = fakeSave();

    await imported(MANAGER, START, [], backend.save, null);
    await imported(MANAGER, START, [], backend.save, fakeStorage());

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

  /**
   * Given an old library, a save that works, and one that fails part way
   * When each is imported
   * Then only the first reports itself complete: a failed import must not be marked done, or the
   *      tactics it never saved would never be tried again.
   */
  it("reports complete only when no save failed", async () => {
    const storage = fakeStorage({ [keyFor("team1")]: JSON.stringify([legacy("b"), legacy("a")]) });
    const backend = fakeSave();
    const flaky = async (tactic: CustomTactic) => {
      if (tactic.id === "b") throw new Error("disk full");
      return backend.save(tactic);
    };

    const good = await importLegacyTactics(MANAGER, START, [], fakeSave().save, storage);
    const failed = await importLegacyTactics(MANAGER, START, [], flaky, storage);

    expect(good.complete).toBe(true);
    expect(failed.complete).toBe(false);
  });

  /**
   * Given the browser's storage cannot be read at all
   * When the import runs
   * Then it is not complete: the library may well be there, so the career must not be marked done
   *      and must be tried again when storage works.
   */
  it("reports incomplete when the storage cannot be read", async () => {
    const result = await importLegacyTactics(MANAGER, START, [], fakeSave().save, null);

    expect(result.complete).toBe(false);
  });

  /**
   * Given nothing to import (storage read fine but empty), and a tactic the game refuses
   * When they are imported
   * Then each is complete: there is nothing left that a retry could save.
   */
  it("reports complete when there is nothing to retry", async () => {
    const refusing = async () => {
      throw "be.error.customTactic.invalidFormation";
    };
    const storage = fakeStorage({ [keyFor("team1")]: JSON.stringify([legacy("a")]) });

    expect(
      (await importLegacyTactics(MANAGER, START, [], fakeSave().save, fakeStorage())).complete,
    ).toBe(true);
    expect((await importLegacyTactics(MANAGER, START, [], refusing, storage)).complete).toBe(true);
  });
});

describe("layPendingChanges", () => {
  const entry = (id: string, name = id): TacticsLibraryEntry => ({
    description: "",
    formation: "4-4-2",
    id,
    name,
    playStyle: "Balanced",
    sourcePresetName: null,
    type: "custom",
  });
  const ids = (entries: TacticsLibraryEntry[]) => entries.map((e) => e.id);

  /**
   * Given a reply and nothing pending
   * When the pending changes are laid over it
   * Then it is the reply itself.
   */
  it("returns the reply untouched when nothing is pending", () => {
    const reply = [entry("a"), entry("b")];

    expect(layPendingChanges(reply, new Map())).toBe(reply);
  });

  /**
   * Given a reply that predates two tactics made after it (b, then c)
   * When they are laid over it
   * Then they are in front of what the reply holds, the newest first.
   */
  it("adds a tactic the reply lacks in front, the newest first", () => {
    const pending = new Map<string, TacticsLibraryEntry | null>([
      ["b", entry("b")],
      ["c", entry("c")],
    ]);

    expect(ids(layPendingChanges([entry("a")], pending))).toEqual(["c", "b", "a"]);
  });

  /**
   * Given a reply that holds a tactic as it was first saved, and the same tactic changed since
   * When the change is laid over it
   * Then it shows the changed version, in the place the reply has it.
   */
  it("shows a changed tactic's pending version in the reply's place", () => {
    const pending = new Map<string, TacticsLibraryEntry | null>([["b", entry("b", "renamed")]]);

    const laid = layPendingChanges([entry("a"), entry("b", "old"), entry("c")], pending);

    expect(ids(laid)).toEqual(["a", "b", "c"]);
    expect(laid[1].name).toBe("renamed");
  });

  /**
   * Given a reply that still holds a tactic deleted since, and one the reply never held
   * When the deletes are laid over it
   * Then neither is shown.
   */
  it("leaves out a tactic with a pending delete, held by the reply or not", () => {
    const pending = new Map<string, TacticsLibraryEntry | null>([
      ["b", null],
      ["ghost", null],
    ]);

    expect(ids(layPendingChanges([entry("a"), entry("b")], pending))).toEqual(["a"]);
  });

  /**
   * Given a reply, and since it was asked for: one tactic made, one changed and one deleted
   * When they are laid over it
   * Then the made one is in front, the changed one shows its new version, the deleted one is gone,
   *      and the rest keep the reply's order.
   */
  it("lays a mix of made, changed and deleted tactics over a reply", () => {
    const pending = new Map<string, TacticsLibraryEntry | null>([
      ["new", entry("new")],
      ["b", entry("b", "changed")],
      ["c", null],
    ]);

    const laid = layPendingChanges([entry("a"), entry("b"), entry("c"), entry("d")], pending);

    expect(ids(laid)).toEqual(["new", "a", "b", "d"]);
    expect(laid.find((e) => e.id === "b")?.name).toBe("changed");
  });

  /**
   * Given a reply that already holds a tactic made since (its own save answered into it)
   * When that tactic's pending entry is laid over it
   * Then it is shown once, as the pending version.
   */
  it("shows a made tactic once when the reply already holds it", () => {
    const pending = new Map<string, TacticsLibraryEntry | null>([["a", entry("a", "mine")]]);

    const laid = layPendingChanges([entry("a", "from the save")], pending);

    expect(laid).toHaveLength(1);
    expect(laid[0].name).toBe("mine");
  });
});
