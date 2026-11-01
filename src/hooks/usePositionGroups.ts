import { useCallback, useEffect, useMemo, useState } from "react";

import { createPositionTable, type PositionTable } from "../lib/positionTable";
import { getPositionGroups, type PositionGroupEntry } from "../services/positionService";
import { createSharedLoader } from "./sharedLoader";

// The table is the same for the whole session, so it has one key.
const TABLE_KEY = "positions";
const loader = createSharedLoader<PositionGroupEntry[]>(
  // A reply that is not a table names no group; a badge then reads as neutral.
  async () => {
    const answer = await getPositionGroups();
    return Array.isArray(answer) ? answer : [];
  },
  "position groups",
);

/** Forget the loaded table, so a test can start from the first request again. */
export function resetPositionGroupsCache(): void {
  loader.reset();
}

/** The backend's position table as the screens read it, empty until it arrives. */
function usePositionEntries(): PositionGroupEntry[] {
  const [table, setTable] = useState<PositionGroupEntry[]>(loader.cached(TABLE_KEY) ?? []);
  useEffect(() => loader.subscribe(TABLE_KEY, setTable), []);
  return table;
}

/**
 * The group a position belongs to, as the backend defines it, or null until the table arrives (and
 * for a position it does not name). No screen keeps its own position-to-group map.
 */
export function usePositionGroups(): (position: string) => string | null {
  const table = usePositionEntries();
  return useCallback(
    (position) => table.find((entry) => entry.position === position)?.group ?? null,
    [table],
  );
}

/**
 * The backend's position table with the questions the squad, tactics and filter screens ask of it:
 * group, sort rank, line rank and a group's members. Pure helpers take it as an argument.
 */
export function usePositionTable(): PositionTable {
  const entries = usePositionEntries();
  return useMemo(() => createPositionTable(entries), [entries]);
}
