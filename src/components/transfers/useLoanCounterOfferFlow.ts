import { useState } from "react";
import { useTranslation } from "react-i18next";

import type {
  GameStateData,
  LoanOfferData,
  PlayerData,
} from "../../store/gameStore";
import {
  counterLoanOffer,
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

interface LoanCounterTarget {
  player: PlayerData;
  offer: LoanOfferData;
}

interface LoanCounterSuggestedTerms {
  wageContributionPct: number;
  endDate: string;
  buyOptionFee?: number | null;
}

interface UseLoanCounterOfferFlowArgs {
  loanRegistrationDate: string;
  transferWindowBlocksRegistration: boolean;
  onGameUpdate?: (game: GameStateData) => void;
}

interface UseLoanCounterOfferFlowResult {
  loanCounterTarget: LoanCounterTarget | null;
  loanCounterPeriodId: LoanPeriodOptionId | "";
  setLoanCounterPeriodId: (value: LoanPeriodOptionId | "") => void;
  loanCounterWageContributionPct: number;
  setLoanCounterWageContributionPct: (value: number) => void;
  loanCounterBuyOptionEnabled: boolean;
  setLoanCounterBuyOptionEnabled: (value: boolean) => void;
  loanCounterBuyOptionFee: string;
  setLoanCounterBuyOptionFee: (value: string) => void;
  loanCounterLoading: boolean;
  loanCounterError: string | null;
  loanCounterResult: LoanOfferResponseData["decision"] | "error" | null;
  loanCounterSuggestedTerms: LoanCounterSuggestedTerms | null;
  loanCounterPeriodOptions: LoanPeriodOption[];
  selectedLoanCounterPeriodOption: LoanPeriodOption | null;
  loanCounterSubmitDisabled: boolean;
  openLoanCounterOffer: (player: PlayerData, offer: LoanOfferData) => void;
  closeLoanCounterOffer: () => void;
  handleCounterLoanOffer: () => Promise<void>;
}

export function useLoanCounterOfferFlow({
  loanRegistrationDate,
  transferWindowBlocksRegistration,
  onGameUpdate,
}: UseLoanCounterOfferFlowArgs): UseLoanCounterOfferFlowResult {
  const { t } = useTranslation();
  const { scheduleAutoClose } = useAutoCloseTimeout();
  const [loanCounterTarget, setLoanCounterTarget] =
    useState<LoanCounterTarget | null>(null);
  const [loanCounterPeriodId, setLoanCounterPeriodId] = useState<
    LoanPeriodOptionId | ""
  >(getDefaultLoanPeriodId(loanRegistrationDate, null));
  const [loanCounterWageContributionPct, setLoanCounterWageContributionPct] =
    useState(100);
  const [loanCounterBuyOptionEnabled, setLoanCounterBuyOptionEnabled] =
    useState(false);
  const [loanCounterBuyOptionFee, setLoanCounterBuyOptionFee] = useState("");
  const [loanCounterLoading, setLoanCounterLoading] = useState(false);
  const [loanCounterError, setLoanCounterError] = useState<string | null>(null);
  const [loanCounterResult, setLoanCounterResult] = useState<
    LoanOfferResponseData["decision"] | "error" | null
  >(null);
  const [loanCounterSuggestedTerms, setLoanCounterSuggestedTerms] =
    useState<LoanCounterSuggestedTerms | null>(null);

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
      (parsedLoanCounterBuyOptionFee === null ||
        parsedLoanCounterBuyOptionFee <= 0));

  const openLoanCounterOffer = (
    player: PlayerData,
    offer: LoanOfferData,
  ): void => {
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
    const buyOptionFee =
      offer.suggested_buy_option_fee ?? offer.buy_option_fee ?? null;
    setLoanCounterBuyOptionEnabled(Boolean(buyOptionFee));
    setLoanCounterBuyOptionFee(
      buyOptionFee ? formatTransferFeeInput(buyOptionFee) : "",
    );
    setLoanCounterError(null);
    setLoanCounterResult(null);
    setLoanCounterSuggestedTerms(null);
  };

  const closeLoanCounterOffer = (): void => {
    setLoanCounterTarget(null);
    setLoanCounterPeriodId(getDefaultLoanPeriodId(loanRegistrationDate, null));
    setLoanCounterWageContributionPct(100);
    setLoanCounterBuyOptionEnabled(false);
    setLoanCounterBuyOptionFee("");
    setLoanCounterError(null);
    setLoanCounterResult(null);
    setLoanCounterSuggestedTerms(null);
  };

  const handleCounterLoanOffer = async (): Promise<void> => {
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
        clampWageContributionPct(loanCounterWageContributionPct),
        loanCounterBuyOptionEnabled
          ? parseTransferFeeInput(loanCounterBuyOptionFee)
          : null,
      );
      setLoanCounterResult(response.decision);
      if (response.decision === "counter_offer") {
        setLoanCounterSuggestedTerms({
          wageContributionPct:
            response.suggested_wage_contribution_pct ??
            loanCounterWageContributionPct,
          endDate:
            response.suggested_end_date ??
            selectedLoanCounterPeriodOption.endDate,
          buyOptionFee: response.suggested_buy_option_fee,
        });
        if (response.suggested_wage_contribution_pct !== null) {
          setLoanCounterWageContributionPct(
            response.suggested_wage_contribution_pct,
          );
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
          setLoanCounterBuyOptionFee(
            formatTransferFeeInput(response.suggested_buy_option_fee),
          );
        }
      }
      if (onGameUpdate) onGameUpdate(response.game);

      if (response.decision === "accepted") {
        scheduleAutoClose(() => {
          closeLoanCounterOffer();
        }, 1500);
      }
    } catch (err: any) {
      setLoanCounterResult("error");
      setLoanCounterError(resolveTranslatedErrorMessage(getErrorMessage(err), t));
    } finally {
      setLoanCounterLoading(false);
    }
  };

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
    loanCounterPeriodOptions,
    selectedLoanCounterPeriodOption,
    loanCounterSubmitDisabled,
    openLoanCounterOffer,
    closeLoanCounterOffer,
    handleCounterLoanOffer,
  };
}
