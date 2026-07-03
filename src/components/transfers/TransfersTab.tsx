import { useEffect, useMemo, useRef, useState } from "react";
import {
  GameStateData,
  LoanOfferData,
  PlayerData,
  PlayerSelectionOptions,
  TransferOfferData,
} from "../../store/gameStore";
import { Card, CardBody, Badge, CountryFlag, PlayerAvatar } from "../ui";
import ContextMenu from "../ContextMenu";
import {
  Search,
  TrendingUp,
  ShoppingCart,
  Handshake,
  ArrowRightLeft,
  Filter,
  Gavel,
  Check,
  X,
  UserPlus,
  ChevronLeft,
  ChevronRight,
} from "lucide-react";
import {
  getTeamName,
  calcAge,
  formatVal,
  formatAnnualAmount,
  getPlayerOvr,
  positionBadgeVariant,
} from "../../lib/helpers";
import { useTranslation } from "react-i18next";
import { countryName } from "../../lib/countries";
import {
  translatePositionAbbreviation,
  translatePositionLabel,
} from "../squad/SquadTab.helpers";
import { resolveSeasonContext } from "../../lib/seasonContext";
import { formatDate } from "../../lib/dateFormatting";
import { type NegotiationFeedbackPanelData } from "../NegotiationFeedbackPanel";
import TransferBidModal, { TransferBidForm } from "./TransferBidModal";
import TransferCounterOfferModal from "./TransferCounterOfferModal";
import LoanOfferModal, { LoanOfferForm } from "./LoanOfferModal";
import PlayerDealWorkspace, { type DealKind } from "./PlayerDealWorkspace";
import {
  getErrorMessage,
  resolveTranslatedErrorMessage,
} from "../../utils/errorMessage";
import {
  counterLoanOffer,
  counterOffer,
  exerciseLoanBuyOption,
  makeLoanOffer,
  respondToOffer,
  respondToLoanOffer,
  toggleLoanList,
  toggleTransferList,
  type TransferNegotiationResponseData,
  type LoanOfferResponseData,
} from "../../services/transfersService";
import { sendScout } from "../../services/scoutingService";
import {
  buildLoanPeriodOptions,
  buildResumedCounterFeedback,
  formatTransferFeeInput,
  getDefaultLoanPeriodId,
  getLoanPeriodIdForEndDate,
  getTransferOfferBadgeVariant,
  getTransferOfferStatusLabel,
  type LoanPeriodOptionId,
  mapTransferNegotiationError,
  normalizeTransferNegotiationFeedback,
  parseTransferFeeInput,
} from "./TransfersTab.helpers";
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
import {
  buildDividerMenuItem,
  buildScoutPlayerMenuItem,
  buildToggleLoanListMenuItem,
  buildToggleTransferListMenuItem,
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";
import FreeAgentContractModal, {
  FreeAgentContractForm,
} from "./FreeAgentContractModal";
import { useFreeAgentContractFlow } from "./useFreeAgentContractFlow";
import { useTransferBidFlow } from "./useTransferBidFlow";

interface TransfersTabProps {
  gameState: GameStateData;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onSelectTeam: (id: string) => void;
  onGameUpdate?: (game: GameStateData) => void;
}

const TRANSFER_MARKET_PAGE_SIZE = 30;

type CounterTarget = {
  player: PlayerData;
  offerId: string;
  fromTeamId: string;
  fee: number;
};

type LoanCounterTarget = {
  player: PlayerData;
  offer: LoanOfferData;
};

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
  const [counterTarget, setCounterTarget] = useState<CounterTarget | null>(
    null,
  );
  const [counterAmount, setCounterAmount] = useState("");
  const [counterLoading, setCounterLoading] = useState(false);
  const [counterError, setCounterError] = useState<string | null>(null);
  const [counterResult, setCounterResult] = useState<
    TransferNegotiationResponseData["decision"] | "error" | null
  >(null);
  const [counterFeedback, setCounterFeedback] =
    useState<NegotiationFeedbackPanelData | null>(null);
  const [scoutingPlayerId, setScoutingPlayerId] = useState<string | null>(null);
  const [scoutError, setScoutError] = useState<string | null>(null);
  const [listingError, setListingError] = useState<string | null>(null);
  const [loanTarget, setLoanTarget] = useState<PlayerData | null>(null);
  const [loanPeriodId, setLoanPeriodId] = useState<LoanPeriodOptionId | "">(
    getDefaultLoanPeriodId(loanRegistrationDate, null),
  );
  const [loanWageContributionPct, setLoanWageContributionPct] = useState(100);
  const [loanBuyOptionEnabled, setLoanBuyOptionEnabled] = useState(false);
  const [loanBuyOptionFee, setLoanBuyOptionFee] = useState("");
  const [loanLoading, setLoanLoading] = useState(false);
  const [loanError, setLoanError] = useState<string | null>(null);
  const [loanResult, setLoanResult] = useState<
    LoanOfferResponseData["decision"] | "error" | null
  >(null);
  const [loanSuggestedTerms, setLoanSuggestedTerms] = useState<{
    wageContributionPct: number;
    endDate: string;
    buyOptionFee?: number | null;
  } | null>(null);
  const [loanCounterTarget, setLoanCounterTarget] =
    useState<LoanCounterTarget | null>(null);
  const [loanCounterPeriodId, setLoanCounterPeriodId] = useState<
    LoanPeriodOptionId | ""
  >(getDefaultLoanPeriodId(loanRegistrationDate, null));
  const [loanCounterWageContributionPct, setLoanCounterWageContributionPct] =
    useState(100);
  const [loanCounterBuyOptionEnabled, setLoanCounterBuyOptionEnabled] =
    useState(false);
  const [loanCounterBuyOptionFee, setLoanCounterBuyOptionFee] = useState("");
  const [loanCounterLoading, setLoanCounterLoading] = useState(false);
  const [loanCounterError, setLoanCounterError] = useState<string | null>(null);
  const [loanCounterResult, setLoanCounterResult] = useState<
    LoanOfferResponseData["decision"] | "error" | null
  >(null);
  const [loanCounterSuggestedTerms, setLoanCounterSuggestedTerms] = useState<{
    wageContributionPct: number;
    endDate: string;
    buyOptionFee?: number | null;
  } | null>(null);
  const [dealWorkspaceTarget, setDealWorkspaceTarget] =
    useState<PlayerData | null>(null);
  const [dealWorkspaceKind, setDealWorkspaceKind] =
    useState<DealKind>("transfer");

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

  const openLoanOffer = (player: PlayerData) => {
    setLoanTarget(player);
    setLoanPeriodId(
      getDefaultLoanPeriodId(loanRegistrationDate, player.contract_end),
    );
    setLoanWageContributionPct(100);
    setLoanBuyOptionEnabled(false);
    setLoanBuyOptionFee("");
    setLoanError(null);
    setLoanResult(null);
    setLoanSuggestedTerms(null);
  };

  const closeLoanOffer = () => {
    setLoanTarget(null);
    setLoanPeriodId(getDefaultLoanPeriodId(loanRegistrationDate, null));
    setLoanWageContributionPct(100);
    setLoanBuyOptionEnabled(false);
    setLoanBuyOptionFee("");
    setLoanError(null);
    setLoanResult(null);
    setLoanSuggestedTerms(null);
  };

  const openLoanCounterOffer = (player: PlayerData, offer: LoanOfferData) => {
    setLoanCounterTarget({ player, offer });
    setLoanCounterPeriodId(
      getLoanPeriodIdForEndDate(
        loanRegistrationDate,
        player.contract_end,
        offer.suggested_end_date ?? offer.end_date,
      ),
    );
    setLoanCounterWageContributionPct(
      Math.min(
        100,
        Math.max(
          offer.suggested_wage_contribution_pct ?? offer.wage_contribution_pct,
          offer.wage_contribution_pct,
        ),
      ),
    );
    const buyOptionFee =
      offer.suggested_buy_option_fee ?? offer.buy_option_fee ?? null;
    setLoanCounterBuyOptionEnabled(Boolean(buyOptionFee));
    setLoanCounterBuyOptionFee(
      buyOptionFee ? formatTransferFeeInput(buyOptionFee) : "",
    );
    setLoanCounterError(null);
    setLoanCounterResult(null);
    setLoanCounterSuggestedTerms(null);
  };

  const closeLoanCounterOffer = () => {
    setLoanCounterTarget(null);
    setLoanCounterPeriodId(getDefaultLoanPeriodId(loanRegistrationDate, null));
    setLoanCounterWageContributionPct(100);
    setLoanCounterBuyOptionEnabled(false);
    setLoanCounterBuyOptionFee("");
    setLoanCounterError(null);
    setLoanCounterResult(null);
    setLoanCounterSuggestedTerms(null);
  };

  const openCounterNegotiation = (
    player: PlayerData,
    offer: TransferOfferData,
  ) => {
    setCounterTarget({
      player,
      offerId: offer.id,
      fromTeamId: offer.from_team_id,
      fee: offer.fee,
    });
    setCounterAmount(
      formatTransferFeeInput(offer.suggested_counter_fee ?? offer.fee),
    );
    setCounterError(null);
    setCounterResult(null);
    setCounterFeedback(buildResumedCounterFeedback(offer));
  };

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

  const handleMakeLoanOffer = async () => {
    if (!loanTarget || !selectedLoanPeriodOption) return;

    setLoanLoading(true);
    setLoanError(null);
    setLoanResult(null);
    setLoanSuggestedTerms(null);

    try {
      const response = await makeLoanOffer(
        loanTarget.id,
        selectedLoanPeriodOption.endDate,
        Math.max(0, Math.min(100, Math.round(loanWageContributionPct))),
        loanBuyOptionEnabled ? parseTransferFeeInput(loanBuyOptionFee) : null,
      );
      setLoanResult(response.decision);
      if (response.decision === "counter_offer") {
        setLoanSuggestedTerms({
          wageContributionPct:
            response.suggested_wage_contribution_pct ?? loanWageContributionPct,
          endDate:
            response.suggested_end_date ?? selectedLoanPeriodOption.endDate,
          buyOptionFee: response.suggested_buy_option_fee,
        });
        if (response.suggested_wage_contribution_pct !== null) {
          setLoanWageContributionPct(
            Math.max(
              0,
              Math.min(
                100,
                Math.round(response.suggested_wage_contribution_pct),
              ),
            ),
          );
        }
        if (response.suggested_end_date) {
          setLoanPeriodId(
            getLoanPeriodIdForEndDate(
              loanRegistrationDate,
              loanTarget.contract_end,
              response.suggested_end_date,
            ),
          );
        }
        if (response.suggested_buy_option_fee !== null) {
          setLoanBuyOptionEnabled(true);
          setLoanBuyOptionFee(
            formatTransferFeeInput(response.suggested_buy_option_fee),
          );
        } else {
          setLoanBuyOptionEnabled(false);
          setLoanBuyOptionFee("");
        }
      }
      if (onGameUpdate) onGameUpdate(response.game);
    } catch (err: any) {
      setLoanResult("error");
      setLoanError(resolveTranslatedErrorMessage(getErrorMessage(err), t));
    } finally {
      setLoanLoading(false);
    }
  };

  const handleCounterLoanOffer = async () => {
    if (!loanCounterTarget || !selectedLoanCounterPeriodOption) return;

    setLoanCounterLoading(true);
    setLoanCounterError(null);
    setLoanCounterResult(null);
    setLoanCounterSuggestedTerms(null);

    try {
      const response = await counterLoanOffer(
        loanCounterTarget.player.id,
        loanCounterTarget.offer.id,
        selectedLoanCounterPeriodOption.endDate,
        Math.max(0, Math.min(100, Math.round(loanCounterWageContributionPct))),
        loanCounterBuyOptionEnabled
          ? parseTransferFeeInput(loanCounterBuyOptionFee)
          : null,
      );
      setLoanCounterResult(response.decision);
      if (response.decision === "counter_offer") {
        setLoanCounterSuggestedTerms({
          wageContributionPct:
            response.suggested_wage_contribution_pct ??
            loanCounterWageContributionPct,
          endDate:
            response.suggested_end_date ??
            selectedLoanCounterPeriodOption.endDate,
          buyOptionFee: response.suggested_buy_option_fee,
        });
        if (response.suggested_wage_contribution_pct !== null) {
          setLoanCounterWageContributionPct(
            response.suggested_wage_contribution_pct,
          );
        }
        if (response.suggested_end_date) {
          setLoanCounterPeriodId(
            getLoanPeriodIdForEndDate(
              loanRegistrationDate,
              loanCounterTarget.player.contract_end,
              response.suggested_end_date,
            ),
          );
        }
        if (response.suggested_buy_option_fee) {
          setLoanCounterBuyOptionEnabled(true);
          setLoanCounterBuyOptionFee(
            formatTransferFeeInput(response.suggested_buy_option_fee),
          );
        }
      }
      if (onGameUpdate) onGameUpdate(response.game);
    } catch (err: any) {
      setLoanCounterResult("error");
      setLoanCounterError(
        resolveTranslatedErrorMessage(getErrorMessage(err), t),
      );
    } finally {
      setLoanCounterLoading(false);
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

  const handleCounterOffer = async () => {
    const requestedFee = parseTransferFeeInput(counterAmount);

    if (!counterTarget || requestedFee === null || requestedFee <= 0) return;

    setCounterLoading(true);
    setCounterError(null);
    setCounterResult(null);
    setCounterFeedback(null);

    try {
      const response = await counterOffer(
        counterTarget.player.id,
        counterTarget.offerId,
        requestedFee,
      );

      if (onGameUpdate) onGameUpdate(response.game);
      setCounterResult(response.decision);
      setCounterFeedback(
        normalizeTransferNegotiationFeedback(response.feedback),
      );
      if (response.suggested_fee !== null) {
        setCounterAmount(formatTransferFeeInput(response.suggested_fee));
      }
    } catch (err: any) {
      setCounterError(
        mapTransferNegotiationError(t, err?.toString() || "error"),
      );
    } finally {
      setCounterLoading(false);
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
  const activeCounterOffer = counterTarget
    ? (counterTarget.player.transfer_offers.find(
        (offer) => offer.id === counterTarget.offerId,
      ) ?? null)
    : null;
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
  const parsedLoanBuyOptionFee = loanBuyOptionEnabled
    ? parseTransferFeeInput(loanBuyOptionFee)
    : null;
  const parsedLoanCounterBuyOptionFee = loanCounterBuyOptionEnabled
    ? parseTransferFeeInput(loanCounterBuyOptionFee)
    : null;
  const loanPeriodOptions = loanTarget
    ? buildLoanPeriodOptions(loanRegistrationDate, loanTarget.contract_end)
    : [];
  const selectedLoanPeriodOption =
    loanPeriodOptions.find(
      (option) => option.id === loanPeriodId && !option.disabled,
    ) ?? null;
  const loanSubmitDisabled =
    loanLoading ||
    !selectedLoanPeriodOption ||
    loanResult === "accepted" ||
    transferWindowBlocksRegistration ||
    (loanBuyOptionEnabled &&
      (parsedLoanBuyOptionFee === null || parsedLoanBuyOptionFee <= 0));
  const loanCounterReferenceEndDate =
    loanCounterSuggestedTerms?.endDate ??
    loanCounterTarget?.offer.suggested_end_date ??
    loanCounterTarget?.offer.end_date ??
    null;
  const loanCounterPeriodOptions = loanCounterTarget
    ? buildLoanPeriodOptions(
        loanRegistrationDate,
        loanCounterTarget.player.contract_end,
        loanCounterReferenceEndDate,
      )
    : [];
  const selectedLoanCounterPeriodOption =
    loanCounterPeriodOptions.find(
      (option) => option.id === loanCounterPeriodId && !option.disabled,
    ) ?? null;
  const loanCounterSubmitDisabled =
    loanCounterLoading ||
    !selectedLoanCounterPeriodOption ||
    loanCounterResult === "accepted" ||
    transferWindowBlocksRegistration ||
    (loanCounterBuyOptionEnabled &&
      (parsedLoanCounterBuyOptionFee === null ||
        parsedLoanCounterBuyOptionFee <= 0));

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
        <Card accent="primary" className="mb-5">
          <div className="bg-gradient-to-r from-navy-700 to-navy-800 p-5 rounded-t-xl flex items-center gap-6">
            <div className="flex-1">
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="text-lg font-heading font-bold text-white uppercase tracking-wide flex items-center gap-2">
                  <TrendingUp className="w-5 h-5 text-accent-400" />
                  {t("transfers.centre")}
                </h2>
                <Badge variant={transferWindowVariant} size="sm">
                  {t(`season.transferWindowStatus.${transferWindow.status}`)}
                </Badge>
              </div>
              <p className="text-gray-400 text-xs mt-0.5">
                {t("transfers.transferWindow", { team: myTeam.name })}
              </p>
              <p className="text-gray-500 text-xs mt-1">
                {transferWindowSummary}
              </p>
            </div>
            <div className="hidden md:flex gap-4">
              <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  {t("finances.transferBudget")}
                </p>
                <p className="font-heading font-bold text-lg text-accent-400">
                  {formatVal(myTeam.transfer_budget)}
                </p>
              </div>
              <div
                data-testid="wage-budget-card"
                className="bg-white/5 rounded-xl px-4 py-2 text-center"
              >
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  {t("finances.wageBudget")}
                </p>
                <p className="font-heading font-bold text-lg text-white">
                  {formatAnnualAmount(
                    formatVal(annualWageBudget),
                    annualSuffix,
                  )}
                </p>
              </div>
              <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  {t("transfers.listed")}
                </p>
                <p className="font-heading font-bold text-lg text-white">
                  {myListedPlayers.length}
                </p>
              </div>
            </div>
          </div>
        </Card>
      )}

      {/* Tab navigation */}
      <div className="flex gap-2 mb-4 flex-wrap">
        {tabs.map((tab) => (
          <button
            type="button"
            key={tab.id}
            onClick={() => {
              setView(tab.id);
              setMarketPage(1);
              if (tab.id !== "players") {
                setAvailabilityFilter("all");
              }
            }}
            className={`px-4 py-2 rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-all flex items-center gap-1.5 ${
              view === tab.id
                ? "bg-primary-700 text-white shadow-md shadow-primary-700/20"
                : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600 hover:text-gray-700 dark:hover:text-gray-200"
            }`}
          >
            {tab.icon} {tab.label} ({tab.count})
          </button>
        ))}
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-3 mb-4 items-center">
        <div className="relative flex-1 min-w-[180px] max-w-xs">
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 dark:text-gray-500" />
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
            className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${specificPositions.length === 0 ? "bg-primary-700 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
          >
            {t("common.all")}
          </button>
          {positions.map((pos) => {
            const groupSpecifics = SPECIFIC_POSITIONS_BY_GROUP[pos] ?? [];
            const refinable = groupSpecifics.length > 1;
            const selectedInGroup = specificPositions.filter((entry) =>
              groupSpecifics.includes(entry),
            ).length;
            const isActive = selectedInGroup > 0;
            const isPartial =
              isActive && refinable && selectedInGroup < groupSpecifics.length;
            const groupLabel = t(`common.positionGroups.${pos}`, {
              defaultValue: t(`common.positions.${pos}`, { defaultValue: pos }),
            });

            return (
              <div key={pos} className="relative">
                <button
                  type="button"
                  onClick={() => handleSelectPositionGroup(pos)}
                  aria-haspopup={refinable ? "true" : undefined}
                  aria-expanded={
                    refinable ? openPositionPopover === pos : undefined
                  }
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
                  className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all inline-flex items-center gap-1 ${isActive ? "bg-primary-700 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
                >
                  {t(`common.posAbbr.${pos}`)}
                  {isPartial && (
                    <span
                      aria-hidden="true"
                      className="bg-white/20 text-[0.65rem] px-1.5 py-0.5 rounded-full leading-none"
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
                        const positionLabel = translatePositionLabel(
                          t,
                          position,
                        );
                        return (
                          <button
                            type="button"
                            key={position}
                            onClick={() =>
                              handleToggleSpecificPosition(position)
                            }
                            aria-pressed={selected}
                            aria-label={positionLabel}
                            title={positionLabel}
                            className={`px-2.5 py-1 rounded-md text-xs font-heading font-bold uppercase tracking-wider transition-all ${selected ? "bg-primary-700 text-white shadow-sm" : "bg-gray-50 dark:bg-navy-700 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600 hover:text-gray-700 dark:hover:text-gray-200"}`}
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
          })}
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
                className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${availabilityFilter === filter.id ? "bg-accent-500 text-navy-900 shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
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
                className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${affordableOnly ? "bg-primary-700 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
              >
                {t("transfers.affordableOnly")}
              </button>
            )}
          </div>
        )}
        <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
          <Filter className="w-3.5 h-3.5 inline mr-1 -mt-0.5" />
          {t("common.nResults", { count: filteredList.length })}
        </p>
      </div>

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
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                      {t("common.ovr")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                      {t("common.status")}
                    </th>
                    {view === "offers" && (
                      <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("transfers.offers")}
                      </th>
                    )}
                    {isScoutingView && (
                      <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("scouting.action")}
                      </th>
                    )}
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                  {visibleList.map((player) => {
                    const ovr = getPlayerOvr(player);
                    const age = calcAge(player.date_of_birth);
                    const transferOffersForThisPlayer =
                      player.transfer_offers ?? [];
                    const loanOffersForThisPlayer: LoanOfferData[] =
                      player.loan_offers ?? [];
                    const hasOffersForThisPlayer =
                      transferOffersForThisPlayer.length > 0 ||
                      loanOffersForThisPlayer.length > 0;
                    const scoutState = alreadyScoutingIds.has(player.id)
                      ? "already-assigned"
                      : scoutingPlayerId === player.id
                        ? "busy"
                        : availableScouts.length === 0
                          ? "unavailable"
                          : "ready";
                    const contextItems = [
                      buildViewProfileMenuItem(t, () =>
                        onSelectPlayer(player.id),
                      ),
                      ...(player.team_id
                        ? [
                            buildViewTeamMenuItem(t, () => {
                              onSelectTeam(player.team_id!);
                            }),
                          ]
                        : []),
                    ];

                    if (view === "my_list") {
                      contextItems.push(buildDividerMenuItem());
                      contextItems.push(
                        buildToggleTransferListMenuItem(
                          t,
                          player.transfer_listed,
                          () => {
                            void handleToggleTransferListing(player.id);
                          },
                        ),
                      );
                      contextItems.push(
                        buildToggleLoanListMenuItem(
                          t,
                          player.loan_listed,
                          () => {
                            void handleToggleLoanListing(player.id);
                          },
                        ),
                      );
                    }

                    if (isScoutingView) {
                      contextItems.push(buildDividerMenuItem());
                      contextItems.push(
                        buildScoutPlayerMenuItem(t, scoutState, () => {
                          void handleScoutPlayer(player.id);
                        }),
                      );
                      contextItems.push({
                        label: getDealEntryLabel(player),
                        icon: getDealEntryIcon(player, "w-4 h-4"),
                        onClick: () => openDealEntry(player),
                      });
                    }

                    const row = (
                      <tr
                        key={player.id}
                        className="hover:bg-gray-50 dark:hover:bg-navy-700/50 transition-colors cursor-pointer group"
                        onClick={() => onSelectPlayer(player.id)}
                      >
                        <td className="py-2.5 px-4">
                          <Badge
                            variant={positionBadgeVariant(
                              player.natural_position || player.position,
                            )}
                            size="sm"
                          >
                            {translatePositionAbbreviation(
                              t,
                              player.natural_position || player.position,
                            )}
                          </Badge>
                        </td>
                        <td className="py-2.5 px-4">
                          <div className="flex items-center gap-3">
                            <PlayerAvatar player={player} />
                            <div className="min-w-0">
                              <span className="block truncate font-semibold text-sm text-gray-800 dark:text-gray-200 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
                                {player.full_name}
                              </span>
                              <div className="text-xs text-gray-400 dark:text-gray-500 mt-0.5 flex items-center gap-1">
                                <CountryFlag
                                  code={player.nationality}
                                  locale={i18n.language}
                                  className="text-sm leading-none"
                                />
                                <span>
                                  {countryName(
                                    player.nationality,
                                    i18n.language,
                                  )}
                                </span>
                              </div>
                            </div>
                          </div>
                        </td>
                        <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                          {age}
                        </td>
                        <td className="py-2.5 px-4">
                          {player.team_id ? (
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                onSelectTeam(player.team_id!);
                              }}
                              className="text-sm text-gray-600 dark:text-gray-400 hover:text-primary-500 hover:underline transition-colors"
                            >
                              {getTeamName(gameState.teams, player.team_id)}
                            </button>
                          ) : (
                            <span className="text-sm text-gray-600 dark:text-gray-400">
                              {t("common.freeAgent")}
                            </span>
                          )}
                        </td>
                        <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 font-medium tabular-nums">
                          {formatVal(player.market_value)}
                        </td>
                        <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                          {formatAnnualAmount(
                            formatVal(player.wage),
                            annualSuffix,
                          )}
                        </td>
                        <td className="py-2.5 px-4">
                          <span
                            className={`font-heading font-bold text-base tabular-nums ${ovr >= 75 ? "text-primary-500" : ovr >= 55 ? "text-accent-500" : "text-gray-400"}`}
                          >
                            {ovr}
                          </span>
                        </td>
                        <td className="py-2.5 px-4">
                          <div className="flex gap-1">
                            {player.transfer_listed && (
                              <Badge variant="accent" size="sm">
                                {t("transfers.transfer")}
                              </Badge>
                            )}
                            {player.loan_listed && (
                              <Badge variant="primary" size="sm">
                                {t("transfers.loan")}
                              </Badge>
                            )}
                            {player.team_id === null && (
                              <Badge variant="neutral" size="sm">
                                {t("common.freeAgent")}
                              </Badge>
                            )}
                          </div>
                        </td>
                        {view === "offers" && (
                          <td className="py-2.5 px-4">
                            <div className="flex flex-col gap-1">
                              {!hasOffersForThisPlayer ? (
                                <span className="text-xs text-gray-400">
                                  {t("transfers.none")}
                                </span>
                              ) : (
                                <>
                                  {transferOffersForThisPlayer.map((offer) => (
                                    <div
                                      key={offer.id}
                                      className="flex items-center gap-2"
                                    >
                                      <span className="text-xs text-gray-600 dark:text-gray-300 font-medium">
                                        {getTeamName(
                                          gameState.teams,
                                          offer.from_team_id,
                                        )}
                                      </span>
                                      <Badge
                                        variant={getTransferOfferBadgeVariant(
                                          offer.status,
                                        )}
                                        size="sm"
                                      >
                                        {formatVal(offer.fee)} —{" "}
                                        {getTransferOfferStatusLabel(
                                          t,
                                          offer.status,
                                        )}
                                      </Badge>
                                      {offer.status === "Pending" &&
                                        player.team_id === userTeamId && (
                                          <div className="flex gap-1 ml-1">
                                            <button
                                              type="button"
                                              onClick={(e) => {
                                                e.stopPropagation();
                                                handleRespondOffer(
                                                  player.id,
                                                  offer.id,
                                                  true,
                                                );
                                              }}
                                              className="p-1 rounded bg-green-500/20 hover:bg-green-500/30 text-green-500"
                                              title={t("transfers.acceptOffer")}
                                            >
                                              <Check className="w-3 h-3" />
                                            </button>
                                            <button
                                              type="button"
                                              onClick={(e) => {
                                                e.stopPropagation();
                                                handleRespondOffer(
                                                  player.id,
                                                  offer.id,
                                                  false,
                                                );
                                              }}
                                              className="p-1 rounded bg-red-500/20 hover:bg-red-500/30 text-red-500"
                                              title={t("transfers.rejectOffer")}
                                            >
                                              <X className="w-3 h-3" />
                                            </button>
                                            <button
                                              type="button"
                                              onClick={(e) => {
                                                e.stopPropagation();
                                                openCounterNegotiation(
                                                  player,
                                                  offer,
                                                );
                                              }}
                                              aria-label={t(
                                                "transfers.counterOffer",
                                              )}
                                              className="flex items-center gap-1 px-2 py-1 rounded bg-amber-500/20 hover:bg-amber-500/30 text-amber-500 text-xs font-heading font-bold uppercase tracking-wider"
                                              title={t(
                                                "transfers.counterOffer",
                                              )}
                                            >
                                              <Gavel className="w-3 h-3" />{" "}
                                              {t("transfers.counter")}
                                            </button>
                                          </div>
                                        )}
                                    </div>
                                  ))}
                                  {loanOffersForThisPlayer.map((offer) => {
                                    const offerBuyOptionFee =
                                      offer.buy_option_fee ??
                                      player.active_loan?.buy_option_fee ??
                                      null;
                                    const canExerciseBuyOption =
                                      offer.status === "Accepted" &&
                                      offer.from_team_id === userTeamId &&
                                      player.active_loan?.loan_team_id ===
                                        userTeamId &&
                                      offerBuyOptionFee !== null &&
                                      offerBuyOptionFee > 0;

                                    return (
                                      <div
                                        key={`loan-${offer.id}`}
                                        className="flex items-center gap-2"
                                      >
                                        <span className="text-xs text-gray-600 dark:text-gray-300 font-medium">
                                          {getTeamName(
                                            gameState.teams,
                                            offer.from_team_id,
                                          )}
                                        </span>
                                        <Badge
                                          variant={getTransferOfferBadgeVariant(
                                            offer.status,
                                          )}
                                          size="sm"
                                        >
                                          {t("transfers.loanOfferTerms", {
                                            percent:
                                              offer.wage_contribution_pct,
                                            endDate: offer.end_date,
                                          })}
                                          {offerBuyOptionFee ? (
                                            <>
                                              {" "}
                                              •{" "}
                                              {t(
                                                "transfers.buyOptionFeeShort",
                                                {
                                                  fee: formatVal(
                                                    offerBuyOptionFee,
                                                  ),
                                                },
                                              )}
                                            </>
                                          ) : null}{" "}
                                          —{" "}
                                          {getTransferOfferStatusLabel(
                                            t,
                                            offer.status,
                                          )}
                                        </Badge>
                                        {offer.status === "Pending" &&
                                          player.team_id === userTeamId &&
                                          offer.from_team_id !== userTeamId && (
                                            <div className="flex gap-1 ml-1">
                                              <button
                                                type="button"
                                                onClick={(e) => {
                                                  e.stopPropagation();
                                                  handleRespondLoanOffer(
                                                    player.id,
                                                    offer.id,
                                                    true,
                                                  );
                                                }}
                                                className="p-1 rounded bg-green-500/20 hover:bg-green-500/30 text-green-500"
                                                title={t(
                                                  "transfers.acceptLoanOffer",
                                                )}
                                              >
                                                <Check className="w-3 h-3" />
                                              </button>
                                              <button
                                                type="button"
                                                onClick={(e) => {
                                                  e.stopPropagation();
                                                  handleRespondLoanOffer(
                                                    player.id,
                                                    offer.id,
                                                    false,
                                                  );
                                                }}
                                                className="p-1 rounded bg-red-500/20 hover:bg-red-500/30 text-red-500"
                                                title={t(
                                                  "transfers.rejectLoanOffer",
                                                )}
                                              >
                                                <X className="w-3 h-3" />
                                              </button>
                                              <button
                                                type="button"
                                                onClick={(e) => {
                                                  e.stopPropagation();
                                                  openLoanCounterOffer(
                                                    player,
                                                    offer,
                                                  );
                                                }}
                                                aria-label={t(
                                                  "transfers.counterLoanOffer",
                                                )}
                                                className="flex items-center gap-1 px-2 py-1 rounded bg-amber-500/20 hover:bg-amber-500/30 text-amber-500 text-xs font-heading font-bold uppercase tracking-wider"
                                                title={t(
                                                  "transfers.counterLoanOffer",
                                                )}
                                              >
                                                <Gavel className="w-3 h-3" />{" "}
                                                {t("transfers.counter")}
                                              </button>
                                            </div>
                                          )}
                                        {canExerciseBuyOption ? (
                                          <button
                                            type="button"
                                            onClick={(e) => {
                                              e.stopPropagation();
                                              void handleExerciseLoanBuyOption(
                                                player.id,
                                              );
                                            }}
                                            className="flex items-center gap-1 px-2 py-1 rounded bg-primary-500/10 hover:bg-primary-500/20 text-primary-500 text-xs font-heading font-bold uppercase tracking-wider"
                                            title={t(
                                              "transfers.exerciseBuyOption",
                                            )}
                                          >
                                            <ShoppingCart className="w-3 h-3" />{" "}
                                            {t("transfers.exerciseBuyOption")}
                                          </button>
                                        ) : null}
                                      </div>
                                    );
                                  })}
                                </>
                              )}
                            </div>
                          </td>
                        )}
                        {isScoutingView && (
                          <td className="py-2.5 px-4">
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                openDealEntry(player);
                              }}
                              className="flex items-center gap-1 px-3 py-1.5 bg-primary-500/10 hover:bg-primary-500/20 text-primary-500 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-colors"
                            >
                              {getDealEntryIcon(player, "w-3 h-3")}
                              {getDealEntryLabel(player)}
                            </button>
                          </td>
                        )}
                      </tr>
                    );

                    return (
                      <ContextMenu items={contextItems} key={player.id}>
                        {row}
                      </ContextMenu>
                    );
                  })}
                </tbody>
              </table>
            </div>
            {showMarketPagination ? (
              <div className="flex items-center justify-between border-t border-gray-100 px-4 py-3 dark:border-navy-600">
                <p className="text-xs font-heading text-gray-400 dark:text-gray-500">
                  {t("players.showingRange", {
                    from: marketRangeFrom,
                    to: marketRangeTo,
                    total: filteredList.length,
                  })}
                </p>
                <div className="flex items-center gap-1">
                  <button
                    type="button"
                    onClick={() =>
                      setMarketPage(Math.max(1, safeMarketPage - 1))
                    }
                    disabled={safeMarketPage === 1}
                    aria-label={t("scouting.previousPage")}
                    className="rounded-lg p-1.5 text-gray-400 transition-colors hover:bg-gray-100 hover:text-gray-700 disabled:pointer-events-none disabled:opacity-30 dark:hover:bg-navy-700 dark:hover:text-white"
                  >
                    <ChevronLeft className="h-4 w-4" />
                  </button>
                  <span className="px-3 py-1 text-xs font-heading font-bold text-gray-600 dark:text-gray-300">
                    {safeMarketPage} / {marketTotalPages}
                  </span>
                  <button
                    type="button"
                    onClick={() =>
                      setMarketPage(
                        Math.min(marketTotalPages, safeMarketPage + 1),
                      )
                    }
                    disabled={safeMarketPage === marketTotalPages}
                    aria-label={t("scouting.nextPage")}
                    className="rounded-lg p-1.5 text-gray-400 transition-colors hover:bg-gray-100 hover:text-gray-700 disabled:pointer-events-none disabled:opacity-30 dark:hover:bg-navy-700 dark:hover:text-white"
                  >
                    <ChevronRight className="h-4 w-4" />
                  </button>
                </div>
              </div>
            ) : null}
          </CardBody>
        </Card>
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
          onClose={() => {
            setCounterTarget(null);
            setCounterAmount("");
            setCounterError(null);
            setCounterResult(null);
            setCounterFeedback(null);
          }}
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
