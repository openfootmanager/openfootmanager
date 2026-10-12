import { Card, CardBody } from "../ui";
import { ChevronLeft, ChevronRight, ChevronUp, ChevronDown } from "lucide-react";
import type { TransfersTabController } from "./useTransfersTabController";
import { TransferPlayerRow } from "./TransfersPlayerRow";
type TransfersMarketPaginationProps = {
  state: Pick<
    TransfersTabController,
    | "showMarketPagination"
    | "t"
    | "marketRangeFrom"
    | "marketRangeTo"
    | "filteredList"
    | "setMarketPage"
    | "safeMarketPage"
    | "marketTotalPages"
  >;
};
type TransfersMarketTableProps = {
  state: Pick<
    TransfersTabController,
    | "filteredList"
    | "t"
    | "ovrSortDir"
    | "cycleOvrSort"
    | "view"
    | "isScoutingView"
    | "visibleList"
    | "alreadyScoutingIds"
    | "scoutingPlayerId"
    | "availableScouts"
    | "onSelectPlayer"
    | "onSelectTeam"
    | "handleToggleTransferListing"
    | "handleToggleLoanListing"
    | "handleScoutPlayer"
    | "getDealEntryLabel"
    | "getDealEntryIcon"
    | "openDealEntry"
    | "openDealHistory"
    | "i18n"
    | "gameState"
    | "weeklySuffix"
    | "userTeamId"
    | "handleRespondOffer"
    | "openCounterNegotiation"
    | "handleRespondLoanOffer"
    | "openLoanCounterOffer"
    | "handleExerciseLoanBuyOption"
    | "showMarketPagination"
    | "marketRangeFrom"
    | "marketRangeTo"
    | "setMarketPage"
    | "safeMarketPage"
    | "marketTotalPages"
  >;
};

export function TransfersMarketPagination({ state }: TransfersMarketPaginationProps) {
  const {
    showMarketPagination,
    t,
    marketRangeFrom,
    marketRangeTo,
    filteredList,
    setMarketPage,
    safeMarketPage,
    marketTotalPages,
  } = state;
  return showMarketPagination ? (
    <div className="flex items-center justify-between border-t border-gray-100 px-4 py-3 dark:border-navy-600">
      <p className="text-xs font-heading text-gray-500 dark:text-gray-300">
        {t("players.showingRange", {
          from: marketRangeFrom,
          to: marketRangeTo,
          total: filteredList.length,
        })}
      </p>
      <div className="flex items-center gap-1">
        <button
          type="button"
          onClick={() => setMarketPage(Math.max(1, safeMarketPage - 1))}
          disabled={safeMarketPage === 1}
          aria-label={t("scouting.previousPage")}
          className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none rounded-lg p-1.5 text-gray-500 dark:text-gray-300 transition-colors hover:bg-gray-100 hover:text-gray-700 disabled:pointer-events-none disabled:opacity-30 dark:hover:bg-navy-700 dark:hover:text-white"
        >
          <ChevronLeft className="h-4 w-4" />
        </button>
        <span className="px-3 py-1 text-xs font-heading font-bold text-gray-600 dark:text-gray-300">
          {safeMarketPage} / {marketTotalPages}
        </span>
        <button
          type="button"
          onClick={() => setMarketPage(Math.min(marketTotalPages, safeMarketPage + 1))}
          disabled={safeMarketPage === marketTotalPages}
          aria-label={t("scouting.nextPage")}
          className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none rounded-lg p-1.5 text-gray-500 dark:text-gray-300 transition-colors hover:bg-gray-100 hover:text-gray-700 disabled:pointer-events-none disabled:opacity-30 dark:hover:bg-navy-700 dark:hover:text-white"
        >
          <ChevronRight className="h-4 w-4" />
        </button>
      </div>
    </div>
  ) : null;
}

export function TransfersMarketTable({ state }: TransfersMarketTableProps) {
  const { filteredList, t, ovrSortDir, cycleOvrSort, view, isScoutingView, visibleList } = state;
  return (
    filteredList.length > 0 && (
      <Card>
        <CardBody className="p-0">
          <div className="overflow-x-auto">
            <table className="w-full text-left border-collapse">
              <thead>
                <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.position")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.player")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.age")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.team")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.value")}
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.wage")}
                  </th>
                  <th
                    aria-sort={
                      ovrSortDir === "desc"
                        ? "descending"
                        : ovrSortDir === "asc"
                          ? "ascending"
                          : "none"
                    }
                    tabIndex={0}
                    className={`py-3 px-4 font-heading font-bold uppercase tracking-wider cursor-pointer select-none hover:text-primary-400 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-400 transition-colors ${ovrSortDir !== "none" ? "text-primary-700 dark:text-primary-400" : "text-gray-500 dark:text-gray-400"}`}
                    onClick={cycleOvrSort}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        cycleOvrSort();
                      }
                    }}
                  >
                    <div className="flex items-center gap-1">
                      {t("common.ovr")}
                      {ovrSortDir === "desc" ? (
                        <ChevronDown className="w-3 h-3" />
                      ) : ovrSortDir === "asc" ? (
                        <ChevronUp className="w-3 h-3" />
                      ) : null}
                    </div>
                  </th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.status")}
                  </th>
                  {view === "offers" && (
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                      {t("transfers.offers")}
                    </th>
                  )}
                  {(isScoutingView || view === "offers") && (
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                      {isScoutingView ? t("scouting.action") : t("common.actions")}
                    </th>
                  )}
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                {visibleList.map((player) => (
                  <TransferPlayerRow key={player.id} state={state} player={player} />
                ))}
              </tbody>
            </table>
          </div>
          {<TransfersMarketPagination state={state} />}
        </CardBody>
      </Card>
    )
  );
}
