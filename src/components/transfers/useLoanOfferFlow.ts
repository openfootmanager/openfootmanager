import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { GameStateData, PlayerData } from "../../store/gameStore";
import {
  makeLoanOffer,
  type LoanOfferResponseData,
} from "../../services/transfersService";
import {
  getErrorMessage,
  resolveTranslatedErrorMessage,
} from "../../utils/errorMessage";
import { useAutoCloseTimeout } from "./useAutoCloseTimeout";
import {
  buildLoanPeriodOptions,
  clampWageContributionPct,
  formatTransferFeeInput,
  getDefaultLoanPeriodId,
  getLoanPeriodIdForEndDate,
  type LoanPeriodOption,
  type LoanPeriodOptionId,
  parseTransferFeeInput,
} from "./TransfersTab.helpers";

interface LoanSuggestedTerms {
  wageContributionPct: number;
  endDate: string;
  buyOptionFee?: number | null;
}

interface UseLoanOfferFlowArgs {
  loanRegistrationDate: string;
  transferWindowBlocksRegistration: boolean;
  onGameUpdate?: (game: GameStateData) => void;
  onAccepted?: (playerId: string) => void;
}

interface UseLoanOfferFlowResult {
  loanTarget: PlayerData | null;
  loanPeriodId: LoanPeriodOptionId | "";
  setLoanPeriodId: (value: LoanPeriodOptionId | "") => void;
  loanWageContributionPct: number;
  setLoanWageContributionPct: (value: number) => void;
  loanBuyOptionEnabled: boolean;
  setLoanBuyOptionEnabled: (value: boolean) => void;
  loanBuyOptionFee: string;
  setLoanBuyOptionFee: (value: string) => void;
  loanLoading: boolean;
  loanError: string | null;
  loanResult: LoanOfferResponseData["decision"] | "error" | null;
  loanSuggestedTerms: LoanSuggestedTerms | null;
  loanPeriodOptions: LoanPeriodOption[];
  selectedLoanPeriodOption: LoanPeriodOption | null;
  loanSubmitDisabled: boolean;
  openLoanOffer: (player: PlayerData) => void;
  closeLoanOffer: () => void;
  handleMakeLoanOffer: () => Promise<void>;
}

export function useLoanOfferFlow({
  loanRegistrationDate,
  transferWindowBlocksRegistration,
  onGameUpdate,
  onAccepted,
}: UseLoanOfferFlowArgs): UseLoanOfferFlowResult {
  const { t } = useTranslation();
  const { scheduleAutoClose } = useAutoCloseTimeout();
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
  const [loanSuggestedTerms, setLoanSuggestedTerms] =
    useState<LoanSuggestedTerms | null>(null);

  const parsedLoanBuyOptionFee = loanBuyOptionEnabled
    ? parseTransferFeeInput(loanBuyOptionFee)
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

  const openLoanOffer = (player: PlayerData): void => {
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

  const closeLoanOffer = (): void => {
    setLoanTarget(null);
    setLoanPeriodId(getDefaultLoanPeriodId(loanRegistrationDate, null));
    setLoanWageContributionPct(100);
    setLoanBuyOptionEnabled(false);
    setLoanBuyOptionFee("");
    setLoanError(null);
    setLoanResult(null);
    setLoanSuggestedTerms(null);
  };

  const handleMakeLoanOffer = async (): Promise<void> => {
    if (!loanTarget || !selectedLoanPeriodOption) return;

    setLoanLoading(true);
    setLoanError(null);
    setLoanResult(null);
    setLoanSuggestedTerms(null);

    try {
      const response = await makeLoanOffer(
        loanTarget.id,
        selectedLoanPeriodOption.endDate,
        clampWageContributionPct(loanWageContributionPct),
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
            clampWageContributionPct(response.suggested_wage_contribution_pct),
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

      if (response.decision === "accepted") {
        const acceptedPlayerId = loanTarget.id;
        scheduleAutoClose(() => {
          closeLoanOffer();
          onAccepted?.(acceptedPlayerId);
        }, 1500);
      }
    } catch (err: any) {
      setLoanResult("error");
      setLoanError(resolveTranslatedErrorMessage(getErrorMessage(err), t));
    } finally {
      setLoanLoading(false);
    }
  };

  return {
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
  };
}
