import { describe, expect, it, vi } from "vitest";

import type { TeamDef } from "../menu/PackageEditor/types";
import {
  acknowledge,
  hasUnsavedChanges,
  planSave,
  saveChanges,
  type EditorCollections,
} from "./savePlan";

function world(): EditorCollections {
  return {
    meta: { id: "w" } as EditorCollections["meta"],
    confederations: [],
    countries: [],
    teams: [{ id: "fc-a" } as TeamDef],
    players: [],
    staff: [],
    names: { version: 1, description: "", pools: {} },
    competitions: [],
  };
}

function deps(initial: EditorCollections, acknowledgedAtOpen = initial) {
  const state = { current: initial, acknowledged: acknowledgedAtOpen, generation: 1 };
  const send = vi.fn(async (collections: Partial<EditorCollections>) => ({
    saved: Object.keys(collections),
  }));
  return {
    state,
    send,
    deps: {
      generation: () => state.generation,
      current: () => state.current,
      acknowledged: () => state.acknowledged,
      send,
      onAcknowledged: (next: EditorCollections) => {
        state.acknowledged = next;
      },
    },
  };
}

describe("planSave", () => {
  /**
   * Given a project saved as opened
   * When nothing was edited
   * Then the plan sends no collection at all
   */
  it("sends nothing for a clean project", () => {
    const opened = world();
    expect(planSave(opened, opened)).toEqual({});
  });

  /**
   * Given a project in which one club was edited
   * When the save is planned
   * Then only the clubs are sent: the players key is omitted, not sent as an empty list
   */
  it("sends only the collection that changed, and omits the rest", () => {
    const opened = world();
    const edited = { ...opened, teams: [{ id: "fc-a", name: "Renamed" } as TeamDef] };

    const plan = planSave(edited, opened);

    expect(Object.keys(plan)).toEqual(["teams"]);
    expect("players" in plan).toBe(false);
  });

  /**
   * Given a collection the author emptied
   * When the save is planned
   * Then it is sent as an empty list: emptied is not omitted
   */
  it("sends a collection that was emptied", () => {
    const opened = world();
    const edited = { ...opened, teams: [] };

    expect(planSave(edited, opened).teams).toEqual([]);
  });

  /**
   * Given an edit whose new list has not been rendered yet
   * When the save is planned with it as an override
   * Then the override is what is sent, not the editor's stale list
   */
  it("sends an override in place of the editor's current collection", () => {
    const opened = world();
    const override = [{ id: "fc-b" } as TeamDef];

    expect(planSave(opened, opened, { teams: override }).teams).toBe(override);
  });

  /**
   * Given an edit undone back to the acknowledged list
   * When the save is planned
   * Then nothing is sent and nothing is unsaved
   */
  it("treats an undo back to the saved state as clean", () => {
    const opened = world();

    expect(planSave({ ...opened, teams: opened.teams }, opened)).toEqual({});
    expect(hasUnsavedChanges({ ...opened }, opened)).toBe(false);
  });
});

describe("acknowledge", () => {
  /**
   * Given a save that submitted the clubs
   * When only the names are reported written
   * Then the clubs are not acknowledged
   */
  it("acknowledges only the collections the backend reported", () => {
    const opened = world();
    const teams = [{ id: "fc-z" } as TeamDef];

    const next = acknowledge(opened, { teams }, ["names"]);

    expect(next.teams).toBe(opened.teams);
  });
});

describe("saveChanges", () => {
  /**
   * Given a save of the clubs in flight
   * When the author edits the clubs again before it finishes
   * Then the finished save acknowledges the clubs it submitted, the newer list stays unsaved, and
   *      the next save sends the clubs again
   */
  it("keeps an edit made during a save unsaved", async () => {
    const opened = world();
    const first = [{ id: "fc-a", name: "One" } as TeamDef];
    const second = [{ id: "fc-a", name: "Two" } as TeamDef];
    const t = deps({ ...opened, teams: first }, opened);
    t.send.mockImplementationOnce(async (collections) => {
      t.state.current = { ...t.state.current, teams: second };
      return { saved: Object.keys(collections) };
    });

    await saveChanges(1, undefined, t.deps);

    expect(t.state.acknowledged.teams).toBe(first);
    expect(hasUnsavedChanges(t.state.current, t.state.acknowledged)).toBe(true);
    await saveChanges(1, undefined, t.deps);
    expect(t.send).toHaveBeenCalledTimes(2);
    expect(t.send.mock.calls[1][0].teams).toBe(second);
  });

  /**
   * Given a save that fails
   * When the author saves again
   * Then nothing was acknowledged by the failure, and the retry sends the same collections
   */
  it("resends everything a failed save did not acknowledge", async () => {
    const opened = world();
    const edited = {
      ...opened,
      teams: [{ id: "fc-a", name: "Renamed" } as TeamDef],
      names: { version: 1, description: "edited", pools: {} },
    };
    const t = deps(edited, opened);
    t.send.mockRejectedValueOnce("be.error.disk");

    await expect(saveChanges(1, undefined, t.deps)).rejects.toBe("be.error.disk");
    expect(t.state.acknowledged).toBe(opened);
    await saveChanges(1, undefined, t.deps);

    expect(Object.keys(t.send.mock.calls[1][0]).sort()).toEqual(["names", "teams"]);
    expect(hasUnsavedChanges(t.state.current, t.state.acknowledged)).toBe(false);
  });

  /**
   * Given a save queued while project A was open
   * When project B has been opened by the time it runs
   * Then nothing is sent: B's collections must not be written into A's folder
   */
  it("sends nothing for a project that was replaced before the save ran", async () => {
    const opened = world();
    const t = deps({ ...opened, teams: [] }, opened);
    t.state.generation = 2;

    expect(await saveChanges(1, undefined, t.deps)).toBe("stale");
    expect(t.send).not.toHaveBeenCalled();
  });

  /**
   * Given a save in flight for project A
   * When project B is opened before it finishes
   * Then the finished save acknowledges nothing, so B does not inherit A's state
   */
  it("acknowledges nothing for a project that was replaced during the save", async () => {
    const opened = world();
    const t = deps({ ...opened, teams: [] }, opened);
    t.send.mockImplementationOnce(async (collections) => {
      t.state.generation = 2;
      return { saved: Object.keys(collections) };
    });

    expect(await saveChanges(1, undefined, t.deps)).toBe("stale");
    expect(t.state.acknowledged).toBe(opened);
  });

  /**
   * Given a clean project
   * When a save runs
   * Then it does not call the backend at all
   */
  it("does not call the backend for a clean project", async () => {
    const opened = world();
    const t = deps(opened);

    expect(await saveChanges(1, undefined, t.deps)).toBe("clean");
    expect(t.send).not.toHaveBeenCalled();
  });
});
