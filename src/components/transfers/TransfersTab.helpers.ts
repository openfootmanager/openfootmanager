import type {
  LoanOfferData,
  PlayerData,
  TransferOfferData,
} from "../../store/gameStore";
import type { TransferNegotiationFeedbackData } from "../../services/transfersService";
import { formatExactMoney } from "../../lib/helpers";

type Translate = (
  key: string,
  options?: Record<string, string | number>,
) => string;

export type LoanPeriodPresetId =
  | "three_months"
  | "january_window"
  | "end_of_season"
  | "twelve_months";

export type LoanPeriodOptionId = LoanPeriodPresetId | "current_offer";

export interface LoanPeriodOption {
  id: LoanPeriodOptionId;
  labelKey: string;
  endDate: string;
  disabled: boolean;
  disabledReasonKey: string | null;
}

const MS_PER_DAY = 24 * 60 * 60 * 1000;
const MIN_LOAN_DAYS = 30;
const MAX_LOAN_DAYS = 370;

export function formatTransferFeeInput(fee: number | null | undefined): string {
  if (fee === null || fee === undefined || !Number.isFinite(fee)) {
    return "";
  }

  return String(Math.round(fee));
}

function parseUtcDate(value: string | null | undefined): Date | null {
  if (!value) {
    return null;
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return null;
  }

  return new Date(
    Date.UTC(
      parsed.getUTCFullYear(),
      parsed.getUTCMonth(),
      parsed.getUTCDate(),
    ),
  );
}

function addUtcDays(date: Date, days: number): Date {
  const next = new Date(date);
  next.setUTCDate(next.getUTCDate() + days);
  return next;
}

function formatUtcDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}

function daysBetween(start: Date, end: Date): number {
  return Math.round((end.getTime() - start.getTime()) / MS_PER_DAY);
}

function nextJanuaryWindow(currentDate: Date): Date {
  return new Date(Date.UTC(currentDate.getUTCFullYear() + 1, 0, 1));
}

function nextSeasonEnd(currentDate: Date): Date {
  const seasonEnd = new Date(Date.UTC(currentDate.getUTCFullYear(), 5, 30));
  return seasonEnd > currentDate
    ? seasonEnd
    : new Date(Date.UTC(currentDate.getUTCFullYear() + 1, 5, 30));
}

function buildLoanPeriodOption(
  id: LoanPeriodOptionId,
  labelKey: string,
  currentDate: Date,
  endDate: Date,
  contractEnd: Date | null,
): LoanPeriodOption {
  const loanDays = daysBetween(currentDate, endDate);
  const outsideLoanRules = loanDays < MIN_LOAN_DAYS || loanDays > MAX_LOAN_DAYS;
  const afterContractEnd =
    contractEnd !== null && endDate.getTime() >= contractEnd.getTime();

  return {
    id,
    labelKey,
    endDate: formatUtcDate(endDate),
    disabled: outsideLoanRules || afterContractEnd,
    disabledReasonKey: outsideLoanRules
      ? "transfers.loanPeriodUnavailableRules"
      : afterContractEnd
        ? "transfers.loanPeriodUnavailableContract"
        : null,
  };
}

export function buildLoanPeriodOptions(
  currentDateValue: string,
  contractEndValue: string | null | undefined,
  currentOfferEndDateValue?: string | null,
): LoanPeriodOption[] {
  const currentDate = parseUtcDate(currentDateValue);
  if (!currentDate) {
    return [];
  }

  const contractEnd = parseUtcDate(contractEndValue);
  const presets: { id: LoanPeriodPresetId; labelKey: string; date: Date }[] = [
    {
      id: "three_months",
      labelKey: "transfers.loanPeriodThreeMonths",
      date: addUtcDays(currentDate, 90),
    },
    {
      id: "january_window",
      labelKey: "transfers.loanPeriodJanuaryWindow",
      date: nextJanuaryWindow(currentDate),
    },
    {
      id: "end_of_season",
      labelKey: "transfers.loanPeriodEndOfSeason",
      date: nextSeasonEnd(currentDate),
    },
    {
      id: "twelve_months",
      labelKey: "transfers.loanPeriodTwelveMonths",
      date: addUtcDays(currentDate, 365),
    },
  ];

  const presetOptions = presets.map((preset) =>
    buildLoanPeriodOption(
      preset.id,
      preset.labelKey,
      currentDate,
      preset.date,
      contractEnd,
    ),
  );
  const currentOfferEndDate = parseUtcDate(currentOfferEndDateValue);
  if (!currentOfferEndDate) {
    return presetOptions;
  }

  const currentOfferEndDateText = formatUtcDate(currentOfferEndDate);
  if (presetOptions.some((option) => option.endDate === currentOfferEndDateText)) {
    return presetOptions;
  }

  return [
    buildLoanPeriodOption(
      "current_offer",
      "transfers.loanPeriodCurrentOffer",
      currentDate,
      currentOfferEndDate,
      contractEnd,
    ),
    ...presetOptions,
  ];
}

