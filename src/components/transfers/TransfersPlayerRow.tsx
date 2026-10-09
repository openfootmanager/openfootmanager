import { History } from "lucide-react";
import type { MouseEventHandler } from "react";
import type { LoanOfferData, PlayerData, TransferOfferData } from "../../store/gameStore";
import { Badge, CountryFlag, PlayerAvatar } from "../ui";
import ContextMenu from "../ContextMenu";
import {
  getTeamName,
  calcAge,
  formatVal,
  formatWeeklyAmount,
  getPlayerOvr,
  positionBadgeVariant,
} from "../../lib/helpers";
import { countryName } from "../../lib/countries";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import {
  buildDividerMenuItem,
  buildScoutPlayerMenuItem,
  buildToggleLoanListMenuItem,
  buildToggleTransferListMenuItem,
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";
import type { TransfersTabController } from "./useTransfersTabController";
import { getRelevantTransferOffers, getRelevantLoanOffers } from "./TransfersTab.model";
import { PlayerOffersCell } from "./TransfersPlayerOffers";
type PlayerRowDetailsProps = {
  state: Pick<
    TransfersTabController,
    | "onSelectPlayer"
    | "t"
    | "i18n"
    | "onSelectTeam"
    | "gameState"
    | "weeklySuffix"
    | "view"
    | "userTeamId"
    | "handleRespondOffer"
    | "openCounterNegotiation"
    | "handleRespondLoanOffer"
    | "openLoanCounterOffer"
    | "handleExerciseLoanBuyOption"
    | "isScoutingView"
    | "openDealEntry"
    | "openDealHistory"
    | "getDealEntryIcon"
    | "getDealEntryLabel"
  >;
  player: PlayerData;
  onContextMenu?: MouseEventHandler<HTMLTableRowElement>;
  age: number;
  ovr: number;
  hasOffersForThisPlayer: boolean;
  transferOffersForThisPlayer: TransferOfferData[];
  loanOffersForThisPlayer: LoanOfferData[];
};
type TransferPlayerRowProps = {
  state: Pick<
    TransfersTabController,
    | "alreadyScoutingIds"
    | "scoutingPlayerId"
    | "availableScouts"
    | "t"
    | "onSelectPlayer"
    | "onSelectTeam"
    | "view"
    | "handleToggleTransferListing"
    | "handleToggleLoanListing"
    | "isScoutingView"
    | "handleScoutPlayer"
    | "getDealEntryLabel"
    | "getDealEntryIcon"
    | "openDealEntry"
    | "openDealHistory"
    | "i18n"
    | "gameState"
    | "weeklySuffix"
    | "userTeamId"
    | "handleRespondOffer"
    | "openCounterNegotiation"
    | "handleRespondLoanOffer"
    | "openLoanCounterOffer"
    | "handleExerciseLoanBuyOption"
  >;
  player: PlayerData;
};

export function PlayerRowDetails({
  state,
  onContextMenu,
  player,
  age,
  ovr,
  hasOffersForThisPlayer,
  transferOffersForThisPlayer,
  loanOffersForThisPlayer,
}: PlayerRowDetailsProps) {
  const {
    view,
    onSelectPlayer,
    t,
    i18n,
    onSelectTeam,
    gameState,
    weeklySuffix,
    isScoutingView,
    openDealEntry,
    openDealHistory,
    getDealEntryIcon,
    getDealEntryLabel,
  } = state;
  return (
    <tr
      onContextMenu={onContextMenu}
      key={player.id}
      className="hover:bg-gray-50 dark:hover:bg-navy-700/50 transition-colors cursor-pointer group"
      onClick={() => onSelectPlayer(player.id)}
    >
      <td className="py-2.5 px-4">
        <Badge variant={positionBadgeVariant(player.natural_position || player.position)} size="sm">
          {translatePositionAbbreviation(t, player.natural_position || player.position)}
        </Badge>
      </td>
      <td className="py-2.5 px-4">
        <div className="flex items-center gap-3">
          <PlayerAvatar player={player} />
          <div className="min-w-0">
            <span className="block truncate font-semibold text-sm text-gray-800 dark:text-gray-200 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
              {player.full_name}
            </span>
            <div className="text-xs text-gray-500 dark:text-gray-300 mt-0.5 flex items-center gap-1">
              <CountryFlag
                code={player.nationality}
                locale={i18n.language}
                className="text-sm leading-none"
              />
              <span>{countryName(player.nationality, i18n.language)}</span>
            </div>
          </div>
        </div>
      </td>
      <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 tabular-nums">{age}</td>
      <td className="py-2.5 px-4">
        {player.team_id ? (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onSelectTeam(player.team_id!);
            }}
            className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none text-sm text-gray-600 dark:text-gray-400 hover:text-primary-500 hover:underline transition-colors"
          >
            {getTeamName(gameState.teams, player.team_id)}
          </button>
        ) : (
          <span className="text-sm text-gray-600 dark:text-gray-400">{t("common.freeAgent")}</span>
        )}
      </td>
      <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 font-medium tabular-nums">
        {formatVal(player.market_value)}
      </td>
      <td className="py-2.5 px-4 text-sm text-gray-600 dark:text-gray-400 tabular-nums">
        {formatWeeklyAmount(formatVal(player.wage), weeklySuffix)}
      </td>
      <td className="py-2.5 px-4">
        <span
          className={`font-heading font-bold text-base tabular-nums ${ovr >= 75 ? "text-primary-700 dark:text-primary-400" : ovr >= 55 ? "text-accent-700 dark:text-accent-400" : "text-gray-500 dark:text-gray-300"}`}
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
      {
        <PlayerOffersCell
          state={state}
          hasOffersForThisPlayer={hasOffersForThisPlayer}
          transferOffersForThisPlayer={transferOffersForThisPlayer}
          player={player}
          loanOffersForThisPlayer={loanOffersForThisPlayer}
        />
      }
      {(isScoutingView || view === "offers") && (
        <td className="py-2.5 px-4">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              if (view === "offers") openDealHistory(player);
              else openDealEntry(player);
            }}
            className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none flex items-center gap-1 px-3 py-1.5 bg-primary-500/10 hover:bg-primary-500/20 text-primary-700 dark:text-primary-400 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-colors"
          >
            {view === "offers" ? (
              <History className="w-3 h-3" />
            ) : (
              getDealEntryIcon(player, "w-3 h-3")
            )}
            {view === "offers" ? t("transfers.viewOfferHistory") : getDealEntryLabel(player)}
          </button>
        </td>
      )}
    </tr>
  );
}

