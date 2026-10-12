import { useState } from "react";
import type { GameStateData, LoanOfferData, PlayerData } from "../../store/gameStore";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import { counterLoanOffer, type LoanOfferResponseData } from "../../services/transfersService";
import {
  buildLoanPeriodOptions,
  formatTransferFeeInput,
  getDefaultLoanPeriodId,
  getLoanPeriodIdForEndDate,
  type LoanPeriodOptionId,
  parseTransferFeeInput,
} from "./TransfersTab.helpers";
import type { TFunction } from "i18next";
import type { LoanCounterTarget } from "./TransfersTab.types";
interface Input {
  loanRegistrationDate: string;
  onGameUpdate: ((game: GameStateData) => void) | undefined;
  t: TFunction<"translation", undefined>;
  transferWindowBlocksRegistration: boolean;
}
export function useLoanCounterFlow({
  loanRegistrationDate,
  onGameUpdate,
  t,
  transferWindowBlocksRegistration,
}: Input) {
  const [loanCounterTarget, setLoanCounterTarget] = useState<LoanCounterTarget | null>(null);
  const [loanCounterPeriodId, setLoanCounterPeriodId] = useState<LoanPeriodOptionId | "">(
    getDefaultLoanPeriodId(loanRegistrationDate, null),
  );
  const [loanCounterWageContributionPct, setLoanCounterWageContributionPct] = useState(100);
  const [loanCounterBuyOptionEnabled, setLoanCounterBuyOptionEnabled] = useState(false);
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
    const buyOptionFee = offer.suggested_buy_option_fee ?? offer.buy_option_fee ?? null;
    setLoanCounterBuyOptionEnabled(Boolean(buyOptionFee));
    setLoanCounterBuyOptionFee(buyOptionFee ? formatTransferFeeInput(buyOptionFee) : "");
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
        loanCounterBuyOptionEnabled ? parseTransferFeeInput(loanCounterBuyOptionFee) : null,
      );
      setLoanCounterResult(response.decision);
      if (response.decision === "counter_offer") {
        setLoanCounterSuggestedTerms({
          wageContributionPct:
            response.suggested_wage_contribution_pct ?? loanCounterWageContributionPct,
          endDate: response.suggested_end_date ?? selectedLoanCounterPeriodOption.endDate,
          buyOptionFee: response.suggested_buy_option_fee,
        });
        if (response.suggested_wage_contribution_pct !== null) {
          setLoanCounterWageContributionPct(response.suggested_wage_contribution_pct);
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
          setLoanCounterBuyOptionFee(formatTransferFeeInput(response.suggested_buy_option_fee));
        }
      }
      if (onGameUpdate) onGameUpdate(response.game);
    } catch (err: unknown) {
      setLoanCounterResult("error");
      setLoanCounterError(resolveTranslatedErrorMessage(getErrorMessage(err), t));
    } finally {
      setLoanCounterLoading(false);
    }
  };
  const parsedLoanCounterBuyOptionFee = loanCounterBuyOptionEnabled
    ? parseTransferFeeInput(loanCounterBuyOptionFee)
    : null;
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
      (parsedLoanCounterBuyOptionFee === null || parsedLoanCounterBuyOptionFee <= 0));
  return {
    loanCounterTarget,
    loanCounterPeriodId,
    setLoanCounterPeriodId,
    loanCounterWageContributionPct,
    setLoanCounterWageContributionPct,
    loanCounterBuyOptionEnabled,
    setLoanCounterBuyOptionEnabled,
    loanCounterBuyOptionFee,
    setLoanCounterBuyOptionFee,
    loanCounterLoading,
    loanCounterError,
    loanCounterResult,
    loanCounterSuggestedTerms,
    openLoanCounterOffer,
    closeLoanCounterOffer,
    handleCounterLoanOffer,
    loanCounterPeriodOptions,
    selectedLoanCounterPeriodOption,
    loanCounterSubmitDisabled,
  };
}
