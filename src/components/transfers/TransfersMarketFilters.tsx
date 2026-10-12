import { Search, Filter } from "lucide-react";
import { translatePositionLabel } from "../squad/SquadTab.helpers";
import { SPECIFIC_POSITIONS_BY_GROUP } from "./TransfersTab.model";
import type { TransfersTabController } from "./useTransfersTabController";
type PositionGroupFilterProps = {
  state: Pick<
    TransfersTabController,
    | "specificPositions"
    | "t"
    | "handleSelectPositionGroup"
    | "openPositionPopover"
    | "handleToggleSpecificPosition"
  >;
  pos: string;
};
type TransfersMarketFiltersProps = {
  state: Pick<
    TransfersTabController,
    | "t"
    | "search"
    | "setSearch"
    | "setMarketPage"
    | "positionFilterRef"
    | "handleSelectPositionGroup"
    | "specificPositions"
    | "positions"
    | "openPositionPopover"
    | "handleToggleSpecificPosition"
    | "isPlayersView"
    | "availabilityFilters"
    | "setAvailabilityFilter"
    | "availabilityFilter"
    | "myTeam"
    | "setAffordableOnly"
    | "affordableOnly"
    | "filteredList"
  >;
};

export function PositionGroupFilter({ state, pos }: PositionGroupFilterProps) {
  const {
    specificPositions,
    t,
    handleSelectPositionGroup,
    openPositionPopover,
    handleToggleSpecificPosition,
  } = state;

  const groupSpecifics = SPECIFIC_POSITIONS_BY_GROUP[pos] ?? [];
  const refinable = groupSpecifics.length > 1;
  const selectedInGroup = specificPositions.filter((entry) =>
    groupSpecifics.includes(entry),
  ).length;
  const isActive = selectedInGroup > 0;
  const isPartial = isActive && refinable && selectedInGroup < groupSpecifics.length;
  const groupLabel = t(`common.positionGroups.${pos}`, {
    defaultValue: t(`common.positions.${pos}`, { defaultValue: pos }),
  });

  return (
    <div key={pos} className="relative">
      <button
        type="button"
        onClick={() => handleSelectPositionGroup(pos)}
        aria-haspopup={refinable ? "true" : undefined}
        aria-expanded={refinable ? openPositionPopover === pos : undefined}
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
        className={`focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all inline-flex items-center gap-1 ${isActive ? "bg-primary-700 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
      >
        {t(`common.posAbbr.${pos}`)}
        {isPartial && (
          <span
            aria-hidden="true"
            className="bg-white/20 text-xs px-1.5 py-0.5 rounded-full leading-none"
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
              const positionLabel = translatePositionLabel(t, position);
              return (
                <button
                  type="button"
                  key={position}
                  onClick={() => handleToggleSpecificPosition(position)}
                  aria-pressed={selected}
                  aria-label={positionLabel}
                  title={positionLabel}
                  className={`focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none px-2.5 py-1 rounded-md text-xs font-heading font-bold uppercase tracking-wider transition-all ${selected ? "bg-primary-700 text-white shadow-sm" : "bg-gray-50 dark:bg-navy-700 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600 hover:text-gray-700 dark:hover:text-gray-200"}`}
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
}

export function TransfersMarketFilters({ state }: TransfersMarketFiltersProps) {
  const {
    t,
    search,
    setSearch,
    setMarketPage,
    positionFilterRef,
    handleSelectPositionGroup,
    specificPositions,
    positions,
    isPlayersView,
    availabilityFilters,
    setAvailabilityFilter,
    availabilityFilter,
    myTeam,
    setAffordableOnly,
    affordableOnly,
    filteredList,
  } = state;
  return (
    <div className="flex flex-wrap gap-3 mb-4 items-center">
      <div className="relative flex-1 min-w-[180px] max-w-xs">
        <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-500 dark:text-gray-300" />
        <input
          type="text"
          placeholder={t("transfers.searchByName")}
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            setMarketPage(1);
          }}
          className="w-full pl-9 pr-3 py-2 rounded-lg bg-white dark:bg-navy-800 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-primary-500/50"
        />
      </div>
      <div ref={positionFilterRef} className="flex gap-1.5">
        <button
          type="button"
          onClick={() => handleSelectPositionGroup(null)}
          aria-pressed={specificPositions.length === 0}
          aria-label={t("transfers.allPositions")}
          className={`focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${specificPositions.length === 0 ? "bg-primary-700 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
        >
          {t("common.all")}
        </button>
        {positions.map((pos) => (
          <PositionGroupFilter key={pos} state={state} pos={pos} />
        ))}
      </div>
      {isPlayersView && (
        <div className="flex flex-wrap gap-1.5">
          {availabilityFilters.map((filter) => (
            <button
              type="button"
              key={filter.id}
              onClick={() => {
                setAvailabilityFilter(filter.id);
                setMarketPage(1);
              }}
              className={`focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${availabilityFilter === filter.id ? "bg-accent-500 text-navy-900 shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
            >
              {filter.label} ({filter.count})
            </button>
          ))}
          {myTeam && (
            <button
              type="button"
              onClick={() => {
                setAffordableOnly((prev) => !prev);
                setMarketPage(1);
              }}
              aria-pressed={affordableOnly}
              title={t("transfers.affordableOnlyHint")}
              className={`focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${affordableOnly ? "bg-primary-700 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
            >
              {t("transfers.affordableOnly")}
            </button>
          )}
        </div>
      )}
      <p className="text-xs text-gray-500 dark:text-gray-300 font-heading uppercase tracking-wider">
        <Filter className="w-3.5 h-3.5 inline mr-1 -mt-0.5" />
        {t("common.nResults", { count: filteredList.length })}
      </p>
    </div>
  );
}
