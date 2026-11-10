import type { RecentProject } from "../store/packageEditorTypes";

const KEY = "worldEditor.recentProjects";

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

export function recordValidation(path: string, errors: number, warnings: number): void {
  const records = readStoredProjects();
  if (!records) return;
  const updated = records.map((record) =>
    record.path === path
      ? {
          ...record,
          lastValidation: { errors, warnings, checkedAt: new Date().toISOString() },
        }
      : record,
  );
  writeRecentProjects(updated);
}
