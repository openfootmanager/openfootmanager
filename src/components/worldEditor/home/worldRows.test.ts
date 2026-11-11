import { describe, expect, it } from "vitest";

import type { WorldsLibrary } from "../../../store/generated/WorldsLibrary";
import type { RecentProject } from "../../../store/packageEditorTypes";
import { buildWorldRows, fallbackWorldRows, filterWorldRows } from "./worldRows";

const row = (path: string, name: string, overrides = {}) => ({
  path,
  name,
  version: null,
  clubs: 12,
  players: 300,
  countries: 1,
  error: null,
  ...overrides,
});

const LIBRARY: WorldsLibrary = {
  projects: [row("/w/nordic", "Nordic Leagues"), row("/w/classic", "Classic 1962")],
  installed: [row("/p/nordic.ofm", "Nordic Leagues", { version: "1.2" })],
};

const RECENT: RecentProject[] = [
  {
    path: "/w/nordic",
    name: "Nordic (recent)",
    openedAt: "2026-11-09T10:00:00Z",
    lastValidation: { errors: 0, warnings: 0, checkedAt: "2026-11-09T10:05:00Z" },
  },
  {
    path: "/w/classic",
    name: "Classic",
    openedAt: "2026-11-02T10:00:00Z",
    lastValidation: { errors: 1, warnings: 1, checkedAt: "2026-11-02T10:05:00Z" },
  },
];

describe("buildWorldRows", () => {
  /**
   * Given two projects the editor has opened and one installed package
   * When the rows are built
   * Then the projects come first, in the editor's order, with the backend's counts, the editor's own
   *      last-opened time and last check; the installed package follows, never opened or checked.
   */
  it("lists projects then installed packages, with the editor's record of each project", () => {
    const rows = buildWorldRows(LIBRARY, RECENT);

    expect(rows.map((world) => [world.kind, world.name, world.clubs, world.players])).toEqual([
      ["project", "Nordic Leagues", 12, 300],
      ["project", "Classic 1962", 12, 300],
      ["installed", "Nordic Leagues", 12, 300],
    ]);
    expect(rows[0]).toMatchObject({ openedAt: "2026-11-09T10:00:00Z", check: { kind: "clean" } });
    expect(rows[1].check).toEqual({ kind: "issues", count: 2 });
    expect(rows[2]).toMatchObject({ openedAt: null, check: { kind: "unchecked" }, version: "1.2" });
  });

  it("says a project never validated is not checked", () => {
    const rows = buildWorldRows(LIBRARY, [
      { path: "/w/nordic", name: "N", openedAt: "2026-11-09T10:00:00Z" },
    ]);

    expect(rows[0].check).toEqual({ kind: "unchecked" });
  });

  /**
   * Given a project folder the backend could not read
   * When the rows are built
   * Then its row is unreadable with the backend's reason, named by the editor's record or, failing
   *      that, its folder, and its counts are zero.
   */
  it("names an unreadable project by the editor's record or its folder, with the backend's reason", () => {
    const library: WorldsLibrary = {
      projects: [
        row("/w/gone", "", {
          clubs: 0,
          players: 0,
          error: "be.error.package.notAProjectDirectory",
        }),
        row("/w/also-gone", "", { clubs: 0, players: 0, error: "be.error.package.readFailed" }),
      ],
      installed: [],
    };

    const rows = buildWorldRows(library, [
      { path: "/w/gone", name: "Gone world", openedAt: "2026-11-01T10:00:00Z" },
    ]);

    expect(rows.map((world) => world.name)).toEqual(["Gone world", "also-gone"]);
    expect(rows[0].check).toEqual({
      kind: "unreadable",
      errorKey: "be.error.package.notAProjectDirectory",
    });
  });
});

describe("fallbackWorldRows", () => {
  /**
   * Given the editor's record of its projects, and a library that could not be read
   * When the rows are built from the record alone
   * Then each project is listed by its name and folder with its last open and check, its counts
   *      unknown (null), so it can still be opened: the folder is known.
   */
  it("lists the recent projects from the editor's own record, with counts unknown", () => {
    const rows = fallbackWorldRows(RECENT);

    expect(
      rows.map((world) => [world.kind, world.name, world.path, world.clubs, world.players]),
    ).toEqual([
      ["project", "Nordic (recent)", "/w/nordic", null, null],
      ["project", "Classic", "/w/classic", null, null],
    ]);
    expect(rows[0]).toMatchObject({ openedAt: "2026-11-09T10:00:00Z", check: { kind: "clean" } });
    expect(rows[1].check).toEqual({ kind: "issues", count: 2 });
  });

  it("names a project the record has no name for by its folder, and says it was never checked", () => {
    const [world] = fallbackWorldRows([
      { path: "/w/some-folder", name: "", openedAt: "2026-11-01T10:00:00Z" },
    ]);

    expect(world).toMatchObject({ name: "some-folder", check: { kind: "unchecked" } });
  });
});

describe("filterWorldRows", () => {
  const rows = buildWorldRows(LIBRARY, RECENT);

  it("keeps the kind the filter names", () => {
    expect(filterWorldRows(rows, "projects", "").map((world) => world.kind)).toEqual([
      "project",
      "project",
    ]);
    expect(filterWorldRows(rows, "installed", "").map((world) => world.kind)).toEqual([
      "installed",
    ]);
    expect(filterWorldRows(rows, "all", "")).toHaveLength(3);
  });

  it("matches a name or a path, ignoring case, within the filter", () => {
    expect(filterWorldRows(rows, "all", "NORDIC")).toHaveLength(2);
    expect(filterWorldRows(rows, "all", "/w/classic").map((world) => world.name)).toEqual([
      "Classic 1962",
    ]);
    expect(filterWorldRows(rows, "installed", "classic")).toEqual([]);
  });
});
