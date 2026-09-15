import { ChevronDown, ChevronUp } from "lucide-react";

import type {
  SquadListSortKey,
  SquadListSortState,
} from "./SquadRosterView.state";

interface SquadSortHeaderProps {
  col: SquadListSortKey;
  label: string;
  sortKey: SquadListSortState["sortKey"];
  sortDir: SquadListSortState["sortDir"];
  onSort: (key: SquadListSortKey) => void;
}

/**
 * One sortable column heading.
 *
 * Lives at module scope on purpose. Declared inside the roster component it was
 * a fresh component type on every render, so React tore down and rebuilt all ten
 * header cells each time the filters or the sort changed.
 */
export default function SquadSortHeader({
  col,
  label,
  sortKey,
  sortDir,
  onSort,
}: SquadSortHeaderProps) {
  return (
  <th
    className={`py-2.5 px-4 font-heading font-bold uppercase tracking-wider cursor-pointer select-none hover:text-primary-400 transition-colors ${sortKey === col ? "text-primary-500 dark:text-primary-400" : "text-gray-500 dark:text-gray-400"}`}
    onClick={() => onSort(col)}
  >
    <div className="flex items-center gap-1">
      {label}
      {sortKey === col ? (
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
