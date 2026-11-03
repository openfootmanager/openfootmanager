import { ArrowDown, ArrowUp } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Select } from "./Select";

interface SortOption<Key extends string> {
  key: Key;
  /** The field's name, translated: "Age", "Value". */
  label: string;
}

interface SortControlProps<Key extends string> {
  options: readonly SortOption<Key>[];
  sortKey: Key;
  ascending: boolean;
  /** Called with the field and direction now chosen. */
  onChange: (key: Key, ascending: boolean) => void;
  /** The direction a field starts in when it is picked (a name runs A to Z, a rating high to low). */
  defaultAscending?: (key: Key) => boolean;
}

/**
 * The sort of a card list, where there are no column headers to click: the field in a compact
 * select and the direction as a toggle. It writes the same sort the table headers write, so
 * switching between the table and the cards keeps the order.
 */
export function SortControl<Key extends string>({
  options,
  sortKey,
  ascending,
  onChange,
  defaultAscending,
}: SortControlProps<Key>) {
  const { t } = useTranslation();
  const DirectionIcon = ascending ? ArrowUp : ArrowDown;
  return (
    <div className="flex items-center gap-1.5">
      <Select
        selectSize="sm"
        aria-label={t("common.sortBy")}
        value={sortKey}
        onChange={(event) => {
          const key = event.target.value as Key;
          onChange(key, defaultAscending ? defaultAscending(key) : ascending);
        }}
      >
        {options.map((option) => (
          <option key={option.key} value={option.key}>
            {option.label}
          </option>
        ))}
      </Select>
      <button
        type="button"
        aria-label={t("common.sortAscending")}
        aria-pressed={ascending}
        onClick={() => onChange(sortKey, !ascending)}
        className="grid size-8 place-items-center rounded-lg border border-line bg-panel text-fg-muted hover:bg-hover hover:text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page"
      >
        <DirectionIcon aria-hidden="true" className="h-4 w-4" />
      </button>
    </div>
  );
}
