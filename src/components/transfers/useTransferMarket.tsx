import { useEffect, useMemo, useRef, useState } from "react";
import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import type { PlayersPageQuery } from "../../services/playersService";
import { usePlayersPage } from "../../hooks/usePlayersPage";
import {
  deriveTransferCollections,
  getCurrentTransferList,
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

interface Input {
  gameState: GameStateData;
  userTeamId: string | null;
  t: TFunction<"translation", undefined>;
  myTeam: TeamData | null;
}

export function useTransferMarket({ gameState, userTeamId, t, myTeam }: Input) {
  const [view, setView] = useState<TransferTabView>("players");
  const {
    query: marketQuery,
    patchQuery: patchMarketQuery,
    slice: marketSlice,
    error: marketError,
    refetch: refetchMarket,
  } = usePlayersPage(MARKET_QUERY, t);

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
  const marketPlayers = useMemo(
    () =>
      (marketSlice?.items ?? [])
        .map((summary) => playersById.get(summary.id))
        .filter((player): player is PlayerData => player !== undefined),
    [marketSlice, playersById],
  );
  const filteredList = isPlayersView
    ? marketPlayers
    : getCurrentTransferList(view, transferCollections);

  const marketTotal = marketSlice?.total ?? 0;
  const marketTotalPages = Math.max(1, Math.ceil(marketTotal / TRANSFER_MARKET_PAGE_SIZE));
  const safeMarketPage = Math.min(marketQuery.page, marketTotalPages);
  const marketPageStart = (safeMarketPage - 1) * TRANSFER_MARKET_PAGE_SIZE;
  const showMarketPagination = isPlayersView && marketTotal > TRANSFER_MARKET_PAGE_SIZE;
  const marketRangeFrom = marketTotal === 0 ? 0 : marketPageStart + 1;
  const marketRangeTo = Math.min(marketPageStart + TRANSFER_MARKET_PAGE_SIZE, marketTotal);
  const setMarketPage = (page: number) => patchMarketQuery({ page });

  // Pressing the OVR header turns the order round; the market starts best first.
  const ovrSortDir: "desc" | "asc" | "none" =
    marketQuery.sort_key !== "ovr" ? "none" : marketQuery.sort_asc ? "asc" : "desc";
  const cycleOvrSort = () => patchMarketQuery({ sort_key: "ovr", sort_asc: ovrSortDir === "desc" });

  const weeklyWageBudget = myTeam?.wage_budget ?? 0;
  return {
    view,
    setView,
    marketQuery,
    patchMarketQuery,
    marketError,
    marketTotal,
    marketLoaded: marketSlice !== null,
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
    showMarketPagination,
    marketRangeFrom,
    marketRangeTo,
    weeklyWageBudget,
  };
}
