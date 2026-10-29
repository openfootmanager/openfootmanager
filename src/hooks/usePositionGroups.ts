import { useCallback, useEffect, useState } from "react";

import { getPositionGroups, type PositionGroupEntry } from "../services/positionService";

// The table is the same for the whole session, and a screen mounts this hook once per badge, so one
// request serves every mount. A failed request is dropped so the next mount asks again; whichever
// request finally succeeds hands the table to every mount, including one whose own request failed.
let pending: Promise<PositionGroupEntry[]> | null = null;
let resolved: PositionGroupEntry[] | null = null;
const mounted = new Set<(table: PositionGroupEntry[]) => void>();

function loadPositionGroups(): Promise<PositionGroupEntry[]> {
  pending ??= getPositionGroups()
    .then((answer) => {
      // A reply that is not a table names no group; a badge then reads as neutral.
      const table = Array.isArray(answer) ? answer : [];
      resolved = table;
      for (const show of mounted) show(table);
      return table;
    })
    .catch((error: unknown) => {
      pending = null;
      throw error;
    });
  return pending;
}

/** Forget the loaded table, so a test can start from the first request again. */
export function resetPositionGroupsCache(): void {
  pending = null;
  resolved = null;
  mounted.clear();
}

/**
 * The group a position belongs to, as the backend defines it, or null until the table arrives (and
 * for a position it does not name). No screen keeps its own position-to-group map.
 */
export function usePositionGroups(): (position: string) => string | null {
  const [table, setTable] = useState<PositionGroupEntry[]>(resolved ?? []);

  useEffect(() => {
    mounted.add(setTable);
    // The table can land between this mount's render and its effect; nothing would tell it then.
    if (resolved) setTable(resolved);
    loadPositionGroups().catch((error: unknown) => {
      console.error("Failed to load the position groups:", error);
    });
    return () => {
      mounted.delete(setTable);
    };
  }, []);

  return useCallback(
    (position) => table.find((entry) => entry.position === position)?.group ?? null,
    [table],
  );
}
