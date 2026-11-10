import type { RecentProject } from "../store/packageEditorTypes";

export function rememberRecentProject(records: RecentProject[], path: string, name: string): RecentProject[] {
  return [{ path, name, openedAt: new Date().toISOString() }, ...records.filter((record) => record.path !== path)].slice(0, 8);
}
