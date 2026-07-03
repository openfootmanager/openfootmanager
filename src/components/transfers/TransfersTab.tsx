import { useEffect, useMemo, useRef, useState } from "react";
import {
  GameStateData,
  PlayerData,
  PlayerSelectionOptions,
} from "../../store/gameStore";
import { Card, CardBody } from "../ui";
import {
  TrendingUp,
  ShoppingCart,
  Handshake,
  ArrowRightLeft,
  Gavel,
  UserPlus,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { resolveSeasonContext } from "../../lib/seasonContext";
import { formatDate } from "../../lib/dateFormatting";
import TransferBidModal, { TransferBidForm } from "./TransferBidModal";
import TransferCounterOfferModal from "./TransferCounterOfferModal";
import LoanOfferModal, { LoanOfferForm } from "./LoanOfferModal";
import PlayerDealWorkspace, { type DealKind } from "./PlayerDealWorkspace";
import {
  getErrorMessage,
  resolveTranslatedErrorMessage,
} from "../../utils/errorMessage";
import {
  exerciseLoanBuyOption,
  respondToOffer,
  respondToLoanOffer,
  toggleLoanList,
  toggleTransferList,
} from "../../services/transfersService";
import { sendScout } from "../../services/scoutingService";
import {
  deriveTransferCollections,
  filterTransferPlayers,
  getCurrentTransferList,
  getMyListedPlayers,
  SPECIFIC_POSITIONS_BY_GROUP,
  type TransferAvailabilityFilter,
  type TransferTabView,
} from "./TransfersTab.model";
import { calculateAvailableScouts } from "../scouting/ScoutingTab.helpers";
import { buildAlreadyScoutingIds } from "../scouting/ScoutingTab.model";
import FreeAgentContractModal, {
  FreeAgentContractForm,
} from "./FreeAgentContractModal";
import TransfersBudgetHeader from "./TransfersBudgetHeader";
import TransfersControls from "./TransfersControls";
import TransferPlayerTable from "./TransferPlayerTable";
import { useFreeAgentContractFlow } from "./useFreeAgentContractFlow";
import { useTransferBidFlow } from "./useTransferBidFlow";
import { useLoanOfferFlow } from "./useLoanOfferFlow";
import { useLoanCounterOfferFlow } from "./useLoanCounterOfferFlow";
import { useTransferCounterOfferFlow } from "./useTransferCounterOfferFlow";

interface TransfersTabProps {
  gameState: GameStateData;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onSelectTeam: (id: string) => void;
  onGameUpdate?: (game: GameStateData) => void;
}

const TRANSFER_MARKET_PAGE_SIZE = 30;


function parseDateOnlyMs(value: string | null | undefined): number | null {
  if (!value) {
    return null;
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return null;
  }

  return Date.UTC(
    parsed.getUTCFullYear(),
    parsed.getUTCMonth(),
    parsed.getUTCDate(),
  );
}

function futureClosedWindowRegistrationDate(
  currentDateValue: string,
  opensOnValue: string | null | undefined,
): string | null {
  const currentDate = parseDateOnlyMs(currentDateValue);
  const opensOn = parseDateOnlyMs(opensOnValue);

  if (currentDate === null || opensOn === null || opensOn <= currentDate) {
    return null;
  }

  return opensOnValue ?? null;
}

export default function TransfersTab({
  gameState,
  onSelectPlayer,
  onSelectTeam,
  onGameUpdate,
}: TransfersTabProps) {
  const { t, i18n } = useTranslation();
  const annualSuffix = t("finances.perYearSuffix", "/yr");
  const userTeamId = gameState.manager.team_id;
  const seasonContext = resolveSeasonContext(gameState);
  const transferWindow = seasonContext.transfer_window;
  const closedWindowRegistrationDate =
    transferWindow.status === "Closed"
      ? futureClosedWindowRegistrationDate(
          gameState.clock.current_date,
          transferWindow.opens_on,
        )
      : null;
  const loanRegistrationDate =
    transferWindow.status === "Closed" && closedWindowRegistrationDate
      ? closedWindowRegistrationDate
      : gameState.clock.current_date;
  const [view, setView] = useState<TransferTabView>("players");
  const [availabilityFilter, setAvailabilityFilter] =
    useState<TransferAvailabilityFilter>("all");
  const [search, setSearch] = useState("");
  const [specificPositions, setSpecificPositions] = useState<string[]>([]);
  const [openPositionPopover, setOpenPositionPopover] = useState<string | null>(
    null,
  );
  const positionFilterRef = useRef<HTMLDivElement | null>(null);
  const [affordableOnly, setAffordableOnly] = useState(false);
  const [marketPage, setMarketPage] = useState(1);
  const [scoutingPlayerId, setScoutingPlayerId] = useState<string | null>(null);
  const [scoutError, setScoutError] = useState<string | null>(null);
  const [listingError, setListingError] = useState<string | null>(null);
  const [dealWorkspaceTarget, setDealWorkspaceTarget] =
    useState<PlayerData | null>(null);
  const [dealWorkspaceKind, setDealWorkspaceKind] =
    useState<DealKind>("transfer");

  const handleRespondOffer = async (
    playerId: string,
    offerId: string,
    accept: boolean,
  ) => {
    try {
      const game = await respondToOffer(playerId, offerId, accept);
      if (onGameUpdate) onGameUpdate(game);
    } catch (err) {
      console.error("Failed to respond to offer:", err);
    }
  };

  const handleRespondLoanOffer = async (
    playerId: string,
    offerId: string,
    accept: boolean,
  ) => {
    try {
      const game = await respondToLoanOffer(playerId, offerId, accept);
      if (onGameUpdate) onGameUpdate(game);
    } catch (err) {
      console.error("Failed to respond to loan offer:", err);
    }
  };

  const handleExerciseLoanBuyOption = async (playerId: string) => {
    try {
      const game = await exerciseLoanBuyOption(playerId);
      if (onGameUpdate) onGameUpdate(game);
    } catch (err) {
      console.error("Failed to exercise loan buy option:", err);
    }
  };

  const {
    bidTarget,
    bidAmount,
    setBidAmount,
    bidResult,
    bidLoading,
    bidFeedback,
    bidProjection,
    bidFee,
    activeBidOffer,
    myTeam,
    hasExistingOffer,
    bidSubmitDisabled,
    openBidNegotiation,
    closeBidNegotiation,
    handleMakeBid,
  } = useTransferBidFlow({
    gameState,
    onGameUpdate,
  });
  const scouts = gameState.staff.filter(
    (staffMember) =>
      staffMember.role === "Scout" && staffMember.team_id === userTeamId,
  );
  const scoutingAssignments = gameState.scouting_assignments || [];
  const allScoutingAssignments = [
    ...scoutingAssignments,
    ...(gameState.youth_scouting_assignments || []),
  ];
  const availableScouts = calculateAvailableScouts(
    scouts,
    allScoutingAssignments,
  );
  const alreadyScoutingIds = buildAlreadyScoutingIds(scoutingAssignments);
  const transferWindowVariant =
    transferWindow.status === "DeadlineDay"
      ? "danger"
      : transferWindow.status === "Open"
        ? "success"
        : "neutral";
  const transferWindowSummary =
    transferWindow.status === "DeadlineDay"
      ? t("season.windowClosesToday")
      : transferWindow.status === "Open" &&
          transferWindow.days_remaining !== null
        ? t("season.windowClosesInDays", {
            count: transferWindow.days_remaining,
          })
        : transferWindow.status === "Closed" &&
            transferWindow.days_until_opens !== null
          ? t("season.windowOpensInDays", {
              count: transferWindow.days_until_opens,
            })
          : t("season.windowClosed");
  const isTransferWindowClosed = transferWindow.status === "Closed";
  const transferWindowBlocksRegistration =
    isTransferWindowClosed && !closedWindowRegistrationDate;
  const transferWindowBlockingTitle = transferWindowBlocksRegistration
    ? t("season.windowClosed")
    : null;
  const transferWindowBlockingDetail =
    transferWindowBlocksRegistration &&
    transferWindowSummary !== transferWindowBlockingTitle
      ? transferWindowSummary
      : null;
  const loanWindowNoticeTitle = isTransferWindowClosed
    ? t("transfers.loanWindowClosedNoticeTitle")
    : null;
  const loanWindowNoticeDetail =
    isTransferWindowClosed && closedWindowRegistrationDate
      ? t("transfers.loanWindowClosedNoticeDetail", {
          date: formatDate(closedWindowRegistrationDate, i18n.language),
        })
      : isTransferWindowClosed
        ? t("transfers.loanWindowClosedUnavailableDetail")
        : null;

  const transferCollections = useMemo(
    () => deriveTransferCollections(gameState, userTeamId),
    [gameState, userTeamId],
  );
  const {
    availablePlayers,
    marketPlayers,
    freeAgentPlayers,
    loanPlayers,
    playersWithOffers,
  } = transferCollections;
  const myListedPlayers = useMemo(
    () => getMyListedPlayers(transferCollections),
    [transferCollections],
  );
  const isPlayersView = view === "players";
  const isScoutingView = isPlayersView;

  const positions = ["Goalkeeper", "Defender", "Midfielder", "Forward"];

  const tabs: {
    id: TransferTabView;
    label: string;
    icon: React.ReactNode;
    count: number;
  }[] = [
    {
      id: "players",
      label: t("dashboard.players"),
      icon: <TrendingUp className="w-4 h-4" />,
      count: availablePlayers.length,
    },
    {
      id: "my_list",
      label: t("transfers.myTransferList"),
      icon: <ShoppingCart className="w-4 h-4" />,
      count: myListedPlayers.length,
    },
    {
      id: "offers",
      label: t("transfers.offers"),
      icon: <Handshake className="w-4 h-4" />,
      count: playersWithOffers.length,
    },
  ];

  const currentList = useMemo(
    () => getCurrentTransferList(view, transferCollections),
    [transferCollections, view],
  );

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
          prev.includes(only)
            ? prev.filter((entry) => entry !== only)
            : [...prev, only],
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
      prev.includes(position)
        ? prev.filter((entry) => entry !== position)
        : [...prev, position],
    );
  };

  const filteredList = useMemo(
    () =>
      filterTransferPlayers(
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
      ),
    [
      affordableOnly,
      availabilityFilter,
      currentList,
      isPlayersView,
      myTeam,
      search,
      specificPositions,
    ],
  );
  const marketTotalPages = Math.max(
    1,
    Math.ceil(filteredList.length / TRANSFER_MARKET_PAGE_SIZE),
  );
  const safeMarketPage = Math.min(marketPage, marketTotalPages);
  const marketPageStart = (safeMarketPage - 1) * TRANSFER_MARKET_PAGE_SIZE;
  const visibleList = isPlayersView
    ? filteredList.slice(
        marketPageStart,
        marketPageStart + TRANSFER_MARKET_PAGE_SIZE,
      )
    : filteredList;
  const showMarketPagination =
    isPlayersView && filteredList.length > TRANSFER_MARKET_PAGE_SIZE;
  const marketRangeFrom = filteredList.length === 0 ? 0 : marketPageStart + 1;
  const marketRangeTo = Math.min(
    marketPageStart + TRANSFER_MARKET_PAGE_SIZE,
    filteredList.length,
  );
  const availabilityFilters: {
    id: TransferAvailabilityFilter;
    label: string;
    count: number;
  }[] = [
    {
      id: "all",
      label: t("common.all"),
      count: availablePlayers.length,
    },
    {
      id: "transfer",
      label: t("transfers.transfer"),
      count: marketPlayers.length,
    },
    {
      id: "loan",
      label: t("transfers.loan"),
      count: loanPlayers.length,
    },
    {
      id: "free_agent",
      label: t("common.freeAgent"),
      count: freeAgentPlayers.length,
    },
  ];
  const annualWageBudget = myTeam?.wage_budget ?? 0;
  const {
    freeAgentTarget,
    contractWage,
    setContractWage,
    contractLength,
    setContractLength,
    contractFeedback,
    contractProjection,
    contractSubmitting,
    contractSubmitDisabled,
    contractStatusMessage,
    contractStatusClassName,
    openFreeAgentContract,
    closeFreeAgentContract,
    submitFreeAgentContract,
  } = useFreeAgentContractFlow({
    gameState,
    onGameUpdate,
  });
  const {
    loanTarget,
    loanPeriodId,
    setLoanPeriodId,
    loanWageContributionPct,
    setLoanWageContributionPct,
    loanBuyOptionEnabled,
    setLoanBuyOptionEnabled,
    loanBuyOptionFee,
    setLoanBuyOptionFee,
    loanLoading,
    loanError,
    loanResult,
    loanSuggestedTerms,
    loanPeriodOptions,
    selectedLoanPeriodOption,
    loanSubmitDisabled,
    openLoanOffer,
    closeLoanOffer,
    handleMakeLoanOffer,
  } = useLoanOfferFlow({
    loanRegistrationDate,
    transferWindowBlocksRegistration,
    onGameUpdate,
  });
  const {
    loanCounterTarget,
    loanCounterPeriodId,
    setLoanCounterPeriodId,
    loanCounterWageContributionPct,
    setLoanCounterWageContributionPct,
    loanCounterBuyOptionEnabled,
    setLoanCounterBuyOptionEnabled,
    loanCounterBuyOptionFee,
    setLoanCounterBuyOptionFee,
    loanCounterLoading,
    loanCounterError,
    loanCounterResult,
    loanCounterSuggestedTerms,
    loanCounterPeriodOptions,
    selectedLoanCounterPeriodOption,
    loanCounterSubmitDisabled,
    openLoanCounterOffer,
    closeLoanCounterOffer,
    handleCounterLoanOffer,
  } = useLoanCounterOfferFlow({
    loanRegistrationDate,
    transferWindowBlocksRegistration,
    onGameUpdate,
  });
  const {
    counterTarget,
    counterAmount,
    setCounterAmount,
    counterLoading,
    counterError,
    counterResult,
    counterFeedback,
    activeCounterOffer,
    openCounterNegotiation,
    closeCounterNegotiation,
    handleCounterOffer,
  } = useTransferCounterOfferFlow({ onGameUpdate });

  const getDealKinds = (player: PlayerData): DealKind[] => {
    const kinds: DealKind[] = [];

    if (player.team_id !== null && player.transfer_listed) {
      kinds.push("transfer");
    }

    if (player.team_id !== null && player.loan_listed) {
      kinds.push("loan");
    }

    if (player.team_id === null) {
      kinds.push("contract");
    }

    return kinds;
  };

  const getStartableDealKinds = (player: PlayerData): DealKind[] =>
    getDealKinds(player).filter((kind) => isDealKindStartable(player, kind));

  const isDealKindStartable = (player: PlayerData, kind: DealKind): boolean => {
    if (kind === "transfer") {
      return (
        player.team_id !== null &&
        player.transfer_listed &&
        !transferWindowBlocksRegistration
      );
    }

    if (kind === "loan") {
      return (
        player.team_id !== null &&
        player.loan_listed &&
        !transferWindowBlocksRegistration
      );
    }

    return player.team_id === null;
  };

  const selectDealWorkspaceKind = (player: PlayerData, kind: DealKind) => {
    setDealWorkspaceKind(kind);

    if (kind === "contract") {
      closeBidNegotiation();
      closeLoanOffer();
      if (isDealKindStartable(player, kind)) {
        openFreeAgentContract(player);
      } else {
        closeFreeAgentContract();
      }
      return;
    }

    if (kind === "loan") {
      closeBidNegotiation();
      closeFreeAgentContract();
      if (isDealKindStartable(player, kind)) {
        openLoanOffer(player);
      } else {
        closeLoanOffer();
      }
      return;
    }

    closeLoanOffer();
    closeFreeAgentContract();
    if (isDealKindStartable(player, kind)) {
      openBidNegotiation(player);
    } else {
      closeBidNegotiation();
    }
  };

  const openDealEntry = (player: PlayerData) => {
    const dealKinds = getDealKinds(player);
    const startableDealKinds = getStartableDealKinds(player);
    const initialKind = startableDealKinds[0] ?? dealKinds[0] ?? "transfer";

    setDealWorkspaceTarget(player);
    selectDealWorkspaceKind(player, initialKind);
  };

  const closeDealWorkspace = () => {
    if (bidLoading || loanLoading || contractSubmitting) {
      return;
    }

    setDealWorkspaceTarget(null);
    closeBidNegotiation();
    closeLoanOffer();
    closeFreeAgentContract();
  };

  const getDealEntryLabel = (player: PlayerData): string => {
    const dealKinds = getDealKinds(player);

    if (dealKinds.length > 1) {
      return t("transfers.makeOffer");
    }

    if (dealKinds[0] === "contract") {
      return t("transfers.offerContract");
    }

    if (dealKinds[0] === "loan") {
      return t("transfers.loanOffer");
    }

    return t("transfers.bid");
  };

  const getDealEntryIcon = (player: PlayerData, className: string) => {
    const dealKinds = getDealKinds(player);

    if (dealKinds.length > 1) {
      return <Handshake className={className} />;
    }

    if (dealKinds[0] === "contract") {
      return <UserPlus className={className} />;
    }

    if (dealKinds[0] === "loan") {
      return <ArrowRightLeft className={className} />;
    }

    return <Gavel className={className} />;
  };

  const handleScoutPlayer = async (playerId: string): Promise<void> => {
    if (availableScouts.length === 0) {
      setScoutError(null);
      return;
    }

    const scout = availableScouts[0];
    setScoutError(null);
    setScoutingPlayerId(playerId);

    try {
      const updated = await sendScout(scout.id, playerId);
      setScoutError(null);
      onGameUpdate?.(updated);
    } catch (error) {
      console.error("Failed to send scout:", error);
      setScoutError(resolveTranslatedErrorMessage(getErrorMessage(error), t));
    } finally {
      setScoutingPlayerId(null);
    }
  };

  const handleToggleTransferListing = async (
    playerId: string,
  ): Promise<void> => {
    setListingError(null);

    try {
      const updated = await toggleTransferList(playerId);
      setListingError(null);
      onGameUpdate?.(updated);
    } catch (error) {
      setListingError(resolveTranslatedErrorMessage(getErrorMessage(error), t));
    }
  };

  const handleToggleLoanListing = async (playerId: string): Promise<void> => {
    setListingError(null);

    try {
      const updated = await toggleLoanList(playerId);
      setListingError(null);
      onGameUpdate?.(updated);
    } catch (error) {
      setListingError(resolveTranslatedErrorMessage(getErrorMessage(error), t));
    }
  };

  return (
    <div>
      {/* Budget header */}
      {myTeam && (
        <TransfersBudgetHeader
          myTeam={myTeam}
          transferWindowVariant={transferWindowVariant}
          transferWindowStatus={transferWindow.status}
          transferWindowSummary={transferWindowSummary}
          annualWageBudget={annualWageBudget}
          annualSuffix={annualSuffix}
          listedCount={myListedPlayers.length}
        />
      )}

      <TransfersControls
        tabs={tabs}
        activeView={view}
        onSelectView={(nextView) => {
          setView(nextView);
          setMarketPage(1);
          if (nextView !== "players") {
            setAvailabilityFilter("all");
          }
        }}
        search={search}
        onSearchChange={(value) => {
          setSearch(value);
          setMarketPage(1);
        }}
        positions={positions}
        specificPositions={specificPositions}
        openPositionPopover={openPositionPopover}
        positionFilterRef={positionFilterRef}
        onSelectPositionGroup={handleSelectPositionGroup}
        onToggleSpecificPosition={handleToggleSpecificPosition}
        showAffordable={Boolean(myTeam)}
        affordableOnly={affordableOnly}
        onToggleAffordable={() => {
          setAffordableOnly((prev) => !prev);
          setMarketPage(1);
        }}
        isPlayersView={isPlayersView}
        availabilityFilters={availabilityFilters}
        availabilityFilter={availabilityFilter}
        onSelectAvailability={(id) => {
          setAvailabilityFilter(id);
          setMarketPage(1);
        }}
        resultCount={filteredList.length}
      />

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

      {/* Content */}
      {view === "my_list" && filteredList.length === 0 && (
        <Card>
          <CardBody>
            <div className="text-center py-8">
              <ShoppingCart className="w-10 h-10 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {t("transfers.noPlayersListed")}
              </p>
              <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
                {t("transfers.goToProfile")}
              </p>
            </div>
          </CardBody>
        </Card>
      )}

      {view === "offers" && filteredList.length === 0 && (
        <Card>
          <CardBody>
            <div className="text-center py-8">
              <Handshake className="w-10 h-10 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {t("transfers.noOffers")}
              </p>
            </div>
          </CardBody>
        </Card>
      )}

      {filteredList.length > 0 && (
        <TransferPlayerTable
          gameState={gameState}
          userTeamId={userTeamId}
          view={view}
          isScoutingView={isScoutingView}
          visibleList={visibleList}
          annualSuffix={annualSuffix}
          alreadyScoutingIds={alreadyScoutingIds}
          scoutingPlayerId={scoutingPlayerId}
          availableScoutsCount={availableScouts.length}
          onSelectPlayer={onSelectPlayer}
          onSelectTeam={onSelectTeam}
          onToggleTransferListing={handleToggleTransferListing}
          onToggleLoanListing={handleToggleLoanListing}
          onScoutPlayer={handleScoutPlayer}
          onOpenDealEntry={openDealEntry}
          getDealEntryLabel={getDealEntryLabel}
          getDealEntryIcon={getDealEntryIcon}
          onRespondOffer={handleRespondOffer}
          onRespondLoanOffer={handleRespondLoanOffer}
          onOpenCounterNegotiation={openCounterNegotiation}
          onOpenLoanCounterOffer={openLoanCounterOffer}
          onExerciseLoanBuyOption={handleExerciseLoanBuyOption}
          showMarketPagination={showMarketPagination}
          marketRangeFrom={marketRangeFrom}
          marketRangeTo={marketRangeTo}
          totalCount={filteredList.length}
          safeMarketPage={safeMarketPage}
          marketTotalPages={marketTotalPages}
          onPageChange={setMarketPage}
        />
      )}
      {isScoutingView && filteredList.length === 0 && (
        <Card>
          <CardBody>
            <div className="text-center py-8">
              <TrendingUp className="w-10 h-10 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {availabilityFilter === "transfer"
                  ? t("transfers.noTransferMarket")
                  : availabilityFilter === "free_agent"
                    ? t("transfers.noFreeAgents")
                    : availabilityFilter === "loan"
                      ? t("transfers.noLoanMarket")
                      : t("transfers.noAvailablePlayers")}
              </p>
            </div>
          </CardBody>
        </Card>
      )}
      {dealWorkspaceTarget && (
        <PlayerDealWorkspace
          player={dealWorkspaceTarget}
          teams={gameState.teams}
          myTeam={myTeam ?? null}
          annualSuffix={annualSuffix}
          transferWindowBlocksRegistration={transferWindowBlocksRegistration}
          transferWindowSummary={transferWindowSummary}
          loanNoticeDetail={loanWindowNoticeDetail}
          selectedKind={dealWorkspaceKind}
          onSelectKind={(kind) =>
            selectDealWorkspaceKind(dealWorkspaceTarget, kind)
          }
          onClose={closeDealWorkspace}
          renderDealPanel={(kind) => {
            if (kind === "transfer" && bidTarget) {
              return (
                <TransferBidForm
                  bidTarget={bidTarget}
                  teams={gameState.teams}
                  bidAmount={bidAmount}
                  onBidAmountChange={setBidAmount}
                  myTeam={myTeam ?? null}
                  bidFee={bidFee}
                  bidProjection={bidProjection}
                  bidFeedback={bidFeedback}
                  activeBidOffer={activeBidOffer}
                  hasExistingOffer={hasExistingOffer}
                  bidResult={bidResult}
                  bidLoading={bidLoading}
                  bidSubmitDisabled={
                    transferWindowBlocksRegistration || bidSubmitDisabled
                  }
                  blockingTitle={transferWindowBlockingTitle}
                  blockingDetail={transferWindowBlockingDetail}
                  showPlayerSummary={false}
                  onSubmit={handleMakeBid}
                  onClose={closeDealWorkspace}
                />
              );
            }

            if (kind === "loan" && loanTarget) {
              return (
                <LoanOfferForm
                  loanTarget={loanTarget}
                  teams={gameState.teams}
                  periodId={loanPeriodId}
                  periodOptions={loanPeriodOptions}
                  selectedEndDate={selectedLoanPeriodOption?.endDate ?? ""}
                  onPeriodChange={setLoanPeriodId}
                  wageContributionPct={loanWageContributionPct}
                  onWageContributionChange={setLoanWageContributionPct}
                  buyOptionEnabled={loanBuyOptionEnabled}
                  buyOptionFee={loanBuyOptionFee}
                  onBuyOptionEnabledChange={setLoanBuyOptionEnabled}
                  onBuyOptionFeeChange={setLoanBuyOptionFee}
                  result={loanResult}
                  suggestedTerms={loanSuggestedTerms}
                  error={loanError}
                  loading={loanLoading}
                  submitDisabled={loanSubmitDisabled}
                  noticeTitle={loanWindowNoticeTitle}
                  noticeDetail={loanWindowNoticeDetail}
                  acceptedMessage={
                    isTransferWindowClosed && closedWindowRegistrationDate
                      ? t("transfers.loanOfferScheduled", {
                          date: formatDate(
                            closedWindowRegistrationDate,
                            i18n.language,
                          ),
                        })
                      : null
                  }
                  showPlayerSummary={false}
                  onSubmit={handleMakeLoanOffer}
                  onClose={closeDealWorkspace}
                />
              );
            }

            if (kind === "contract" && freeAgentTarget) {
              return (
                <FreeAgentContractForm
                  player={freeAgentTarget}
                  teams={gameState.teams}
                  wage={contractWage}
                  onWageChange={setContractWage}
                  contractLength={contractLength}
                  onContractLengthChange={setContractLength}
                  projection={contractProjection}
                  feedback={contractFeedback}
                  statusMessage={contractStatusMessage(t)}
                  statusClassName={contractStatusClassName}
                  submitting={contractSubmitting}
                  submitDisabled={contractSubmitDisabled}
                  showPlayerSummary={false}
                  onSubmit={submitFreeAgentContract}
                  onClose={closeDealWorkspace}
                />
              );
            }

            return (
              <div className="rounded-lg bg-gray-50 p-6 text-sm text-gray-600 dark:bg-navy-900/50 dark:text-gray-300">
                {t("transfers.dealChooserHint")}
              </div>
            );
          }}
        />
      )}
      {/* Bid Modal */}
      {bidTarget && !dealWorkspaceTarget && (
        <TransferBidModal
          bidTarget={bidTarget}
          teams={gameState.teams}
          bidAmount={bidAmount}
          onBidAmountChange={setBidAmount}
          myTeam={myTeam ?? null}
          bidFee={bidFee}
          bidProjection={bidProjection}
          bidFeedback={bidFeedback}
          activeBidOffer={activeBidOffer}
          hasExistingOffer={hasExistingOffer}
          bidResult={bidResult}
          bidLoading={bidLoading}
          bidSubmitDisabled={
            transferWindowBlocksRegistration || bidSubmitDisabled
          }
          blockingTitle={transferWindowBlockingTitle}
          blockingDetail={transferWindowBlockingDetail}
          onSubmit={handleMakeBid}
          onClose={closeBidNegotiation}
        />
      )}
      {counterTarget && (
        <TransferCounterOfferModal
          counterTarget={counterTarget}
          teams={gameState.teams}
          counterAmount={counterAmount}
          onCounterAmountChange={setCounterAmount}
          counterFeedback={counterFeedback}
          activeCounterOffer={activeCounterOffer}
          counterResult={counterResult}
          counterError={counterError}
          counterLoading={counterLoading}
          submitDisabled={transferWindowBlocksRegistration}
          blockingTitle={transferWindowBlockingTitle}
          blockingDetail={transferWindowBlockingDetail}
          onSubmit={handleCounterOffer}
          onClose={closeCounterNegotiation}
        />
      )}
      {freeAgentTarget && !dealWorkspaceTarget && (
        <FreeAgentContractModal
          player={freeAgentTarget}
          teams={gameState.teams}
          wage={contractWage}
          onWageChange={setContractWage}
          contractLength={contractLength}
          onContractLengthChange={setContractLength}
          projection={contractProjection}
          feedback={contractFeedback}
          statusMessage={contractStatusMessage(t)}
          statusClassName={contractStatusClassName}
          submitting={contractSubmitting}
          submitDisabled={contractSubmitDisabled}
          onSubmit={submitFreeAgentContract}
          onClose={closeFreeAgentContract}
        />
      )}
      {loanTarget && !dealWorkspaceTarget && (
        <LoanOfferModal
          loanTarget={loanTarget}
          teams={gameState.teams}
          periodId={loanPeriodId}
          periodOptions={loanPeriodOptions}
          selectedEndDate={selectedLoanPeriodOption?.endDate ?? ""}
          onPeriodChange={setLoanPeriodId}
          wageContributionPct={loanWageContributionPct}
          onWageContributionChange={setLoanWageContributionPct}
          buyOptionEnabled={loanBuyOptionEnabled}
          buyOptionFee={loanBuyOptionFee}
          onBuyOptionEnabledChange={setLoanBuyOptionEnabled}
          onBuyOptionFeeChange={setLoanBuyOptionFee}
          result={loanResult}
          suggestedTerms={loanSuggestedTerms}
          error={loanError}
          loading={loanLoading}
          submitDisabled={loanSubmitDisabled}
          noticeTitle={loanWindowNoticeTitle}
          noticeDetail={loanWindowNoticeDetail}
          acceptedMessage={
            isTransferWindowClosed && closedWindowRegistrationDate
              ? t("transfers.loanOfferScheduled", {
                  date: formatDate(closedWindowRegistrationDate, i18n.language),
                })
              : null
          }
          onSubmit={handleMakeLoanOffer}
          onClose={closeLoanOffer}
        />
      )}
      {loanCounterTarget && (
        <LoanOfferModal
          loanTarget={loanCounterTarget.player}
          teams={gameState.teams}
          periodId={loanCounterPeriodId}
          periodOptions={loanCounterPeriodOptions}
          selectedEndDate={selectedLoanCounterPeriodOption?.endDate ?? ""}
          onPeriodChange={setLoanCounterPeriodId}
          wageContributionPct={loanCounterWageContributionPct}
          onWageContributionChange={setLoanCounterWageContributionPct}
          buyOptionEnabled={loanCounterBuyOptionEnabled}
          buyOptionFee={loanCounterBuyOptionFee}
          onBuyOptionEnabledChange={setLoanCounterBuyOptionEnabled}
          onBuyOptionFeeChange={setLoanCounterBuyOptionFee}
          result={loanCounterResult}
          titleKey="transfers.counterLoanOffer"
          submitLabelKey="transfers.submitLoanCounter"
          acceptedLabelKey="transfers.loanCounterAccepted"
          rejectedLabelKey="transfers.loanCounterRejected"
          counteredLabelKey="transfers.loanCounterCountered"
          suggestedTerms={loanCounterSuggestedTerms}
          error={loanCounterError}
          loading={loanCounterLoading}
          submitDisabled={loanCounterSubmitDisabled}
          noticeTitle={loanWindowNoticeTitle}
          noticeDetail={loanWindowNoticeDetail}
          acceptedMessage={
            isTransferWindowClosed && closedWindowRegistrationDate
              ? t("transfers.loanCounterScheduled", {
                  date: formatDate(closedWindowRegistrationDate, i18n.language),
                })
              : null
          }
          onSubmit={handleCounterLoanOffer}
          onClose={closeLoanCounterOffer}
        />
      )}
    </div>
  );
}
