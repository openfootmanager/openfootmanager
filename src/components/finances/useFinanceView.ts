import { useState } from "react";

import { useContractRiskLevels } from "../../hooks/useContractWarningStages";
import type { GameStateData, PlayerSelectionOptions, TeamData } from "../../store/gameStore";

import { formatExactMoney } from "../../lib/helpers";
import { weeklyWageAmount } from "../../lib/finance";
import type { FinanceSnapshotData } from "../../services/financeService";

import { useTranslation } from "react-i18next";
import { resolveMessage } from "../../utils/backendI18n";
import { DEFAULT_FACILITIES, isPendingSponsorOffer } from "./FinancesTab.helpers";

import { useFinanceActions } from "./useFinanceActions";

interface FinanceViewProps {
  gameState: GameStateData;
  myTeam: TeamData;
  financeData: FinanceSnapshotData;
  previewsCurrent: boolean;
  onGameUpdate?: (state: GameStateData) => void;
  onSelectPlayer?: (id: string, options?: PlayerSelectionOptions) => void;
}

/** Prepares presentation facts and exclusion-based renewal selection. */
export function useFinanceView({
  gameState,
  myTeam,
  financeData,
  previewsCurrent,
  onGameUpdate,
  onSelectPlayer,
}: FinanceViewProps) {
  const { t, i18n } = useTranslation();
  const {
    actionLoading,
    facilityUpgradeError,
    boardSupportFeedback,
    sponsorPitchFeedback,
    marketingCampaignFeedback,
    delegatedRenewalsSummary,
    handleUpgradeFacility,
    handleRequestBoardSupport,
    handleRequestSponsorPitch,
    handleRequestMarketingCampaign,
    handleSponsorOption,
    handleDelegateRenewals: delegateRenewals,
  } = useFinanceActions(onGameUpdate);
  // Every at-risk player starts selected; only explicit deselections are stored.
  const [deselectedRiskPlayerIds, setDeselectedRiskPlayerIds] = useState<string[]>([]);
  const roster = gameState.players.filter((p) => p.team_id === myTeam.id);
  const recoveryPreviews = previewsCurrent ? financeData.previews : null;
  const financeSnapshot = financeData.snapshot;
  const totalWages = financeSnapshot.weeklyWageSpend;
  const totalValue = roster.reduce((s, p) => s + p.market_value, 0);
  const facilities = myTeam.facilities ?? DEFAULT_FACILITIES;
  const activeSponsorship = myTeam.sponsorship ?? null;
  const weeklySponsorIncome = financeSnapshot.weeklySponsorIncome;
  const projectedWeeklyNet = financeSnapshot.projectedWeeklyNet;
  const cashRunwayWeeks = financeSnapshot.cashRunwayWeeks;
  const wageBudgetUsagePercent = financeSnapshot.wageBudgetUsagePercent;
  const weeklyWageBudget = financeSnapshot.weeklyWageBudget;
  const sponsorOffers = gameState.messages.filter(isPendingSponsorOffer).map(resolveMessage);
  const hasPendingSponsorOffer = sponsorOffers.length > 0;
  const hasActiveSponsor = Boolean(activeSponsorship && activeSponsorship.remaining_weeks > 0);
  // Availability is the backend's verdict alone. While the previews describe an older club state
  // (`recoveryPreviews` is null) nothing is offered, rather than a guess from the snapshot.
  const canRequestBoardSupport = Boolean(recoveryPreviews?.boardSupport);
  const sponsorPitchOffered = Boolean(recoveryPreviews?.sponsorPitch);
  const marketingCampaignOffered = Boolean(recoveryPreviews?.marketingCampaign);
  const canRequestSponsorPitch =
    sponsorPitchOffered && !hasPendingSponsorOffer && !hasActiveSponsor;
  const canRequestMarketingCampaign =
    marketingCampaignOffered && financeSnapshot.marketingCampaignCooldownDaysRemaining === 0;
  const sponsorPitchDisabledReason = hasActiveSponsor
    ? t("finances.sponsorPitchActiveSponsor")
    : hasPendingSponsorOffer
      ? t("finances.sponsorPitchPendingOffer")
      : previewsCurrent && !sponsorPitchOffered
        ? t("finances.sponsorPitchUnavailable")
        : null;
  const marketingCampaignDisabledReason =
    financeSnapshot.marketingCampaignCooldownDaysRemaining > 0
      ? t("finances.marketingCampaignCoolingDown", {
          days: financeSnapshot.marketingCampaignCooldownDaysRemaining,
        })
      : previewsCurrent && !marketingCampaignOffered
        ? t("finances.marketingCampaignUnavailable")
        : null;
  const boardSupportPreviewText = recoveryPreviews?.boardSupport
    ? t("finances.boardSupportSummary", {
        amount: formatExactMoney(recoveryPreviews.boardSupport.supportAmount),
        transferBudgetReduction: formatExactMoney(
          recoveryPreviews.boardSupport.transferBudgetReduction,
        ),
        satisfactionPenalty: recoveryPreviews.boardSupport.satisfactionPenalty,
      })
    : null;
  const sponsorPitchPreviewText = recoveryPreviews?.sponsorPitch
    ? t("finances.sponsorPitchSummary", {
        sponsor: recoveryPreviews.sponsorPitch.sponsorName,
        amount: formatExactMoney(recoveryPreviews.sponsorPitch.weeklyAmount),
        weeks: recoveryPreviews.sponsorPitch.durationWeeks,
      })
    : null;
  const marketingCampaignPreviewText = recoveryPreviews?.marketingCampaign
    ? t("finances.marketingCampaignSummary", {
        netIncome: formatExactMoney(recoveryPreviews.marketingCampaign.netIncome),
        grossRevenue: formatExactMoney(recoveryPreviews.marketingCampaign.grossRevenue),
        cost: formatExactMoney(recoveryPreviews.marketingCampaign.campaignCost),
        days: recoveryPreviews.marketingCampaign.cooldownDays,
      })
    : null;
  const contractRiskLevel = useContractRiskLevels(roster, gameState.clock.current_date);
  const contractRiskPlayers = roster
    .map((player) => {
      const riskLevel = contractRiskLevel(player.id);

      return {
        player,
        riskLevel,
      };
    })
    .filter(({ riskLevel, player }) => player.contract_end && riskLevel !== "stable")
    .sort((left, right) => {
      const leftDate = left.player.contract_end ?? "9999-12-31";
      const rightDate = right.player.contract_end ?? "9999-12-31";
      return leftDate.localeCompare(rightDate);
    });
  const atRiskWages = contractRiskPlayers.reduce(
    (sum, { player }) => sum + weeklyWageAmount(player.wage),
    0,
  );
  const selectedRiskPlayers = contractRiskPlayers.filter(
    ({ player }) => !deselectedRiskPlayerIds.includes(player.id),
  );
  const allRiskPlayerIds = contractRiskPlayers.map(({ player }) => player.id);

  function handleToggleRiskPlayer(playerId: string): void {
    setDeselectedRiskPlayerIds((currentIds) =>
      currentIds.includes(playerId)
        ? currentIds.filter((currentId) => currentId !== playerId)
        : [...currentIds, playerId],
    );
  }

  function handleToggleAllRiskPlayers(): void {
    setDeselectedRiskPlayerIds(
      selectedRiskPlayers.length === allRiskPlayerIds.length ? allRiskPlayerIds : [],
    );
  }

  const handleDelegateRenewals = () =>
    delegateRenewals(selectedRiskPlayers.map(({ player }) => player.id));
  return {
    myTeam,
    financeSnapshot,
    totalValue,
    t,
    totalWages,
    weeklyWageBudget,
    weeklySponsorIncome,
    projectedWeeklyNet,
    cashRunwayWeeks,
    boardSupportPreviewText,
    canRequestBoardSupport,
    actionLoading,
    handleRequestBoardSupport,
    boardSupportFeedback,
    previewsCurrent,
    wageBudgetUsagePercent,
    delegatedRenewalsSummary,
    atRiskWages,
    contractRiskPlayers,
    handleToggleAllRiskPlayers,
    handleDelegateRenewals,
    selectedRiskPlayers,
    deselectedRiskPlayerIds,
    handleToggleRiskPlayer,
    i18n,
    gameState,
    onSelectPlayer,
    activeSponsorship,
    sponsorPitchPreviewText,
    handleRequestSponsorPitch,
    canRequestSponsorPitch,
    sponsorPitchDisabledReason,
    sponsorPitchFeedback,
    marketingCampaignPreviewText,
    handleRequestMarketingCampaign,
    canRequestMarketingCampaign,
    marketingCampaignDisabledReason,
    marketingCampaignFeedback,
    sponsorOffers,
    handleSponsorOption,
    facilities,
    financeData,
    facilityUpgradeError,
    handleUpgradeFacility,
    roster,
  };
}

export type FinanceViewState = ReturnType<typeof useFinanceView>;
