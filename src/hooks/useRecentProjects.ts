import { useState } from "react";
import type { RecentProject } from "../components/worldEditor/WorldEditorHome";

const RECENT_PROJECTS_KEY = "worldEditor.recentProjects";
const MAX_RECENT = 8;

function readRecentProjects(): RecentProject[] {
  try {
    const raw = localStorage.getItem(RECENT_PROJECTS_KEY);
    return raw ? (JSON.parse(raw) as RecentProject[]) : [];
  } catch {
    return [];
  }
}

export interface RecentProjects {
  recentProjects: RecentProject[];
  addRecentProject: (path: string, name: string) => void;
}

/**
 * Most-recently-opened World Editor projects, persisted to localStorage and
 * capped at MAX_RECENT entries (newest first, de-duplicated by path).
 */
export function useRecentProjects(): RecentProjects {
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>(readRecentProjects);

  function addRecentProject(path: string, name: string) {
    setRecentProjects((prev) => {
      const filtered = prev.filter((p) => p.path !== path);
      const updated = [{ path, name, openedAt: new Date().toISOString() }, ...filtered].slice(
        0,
        MAX_RECENT,
      );
      try {
        localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(updated));
      } catch { /* ignore */ }
      return updated;
    });
  }

  return { recentProjects, addRecentProject };
}
