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

  // The <th> stays the column header and carries the sort state; the control inside it is a real
  // button, so it is in the tab order and Enter and Space work. The ring is inset rather than
  // offset because the table scrolls inside an `overflow-x-auto` box, which clips anything drawn
  // outside the cell — an offset ring would be cut off along the header row.
  return (
    <th
      aria-sort={active ? (sortDir === "asc" ? "ascending" : "descending") : undefined}
      className="p-0"
    >
      <button
        type="button"
        onClick={() => onSort(col)}
        className={`flex w-full items-center gap-1 py-2.5 px-4 font-heading font-bold uppercase tracking-wider select-none transition-colors hover:text-primary-400 dark:hover:text-primary-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary-500 ${active ? "text-primary-500 dark:text-primary-400" : "text-gray-500 dark:text-gray-400"}`}
      >
        {label}
        {active ? (
          sortDir === "asc" ? (
            <ChevronUp className="w-3 h-3" aria-hidden="true" />
          ) : (
            <ChevronDown className="w-3 h-3" aria-hidden="true" />
          )
        ) : null}
      </button>
    </th>
  );
}
