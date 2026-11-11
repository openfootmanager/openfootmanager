import { Filter } from "lucide-react";
import { useListView } from "../../hooks/useListView";
import { ViewSwitch } from "../ui";
import PlayerListFilters from "../players/PlayerListFilters";
import type { PlayerStatusFilter } from "../../services/playersService";
import type { TransferTabView } from "./TransfersTab.model";
import type { TransfersTabController } from "./useTransfersTabController";

type TransfersMarketFiltersProps = {
  state: Pick<
    TransfersTabController,
    | "t"
    | "view"
    | "marketQuery"
    | "patchMarketQuery"
    | "marketTotal"
    | "marketStatusCounts"
    | "myTeam"
  >;
};

const STATUS_OPTIONS: Record<TransferTabView, PlayerStatusFilter[]> = {
  players: ["available", "transfer", "loan", "free_agent"],
  // The club's own players all have a club.
  my_list: ["all", "transfer", "loan"],
  offers: ["all", "transfer", "loan", "free_agent"],
};

/** Every view of the centre filters its list with the shared player-list controls. */
export function TransfersMarketFilters({ state }: TransfersMarketFiltersProps) {
  const [listView, setListView] = useListView();
  const { t, view, marketQuery, patchMarketQuery, marketTotal, marketStatusCounts, myTeam } = state;
  return (
    <div>
      <PlayerListFilters
        query={marketQuery}
        onChange={patchMarketQuery}
        statusOptions={STATUS_OPTIONS[view]}
        statusCounts={marketStatusCounts}
        showInBudget={view === "players" && myTeam !== null}
        trailing={<ViewSwitch value={listView} onChange={setListView} />}
      />
      <p className="text-xs text-gray-500 dark:text-gray-300 font-heading uppercase tracking-wider mb-4">
        <Filter className="w-3.5 h-3.5 inline mr-1 -mt-0.5" />
        {t("common.nResults", { count: marketTotal })}
      </p>
    </div>
  );
}
