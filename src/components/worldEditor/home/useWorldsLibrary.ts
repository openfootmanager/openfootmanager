import { useEffect, useState } from "react";

import { getWorldsLibrary } from "../../../services/packageEditorService";
import type { WorldsLibrary } from "../../../store/generated/WorldsLibrary";

export interface WorldsLibraryState {
  /** The backend's reading of the projects and installed packages; null until it has answered. */
  library: WorldsLibrary | null;
  loading: boolean;
}

/**
 * The worlds library for the projects at `paths` (`get_worlds_library`): their counts and the installed
 * packages'. Asked again when the set of paths changes; a failed ask leaves the table empty rather than
 * the screen broken, as the editor's home has no other way to say it.
 */
export function useWorldsLibrary(paths: readonly string[]): WorldsLibraryState {
  const [state, setState] = useState<WorldsLibraryState>({ library: null, loading: true });
  // The paths are an array rebuilt on every render; their content is what asks again.
  const pathsKey = JSON.stringify(paths);

  useEffect(() => {
    let cancelled = false;
    setState((current) => ({ ...current, loading: true }));
    getWorldsLibrary(JSON.parse(pathsKey) as string[])
      .then((library) => {
        if (!cancelled) setState({ library, loading: false });
      })
      .catch((error: unknown) => {
        console.error("Failed to read the worlds library", error);
        if (!cancelled) setState({ library: { projects: [], installed: [] }, loading: false });
      });
    return () => {
      cancelled = true;
    };
  }, [pathsKey]);

  return state;
}
