import { useTranslation } from "react-i18next";

import type { PlayerData, TeamData } from "../../store/gameStore";
import type { NegotiationFeedbackPanelData } from "../NegotiationFeedbackPanel";
import NegotiationFeedbackPanel from "../NegotiationFeedbackPanel";
import { Badge } from "../ui";
import { formatVal, getTeamName, positionBadgeVariant } from "../../lib/helpers";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";

const MAX_CONTRACT_YEARS = 5;

export interface PersonalTermsFormProps {
  player: PlayerData;
  teams: TeamData[];
  wage: string;
  onWageChange: (value: string) => void;
  contractYears: string;
  onContractYearsChange: (value: string) => void;
  round: number;
  suggestedWage: number | null;
  suggestedYears: number | null;
  feedback: NegotiationFeedbackPanelData | null | undefined;
  error: string | null;
  submitting: boolean;
  submitDisabled: boolean;
  terminal: boolean;
  onSubmit: () => void;
  onClose: () => void;
}

type PersonalTermsModalProps = PersonalTermsFormProps;

export function PersonalTermsForm({
  player,
  teams,
  wage,
  onWageChange,
  contractYears,
  onContractYearsChange,
  round,
  suggestedWage,
  suggestedYears,
  feedback,
  error,
  submitting,
  submitDisabled,
  terminal,
  onSubmit,
  onClose,
}: PersonalTermsFormProps) {
  const { t } = useTranslation();
  const titleId = `personal-terms-title-${player.id}`;

  return (
    <>
      <h3
        id={titleId}
        className="text-sm font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1"
      >
        {t("transfers.personalTermsModalTitle", { player: player.full_name })}
      </h3>
      <p className="text-xs text-gray-400 mb-3">
        {t("transfers.personalTermsModalIntro")}
      </p>

      <div className="flex items-center gap-3 mb-4">
        <Badge variant={positionBadgeVariant(player.position)} size="sm">
          {translatePositionAbbreviation(t, player.position)}
        </Badge>
        <div>
          <p className="font-semibold text-sm text-gray-800 dark:text-gray-200">
            {player.full_name}
          </p>
          <p className="text-xs text-gray-400">
            {player.team_id
              ? getTeamName(teams, player.team_id)
              : t("common.freeAgent")}{" "}
            •{" "}
            {t("transfers.playerValue", {
              value: formatVal(player.market_value),
            })}{" "}
            • {t("transfers.personalTermsRoundLabel", { round })}
          </p>
        </div>
      </div>

      <label
        htmlFor="personal-terms-wage"
        className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1 block"
      >
        {t("transfers.personalTermsWageLabel")}
      </label>
      <input
        id="personal-terms-wage"
        type="number"
        min="0"
        step="100"
        value={wage}
        disabled={terminal}
        onChange={(event) => onWageChange(event.target.value)}
        className="w-full px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 mb-3 focus:outline-none focus:ring-2 focus:ring-primary-500/50 disabled:opacity-50"
      />

      <label
        htmlFor="personal-terms-years"
        className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1 block"
      >
        {t("transfers.personalTermsLengthLabel")}
      </label>
      <input
        id="personal-terms-years"
        type="number"
        min="1"
        max={String(MAX_CONTRACT_YEARS)}
        step="1"
        value={contractYears}
        disabled={terminal}
        onChange={(event) => onContractYearsChange(event.target.value)}
        className="w-full px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 mb-3 focus:outline-none focus:ring-2 focus:ring-primary-500/50 disabled:opacity-50"
      />

      {suggestedWage !== null && !terminal ? (
        <p className="text-xs text-amber-500 mb-3">
          {t("transfers.personalTermsSuggestedHint", {
            wage: formatVal(suggestedWage),
            years: suggestedYears ?? contractYears,
          })}
        </p>
      ) : null}

      <NegotiationFeedbackPanel
        feedback={feedback ?? null}
        titleKey="transfers.negotiationPulse"
        roundKey="transfers.negotiationRound"
        patienceKey="transfers.negotiationPatience"
        tensionKey="transfers.negotiationTension"
        className="mb-3"
      />

      {error ? (
        <div className="text-xs font-heading font-bold uppercase tracking-wider mb-3 text-red-600 dark:text-red-300">
          {error}
        </div>
      ) : null}

      <div className="flex gap-2">
        {!terminal ? (
          <button
            onClick={onSubmit}
            disabled={submitDisabled}
            className="flex-1 py-2 bg-primary-700 hover:bg-primary-800 text-white rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-colors disabled:opacity-50"
          >
            {submitting
              ? t("transfers.submitting")
              : t("transfers.personalTermsSubmit")}
          </button>
        ) : null}
        <button
          onClick={onClose}
          className="px-4 py-2 bg-gray-200 dark:bg-navy-700 text-gray-600 dark:text-gray-300 rounded-lg font-heading font-bold text-sm uppercase tracking-wider hover:bg-gray-300 dark:hover:bg-navy-600 transition-colors flex-1"
        >
          {t("transfers.close")}
        </button>
      </div>
    </>
  );
}

export default function PersonalTermsModal(props: PersonalTermsModalProps) {
  const titleId = `personal-terms-title-${props.player.id}`;

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      onClick={props.onClose}
    >
      <div
        className="bg-white dark:bg-navy-800 rounded-xl shadow-2xl border border-gray-200 dark:border-navy-600 p-6 w-full max-w-sm"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onClick={(event) => event.stopPropagation()}
      >
        <PersonalTermsForm {...props} />
      </div>
    </div>
  );
}
