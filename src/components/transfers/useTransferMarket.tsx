import { getTransferNavigation } from "./TransfersTab.navigation";
import { useEffect, useMemo, useRef, useState } from "react";
import type { GameStateData } from "../../store/gameStore";
import { getPlayerOvr } from "../../lib/helpers";
import {
  deriveTransferCollections,
  filterTransferPlayers,
  getCurrentTransferList,
  getMyListedPlayers,
  SPECIFIC_POSITIONS_BY_GROUP,
  type TransferAvailabilityFilter,
  type TransferTabView,
} from "./TransfersTab.model";
import type { TeamData } from "../../store/gameStore";
import type { TFunction } from "i18next";
const TRANSFER_MARKET_PAGE_SIZE = 30;
interface Input {
  gameState: GameStateData;
  userTeamId: string | null;
  t: TFunction<"translation", undefined>;
  myTeam: TeamData | null;
}
export function useTransferMarket({ gameState, userTeamId, t, myTeam }: Input) {
  const [view, setView] = useState<TransferTabView>("players");
  const [availabilityFilter, setAvailabilityFilter] = useState<TransferAvailabilityFilter>("all");
  const [search, setSearch] = useState("");
  const [specificPositions, setSpecificPositions] = useState<string[]>([]);
  const [openPositionPopover, setOpenPositionPopover] = useState<string | null>(null);
  const positionFilterRef = useRef<HTMLDivElement | null>(null);
  const [affordableOnly, setAffordableOnly] = useState(false);
  // Click the OVR header to cycle: none → desc → asc → none. Applied over the
  // filter output before pagination so sorted rows stay stable across pages.
  const [ovrSortDir, setOvrSortDir] = useState<"none" | "desc" | "asc">("none");
  const [marketPage, setMarketPage] = useState(1);
  const cycleOvrSort = () => {
    setOvrSortDir((current) => (current === "none" ? "desc" : current === "desc" ? "asc" : "none"));
    setMarketPage(1);
  };
  useEffect(() => {
    if (!openPositionPopover) return;

    const handleClickOutside = (event: MouseEvent) => {
      if (!positionFilterRef.current) return;
      if (positionFilterRef.current.contains(event.target as Node)) return;
      setOpenPositionPopover(null);
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [openPositionPopover]);
  const handleSelectPositionGroup = (group: string | null) => {
    setMarketPage(1);

    if (group === null) {
      setSpecificPositions([]);
      setOpenPositionPopover(null);
      return;
    }

    const groupSpecifics = SPECIFIC_POSITIONS_BY_GROUP[group] ?? [];

    // No popover for single-position groups (just GK). Treat as a toggle on
    // its lone specific so the chip can also be used to deactivate.
    if (groupSpecifics.length <= 1) {
      const only = groupSpecifics[0];
      if (only) {
        setSpecificPositions((prev) =>
          prev.includes(only) ? prev.filter((entry) => entry !== only) : [...prev, only],
        );
      }
      setOpenPositionPopover(null);
      return;
    }

    // Re-clicking the chip whose popover is open just closes the popover —
    // the user is done refining.
    if (openPositionPopover === group) {
      setOpenPositionPopover(null);
      return;
    }

    // Otherwise: union this group's specifics into the existing selection and
    // open the refinement popover. This makes the category a "select all"
    // shortcut without resetting earlier picks from other groups.
    setSpecificPositions((prev) => {
      const set = new Set(prev);
      for (const position of groupSpecifics) set.add(position);
      return Array.from(set);
    });
    setOpenPositionPopover(group);
  };
  const handleToggleSpecificPosition = (position: string) => {
    setMarketPage(1);
    setSpecificPositions((prev) =>
      prev.includes(position) ? prev.filter((entry) => entry !== position) : [...prev, position],
    );
  };
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
  const { positions, tabs, availabilityFilters } = getTransferNavigation({
    collections: transferCollections,
    myListedPlayers,
    t,
  });

  const currentList = useMemo(
    () => getCurrentTransferList(view, transferCollections),
    [transferCollections, view],
  );
  const filteredList = useMemo(() => {
    const filtered = filterTransferPlayers(
      currentList,
      search,
      null,
      isPlayersView ? availabilityFilter : "all",
      isPlayersView && affordableOnly && myTeam
        ? {
            transferBudget: myTeam.transfer_budget,
            finance: myTeam.finance,
          }
        : null,
      specificPositions,
    );

    if (ovrSortDir === "none") {
      return filtered;
    }

    // Direction encoded in the comparator (rather than sort().reverse()) so
    // tied players keep the same relative order in both asc and desc.
    const factor = ovrSortDir === "desc" ? -1 : 1;
    return [...filtered].sort((left, right) => factor * (getPlayerOvr(left) - getPlayerOvr(right)));
  }, [
    affordableOnly,
    availabilityFilter,
    currentList,
    isPlayersView,
    myTeam,
    ovrSortDir,
    search,
    specificPositions,
  ]);
  const marketTotalPages = Math.max(1, Math.ceil(filteredList.length / TRANSFER_MARKET_PAGE_SIZE));
  const safeMarketPage = Math.min(marketPage, marketTotalPages);
  const marketPageStart = (safeMarketPage - 1) * TRANSFER_MARKET_PAGE_SIZE;
  const visibleList = isPlayersView
    ? filteredList.slice(marketPageStart, marketPageStart + TRANSFER_MARKET_PAGE_SIZE)
    : filteredList;
  const showMarketPagination = isPlayersView && filteredList.length > TRANSFER_MARKET_PAGE_SIZE;
  const marketRangeFrom = filteredList.length === 0 ? 0 : marketPageStart + 1;
  const marketRangeTo = Math.min(marketPageStart + TRANSFER_MARKET_PAGE_SIZE, filteredList.length);

  const annualWageBudget = myTeam?.wage_budget ?? 0;
  return {
    view,
    setView,
    availabilityFilter,
    setAvailabilityFilter,
    search,
    setSearch,
    specificPositions,
    openPositionPopover,
    positionFilterRef,
    affordableOnly,
    setAffordableOnly,
    ovrSortDir,
    setMarketPage,
    cycleOvrSort,
    handleSelectPositionGroup,
    handleToggleSpecificPosition,
    myListedPlayers,
    isPlayersView,
    isScoutingView,
    positions,
    tabs,
    filteredList,
    marketTotalPages,
    safeMarketPage,
    visibleList,
    showMarketPagination,
    marketRangeFrom,
    marketRangeTo,
    availabilityFilters,
    annualWageBudget,
  };
}
