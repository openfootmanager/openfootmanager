import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  ShoppingCart,
  Gavel,
  Check,
  X,
  ChevronLeft,
  ChevronRight,
} from "lucide-react";

import {
  GameStateData,
  LoanOfferData,
  PlayerData,
  TransferOfferData,
} from "../../store/gameStore";
import { Card, CardBody, Badge, CountryFlag, PlayerAvatar } from "../ui";
import ContextMenu from "../ContextMenu";
import {
  getTeamName,
  calcAge,
  formatVal,
  formatAnnualAmount,
  getPlayerOvr,
  positionBadgeVariant,
} from "../../lib/helpers";
import { countryName } from "../../lib/countries";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import {
  getTransferOfferBadgeVariant,
  getTransferOfferStatusLabel,
} from "./TransfersTab.helpers";
import type { TransferTabView } from "./TransfersTab.model";
import {
  buildDividerMenuItem,
  buildScoutPlayerMenuItem,
  buildToggleLoanListMenuItem,
  buildToggleTransferListMenuItem,
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";

interface TransferPlayerTableProps {
  gameState: GameStateData;
  userTeamId: string | null;
  view: TransferTabView;
  isScoutingView: boolean;
  visibleList: PlayerData[];
  annualSuffix: string;
  alreadyScoutingIds: Set<string>;
  scoutingPlayerId: string | null;
  availableScoutsCount: number;
  onSelectPlayer: (id: string) => void;
  onSelectTeam: (id: string) => void;
  onToggleTransferListing: (playerId: string) => void;
  onToggleLoanListing: (playerId: string) => void;
  onScoutPlayer: (playerId: string) => void;
  onOpenDealEntry: (player: PlayerData) => void;
  getDealEntryLabel: (player: PlayerData) => string;
  getDealEntryIcon: (player: PlayerData, className: string) => ReactNode;
  onRespondOffer: (playerId: string, offerId: string, accept: boolean) => void;
  onRespondLoanOffer: (
    playerId: string,
    offerId: string,
    accept: boolean,
  ) => void;
  onOpenCounterNegotiation: (
    player: PlayerData,
    offer: TransferOfferData,
  ) => void;
  onOpenLoanCounterOffer: (player: PlayerData, offer: LoanOfferData) => void;
  onExerciseLoanBuyOption: (playerId: string) => void;
  showMarketPagination: boolean;
  marketRangeFrom: number;
  marketRangeTo: number;
  totalCount: number;
  safeMarketPage: number;
  marketTotalPages: number;
  onPageChange: (page: number) => void;
}

export default function TransferPlayerTable({
  gameState,
  userTeamId,
  view,
  isScoutingView,
  visibleList,
  annualSuffix,
  alreadyScoutingIds,
  scoutingPlayerId,
  availableScoutsCount,
  onSelectPlayer,
  onSelectTeam,
  onToggleTransferListing,
  onToggleLoanListing,
  onScoutPlayer,
  onOpenDealEntry,
  getDealEntryLabel,
  getDealEntryIcon,
  onRespondOffer,
  onRespondLoanOffer,
  onOpenCounterNegotiation,
  onOpenLoanCounterOffer,
  onExerciseLoanBuyOption,
  showMarketPagination,
  marketRangeFrom,
  marketRangeTo,
  totalCount,
  safeMarketPage,
  marketTotalPages,
  onPageChange,
}: TransferPlayerTableProps) {
  const { t, i18n } = useTranslation();

  return (
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
                    : availableScoutsCount === 0
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
                      () => {
                        void onToggleTransferListing(player.id);
                      },
                    ),
                  );
                  contextItems.push(
                    buildToggleLoanListMenuItem(t, player.loan_listed, () => {
                      void onToggleLoanListing(player.id);
                    }),
                  );
                }

                if (isScoutingView) {
                  contextItems.push(buildDividerMenuItem());
                  contextItems.push(
                    buildScoutPlayerMenuItem(t, scoutState, () => {
                      void onScoutPlayer(player.id);
                    }),
                  );
                  contextItems.push({
                    label: getDealEntryLabel(player),
                    icon: getDealEntryIcon(player, "w-4 h-4"),
                    onClick: () => onOpenDealEntry(player),
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
                              {countryName(player.nationality, i18n.language)}
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
                      {formatAnnualAmount(formatVal(player.wage), annualSuffix)}
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
                                            onRespondOffer(
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
                                            onRespondOffer(
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
                                            onOpenCounterNegotiation(
                                              player,
                                              offer,
                                            );
                                          }}
                                          aria-label={t(
                                            "transfers.counterOffer",
                                          )}
                                          className="flex items-center gap-1 px-2 py-1 rounded bg-amber-500/20 hover:bg-amber-500/30 text-amber-500 text-xs font-heading font-bold uppercase tracking-wider"
                                          title={t("transfers.counterOffer")}
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
                                        percent: offer.wage_contribution_pct,
                                        endDate: offer.end_date,
                                      })}
                                      {offerBuyOptionFee ? (
                                        <>
                                          {" "}
                                          •{" "}
                                          {t("transfers.buyOptionFeeShort", {
                                            fee: formatVal(offerBuyOptionFee),
                                          })}
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
                                              onRespondLoanOffer(
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
                                              onRespondLoanOffer(
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
                                              onOpenLoanCounterOffer(
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
                                          void onExerciseLoanBuyOption(
                                            player.id,
                                          );
                                        }}
                                        className="flex items-center gap-1 px-2 py-1 rounded bg-primary-500/10 hover:bg-primary-500/20 text-primary-500 text-xs font-heading font-bold uppercase tracking-wider"
                                        title={t("transfers.exerciseBuyOption")}
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
                            onOpenDealEntry(player);
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
                total: totalCount,
              })}
            </p>
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => onPageChange(Math.max(1, safeMarketPage - 1))}
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
                  onPageChange(Math.min(marketTotalPages, safeMarketPage + 1))
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
  );
}
