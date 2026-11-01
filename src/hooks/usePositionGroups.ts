import { useCallback, useEffect, useMemo, useState } from "react";

import { createPositionTable, type PositionTable } from "../lib/positionTable";
import { getPositionGroups, type PositionGroupEntry } from "../services/positionService";

// The table is the same for the whole session, and a screen mounts this hook once per badge, so one
// request serves every mount. A failed request is dropped so the next mount asks again, and while a
// screen stays mounted it asks again on its own (a screen that waits for the table, such as the
// position chips, would otherwise stay disabled for good); whichever request finally succeeds hands
// the table to every mount, including one whose own request failed.
let pending: Promise<PositionGroupEntry[]> | null = null;
let resolved: PositionGroupEntry[] | null = null;
const mounted = new Set<(table: PositionGroupEntry[]) => void>();

/** How long to wait before asking again after the nth failure in a row; the last delay repeats. */
const RETRY_DELAYS_MS = [1_000, 2_000, 5_000, 15_000];
let failures = 0;
let retryTimer: ReturnType<typeof setTimeout> | null = null;

function cancelRetry(): void {
  if (retryTimer !== null) clearTimeout(retryTimer);
  retryTimer = null;
}

function scheduleRetry(): void {
  if (retryTimer !== null || mounted.size === 0) return;
  const delay = RETRY_DELAYS_MS[Math.min(failures - 1, RETRY_DELAYS_MS.length - 1)];
  retryTimer = setTimeout(() => {
    retryTimer = null;
    // Everyone left, or another mount's request already won: nothing to ask for.
    if (resolved || mounted.size === 0) return;
    loadPositionGroups().catch((error: unknown) => {
      console.error("Failed to load the position groups:", error);
    });
  }, delay);
}

function loadPositionGroups(): Promise<PositionGroupEntry[]> {
  pending ??= getPositionGroups()
    .then((answer) => {
      // A reply that is not a table names no group; a badge then reads as neutral.
      const table = Array.isArray(answer) ? answer : [];
      resolved = table;
      failures = 0;
      cancelRetry();
      for (const show of mounted) show(table);
      return table;
    })
    .catch((error: unknown) => {
      pending = null;
      failures += 1;
      scheduleRetry();
      throw error;
    });
  return pending;
}

/** Forget the loaded table, so a test can start from the first request again. */
export function resetPositionGroupsCache(): void {
  pending = null;
  resolved = null;
  failures = 0;
  cancelRetry();
  mounted.clear();
}

/** The backend's position table as the screens read it, empty until it arrives. */
function usePositionEntries(): PositionGroupEntry[] {
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
