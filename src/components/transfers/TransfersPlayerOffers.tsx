import type { LoanOfferData, PlayerData, TransferOfferData } from "../../store/gameStore";
import { Badge } from "../ui";
import { ShoppingCart, Gavel, Check, X } from "lucide-react";
import { getTeamName, formatVal } from "../../lib/helpers";
import { getTransferOfferBadgeVariant, getTransferOfferStatusLabel } from "./TransfersTab.helpers";
import type { TransfersTabController } from "./useTransfersTabController";
type TransferOfferRecordProps = {
  state: Pick<
    TransfersTabController,
    "gameState" | "t" | "userTeamId" | "handleRespondOffer" | "openCounterNegotiation"
  >;
  offer: TransferOfferData;
  player: PlayerData;
};
type LoanOfferRecordProps = {
  state: Pick<
    TransfersTabController,
    | "userTeamId"
    | "gameState"
    | "t"
    | "handleRespondLoanOffer"
    | "openLoanCounterOffer"
    | "handleExerciseLoanBuyOption"
  >;
  offer: LoanOfferData;
  player: PlayerData;
};
type PlayerOffersCellProps = {
  state: Pick<
    TransfersTabController,
    | "view"
    | "t"
    | "gameState"
    | "userTeamId"
    | "handleRespondOffer"
    | "openCounterNegotiation"
    | "handleRespondLoanOffer"
    | "openLoanCounterOffer"
    | "handleExerciseLoanBuyOption"
  >;
  hasOffersForThisPlayer: boolean;
  transferOffersForThisPlayer: TransferOfferData[];
  player: PlayerData;
  loanOffersForThisPlayer: LoanOfferData[];
};

export function TransferOfferRecord({ state, offer, player }: TransferOfferRecordProps) {
  const { gameState, t, userTeamId, handleRespondOffer, openCounterNegotiation } = state;
  return (
    <div key={offer.id} className="flex items-center gap-2">
      <span className="text-xs text-gray-600 dark:text-gray-300 font-medium">
        {getTeamName(gameState.teams, offer.from_team_id)}
      </span>
      <Badge variant={getTransferOfferBadgeVariant(offer.status)} size="sm">
        {formatVal(offer.fee)} — {getTransferOfferStatusLabel(t, offer.status)}
      </Badge>
      {offer.status === "Pending" && player.team_id === userTeamId && (
        <div className="flex gap-1 ml-1">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleRespondOffer(player.id, offer.id, true);
            }}
            className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none p-1 rounded bg-success-500/20 hover:bg-success-500/30 text-primary-700 dark:text-success-400"
            title={t("transfers.acceptOffer")}
          >
            <Check className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleRespondOffer(player.id, offer.id, false);
            }}
            className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none p-1 rounded bg-red-500/20 hover:bg-red-500/30 text-red-500"
            title={t("transfers.rejectOffer")}
          >
            <X className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              openCounterNegotiation(player, offer);
            }}
            aria-label={t("transfers.counterOffer")}
            className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none flex items-center gap-1 px-2 py-1 rounded bg-accent-500/20 hover:bg-accent-500/30 text-accent-700 dark:text-accent-400 text-xs font-heading font-bold uppercase tracking-wider"
            title={t("transfers.counterOffer")}
          >
            <Gavel className="w-3 h-3" /> {t("transfers.counter")}
          </button>
        </div>
      )}
    </div>
  );
}

export function LoanOfferRecord({ state, offer, player }: LoanOfferRecordProps) {
  const {
    userTeamId,
    gameState,
    t,
    handleRespondLoanOffer,
    openLoanCounterOffer,
    handleExerciseLoanBuyOption,
  } = state;

  const offerBuyOptionFee = offer.buy_option_fee ?? player.active_loan?.buy_option_fee ?? null;
  const canExerciseBuyOption =
    offer.status === "Accepted" &&
    offer.from_team_id === userTeamId &&
    player.active_loan?.loan_team_id === userTeamId &&
    offerBuyOptionFee !== null &&
    offerBuyOptionFee > 0;

  return (
    <div key={`loan-${offer.id}`} className="flex items-center gap-2">
      <span className="text-xs text-gray-600 dark:text-gray-300 font-medium">
        {getTeamName(gameState.teams, offer.from_team_id)}
      </span>
      <Badge variant={getTransferOfferBadgeVariant(offer.status)} size="sm">
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
        — {getTransferOfferStatusLabel(t, offer.status)}
      </Badge>
      {offer.status === "Pending" &&
        player.team_id === userTeamId &&
        offer.from_team_id !== userTeamId && (
          <div className="flex gap-1 ml-1">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                handleRespondLoanOffer(player.id, offer.id, true);
              }}
              className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none p-1 rounded bg-success-500/20 hover:bg-success-500/30 text-primary-700 dark:text-success-400"
              title={t("transfers.acceptLoanOffer")}
            >
              <Check className="w-3 h-3" />
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                handleRespondLoanOffer(player.id, offer.id, false);
              }}
              className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none p-1 rounded bg-red-500/20 hover:bg-red-500/30 text-red-500"
              title={t("transfers.rejectLoanOffer")}
            >
              <X className="w-3 h-3" />
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                openLoanCounterOffer(player, offer);
              }}
              aria-label={t("transfers.counterLoanOffer")}
              className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none flex items-center gap-1 px-2 py-1 rounded bg-accent-500/20 hover:bg-accent-500/30 text-accent-700 dark:text-accent-400 text-xs font-heading font-bold uppercase tracking-wider"
              title={t("transfers.counterLoanOffer")}
            >
              <Gavel className="w-3 h-3" /> {t("transfers.counter")}
            </button>
          </div>
        )}
      {canExerciseBuyOption ? (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            void handleExerciseLoanBuyOption(player.id);
          }}
          className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-600 dark:focus-visible:ring-primary-400 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-navy-800 motion-reduce:transition-none flex items-center gap-1 px-2 py-1 rounded bg-primary-500/10 hover:bg-primary-500/20 text-primary-700 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider"
          title={t("transfers.exerciseBuyOption")}
        >
          <ShoppingCart className="w-3 h-3" /> {t("transfers.exerciseBuyOption")}
        </button>
      ) : null}
    </div>
  );
}

export function PlayerOffersCell({
  state,
  hasOffersForThisPlayer,
  transferOffersForThisPlayer,
  player,
  loanOffersForThisPlayer,
}: PlayerOffersCellProps) {
  const { view, t } = state;
  return (
    view === "offers" && (
      <td className="py-2.5 px-4">
        <div className="flex flex-col gap-1">
          {!hasOffersForThisPlayer ? (
            <span className="text-xs text-gray-500 dark:text-gray-300">{t("transfers.none")}</span>
          ) : (
            <>
              {transferOffersForThisPlayer.map((offer) => (
                <TransferOfferRecord key={offer.id} state={state} offer={offer} player={player} />
              ))}
              {loanOffersForThisPlayer.map((offer) => (
                <LoanOfferRecord key={offer.id} state={state} offer={offer} player={player} />
              ))}
            </>
          )}
        </div>
      </td>
    )
  );
}
