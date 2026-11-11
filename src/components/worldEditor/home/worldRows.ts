import type { WorldLibraryRow } from "../../../store/generated/WorldLibraryRow";
import type { WorldsLibrary } from "../../../store/generated/WorldsLibrary";
import type { RecentProject } from "../../../store/packageEditorTypes";

export type WorldKind = "project" | "installed";
export type WorldFilter = "all" | "projects" | "installed";

/** What the last Validate said about a project, as a fact for its row; an installed one is never checked. */
export type WorldCheck =
  | { kind: "clean" }
  | { kind: "issues"; count: number }
  | { kind: "unchecked" }
  /** The folder could not be read: `errorKey` is the backend's translation key for why. */
  | { kind: "unreadable"; errorKey: string };

export interface WorldRow {
  /** The folder of a project or the `.ofm` file of an installed package: what opening it needs. */
  path: string;
  kind: WorldKind;
  name: string;
  version: string | null;
  clubs: number;
  players: number;
  /** When a project was last opened in the editor; null for an installed package. */
  openedAt: string | null;
  check: WorldCheck;
}

function lastFolderOf(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

function projectRow(row: WorldLibraryRow, recent: ReadonlyMap<string, RecentProject>): WorldRow {
  const record = recent.get(row.path);
  let check: WorldCheck = { kind: "unchecked" };
  if (row.error) {
    check = { kind: "unreadable", errorKey: row.error };
  } else if (record?.lastValidation) {
    const count = record.lastValidation.errors + record.lastValidation.warnings;
    check = count === 0 ? { kind: "clean" } : { kind: "issues", count };
  }
  return {
    path: row.path,
    kind: "project",
    // A folder the library could not read has no name of its own: the editor's record, else its folder.
    name: row.name || record?.name || lastFolderOf(row.path),
    version: row.version,
    clubs: row.clubs,
    players: row.players,
    openedAt: record?.openedAt ?? null,
    check,
  };
}

/**
 * The worlds library as the table lists it: the projects in the order the editor opened them, then the
 * installed packages. A project's last-opened time and last check are the editor's own record of it
 * (`recentProjects`); the counts and the name are the backend's reading of its files.
 */
export function buildWorldRows(
  library: WorldsLibrary,
  recentProjects: readonly RecentProject[],
): WorldRow[] {
  const recent = new Map(recentProjects.map((project) => [project.path, project]));
  const installed: WorldRow[] = library.installed.map((row) => ({
    path: row.path,
    kind: "installed",
    name: row.name,
    version: row.version,
    clubs: row.clubs,
    players: row.players,
    openedAt: null,
    check: { kind: "unchecked" },
  }));
  return [...library.projects.map((row) => projectRow(row, recent)), ...installed];
}

/** The rows the filter and the search keep: a search matches a name or a path, ignoring case. */
export function filterWorldRows(
  rows: readonly WorldRow[],
  filter: WorldFilter,
  query: string,
): WorldRow[] {
  const needle = query.trim().toLowerCase();
  return rows.filter((row) => {
    if (filter === "projects" && row.kind !== "project") return false;
    if (filter === "installed" && row.kind !== "installed") return false;
    return (
      needle === "" ||
      row.name.toLowerCase().includes(needle) ||
      row.path.toLowerCase().includes(needle)
    );
  });
}
