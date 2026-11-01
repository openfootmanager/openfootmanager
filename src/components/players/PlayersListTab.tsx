import { Filter } from "lucide-react";
import { useTranslation } from "react-i18next";

import { useListView } from "../../hooks/useListView";
import { usePlayersPage } from "../../hooks/usePlayersPage";
import type { PlayerSortKey } from "../../services/playersService";
import type { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import { Select, ViewSwitch } from "../ui";
import PlayerListFilters from "./PlayerListFilters";
import PlayersCardsView from "./PlayersCardsView";
import PlayersDealModals from "./PlayersDealModals";
import PlayersTableView from "./PlayersTableView";
import { PLAYERS_OPENING_QUERY } from "./playersOpeningQuery";
import { usePlayersListActions } from "./usePlayersListActions";

interface PlayersListTabProps {
  gameState: GameStateData;
  onGameUpdate?: (game: GameStateData) => void;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onSelectTeam: (id: string) => void;
}

/**
 * The Players list: its filters and one page of players, as a table or as cards (the global
 * `list_view`). Both views read the same query and offer the same row actions.
 */
export default function PlayersListTab({
  gameState,
  onGameUpdate,
  onSelectPlayer,
  onSelectTeam,
}: PlayersListTabProps) {
  const { t } = useTranslation();
  const [view, setView] = useListView();
  const {
    query,
    patchQuery,
    slice,
    error: fetchError,
    refetch: refetchSlice,
  } = usePlayersPage(PLAYERS_OPENING_QUERY, t);
  const { contextItemsFor, scoutError, listingError, bidFlow, contractFlow } =
    usePlayersListActions({
      gameState,
      onGameUpdate,
      onSelectPlayer,
      onSelectTeam,
      refetchSlice,
    });

  const handleSort = (key: PlayerSortKey) => {
    if (query.sort_key === key) {
      patchQuery({ sort_asc: !query.sort_asc });
    } else {
      patchQuery({ sort_key: key, sort_asc: key === "name" });
    }
  };

  const items = slice?.items ?? [];
  const total = slice?.total ?? 0;
  const pageSize = slice?.page_size ?? query.page_size;
  const listView = {
    items,
    query,
    contextItemsFor,
    onSelectPlayer,
    onSelectTeam,
    total,
    fetchError,
    page: query.page,
    pageSize,
    totalPages: Math.max(1, Math.ceil(total / pageSize)),
    onPage: (next: number) => patchQuery({ page: next }),
  };

  return (
    <div>
      <PlayerListFilters
        query={query}
        onChange={patchQuery}
        statusOptions={["all", "transfer", "loan"]}
        trailing={
          <div className="flex items-center gap-3">
            <Select
              value={query.team_id ?? ""}
              onChange={(e) => patchQuery({ team_id: e.target.value || null })}
              selectSize="sm"
              className="min-w-44 font-heading font-bold uppercase tracking-wider"
            >
              <option value="">{t("players.allTeams")}</option>
              {gameState.teams.map((tm) => (
                <option key={tm.id} value={tm.id}>
                  {tm.name}
                </option>
              ))}
            </Select>
            <ViewSwitch value={view} onChange={setView} />
          </div>
        }
      />

      <p className="text-xs text-gray-400 dark:text-gray-500 mb-3 font-heading uppercase tracking-wider">
        <Filter className="w-3.5 h-3.5 inline mr-1 -mt-0.5" />
        {t("players.nPlayersFound", { count: total })}
      </p>

      {fetchError ? (
        <p
          role="alert"
          className="mb-3 text-xs font-heading font-bold uppercase tracking-wider text-red-500"
        >
          {fetchError}
        </p>
      ) : null}

      {scoutError ? (
        <p
          role="alert"
          className="mb-3 text-xs font-heading font-bold uppercase tracking-wider text-red-500"
        >
          {scoutError}
        </p>
      ) : null}

      {listingError ? <ListingAlert message={listingError} /> : null}

      {view === "cards" ? (
        <PlayersCardsView
          {...listView}
          teams={gameState.teams}
          onSortChange={(key, ascending) => patchQuery({ sort_key: key, sort_asc: ascending })}
        />
      ) : (
        <PlayersTableView {...listView} onSort={handleSort} />
      )}
      <PlayersDealModals teams={gameState.teams} bidFlow={bidFlow} contractFlow={contractFlow} />
    </div>
  );
}

/** A listing that was not saved, as a sentence on the danger tint (read as a warning in both themes). */
function ListingAlert({ message }: { message: string }) {
  return (
    <p
      role="alert"
      className="mb-3 rounded-lg border border-line bg-danger-tint px-4 py-2 text-body font-medium text-danger"
    >
      {message}
    </p>
  );
}
