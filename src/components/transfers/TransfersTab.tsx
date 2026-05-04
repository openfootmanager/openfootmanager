import { useState } from "react";
import {
  GameStateData,
  PlayerData,
  PlayerSelectionOptions,
  TransferOfferData,
} from "../../store/gameStore";
import { Card, CardBody, Badge, CountryFlag } from "../ui";
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
} from "lucide-react";
import {
  getTeamName,
  calcOvr,
  calcAge,
  formatVal,
  formatWeeklyAmount,
  positionBadgeVariant,
} from "../../lib/helpers";
import {
  annualAmountToWeeklyCommitment,
} from "../../lib/finance";
import { useTranslation } from "react-i18next";
import { countryName } from "../../lib/countries";
import {
  translatePositionAbbreviation,
} from "../squad/SquadTab.helpers";
import { resolveSeasonContext } from "../../lib/seasonContext";
import { type NegotiationFeedbackPanelData } from "../NegotiationFeedbackPanel";
import TransferBidModal from "./TransferBidModal";
import TransferCounterOfferModal from "./TransferCounterOfferModal";
import {
  counterOffer,
  respondToOffer,
  toggleLoanList,
  toggleTransferList,
  type TransferNegotiationResponseData,
} from "../../services/transfersService";
import { sendScout } from "../../services/scoutingService";
import {
  buildResumedCounterFeedback,
  getTransferOfferBadgeVariant,
  getTransferOfferStatusLabel,
  mapTransferNegotiationError,
} from "./TransfersTab.helpers";
import {
  deriveTransferCollections,
  filterTransferPlayers,
  getCurrentTransferList,
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
import { useTransferBidFlow } from "./useTransferBidFlow";

interface TransfersTabProps {
  gameState: GameStateData;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onSelectTeam: (id: string) => void;
  onGameUpdate?: (game: GameStateData) => void;
}

type CounterTarget = {
  player: PlayerData;
  offerId: string;
  fromTeamId: string;
  fee: number;
};

export default function TransfersTab({
  gameState,
  onSelectPlayer,
  onSelectTeam,
  onGameUpdate,
}: TransfersTabProps) {
  const { t, i18n } = useTranslation();
  const weeklySuffix = t("finances.perWeekSuffix", "/wk");
  const userTeamId = gameState.manager.team_id;
  const [view, setView] = useState<TransferTabView>("my_list");
  const [search, setSearch] = useState("");
  const [posFilter, setPosFilter] = useState<string | null>(null);
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
      ((offer.suggested_counter_fee ?? offer.fee) / 1_000_000).toFixed(
        offer.negotiation_round > 1 ? 2 : 1,
      ),
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

  const handleCounterOffer = async () => {
    if (!counterTarget || !counterAmount) return;

    setCounterLoading(true);
    setCounterError(null);
    setCounterResult(null);
    setCounterFeedback(null);

    try {
      const requestedFee = Math.round(parseFloat(counterAmount) * 1_000_000);
      const response = await counterOffer(
        counterTarget.player.id,
        counterTarget.offerId,
        requestedFee,
      );

      if (onGameUpdate) onGameUpdate(response.game);
      setCounterResult(response.decision);
      setCounterFeedback(response.feedback);
      if (response.suggested_fee !== null) {
        setCounterAmount((response.suggested_fee / 1_000_000).toFixed(2));
      }
      if (response.decision === "accepted") {
        setTimeout(() => {
          setCounterTarget(null);
          setCounterAmount("");
          setCounterResult(null);
          setCounterFeedback(null);
        }, 1500);
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
  const availableScouts = calculateAvailableScouts(scouts, scoutingAssignments);
  const alreadyScoutingIds = buildAlreadyScoutingIds(scoutingAssignments);
  const activeCounterOffer = counterTarget
    ? counterTarget.player.transfer_offers.find(
      (offer) => offer.id === counterTarget.offerId,
    ) ?? null
    : null;
  const seasonContext = resolveSeasonContext(gameState);
  const transferWindow = seasonContext.transfer_window;
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

  const transferCollections = deriveTransferCollections(gameState, userTeamId);
  const {
    myTransferList,
    myLoanList,
    marketPlayers,
    loanPlayers,
    playersWithOffers,
  } = transferCollections;

  const positions = ["Goalkeeper", "Defender", "Midfielder", "Forward"];

  const tabs: {
    id: TransferTabView;
    label: string;
    icon: React.ReactNode;
    count: number;
  }[] = [
      {
        id: "my_list",
        label: t("transfers.myTransferList"),
        icon: <ShoppingCart className="w-4 h-4" />,
        count: myTransferList.length + myLoanList.length,
      },
      {
        id: "market",
        label: t("transfers.transferMarket"),
        icon: <TrendingUp className="w-4 h-4" />,
        count: marketPlayers.length,
      },
      {
        id: "loans",
        label: t("transfers.loanMarket"),
        icon: <ArrowRightLeft className="w-4 h-4" />,
        count: loanPlayers.length,
      },
      {
        id: "offers",
        label: t("transfers.offers"),
        icon: <Handshake className="w-4 h-4" />,
        count: playersWithOffers.length,
      },
    ];

  const currentList = getCurrentTransferList(view, transferCollections);
  const filteredList = filterTransferPlayers(currentList, search, posFilter);
  const weeklyWageBudget = myTeam
    ? annualAmountToWeeklyCommitment(myTeam.wage_budget)
    : 0;

  const handleScoutPlayer = async (playerId: string): Promise<void> => {
    if (availableScouts.length === 0) {
      return;
    }

    const scout = availableScouts[0];
    setScoutingPlayerId(playerId);

    try {
      const updated = await sendScout(scout.id, playerId);
      onGameUpdate?.(updated);
    } catch (error) {
      console.error("Failed to send scout:", error);
    } finally {
      setScoutingPlayerId(null);
    }
  };

  return (
    <div className="max-w-6xl mx-auto">
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
              <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  {t("finances.wageBudget")}
                </p>
                <p className="font-heading font-bold text-lg text-white">
                  {formatWeeklyAmount(
                    formatVal(weeklyWageBudget),
                    weeklySuffix,
                  )}
                </p>
              </div>
              <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  {t("transfers.listed")}
                </p>
                <p className="font-heading font-bold text-lg text-white">
                  {myTransferList.length + myLoanList.length}
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
            key={tab.id}
            onClick={() => setView(tab.id)}
            className={`px-4 py-2 rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-all flex items-center gap-1.5 ${view === tab.id
              ? "bg-primary-500 text-white shadow-md shadow-primary-500/20"
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
            onChange={(e) => setSearch(e.target.value)}
            className="w-full pl-9 pr-3 py-2 rounded-lg bg-white dark:bg-navy-800 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-primary-500/50"
          />
        </div>
        <div className="flex gap-1.5">
          <button
            onClick={() => setPosFilter(null)}
            className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${!posFilter ? "bg-primary-500 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
          >
            {t("common.all")}
          </button>
          {positions.map((pos) => (
            <button
              key={pos}
              onClick={() => setPosFilter(posFilter === pos ? null : pos)}
              className={`px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${posFilter === pos ? "bg-primary-500 text-white shadow-sm" : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600"}`}
            >
              {t(`common.posAbbr.${pos}`)}
            </button>
          ))}
        </div>
        <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
          <Filter className="w-3.5 h-3.5 inline mr-1 -mt-0.5" />
          {t("common.nResults", { count: filteredList.length })}
        </p>
      </div>

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
                    {(view === "market" || view === "loans") && (
                      <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.action")}
                      </th>
                    )}
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                  {filteredList.map((player) => {
                    const ovr = calcOvr(
                      player,
                      player.natural_position || player.position,
                    );
                    const age = calcAge(player.date_of_birth);
                    const offersForThisPlayer = player.transfer_offers;
                    const scoutState = alreadyScoutingIds.has(player.id)
                      ? "already-assigned"
                      : scoutingPlayerId === player.id
                        ? "busy"
                        : availableScouts.length === 0
                          ? "unavailable"
                          : "ready";
                    const contextItems = [
                      buildViewProfileMenuItem(t, () => onSelectPlayer(player.id)),
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
                          async () => {
                            try {
                              const updated = await toggleTransferList(player.id);
                              onGameUpdate?.(updated);
                            } catch {
                              return;
                            }
                          },
                        ),
                      );
                      contextItems.push(
                        buildToggleLoanListMenuItem(t, player.loan_listed, async () => {
                          try {
                            const updated = await toggleLoanList(player.id);
                            onGameUpdate?.(updated);
                          } catch {
                            return;
                          }
                        }),
                      );
                    }

                    if (view === "market" || view === "loans") {
                      contextItems.push(buildDividerMenuItem());
                      contextItems.push(
                        buildScoutPlayerMenuItem(t, scoutState, () => {
                          void handleScoutPlayer(player.id);
                        }),
                      );
                      contextItems.push({
                        label: t("transfers.bid"),
                        icon: <Gavel className="w-4 h-4" />,
                        onClick: () => openBidNegotiation(player),
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
                          <span className="font-semibold text-sm text-gray-800 dark:text-gray-200 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
                            {player.full_name}
                          </span>
                          <div className="text-xs text-gray-400 dark:text-gray-500 mt-0.5 flex items-center gap-1">
                            <CountryFlag
                              code={player.nationality}
                              locale={i18n.language}
                              className="text-sm leading-none"
                            />
                            <span>
                              {countryName(player.nationality, i18n.language)}
                            </span>
                          </div>
                        </td>
                        <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                          {age}
                        </td>
                        <td className="py-2.5 px-4">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              if (player.team_id) onSelectTeam(player.team_id);
                            }}
                            className="text-sm text-gray-600 dark:text-gray-400 hover:text-primary-500 hover:underline transition-colors"
                          >
                            {getTeamName(gameState.teams, player.team_id)}
                          </button>
                        </td>
                        <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 font-medium tabular-nums">
                          {formatVal(player.market_value)}
                        </td>
                        <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                          {formatVal(player.wage)}/yr
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
                          </div>
                        </td>
                        {view === "offers" && (
                          <td className="py-2.5 px-4">
                            <div className="flex flex-col gap-1">
                              {offersForThisPlayer.length === 0 ? (
                                <span className="text-xs text-gray-400">
                                  {t("transfers.none")}
                                </span>
                              ) : (
                                offersForThisPlayer.map((offer) => (
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
                                      {formatVal(offer.fee)} — {getTransferOfferStatusLabel(t, offer.status)}
                                    </Badge>
                                    {offer.status === "Pending" &&
                                      player.team_id === userTeamId && (
                                        <div className="flex gap-1 ml-1">
                                          <button
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
                                            onClick={(e) => {
                                              e.stopPropagation();
                                              openCounterNegotiation(player, offer);
                                            }}
                                            aria-label={t("transfers.counterOffer")}
                                            className="flex items-center gap-1 px-2 py-1 rounded bg-amber-500/20 hover:bg-amber-500/30 text-amber-500 text-xs font-heading font-bold uppercase tracking-wider"
                                            title={t("transfers.counterOffer")}
                                          >
                                            <Gavel className="w-3 h-3" />{" "}
                                            {t("transfers.counter")}
                                          </button>
                                        </div>
                                      )}
                                  </div>
                                ))
                              )}
                            </div>
                          </td>
                        )}
                        {(view === "market" || view === "loans") && (
                          <td className="py-2.5 px-4">
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                openBidNegotiation(player);
                              }}
                              className="flex items-center gap-1 px-3 py-1.5 bg-primary-500/10 hover:bg-primary-500/20 text-primary-500 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-colors"
                            >
                              <Gavel className="w-3 h-3" /> {t("transfers.bid")}
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
          </CardBody>
        </Card>
      )}

      {(view === "market" || view === "loans") && filteredList.length === 0 && (
        <Card>
          <CardBody>
            <div className="text-center py-8">
              <TrendingUp className="w-10 h-10 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {view === "market"
                  ? t("transfers.noTransferMarket")
                  : t("transfers.noLoanMarket")}
              </p>
            </div>
          </CardBody>
        </Card>
      )}
      {/* Bid Modal */}
      {bidTarget && (
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
          bidSubmitDisabled={bidSubmitDisabled}
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
    </div>
  );
}
