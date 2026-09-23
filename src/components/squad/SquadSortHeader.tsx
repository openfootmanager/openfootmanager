import { ChevronDown, ChevronUp } from "lucide-react";

import type { SquadListSortKey } from "./SquadRosterView.state";

/**
 * One sortable column heading in the squad roster.
 *
 * Must stay a module-level component. Declared inside `SquadRosterView` it would be a new
 * component type on every render, so React would unmount and remount every header cell each time
 * the sort or a filter changed — which `noNestedComponentDefinitions` exists to catch.
 */
export function SquadSortHeader({
  col,
  label,
  sortKey,
  sortDir,
  onSort,
}: {
  col: SquadListSortKey;
  label: string;
  sortKey: SquadListSortKey;
  sortDir: "asc" | "desc";
  onSort: (col: SquadListSortKey) => void;
}) {
  const active = sortKey === col;

  return (
    <th
      className={`py-2.5 px-4 font-heading font-bold uppercase tracking-wider cursor-pointer select-none hover:text-primary-400 transition-colors ${active ? "text-primary-500 dark:text-primary-400" : "text-gray-500 dark:text-gray-400"}`}
      onClick={() => onSort(col)}
    >
      <div className="flex items-center gap-1">
        {label}
        {active ? (
          sortDir === "asc" ? (
            <ChevronUp className="w-3 h-3" />
          ) : (
            <ChevronDown className="w-3 h-3" />
          )
        ) : null}
      </div>
    </th>
  );
}