export function getDefaultLoanPeriodId(
  currentDateValue: string,
  contractEndValue: string | null | undefined,
  currentOfferEndDateValue?: string | null,
): LoanPeriodOptionId | "" {
  const loanStartDate = parseUtcDate(currentDateValue);
  const options = buildLoanPeriodOptions(
    currentDateValue,
    contractEndValue,
    currentOfferEndDateValue,
  );
  const currentOfferOption = options.find(
    (option) => option.id === "current_offer" && !option.disabled,
  );
  const priority: LoanPeriodPresetId[] =
    loanStartDate?.getUTCMonth() === 0
      ? ["end_of_season", "three_months", "twelve_months", "january_window"]
      : ["january_window", "end_of_season", "three_months", "twelve_months"];

  return (
    currentOfferOption?.id ??
    priority.find((id) =>
      options.some((option) => option.id === id && !option.disabled),
    ) ??
    options.find((option) => !option.disabled)?.id ??
    ""
  );
}

export function getLoanPeriodIdForEndDate(
  currentDateValue: string,
  contractEndValue: string | null | undefined,
  endDateValue: string | null | undefined,
): LoanPeriodOptionId | "" {
  const normalizedEndDate = parseUtcDate(endDateValue);
  const normalizedEndDateValue = normalizedEndDate
    ? formatUtcDate(normalizedEndDate)
    : endDateValue;
  const options = buildLoanPeriodOptions(
    currentDateValue,
    contractEndValue,
    normalizedEndDateValue,
  );
  const matchingOption = options.find(
    (option) => !option.disabled && option.endDate === normalizedEndDateValue,
  );

  return (
    matchingOption?.id ??
    getDefaultLoanPeriodId(currentDateValue, contractEndValue)
  );
}

export function parseTransferFeeInput(value: string): number | null {
  const digits = value.replace(/\D/g, "");

  if (!digits) {
    return null;
  }

  const parsed = Number.parseInt(digits, 10);
  return Number.isFinite(parsed) ? parsed : null;
}

export function normalizeTransferNegotiationFeedback(
  feedback: TransferNegotiationFeedbackData | null,
): TransferNegotiationFeedbackData | null {
  if (!feedback?.params?.fee) {
    return feedback;
  }

  const parsedFee = Number.parseInt(feedback.params.fee, 10);
  if (!Number.isFinite(parsedFee)) {
    return feedback;
  }

  return {
    ...feedback,
    params: {
      ...feedback.params,
      fee: formatExactMoney(parsedFee),
    },
  };
}

export function getOutgoingNegotiationOffer(
  player: PlayerData,
  userTeamId: string | null,
): TransferOfferData | null {
  if (!userTeamId) {
    return null;
  }

  return (
    player.transfer_offers.find(
      (offer) =>
        offer.from_team_id === userTeamId && offer.status === "Pending",
    ) ?? null
  );
}

export function buildResumedBidFeedback(
  offer: TransferOfferData | null,
): TransferNegotiationFeedbackData | null {
  if (!offer) {
    return null;
  }

  const round = Math.max(offer.negotiation_round || 1, 1);
  const tension = Math.min(36 + (round - 1) * 16, 84);
  const patience = Math.max(82 - (round - 1) * 16, 30);

  return normalizeTransferNegotiationFeedback({
    mood: round >= 3 ? "tense" : "firm",
    headline_key: "transfers.resumeNegotiationHeadline",
    detail_key: "transfers.resumeNegotiationDetail",
    tension,
    patience,
    round,
    params: {
      fee: String(offer.suggested_counter_fee ?? offer.fee),
    },
  });
}

export function buildResumedCounterFeedback(
  offer: TransferOfferData | null,
): TransferNegotiationFeedbackData | null {
  if (!offer) {
    return null;
  }

  const round = Math.max(offer.negotiation_round || 1, 1);
  const tension = Math.min(40 + (round - 1) * 14, 86);
  const patience = Math.max(78 - (round - 1) * 14, 28);

  return normalizeTransferNegotiationFeedback({
    mood: round >= 3 ? "tense" : "firm",
    headline_key: "transfers.resumeNegotiationHeadline",
    detail_key: "transfers.resumeNegotiationDetail",
    tension,
    patience,
    round,
    params: {
      fee: String(offer.suggested_counter_fee ?? offer.fee),
    },
  });
}

export function getTransferOfferStatusLabel(
  t: Translate,
  status: TransferOfferData["status"] | LoanOfferData["status"],
): string {
  switch (status) {
    case "Pending":
      return t("transfers.offerStatusPending");
    case "PendingRegistration":
      return t("transfers.offerStatusPendingRegistration");
    case "Accepted":
      return t("transfers.offerStatusAccepted");
    case "Rejected":
      return t("transfers.offerStatusRejected");
    case "Withdrawn":
      return t("transfers.offerStatusWithdrawn");
    default:
      return status;
  }
}

export function getTransferOfferBadgeVariant(
  status: TransferOfferData["status"] | LoanOfferData["status"],
) {
  switch (status) {
    case "Pending":
      return "accent" as const;
    case "PendingRegistration":
      return "accent" as const;
    case "Accepted":
      return "success" as const;
    case "Withdrawn":
      return "neutral" as const;
    case "Rejected":
    default:
      return "danger" as const;
  }
}

export function mapTransferNegotiationError(
  t: Translate,
  error: string,
): string {
  if (error.includes("Offer not found or not pending")) {
    return t("transfers.negotiationExpiredError");
  }

  return error;
}
