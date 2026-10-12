import { useState } from "react";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import { makeLoanOffer, type LoanOfferResponseData } from "../../services/transfersService";
import {
  buildLoanPeriodOptions,
  formatTransferFeeInput,
  getDefaultLoanPeriodId,
  getLoanPeriodIdForEndDate,
  type LoanPeriodOptionId,
  parseTransferFeeInput,
} from "./TransfersTab.helpers";
import type { TFunction } from "i18next";
interface Input {
  loanRegistrationDate: string;
  onGameUpdate: ((game: GameStateData) => void) | undefined;
  t: TFunction<"translation", undefined>;
  transferWindowBlocksRegistration: boolean;
}
export function useLoanOfferFlow({
  loanRegistrationDate,
  onGameUpdate,
  t,
  transferWindowBlocksRegistration,
}: Input) {
  const [loanTarget, setLoanTarget] = useState<PlayerData | null>(null);
  const [loanPeriodId, setLoanPeriodId] = useState<LoanPeriodOptionId | "">(
    getDefaultLoanPeriodId(loanRegistrationDate, null),
  );
  const [loanWageContributionPct, setLoanWageContributionPct] = useState(100);
  const [loanBuyOptionEnabled, setLoanBuyOptionEnabled] = useState(false);
  const [loanBuyOptionFee, setLoanBuyOptionFee] = useState("");
  const [loanLoading, setLoanLoading] = useState(false);
  const [loanError, setLoanError] = useState<string | null>(null);
  const [loanResult, setLoanResult] = useState<LoanOfferResponseData["decision"] | "error" | null>(
    null,
  );
  const [loanSuggestedTerms, setLoanSuggestedTerms] = useState<{
    wageContributionPct: number;
    endDate: string;
    buyOptionFee?: number | null;
  } | null>(null);
  const openLoanOffer = (player: PlayerData) => {
    setLoanTarget(player);
    setLoanPeriodId(getDefaultLoanPeriodId(loanRegistrationDate, player.contract_end));
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
          wageContributionPct: response.suggested_wage_contribution_pct ?? loanWageContributionPct,
          endDate: response.suggested_end_date ?? selectedLoanPeriodOption.endDate,
          buyOptionFee: response.suggested_buy_option_fee,
        });
        if (response.suggested_wage_contribution_pct !== null) {
          setLoanWageContributionPct(
            Math.max(0, Math.min(100, Math.round(response.suggested_wage_contribution_pct))),
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
          setLoanBuyOptionFee(formatTransferFeeInput(response.suggested_buy_option_fee));
        } else {
          setLoanBuyOptionEnabled(false);
          setLoanBuyOptionFee("");
        }
      }
      if (onGameUpdate) onGameUpdate(response.game);
    } catch (err: unknown) {
      setLoanResult("error");
      setLoanError(resolveTranslatedErrorMessage(getErrorMessage(err), t));
    } finally {
      setLoanLoading(false);
    }
  };
  const parsedLoanBuyOptionFee = loanBuyOptionEnabled
    ? parseTransferFeeInput(loanBuyOptionFee)
    : null;
  const loanPeriodOptions = loanTarget
    ? buildLoanPeriodOptions(loanRegistrationDate, loanTarget.contract_end)
    : [];
  const selectedLoanPeriodOption =
    loanPeriodOptions.find((option) => option.id === loanPeriodId && !option.disabled) ?? null;
  const loanSubmitDisabled =
    loanLoading ||
    !selectedLoanPeriodOption ||
    loanResult === "accepted" ||
    transferWindowBlocksRegistration ||
    (loanBuyOptionEnabled && (parsedLoanBuyOptionFee === null || parsedLoanBuyOptionFee <= 0));
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
    openLoanOffer,
    closeLoanOffer,
    handleMakeLoanOffer,
    loanPeriodOptions,
    selectedLoanPeriodOption,
    loanSubmitDisabled,
  };
}
