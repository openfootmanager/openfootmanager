import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  gameKnowledgeRevision,
  NOTHING_VISIBLE,
  type VisibleFigures,
} from "../../lib/playerKnowledge";
import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import type { PlayersPageQuery } from "../../services/playersService";
import { usePlayersPage } from "../../hooks/usePlayersPage";
import {
  deriveTransferCollections,
  getMyListedPlayers,
  type TransferTabView,
} from "./TransfersTab.model";
import { getTransferNavigation } from "./TransfersTab.navigation";
import type { TFunction } from "i18next";

const TRANSFER_MARKET_PAGE_SIZE = 30;

// The market is what the user's club could still approach: listed for transfer or loan, or without
// a club. The backend applies the rules; this is only the question asked of it.
const MARKET_QUERY: PlayersPageQuery = {
  search: null,
  position: null,
  positions: [],
  age_min: null,
  age_max: null,
  ovr_min: null,
  ovr_max: null,
  value_min: null,
  value_max: null,
  in_budget: false,
  scope: "market",
  team_id: null,
  status: "available",
  sort_key: "ovr",
  sort_asc: false,
  page: 1,
  page_size: TRANSFER_MARKET_PAGE_SIZE,
};

// Each view of the centre is its own list with its own filters, asked of the same backend query.
const VIEW_LISTS: Record<TransferTabView, Pick<PlayersPageQuery, "scope" | "status">> = {
  players: { scope: "market", status: "available" },
  my_list: { scope: "user_listed", status: "all" },
  offers: { scope: "with_offers", status: "all" },
};

interface Input {
  gameState: GameStateData;
  userTeamId: string | null;
  t: TFunction<"translation", undefined>;
  myTeam: TeamData | null;
}

export function useTransferMarket({ gameState, userTeamId, t, myTeam }: Input) {
  const [view, setView] = useState<TransferTabView>("players");
  // What the club knows moves with the career, the manager's club, the date and the season: an
  // answer asked under another of these never shows its figures (see `visibleOf`).
  const revision = gameKnowledgeRevision(gameState);
  const {
    query: marketQuery,
    patchQuery: patchMarketQuery,
    slice: answeredSlice,
    answeredQuery,
    answeredRevision,
    error: marketError,
    refetch: refetchMarket,
  } = usePlayersPage(MARKET_QUERY, t, revision);
  const marketSlice = answeredQuery?.scope === marketQuery.scope ? answeredSlice : null;

  // Coming back to a view finds its filters as they were left.
  const leftQueries = useRef<Partial<Record<TransferTabView, PlayersPageQuery>>>({});
  const changeView = (next: TransferTabView) => {
    if (next === view) return;
    leftQueries.current[view] = marketQuery;
    setView(next);
    patchMarketQuery(leftQueries.current[next] ?? { ...MARKET_QUERY, ...VIEW_LISTS[next] });
  };

  // A deal, a listing or a day passing changes who is on the market: ask again whenever the game
  // the screen is showing is a new one (not on first render, which has just asked).
  const shownGame = useRef(gameState);
  useEffect(() => {
    if (shownGame.current === gameState) return;
    shownGame.current = gameState;
    refetchMarket();
  }, [gameState, refetchMarket]);

  const transferCollections = useMemo(
    () => deriveTransferCollections(gameState, userTeamId),
    [gameState, userTeamId],
  );
  const myListedPlayers = useMemo(
    () => getMyListedPlayers(transferCollections),
    [transferCollections],
  );
  const isPlayersView = view === "players";
  const isScoutingView = isPlayersView;
  const { tabs } = getTransferNavigation({
    collections: transferCollections,
    myListedPlayers,
    t,
  });

  // Rows show offers, contracts and listings, which live on the full player; the page only says
  // who is on it and in what order.
  const playersById = useMemo(
    () => new Map(gameState.players.map((player) => [player.id, player])),
    [gameState.players],
  );
  const listedPlayers = useMemo(
    () =>
      (marketSlice?.items ?? [])
        .map((summary) => playersById.get(summary.id))
        .filter((player): player is PlayerData => player !== undefined),
    [marketSlice, playersById],
  );
  const filteredList = listedPlayers;
  // The rating and wage the club may see of each listed player, as the page's projection sends
  // them. A player the page did not list shows nothing private, and neither does any player while
  // the page answers another revision: after the manager changes club, the old club's answer stays
  // on screen until the new one lands (or for good, if it fails), and must not show what the old
  // club knew. The rows themselves stay, so the table does not empty during the refetch.
  const figuresAreCurrent = answeredRevision === revision;
  const visibilityById = useMemo(
    () =>
      new Map<string, VisibleFigures>(
        figuresAreCurrent
          ? (marketSlice?.items ?? []).map((summary) => [
              summary.id,
              { ovr: summary.ovr, wage: summary.wage },
            ])
          : [],
      ),
    [marketSlice, figuresAreCurrent],
  );
  const visibleOf = useCallback(
    (playerId: string): VisibleFigures => visibilityById.get(playerId) ?? NOTHING_VISIBLE,
    [visibilityById],
  );

  const marketTotal = marketSlice?.total ?? 0;
  const marketTotalPages = Math.max(1, Math.ceil(marketTotal / TRANSFER_MARKET_PAGE_SIZE));
  const safeMarketPage = Math.min(marketQuery.page, marketTotalPages);
  const marketPageStart = (safeMarketPage - 1) * TRANSFER_MARKET_PAGE_SIZE;
  const showMarketPagination = marketTotal > TRANSFER_MARKET_PAGE_SIZE;
  const marketRangeFrom = marketTotal === 0 ? 0 : marketPageStart + 1;
  const marketRangeTo = Math.min(marketPageStart + TRANSFER_MARKET_PAGE_SIZE, marketTotal);
  const setMarketPage = (page: number) => patchMarketQuery({ page });

  // Pressing the OVR header turns the order round; the market starts best first.
  const ovrSortDir: "desc" | "asc" | "none" =
    marketQuery.sort_key !== "ovr" ? "none" : marketQuery.sort_asc ? "asc" : "desc";
  const cycleOvrSort = () => patchMarketQuery({ sort_key: "ovr", sort_asc: ovrSortDir === "desc" });

  // Whether the user has narrowed the list, so an empty one can say "nothing matches" rather than
  // "there is nothing".
  const marketFiltered =
    marketQuery.search !== null ||
    marketQuery.positions.length > 0 ||
    marketQuery.in_budget ||
    marketQuery.status !== VIEW_LISTS[view].status ||
    [marketQuery.age_min, marketQuery.age_max, marketQuery.ovr_min, marketQuery.ovr_max].some(
      (bound) => bound !== null,
    );

  const weeklyWageBudget = myTeam?.wage_budget ?? 0;
  return {
    view,
    setView: changeView,
    marketQuery,
    patchMarketQuery,
    marketError,
    marketTotal,
    marketFiltered,
    marketLoaded: marketSlice !== null,
    marketStatusCounts: marketSlice?.status_counts,
    ovrSortDir,
    setMarketPage,
    cycleOvrSort,
    myListedPlayers,
    isPlayersView,
    isScoutingView,
    tabs,
    filteredList,
    marketTotalPages,
    safeMarketPage,
    visibleList: filteredList,
    visibleOf,
    showMarketPagination,
    marketRangeFrom,
    marketRangeTo,
    weeklyWageBudget,
  };
}
