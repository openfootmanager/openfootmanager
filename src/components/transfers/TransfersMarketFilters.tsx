import { Filter } from "lucide-react";
import PlayerListFilters from "../players/PlayerListFilters";
import type { TransfersTabController } from "./useTransfersTabController";

type TransfersMarketFiltersProps = {
  state: Pick<
    TransfersTabController,
    "t" | "isPlayersView" | "marketQuery" | "patchMarketQuery" | "marketTotal" | "myTeam"
  >;
};

/**
 * The market's filters are the shared player-list ones. The other two views (the user's own list,
 * and offers) are short lists of the club's own business and are not filtered.
 */
export function TransfersMarketFilters({ state }: TransfersMarketFiltersProps) {
  const { t, isPlayersView, marketQuery, patchMarketQuery, marketTotal, myTeam } = state;
  if (!isPlayersView) return null;
  return (
    <div>
      <PlayerListFilters
        query={marketQuery}
        onChange={patchMarketQuery}
        statusOptions={["available", "transfer", "loan", "free_agent"]}
        showInBudget={myTeam !== null}
      />
      <p className="text-xs text-gray-500 dark:text-gray-300 font-heading uppercase tracking-wider mb-4">
        <Filter className="w-3.5 h-3.5 inline mr-1 -mt-0.5" />
        {t("common.nResults", { count: marketTotal })}
      </p>
    </div>
  );
}
