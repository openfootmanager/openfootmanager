import { useTranslation } from "react-i18next";
import type { LoanOfferData, PlayerData, TeamData, TransferOfferData } from "../../store/gameStore";
import { formatDate } from "../../lib/dateFormatting";
import { formatExactMoney, getTeamName } from "../../lib/helpers";
import { getRelevantTransferOffers, getRelevantLoanOffers } from "./TransfersTab.model";
import { Badge } from "../ui";
import { getTransferOfferBadgeVariant, getTransferOfferStatusLabel } from "./TransfersTab.helpers";

type SavedOffer =
  | { kind: "transfer"; offer: TransferOfferData }
  | { kind: "loan"; offer: LoanOfferData };

interface PlayerOfferHistoryProps {
  player: PlayerData;
  userTeamId: string | null;
  teams: TeamData[];
}

function getPlayerOffers(player: PlayerData, userTeamId: string | null): SavedOffer[] {
  if (!userTeamId) return [];
  const transfers: SavedOffer[] = getRelevantTransferOffers(player, userTeamId).map((offer) => ({
    kind: "transfer",
    offer,
  }));
  const loans: SavedOffer[] = getRelevantLoanOffers(player, userTeamId).map((offer) => ({
    kind: "loan",
    offer,
  }));
  // Offers retain their latest round, not an append-only ledger of every exchange.
  return [...transfers, ...loans].sort((a, b) => b.offer.date.localeCompare(a.offer.date));
}

function SavedOfferEntry({ entry, teams }: { entry: SavedOffer; teams: TeamData[] }) {
  const { t, i18n } = useTranslation();
  const { offer } = entry;
  return (
    <li className="rounded-lg border border-gray-200 bg-gray-50 p-3 text-sm text-gray-900 dark:border-navy-600 dark:bg-navy-900 dark:text-gray-100">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-heading font-bold uppercase tracking-wider">
          {entry.kind === "transfer" ? t("transfers.transfer") : t("transfers.loan")}
        </span>
        <span>{getTeamName(teams, offer.from_team_id, t("common.unknown"))}</span>
        <Badge variant={getTransferOfferBadgeVariant(offer.status)} size="sm">
          {getTransferOfferStatusLabel(t, offer.status)}
        </Badge>
        <span>{t("transfers.negotiationRound", { count: offer.negotiation_round ?? 1 })}</span>
      </div>
      <p className="mt-2 text-gray-600 dark:text-gray-300">
        {t("transfers.offerRecordedOn", { date: formatDate(offer.date, i18n.language) })}
      </p>
      {entry.kind === "transfer" ? (
        <SavedTransferTerms offer={entry.offer} />
      ) : (
        <SavedLoanTerms offer={entry.offer} />
      )}
    </li>
  );
}

function SavedTransferTerms({ offer }: { offer: TransferOfferData }) {
  const { t, i18n } = useTranslation();
  return (
    <>
      <dl className="mt-2 flex gap-2">
        <dt>{t("transferCentreWorld.fee")}</dt>
        <dd className="font-semibold tabular-nums">{formatExactMoney(offer.fee)}</dd>
      </dl>
      {offer.status === "PendingRegistration" && offer.registration_date ? (
        <p className="mt-2 font-semibold">
          {t("transfers.transferFeedbackScheduledDetail", {
            date: formatDate(offer.registration_date, i18n.language),
          })}
        </p>
      ) : null}
    </>
  );
}

function SavedLoanTerms({ offer }: { offer: LoanOfferData }) {
  const { t, i18n } = useTranslation();
  return (
    <div className="mt-2 space-y-1">
      <p>
        {t("transfers.loanOfferTerms", {
          percent: offer.wage_contribution_pct,
          endDate: formatDate(offer.end_date, i18n.language),
        })}
      </p>
      <p>
        {t("transfers.loanOfferStartsOn", { date: formatDate(offer.start_date, i18n.language) })}
      </p>
      {offer.buy_option_fee != null ? (
        <p>{t("transfers.buyOptionFeeShort", { fee: formatExactMoney(offer.buy_option_fee) })}</p>
      ) : null}
    </div>
  );
}

export default function PlayerOfferHistory({ player, userTeamId, teams }: PlayerOfferHistoryProps) {
  const { t } = useTranslation();
  const offers = getPlayerOffers(player, userTeamId);
  if (offers.length === 0) return null;
  return (
    <section aria-label={t("transfers.offerHistory")} className="mb-5 space-y-3">
      <h3 className="font-heading text-lg font-bold uppercase tracking-wider text-gray-900 dark:text-white">
        {t("transfers.offerHistory")}
      </h3>
      <ul className="space-y-3">
        {offers.map((entry) => (
          <SavedOfferEntry key={`${entry.kind}-${entry.offer.id}`} entry={entry} teams={teams} />
        ))}
      </ul>
    </section>
  );
}
