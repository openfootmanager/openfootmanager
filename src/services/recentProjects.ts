import type { RecentProject } from "../store/packageEditorTypes";

const KEY = "worldEditor.recentProjects";

export function updateRecentValidation(
  records: RecentProject[],
  path: string,
  validation: NonNullable<RecentProject["lastValidation"]>,
): RecentProject[] {
  const updated = records.map((record) =>
    record.path === path ? { ...record, lastValidation: validation } : record,
  );
  writeRecentProjects(updated);
  return updated;
}

function readStoredProjects(): RecentProject[] | null {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) &&
      parsed.every(
        (record: unknown) =>
          typeof record === "object" &&
          record !== null &&
          "path" in record &&
          typeof record.path === "string" &&
          "name" in record &&
          typeof record.name === "string" &&
          "openedAt" in record &&
          typeof record.openedAt === "string",
      )
      ? (parsed as RecentProject[])
      : null;
  } catch {
    return null;
  }
}

export function readRecentProjects(fallback: RecentProject[] = []): RecentProject[] {
  return readStoredProjects() ?? fallback;
}

function writeRecentProjects(records: RecentProject[]): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(records));
  } catch {
    // The editor remains usable when persistent storage is unavailable.
  }
}

export function rememberRecentProject(
  records: RecentProject[],
  path: string,
  name: string,
): RecentProject[] {
  const previous = records.find((record) => record.path === path);
  const updated = [
    { ...previous, path, name, openedAt: new Date().toISOString() },
    ...records.filter((record) => record.path !== path),
  ].slice(0, 8);
  writeRecentProjects(updated);
  return updated;
}
