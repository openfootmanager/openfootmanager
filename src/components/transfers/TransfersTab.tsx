import { Card, CardBody } from "../ui";
import { TrendingUp, ShoppingCart, Handshake } from "lucide-react";
import type { TransfersTabProps } from "./TransfersTab.types";
import { useTransfersTabController } from "./useTransfersTabController";
import { TransfersMarketFilters } from "./TransfersMarketFilters";
import { TransfersBudgetHeader, TransfersTabNavigation } from "./TransfersTabHeader";
import { TransfersMarketTable } from "./TransfersMarketTable";
import {
  ActiveDealWorkspace,
  TransferBidDialog,
  TransferCounterDialog,
  FreeAgentDealDialog,
  LoanOfferDialog,
  LoanCounterDialog,
} from "./TransfersDealDialogs";
export default function TransfersTab(props: TransfersTabProps) {
  const state = useTransfersTabController(props);
  const {
    t,
    view,
    marketQuery,
    marketError,
    marketLoaded,
    marketFiltered,
    filteredList,
    scoutError,
    isScoutingView,
    listingError,
    responseError,
  } = state;
  return (
    <div>
      {/* Budget header */}
      {<TransfersBudgetHeader state={state} />}

      {/* Tab navigation */}
      <TransfersTabNavigation state={state} />

      {/* Filters */}
      <TransfersMarketFilters state={state} />

      {marketError && isScoutingView ? (
        <p
          role="alert"
          className="mb-4 text-xs font-heading font-bold uppercase tracking-wider text-red-500"
        >
          {marketError}
        </p>
      ) : null}
      {scoutError && isScoutingView ? (
        <p
          role="alert"
          className="mb-4 text-xs font-heading font-bold uppercase tracking-wider text-red-500"
        >
          {scoutError}
        </p>
      ) : null}
      {listingError && view === "my_list" ? (
        <p
          role="alert"
          className="mb-4 text-xs font-heading font-bold uppercase tracking-wider text-red-500"
        >
          {listingError}
        </p>
      ) : null}

      {responseError && view === "offers" ? (
        <p
          role="alert"
          className="mb-4 text-xs font-heading font-bold uppercase tracking-wider text-red-500"
        >
          {responseError}
        </p>
      ) : null}

      {/* Content */}
      {view === "my_list" && marketLoaded && filteredList.length === 0 && (
        <Card>
          <CardBody>
            <div className="text-center py-8">
              <ShoppingCart className="w-10 h-10 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {marketFiltered ? t("players.noMatch") : t("transfers.noPlayersListed")}
              </p>
              {!marketFiltered && (
                <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
                  {t("transfers.goToProfile")}
                </p>
              )}
            </div>
          </CardBody>
        </Card>
      )}

      {view === "offers" && marketLoaded && filteredList.length === 0 && (
        <Card>
          <CardBody>
            <div className="text-center py-8">
              <Handshake className="w-10 h-10 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {marketFiltered ? t("players.noMatch") : t("transfers.noOffers")}
              </p>
            </div>
          </CardBody>
        </Card>
      )}

      {<TransfersMarketTable state={state} />}

      {isScoutingView && marketLoaded && filteredList.length === 0 && (
        <Card>
          <CardBody>
            <div className="text-center py-8">
              <TrendingUp className="w-10 h-10 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {marketQuery.status === "transfer"
                  ? t("transfers.noTransferMarket")
                  : marketQuery.status === "free_agent"
                    ? t("transfers.noFreeAgents")
                    : marketQuery.status === "loan"
                      ? t("transfers.noLoanMarket")
                      : t("transfers.noAvailablePlayers")}
              </p>
            </div>
          </CardBody>
        </Card>
      )}
      {<ActiveDealWorkspace state={state} />}
      {/* Bid Modal */}
      {<TransferBidDialog state={state} />}
      {<TransferCounterDialog state={state} />}
      {<FreeAgentDealDialog state={state} />}
      {<LoanOfferDialog state={state} />}
      {<LoanCounterDialog state={state} />}
    </div>
  );
}
