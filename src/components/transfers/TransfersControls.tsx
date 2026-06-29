import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Search, Filter } from "lucide-react";

import type {
  TransferAvailabilityFilter,
  TransferTabView,
} from "./TransfersTab.model";

interface TabDescriptor {
  id: TransferTabView;
  label: string;
  icon: ReactNode;
  count: number;
}

interface AvailabilityFilterDescriptor {
  id: TransferAvailabilityFilter;
  label: string;
  count: number;
}

interface TransfersControlsProps {
  tabs: TabDescriptor[];
  activeView: TransferTabView;
  onSelectView: (view: TransferTabView) => void;
  search: string;
  onSearchChange: (value: string) => void;
  positions: string[];
  posFilter: string | null;
  onSelectPosition: (pos: string | null) => void;
  isPlayersView: boolean;
  availabilityFilters: AvailabilityFilterDescriptor[];
  availabilityFilter: TransferAvailabilityFilter;
  onSelectAvailability: (id: TransferAvailabilityFilter) => void;
  resultCount: number;
}

export default function TransfersControls({
  tabs,
  activeView,
  onSelectView,
  search,
  onSearchChange,
  positions,
  posFilter,
  onSelectPosition,
  isPlayersView,
  availabilityFilters,
  availabilityFilter,
  onSelectAvailability,
  resultCount,
}: TransfersControlsProps) {
  const { t } = useTranslation();

  return (
    <>
      {/* Tab navigation */}
      <div className="flex gap-2 mb-4 flex-wrap">
        {tabs.map((tab) => (
          <button
            type="button"
            key={tab.id}
            onClick={() => onSelectView(tab.id)}
            className={`px-4 py-2 rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-all flex items-center gap-1.5 ${
              activeView === tab.id
                ? "bg-primary-700 text-white shadow-md shadow-primary-700/20"
                : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600 hover:text-gray-700 dark:hover:text-gray-200"
            }`}
          >
            {tab.icon} {tab.label} ({tab.count})
          </button>
        ))}
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-3 mb-4 items-center">
        <div className="relative flex-1 min-w-[180px] max-w-xs">
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 dark:text-gray-500" />
          <input
            type="text"
            placeholder={t("transfers.searchByName")}
            value={search}
            onChange={(e) => onSearchChange(e.target.value)}
            className="w-full pl-9 pr-3 py-2 rounded-lg bg-white dark:bg-navy-800 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-primary-500/50"
          />
        </div>
        <div className="flex gap-1.5">
          <button
            type="button"
            onClick={() => onSelectPosition(null)}
            className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${!posFilter ? "bg-primary-700 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
          >
            {t("common.all")}
          </button>
          {positions.map((pos) => (
            <button
              type="button"
              key={pos}
              onClick={() => onSelectPosition(posFilter === pos ? null : pos)}
              className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${posFilter === pos ? "bg-primary-700 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
            >
              {t(`common.posAbbr.${pos}`)}
            </button>
          ))}
        </div>
        {isPlayersView && (
          <div className="flex flex-wrap gap-1.5">
            {availabilityFilters.map((filter) => (
              <button
                type="button"
                key={filter.id}
                onClick={() => onSelectAvailability(filter.id)}
                className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${availabilityFilter === filter.id ? "bg-accent-500 text-navy-900 shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
              >
                {filter.label} ({filter.count})
              </button>
            ))}
          </div>
        )}
        <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
          <Filter className="w-3.5 h-3.5 inline mr-1 -mt-0.5" />
          {t("common.nResults", { count: resultCount })}
        </p>
      </div>
    </>
  );
}
