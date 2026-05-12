import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import {
  CORE_POSITIONS,
  translatePositionAbbreviation,
} from "../squad/SquadTab.helpers";
import { Card, Select } from "../ui";

interface TacticsFiltersProps {
  onClear: () => void;
  onPlayerSearchChange: (value: string) => void;
  onPositionFilterChange: (value: string) => void;
  playerSearch: string;
  positionFilter: string;
}

function getClearButtonClassName(isEnabled: boolean): string {
  if (isEnabled) {
    return "rounded-lg bg-gray-100 px-3 py-2 text-xs font-heading font-bold uppercase tracking-wider text-gray-600 transition-all hover:bg-gray-200 dark:bg-navy-700 dark:text-gray-300 dark:hover:bg-navy-600";
  }

  return "cursor-not-allowed rounded-lg bg-gray-100 px-3 py-2 text-xs font-heading font-bold uppercase tracking-wider text-gray-400 transition-all dark:bg-navy-700";
}

export default function TacticsFilters({
  onClear,
  onPlayerSearchChange,
  onPositionFilterChange,
  playerSearch,
  positionFilter,
}: TacticsFiltersProps): JSX.Element {
  const { t } = useTranslation();
  const canClear = playerSearch.trim().length > 0 || positionFilter !== "All";

  return (
    <Card>
      <div className="grid grid-cols-1 items-end gap-3 p-4 lg:grid-cols-[minmax(0,1.3fr)_220px_auto]">
        <div>
          <label className="mb-2 block text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
            {t("common.search")}
          </label>
          <input
            type="text"
            value={playerSearch}
            onChange={(event) => onPlayerSearchChange(event.target.value)}
            placeholder={t("squad.filterPlayers")}
            className="w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm text-gray-700 placeholder:text-gray-400 focus:outline-none focus:ring-2 focus:ring-primary-500/30 dark:border-navy-600 dark:bg-navy-800 dark:text-gray-200"
          />
        </div>
        <div>
          <label className="mb-2 block text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
            {t("squad.pos")}
          </label>
          <Select
            value={positionFilter}
            onChange={(event) => onPositionFilterChange(event.target.value)}
            fullWidth
          >
            <option value="All">{t("common.all")}</option>
            {CORE_POSITIONS.map((position) => (
              <option key={position} value={position}>
                {translatePositionAbbreviation(t, position)}
              </option>
            ))}
          </Select>
        </div>
        <button
          type="button"
          onClick={onClear}
          disabled={!canClear}
          className={getClearButtonClassName(canClear)}
        >
          {t("common.clear")}
        </button>
      </div>
    </Card>
  );
}
