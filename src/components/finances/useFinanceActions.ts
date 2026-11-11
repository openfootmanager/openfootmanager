import { useRef, useState } from "react";

import type { GameStateData } from "../../store/gameStore";

import { formatExactMoney } from "../../lib/helpers";

import { invokeForCareer } from "../../services/careerInvoke";
import { useTranslation } from "react-i18next";
import { resolveBackendError } from "../../utils/backendI18n";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import type {
  FacilityId,
  FacilityUpgradeErrorState,
  ResolveMessageActionResult,
  DelegatedRenewalResponseData,
  BoardSupportResponseData,
  SponsorPitchResponseData,
  MarketingCampaignResponseData,
} from "./FinancesTab.helpers";

/** Coordinates finance mutations with one shared in-flight action key. */
export function useFinanceActions(onGameUpdate?: (state: GameStateData) => void) {
  const { t } = useTranslation();
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  // The sponsor offer being answered, for the whole request: its own state, because `actionLoading`
  // is one slot every finance action takes over, and an answer in flight must not look finished when
  // another action starts. The ref guards a second press before the state has rendered.
  const [answeringOfferId, setAnsweringOfferId] = useState<string | null>(null);
  const answeringOfferRef = useRef<string | null>(null);
  // Why the backend refused the last answer to a sponsor offer, and which offer's it was; null once
  // an answer is accepted.
  const [sponsorOfferError, setSponsorOfferError] = useState<{
    messageId: string;
    text: string;
  } | null>(null);
  const [delegatedRenewalsSummary, setDelegatedRenewalsSummary] = useState<string | null>(null);
  const [facilityUpgradeError, setFacilityUpgradeError] =
    useState<FacilityUpgradeErrorState | null>(null);
  const [boardSupportFeedback, setBoardSupportFeedback] = useState<{
    tone: "success" | "error";
    text: string;
  } | null>(null);
  const [sponsorPitchFeedback, setSponsorPitchFeedback] = useState<{
    tone: "success" | "error";
    text: string;
  } | null>(null);
  const [marketingCampaignFeedback, setMarketingCampaignFeedback] = useState<{
    tone: "success" | "error";
    text: string;
  } | null>(null);

  async function handleUpgradeFacility(facility: FacilityId): Promise<void> {
    setFacilityUpgradeError(null);
    setActionLoading(facility);
    try {
      const updated = await invokeForCareer<GameStateData>("upgrade_facility", {
        facility,
      });
      onGameUpdate?.(updated);
    } catch (error) {
      console.error("Failed to upgrade facility:", error);
      setFacilityUpgradeError({
        facilityId: facility,
        message: resolveBackendError(error),
      });
    } finally {
      setActionLoading(null);
    }
  }

  async function handleRequestBoardSupport(): Promise<void> {
    const loadingKey = "board-support";
    setBoardSupportFeedback(null);
    setActionLoading(loadingKey);

    try {
      const response = await invokeForCareer<BoardSupportResponseData>("request_board_support");
      onGameUpdate?.(response.game);
      setBoardSupportFeedback({
        tone: "success",
        text: t("finances.boardSupportSummary", {
          amount: formatExactMoney(response.result.support_amount),
          transferBudgetReduction: formatExactMoney(response.result.transfer_budget_reduction),
          satisfactionPenalty: response.result.satisfaction_penalty,
        }),
      });
    } catch (error) {
      console.error("Failed to request board support:", error);
      setBoardSupportFeedback({
        tone: "error",
        text: resolveBackendError(error),
      });
    } finally {
      setActionLoading(null);
    }
  }

  async function handleRequestSponsorPitch(): Promise<void> {
    const loadingKey = "sponsor-pitch";
    setSponsorPitchFeedback(null);
    setActionLoading(loadingKey);

    try {
      const response = await invokeForCareer<SponsorPitchResponseData>("request_sponsor_pitch");
      onGameUpdate?.(response.game);
      setSponsorPitchFeedback({
        tone: "success",
        text: t("finances.sponsorPitchSummary", {
          sponsor: response.result.sponsor_name,
          amount: formatExactMoney(response.result.weekly_amount),
          weeks: response.result.duration_weeks,
        }),
      });
    } catch (error) {
      console.error("Failed to pitch sponsors:", error);
      setSponsorPitchFeedback({
        tone: "error",
        text: resolveBackendError(error),
      });
    } finally {
      setActionLoading(null);
    }
  }

  async function handleRequestMarketingCampaign(): Promise<void> {
    const loadingKey = "marketing-campaign";
    setMarketingCampaignFeedback(null);
    setActionLoading(loadingKey);

    try {
      const response = await invokeForCareer<MarketingCampaignResponseData>(
        "request_marketing_campaign",
      );
      onGameUpdate?.(response.game);
      setMarketingCampaignFeedback({
        tone: "success",
        text: t("finances.marketingCampaignSummary", {
          netIncome: formatExactMoney(response.result.net_income),
          grossRevenue: formatExactMoney(response.result.gross_revenue),
          cost: formatExactMoney(response.result.campaign_cost),
          days: response.result.cooldown_days,
        }),
      });
    } catch (error) {
      console.error("Failed to launch marketing campaign:", error);
      setMarketingCampaignFeedback({
        tone: "error",
        text: resolveBackendError(error),
      });
    } finally {
      setActionLoading(null);
    }
  }

  async function handleDelegateRenewals(playerIds: string[]): Promise<void> {
    if (playerIds.length === 0) {
      return;
    }

    const loadingKey = "delegate-renewals";
    setActionLoading(loadingKey);
    setDelegatedRenewalsSummary(null);

    try {
      const result = await invokeForCareer<DelegatedRenewalResponseData>("delegate_renewals", {
        playerIds,
        maxWageIncreasePct: 35,
        maxContractYears: 3,
      });
      onGameUpdate?.(result.game);
      setDelegatedRenewalsSummary(
        t("finances.delegatedRenewalsSummary", {
          successes: result.report.success_count,
          stalled: result.report.stalled_count,
          failures: result.report.failure_count,
        }),
      );
    } catch (error) {
      console.error("Failed to delegate renewals:", error);
    } finally {
      setActionLoading(null);
    }
  }

  async function handleSponsorOption(
    messageId: string,
    actionId: string,
    optionId: string,
  ): Promise<void> {
    if (answeringOfferRef.current !== null) return;
    answeringOfferRef.current = messageId;
    setAnsweringOfferId(messageId);
    setSponsorOfferError(null);
    try {
      const result = await invokeForCareer<ResolveMessageActionResult>("resolve_message_action", {
        messageId,
        actionId,
        optionId,
      });
      onGameUpdate?.(result.game);
    } catch (error) {
      console.error("Failed to resolve sponsor offer:", error);
      setSponsorOfferError({ messageId, text: resolveTranslatedErrorMessage(error) });
    } finally {
      answeringOfferRef.current = null;
      setAnsweringOfferId(null);
    }
  }
  return {
    actionLoading,
    answeringOfferId,
    sponsorOfferError,
    facilityUpgradeError,
    boardSupportFeedback,
    sponsorPitchFeedback,
    marketingCampaignFeedback,
    delegatedRenewalsSummary,
    handleUpgradeFacility,
    handleRequestBoardSupport,
    handleRequestSponsorPitch,
    handleRequestMarketingCampaign,
    handleDelegateRenewals,
    handleSponsorOption,
  };
}
