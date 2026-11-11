import type { WorldsLibrary } from "../../store/generated/WorldsLibrary";
import type { RecentProject } from "../../store/packageEditorTypes";
import type { HarnessWorldLibrary } from "./harnessPage";

const row = (path: string, name: string, clubs: number, players: number, extra = {}) => ({
  path,
  name,
  version: null,
  clubs,
  players,
  countries: 3,
  error: null,
  ...extra,
});

/** The projects the editor has opened, as its own record holds them (the last open and check). */
const RECENT_PROJECTS: RecentProject[] = [
  {
    path: "/home/user/Worlds/nordic-leagues-2027",
    name: "Nordic Leagues 2027",
    openedAt: "2026-11-09T10:00:00Z",
    lastValidation: { errors: 0, warnings: 0, checkedAt: "2026-11-09T10:05:00Z" },
  },
  {
    path: "/home/user/Worlds/classic-1962-with-a-very-long-folder-name-to-see-it-wrap",
    name: "Classic 1962",
    openedAt: "2026-11-02T10:00:00Z",
    lastValidation: { errors: 1, warnings: 1, checkedAt: "2026-11-02T10:05:00Z" },
  },
  {
    path: "/home/user/Worlds/test-small",
    name: "Test world (small)",
    openedAt: "2026-10-28T10:00:00Z",
  },
  {
    path: "/home/user/Worlds/moved-away",
    name: "A world whose folder moved",
    openedAt: "2026-10-20T10:00:00Z",
  },
];

/** What `get_worlds_library` answers for the recent projects above and two installed packages. */
const POPULATED: WorldsLibrary = {
  projects: [
    row("/home/user/Worlds/nordic-leagues-2027", "Nordic Leagues 2027", 412, 9860),
    row(
      "/home/user/Worlds/classic-1962-with-a-very-long-folder-name-to-see-it-wrap",
      "Classic 1962",
      188,
      4120,
    ),
    row("/home/user/Worlds/test-small", "Test world (small)", 32, 740),
    row("/home/user/Worlds/moved-away", "", 0, 0, {
      error: "be.error.package.notAProjectDirectory",
    }),
  ],
  installed: [
    row("/packages/nordic-leagues-2027.ofm", "Nordic Leagues 2027", 412, 9860, { version: "1.2" }),
    row("/packages/classic-1962.ofm", "Classic 1962", 188, 4120, { version: "1.0" }),
  ],
};

/** What the home is given for a library state: the editor's recent projects, and the backend's answer. */
export function worldsFixture(library: HarnessWorldLibrary): {
  recentProjects: RecentProject[];
  answer: WorldsLibrary;
} {
  return library === "empty"
    ? { recentProjects: [], answer: { projects: [], installed: [] } }
    : { recentProjects: RECENT_PROJECTS, answer: POPULATED };
}
