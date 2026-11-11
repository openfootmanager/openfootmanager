import { useEffect, useState } from "react";

import { getWorldsLibrary } from "../../../services/packageEditorService";
import type { WorldsLibrary } from "../../../store/generated/WorldsLibrary";

export interface WorldsLibraryState {
  /** The backend's reading of the projects and installed packages; null until it has answered. */
  library: WorldsLibrary | null;
  loading: boolean;
  /** The last ask was refused: the library is unknown, not empty. */
  failed: boolean;
  /** Asks again. */
  retry: () => void;
}

/**
 * The worlds library for the projects at `paths` (`get_worlds_library`): their counts and the installed
 * packages'. Asked again when the set of paths changes, or by `retry`. A failed ask is `failed`, not
 * an empty library: the editor's own record of its projects is still known and still openable.
 */
export function useWorldsLibrary(paths: readonly string[]): WorldsLibraryState {
  const [state, setState] = useState<Omit<WorldsLibraryState, "retry">>({
    library: null,
    loading: true,
    failed: false,
  });
  const [attempt, setAttempt] = useState(0);
  // The paths are an array rebuilt on every render; their content is what asks again.
  const pathsKey = JSON.stringify(paths);

  useEffect(() => {
    let cancelled = false;
    // A failure stays until a request succeeds: a retry that is slow or stalls must not turn it into
    // a loading row that hides the recent projects the failure still lists.
    setState((current) => ({ ...current, loading: true }));
    getWorldsLibrary(JSON.parse(pathsKey) as string[])
      .then((library) => {
        if (!cancelled) setState({ library, loading: false, failed: false });
      })
      .catch((error: unknown) => {
        console.error("Failed to read the worlds library", error);
        if (!cancelled) setState({ library: null, loading: false, failed: true });
      });
    return () => {
      cancelled = true;
    };
  }, [pathsKey, attempt]);

  return { ...state, retry: () => setAttempt((count) => count + 1) };
}