export function TransferPlayerRow({ state, player }: TransferPlayerRowProps) {
  const {
    userTeamId,
    alreadyScoutingIds,
    scoutingPlayerId,
    availableScouts,
    t,
    onSelectPlayer,
    onSelectTeam,
    view,
    handleToggleTransferListing,
    handleToggleLoanListing,
    isScoutingView,
    handleScoutPlayer,
    getDealEntryLabel,
    getDealEntryIcon,
    openDealEntry,
  } = state;

  const ovr = getPlayerOvr(player);
  const age = calcAge(player.date_of_birth);
  const transferOffersForThisPlayer = getRelevantTransferOffers(player, userTeamId);
  const loanOffersForThisPlayer = getRelevantLoanOffers(player, userTeamId);
  const hasOffersForThisPlayer =
    transferOffersForThisPlayer.length > 0 || loanOffersForThisPlayer.length > 0;
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
      buildToggleTransferListMenuItem(t, player.transfer_listed, () => {
        void handleToggleTransferListing(player.id);
      }),
    );
    contextItems.push(
      buildToggleLoanListMenuItem(t, player.loan_listed, () => {
        void handleToggleLoanListing(player.id);
      }),
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
    <PlayerRowDetails
      state={state}
      player={player}
      age={age}
      ovr={ovr}
      hasOffersForThisPlayer={hasOffersForThisPlayer}
      transferOffersForThisPlayer={transferOffersForThisPlayer}
      loanOffersForThisPlayer={loanOffersForThisPlayer}
    />
  );

  return (
    <ContextMenu items={contextItems} key={player.id}>
      {row}
    </ContextMenu>
  );
}
