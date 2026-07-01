import type { ReactNode, RefObject } from "react";
import { useTranslation } from "react-i18next";
import { Search, Filter } from "lucide-react";

import { translatePositionLabel } from "../squad/SquadTab.helpers";
import {
  SPECIFIC_POSITIONS_BY_GROUP,
  type TransferAvailabilityFilter,
  type TransferTabView,
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
  specificPositions: string[];
  openPositionPopover: string | null;
  positionFilterRef: RefObject<HTMLDivElement | null>;
  onSelectPositionGroup: (group: string | null) => void;
  onToggleSpecificPosition: (position: string) => void;
  showAffordable: boolean;
  affordableOnly: boolean;
  onToggleAffordable: () => void;
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
  specificPositions,
  openPositionPopover,
  positionFilterRef,
  onSelectPositionGroup,
  onToggleSpecificPosition,
  showAffordable,
  affordableOnly,
  onToggleAffordable,
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
        <div ref={positionFilterRef} className="flex gap-1.5">
          <button
            type="button"
            onClick={() => onSelectPositionGroup(null)}
            aria-pressed={specificPositions.length === 0}
            aria-label={t("transfers.allPositions")}
            className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${specificPositions.length === 0 ? "bg-primary-700 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
          >
            {t("common.all")}
          </button>
          {positions.map((pos) => {
            const groupSpecifics = SPECIFIC_POSITIONS_BY_GROUP[pos] ?? [];
            const refinable = groupSpecifics.length > 1;
            const selectedInGroup = specificPositions.filter((entry) =>
              groupSpecifics.includes(entry),
            ).length;
            const isActive = selectedInGroup > 0;
            const isPartial =
              isActive && refinable && selectedInGroup < groupSpecifics.length;
            const groupLabel = t(`common.positionGroups.${pos}`, {
              defaultValue: t(`common.positions.${pos}`, { defaultValue: pos }),
            });

            return (
              <div key={pos} className="relative">
                <button
                  type="button"
                  onClick={() => onSelectPositionGroup(pos)}
                  aria-haspopup={refinable ? "true" : undefined}
                  aria-expanded={
                    refinable ? openPositionPopover === pos : undefined
                  }
                  aria-pressed={isPartial ? "mixed" : isActive}
                  aria-label={
                    isPartial
                      ? t("transfers.positionGroupPartialSelection", {
                          group: groupLabel,
                          selected: selectedInGroup,
                          total: groupSpecifics.length,
                        })
                      : groupLabel
                  }
                  className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all inline-flex items-center gap-1 ${isActive ? "bg-primary-700 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
                >
                  {t(`common.posAbbr.${pos}`)}
                  {isPartial && (
                    <span
                      aria-hidden="true"
                      className="bg-white/20 text-[0.65rem] px-1.5 py-0.5 rounded-full leading-none"
                    >
                      {selectedInGroup}/{groupSpecifics.length}
                    </span>
                  )}
                </button>
                {refinable && openPositionPopover === pos && (
                  <div
                    role="dialog"
                    aria-label={t("transfers.refinePositionGroup", {
                      group: groupLabel,
                    })}
                    className="absolute left-0 top-full mt-1 z-20 min-w-[180px] p-2 rounded-lg bg-white dark:bg-navy-800 border border-gray-200 dark:border-navy-600 shadow-lg"
                  >
                    <div className="flex flex-wrap gap-1.5">
                      {groupSpecifics.map((position) => {
                        const selected = specificPositions.includes(position);
                        const positionLabel = translatePositionLabel(
                          t,
                          position,
                        );
                        return (
                          <button
                            type="button"
                            key={position}
                            onClick={() =>
                              onToggleSpecificPosition(position)
                            }
                            aria-pressed={selected}
                            aria-label={positionLabel}
                            title={positionLabel}
                            className={`px-2.5 py-1 rounded-md text-xs font-heading font-bold uppercase tracking-wider transition-all ${selected ? "bg-primary-700 text-white shadow-sm" : "bg-gray-50 dark:bg-navy-700 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600 hover:text-gray-700 dark:hover:text-gray-200"}`}
                          >
                            {t(`common.posAbbr.${position}`)}
                          </button>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>
            );
          })}
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
            {showAffordable && (
              <button
                type="button"
                onClick={onToggleAffordable}
                aria-pressed={affordableOnly}
                title={t("transfers.affordableOnlyHint")}
                className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${affordableOnly ? "bg-primary-700 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
              >
                {t("transfers.affordableOnly")}
              </button>
            )}
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
