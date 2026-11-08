import { describe, expect, it, vi } from "vitest";

import type { TeamDef } from "../menu/PackageEditor/types";
import { hasUnsavedChanges, type Acknowledged, type EditorCollections } from "./savePlan";
import { createSaveSession } from "./saveSession";

function project(name: string): EditorCollections {
  return {
    meta: { id: name } as EditorCollections["meta"],
    confederations: [],
    countries: [],
    teams: [{ id: `${name}-fc` } as TeamDef],
    players: [],
    staff: [],
    names: { version: 1, description: "", pools: {} },
    competitions: [],
  };
}

function editor(first: EditorCollections) {
  const state = { held: first, acknowledged: null as Acknowledged | null };
  const writes: { dir: string; keys: string[] }[] = [];
  const release: Array<() => void> = [];
  const session = createSaveSession({
    current: () => state.held,
    send: (dir, collections) =>
      new Promise((resolve) => {
        writes.push({ dir, keys: Object.keys(collections) });
        release.push(() => resolve({ saved: Object.keys(collections) }));
      }),
    onChange: (next) => {
      state.acknowledged = next;
    },
  });
  return { state, writes, release, session };
}

describe("createSaveSession", () => {
  /**
   * Given a save of project A in flight and a second save of A queued behind it
   * When project B is opened before the queued save reaches the front
   * Then the queued save sends nothing: B's collections must not be written into A's folder
   *      (the project that is open is read when the save is asked for, not when it runs)
   */
  it("drops a save queued for a project that was replaced before it ran", async () => {
    const a = project("a");
    const t = editor(a);
    t.session.openProject(a);
    t.state.held = { ...a, teams: [{ id: "a-fc", name: "One" } as TeamDef] };
    const running = t.session.save("/projects/a");
    t.state.held = { ...a, teams: [{ id: "a-fc", name: "Two" } as TeamDef] };
    const queued = t.session.save("/projects/a");

    await vi.waitFor(() => expect(t.writes).toHaveLength(1));
    const b = project("b");
    t.state.held = b;
    t.session.openProject(b);
    t.release[0]();

    expect(await running).toBe("stale");
    expect(await queued).toBe("stale");
    expect(t.writes).toEqual([{ dir: "/projects/a", keys: ["teams"] }]);
    expect(hasUnsavedChanges(b, t.state.acknowledged as Acknowledged)).toBe(false);
  });

  /**
   * Given the project is opened
   * When one club is edited and saved
   * Then only the clubs are sent, to the folder the save was asked for, and the project is clean
   */
  it("sends the changed collection to the project it was asked for and acknowledges it", async () => {
    const a = project("a");
    const t = editor(a);
    t.session.openProject(a);
    const edited = { ...a, teams: [{ id: "a-fc", name: "Renamed" } as TeamDef] };
    t.state.held = edited;

    const saving = t.session.save("/projects/a");
    await vi.waitFor(() => expect(t.writes).toHaveLength(1));
    t.release[0]();

    expect(await saving).toBe("saved");
    expect(t.writes).toEqual([{ dir: "/projects/a", keys: ["teams"] }]);
    expect(hasUnsavedChanges(edited, t.state.acknowledged as Acknowledged)).toBe(false);
  });

  /**
   * Given a save of the clubs in flight
   * When the author edits them again and asks for another save
   * Then the second does not start until the first finishes, and then sends the newer clubs:
   *      writes run one at a time, and the edit made during the first stays unsaved until it
   */
  it("runs saves one at a time and resends an edit made during the first", async () => {
    const a = project("a");
    const t = editor(a);
    t.session.openProject(a);
    t.state.held = { ...a, teams: [{ id: "a-fc", name: "One" } as TeamDef] };
    const first = t.session.save("/projects/a");
    await vi.waitFor(() => expect(t.writes).toHaveLength(1));
    t.state.held = { ...a, teams: [{ id: "a-fc", name: "Two" } as TeamDef] };
    const second = t.session.save("/projects/a");
    await Promise.resolve();

    expect(t.writes).toHaveLength(1);
    t.release[0]();
    await first;
    await vi.waitFor(() => expect(t.writes).toHaveLength(2));
    t.release[1]();
    expect(await second).toBe("saved");
    expect(hasUnsavedChanges(t.state.held, t.state.acknowledged as Acknowledged)).toBe(false);
  });
});
