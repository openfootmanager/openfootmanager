import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FinanceCashFlowChart } from "./FinanceCashFlowChart";
import { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import { Card, CardHeader, CardBody, Badge, ProgressBar, Button, Checkbox } from "../ui";
import {
  formatExactMoney,
  formatVal,
  formatWeeklyAmount,
  getContractRiskBadgeVariant,
  getContractRiskLevel,
  getContractYearsRemaining,
} from "../../lib/helpers";
import {
  annualAmountToWeeklyCommitment,
  getTeamFinanceSnapshot,
} from "../../lib/finance";
import { getFinanceSnapshot } from "../../services/financeService";
import { useTranslation } from "react-i18next";
import { resolveBackendError, resolveMessage } from "../../utils/backendI18n";
import {
  type FacilityId,
  type FacilityUpgradeErrorState,
  type TaggedFinanceSnapshotData,
  type ResolveMessageActionResult,
  type DelegatedRenewalResponseData,
  type BoardSupportResponseData,
  type SponsorPitchResponseData,
  type MarketingCampaignResponseData,
  DEFAULT_FACILITIES,
  formatSignedAmount,
  boardSupportAvailable,
  sponsorPitchAvailable,
  marketingCampaignAvailable,
  mapLocalFinanceSnapshot,
  isChooseOptionAction,
  isPendingSponsorOffer,
} from "./FinancesTab.helpers";
import FinancesFacilitiesCard from "./FinancesFacilitiesCard";
import FinancesPayrollTable from "./FinancesPayrollTable";

interface FinancesTabProps {
  gameState: GameStateData;
  onGameUpdate?: (state: GameStateData) => void;
  onSelectPlayer?: (id: string, options?: PlayerSelectionOptions) => void;
}

export default function FinancesTab({
  gameState,
  onGameUpdate,
  onSelectPlayer,
}: FinancesTabProps) {
  const { t } = useTranslation();
  const myTeam = gameState.teams.find(
    (tm) => tm.id === gameState.manager.team_id,
  );
  if (!myTeam)
    return (
      <p className="text-gray-500 dark:text-gray-400">{t("common.noTeam")}</p>
    );
  const weeklySuffix = t("finances.perWeekSuffix");
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [delegatedRenewalsSummary, setDelegatedRenewalsSummary] = useState<
    string | null
  >(null);
  const [selectedRiskPlayerIds, setSelectedRiskPlayerIds] = useState<string[]>(
    [],
  );
  const [remoteFinanceData, setRemoteFinanceData] =
    useState<TaggedFinanceSnapshotData | null>(null);
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

  const roster = gameState.players.filter((p) => p.team_id === myTeam.id);
  const financePlayers = gameState.players.filter(
    (player) =>
      player.team_id === myTeam.id ||
      player.active_loan?.parent_team_id === myTeam.id ||
      player.active_loan?.loan_team_id === myTeam.id,
  );
  const teamStaff = gameState.staff.filter(
    (staffMember) => staffMember.team_id === myTeam.id,
  );
  const financeSnapshotKey = [
    myTeam.id,
    gameState.clock.current_date,
    myTeam.finance,
    myTeam.wage_budget,
    myTeam.transfer_budget,
    myTeam.season_income,
    myTeam.season_expenses,
    myTeam.facilities?.training ?? DEFAULT_FACILITIES.training,
    myTeam.facilities?.medical ?? DEFAULT_FACILITIES.medical,
    myTeam.facilities?.scouting ?? DEFAULT_FACILITIES.scouting,
    myTeam.sponsorship?.sponsor_name ?? "",
    myTeam.sponsorship?.base_value ?? 0,
    myTeam.sponsorship?.remaining_weeks ?? 0,
    financePlayers
      .map(
        (player) =>
          [
            player.id,
            player.team_id ?? "",
            player.wage,
            player.contract_end ?? "",
            player.active_loan?.parent_team_id ?? "",
            player.active_loan?.loan_team_id ?? "",
            player.active_loan?.wage_contribution_pct ?? "",
            player.active_loan?.end_date ?? "",
          ].join(":"),
      )
      .join("|"),
    teamStaff
      .map((staffMember) => `${staffMember.id}:${staffMember.wage}`)
      .join("|"),
    gameState.messages
      .filter(isPendingSponsorOffer)
      .map((message) => message.id)
      .join("|"),
  ].join("::");
  const isRemoteFinanceDataCurrent = remoteFinanceData?.key === financeSnapshotKey;
  const localFinanceSnapshot = mapLocalFinanceSnapshot(
    myTeam,
    getTeamFinanceSnapshot(
      myTeam,
      financePlayers,
      teamStaff,
      gameState.clock.current_date,
    ),
  );
  const financeSnapshot = isRemoteFinanceDataCurrent
    ? remoteFinanceData.data.snapshot
    : localFinanceSnapshot;
  const recoveryPreviews = isRemoteFinanceDataCurrent
    ? remoteFinanceData.data.previews
    : null;
  const totalWages = financeSnapshot.weeklyWageSpend;
  const totalValue = roster.reduce((s, p) => s + p.market_value, 0);
  const facilities = myTeam.facilities ?? DEFAULT_FACILITIES;
  const activeSponsorship = myTeam.sponsorship ?? null;
  const weeklySponsorIncome = financeSnapshot.weeklySponsorIncome;
  const projectedWeeklyNet = financeSnapshot.projectedWeeklyNet;
  const cashRunwayWeeks = financeSnapshot.cashRunwayWeeks;
  const wageBudgetUsagePercent = financeSnapshot.wageBudgetUsagePercent;
  const weeklyWageBudget = financeSnapshot.weeklyWageBudget;
  const sponsorOffers = gameState.messages
    .filter(isPendingSponsorOffer)
    .map(resolveMessage);
  const hasPendingSponsorOffer = sponsorOffers.length > 0;
  const hasActiveSponsor = Boolean(
    activeSponsorship && activeSponsorship.remaining_weeks > 0,
  );
  const previewsLoaded = recoveryPreviews !== null;
  const previewBoardSupportAvailable = recoveryPreviews
    ? Boolean(recoveryPreviews.boardSupport)
    : null;
  const previewSponsorPitchAvailable = recoveryPreviews
    ? Boolean(recoveryPreviews.sponsorPitch)
    : null;
  const previewMarketingCampaignAvailable = recoveryPreviews
    ? Boolean(recoveryPreviews.marketingCampaign)
    : null;
  const canRequestBoardSupport = previewsLoaded
    ? previewBoardSupportAvailable ?? false
    : boardSupportAvailable(financeSnapshot);
  const canRequestSponsorPitch =
    (previewsLoaded
      ? previewSponsorPitchAvailable ?? false
      : sponsorPitchAvailable(financeSnapshot)) &&
    !hasPendingSponsorOffer &&
    !hasActiveSponsor;
  const canRequestMarketingCampaign =
    (previewsLoaded
      ? previewMarketingCampaignAvailable ?? false
      : marketingCampaignAvailable(financeSnapshot)) &&
    financeSnapshot.marketingCampaignCooldownDaysRemaining === 0;
  const sponsorPitchDisabledReason = hasActiveSponsor
    ? t("finances.sponsorPitchActiveSponsor")
    : hasPendingSponsorOffer
      ? t("finances.sponsorPitchPendingOffer")
      : !(previewsLoaded
        ? previewSponsorPitchAvailable ?? false
        : sponsorPitchAvailable(financeSnapshot))
        ? t("finances.sponsorPitchUnavailable")
        : null;
  const marketingCampaignDisabledReason =
    financeSnapshot.marketingCampaignCooldownDaysRemaining > 0
      ? t("finances.marketingCampaignCoolingDown", {
        days: financeSnapshot.marketingCampaignCooldownDaysRemaining,
      })
      : !(previewsLoaded
        ? previewMarketingCampaignAvailable ?? false
        : marketingCampaignAvailable(financeSnapshot))
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
      grossRevenue: formatExactMoney(
        recoveryPreviews.marketingCampaign.grossRevenue,
      ),
      cost: formatExactMoney(recoveryPreviews.marketingCampaign.campaignCost),
      campaignCost: formatExactMoney(
        recoveryPreviews.marketingCampaign.campaignCost,
      ),
      days: recoveryPreviews.marketingCampaign.cooldownDays,
    })
    : null;
  const contractRiskPlayers = roster
    .map((player) => {
      const riskLevel = getContractRiskLevel(
        player.contract_end,
        gameState.clock.current_date,
      );

      return {
        player,
        riskLevel,
      };
    })
    .filter(
      ({ riskLevel, player }) => player.contract_end && riskLevel !== "stable",
    )
    .sort((left, right) => {
      const leftDate = left.player.contract_end ?? "9999-12-31";
      const rightDate = right.player.contract_end ?? "9999-12-31";
      return leftDate.localeCompare(rightDate);
    });
  const atRiskWages = contractRiskPlayers.reduce(
    (sum, { player }) => sum + annualAmountToWeeklyCommitment(player.wage),
    0,
  );
  const selectedRiskPlayers = contractRiskPlayers.filter(({ player }) =>
    selectedRiskPlayerIds.includes(player.id),
  );
  const allRiskPlayerIds = contractRiskPlayers.map(({ player }) => player.id);

  useEffect(() => {
    let cancelled = false;
    const requestKey = financeSnapshotKey;

    setRemoteFinanceData(null);

    void getFinanceSnapshot(myTeam.id)
      .then((financeData) => {
        if (!cancelled) {
          setRemoteFinanceData({ key: requestKey, data: financeData });
        }
      })
      .catch((error) => {
        console.error("Failed to load finance snapshot:", error);
        if (!cancelled) {
          setRemoteFinanceData(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [financeSnapshotKey, myTeam.id]);

  useEffect(() => {
    setSelectedRiskPlayerIds((currentIds) => {
      const availableIdSet = new Set(allRiskPlayerIds);
      const nextIds = currentIds.filter((playerId) =>
        availableIdSet.has(playerId),
      );

      if (nextIds.length > 0) {
        return nextIds;
      }

      return allRiskPlayerIds;
    });
  }, [allRiskPlayerIds.join("|")]);

  function handleToggleRiskPlayer(playerId: string): void {
    setSelectedRiskPlayerIds((currentIds) => {
      if (currentIds.includes(playerId)) {
        return currentIds.filter((currentId) => currentId !== playerId);
      }

      return [...currentIds, playerId];
    });
  }

  function handleToggleAllRiskPlayers(): void {
    setSelectedRiskPlayerIds((currentIds) => {
      if (currentIds.length === allRiskPlayerIds.length) {
        return [];
      }

      return allRiskPlayerIds;
    });
  }

  async function handleUpgradeFacility(facility: FacilityId): Promise<void> {
    setFacilityUpgradeError(null);
    setActionLoading(facility);
    try {
      const updated = await invoke<GameStateData>("upgrade_facility", {
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
      const response = await invoke<BoardSupportResponseData>(
        "request_board_support",
      );
      onGameUpdate?.(response.game);
      setBoardSupportFeedback({
        tone: "success",
        text: t("finances.boardSupportSummary", {
          amount: formatExactMoney(response.result.support_amount),
          transferBudgetReduction: formatExactMoney(
            response.result.transfer_budget_reduction,
          ),
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
      const response = await invoke<SponsorPitchResponseData>(
        "request_sponsor_pitch",
      );
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
      const response = await invoke<MarketingCampaignResponseData>(
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

  async function handleDelegateRenewals(): Promise<void> {
    if (selectedRiskPlayers.length === 0) {
      return;
    }

    const loadingKey = "delegate-renewals";
    setActionLoading(loadingKey);
    setDelegatedRenewalsSummary(null);

    try {
      const result = await invoke<DelegatedRenewalResponseData>(
        "delegate_renewals",
        {
          playerIds: selectedRiskPlayers.map(({ player }) => player.id),
          maxWageIncreasePct: 35,
          maxContractYears: 3,
        },
      );
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
    const loadingKey = `sponsor:${messageId}:${optionId}`;
    setActionLoading(loadingKey);
    try {
      const result = await invoke<ResolveMessageActionResult>(
        "resolve_message_action",
        {
          messageId,
          actionId,
          optionId,
        },
      );
      onGameUpdate?.(result.game);
    } catch (error) {
      console.error("Failed to resolve sponsor offer:", error);
    } finally {
      setActionLoading(null);
    }
  }

  const financeItems = [
    {
      label: t("finances.clubBalance"),
      value: myTeam.finance,
      color: myTeam.finance >= 0 ? "text-primary-500" : "text-red-500",
    },
    {
      label: t("finances.wageBudget"),
      value: myTeam.wage_budget,
      color: "text-gray-800 dark:text-gray-200",
    },
    {
      label: t("finances.transferBudget"),
      value: myTeam.transfer_budget,
      color: "text-gray-800 dark:text-gray-200",
    },
    {
      label: t("finances.seasonIncome"),
      value: myTeam.season_income,
      color: "text-primary-500",
    },
    {
      label: t("finances.seasonExpenses"),
      value: myTeam.season_expenses,
      color: "text-red-500",
    },
  ];

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
      {/* Financial overview */}
      <Card accent="accent" className="lg:col-span-2">
        <CardHeader>{t("finances.overview")}</CardHeader>
        <CardBody>
          <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
            {financeItems.map((item) => (
              <div
                key={item.label}
                className="bg-gray-50 dark:bg-navy-800 rounded-xl p-4 text-center"
              >
                <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500 mb-1">
                  {item.label}
                </p>
                <p className={`font-heading font-bold text-xl ${item.color}`}>
                  {formatVal(item.value)}
                </p>
              </div>
            ))}
            <div className="bg-gray-50 dark:bg-navy-800 rounded-xl p-4 text-center">
              <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500 mb-1">
                {t("finances.squadValue")}
              </p>
              <p className="font-heading font-bold text-xl text-gray-800 dark:text-gray-200">
                {formatVal(totalValue)}
              </p>
            </div>
          </div>
        </CardBody>
      </Card>

      {/* Wage summary */}
      <Card>
        <CardHeader>{t("finances.wageBill")}</CardHeader>
        <CardBody>
          <div className="text-center mb-4">
            <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500">
              {t("finances.weeklyTotal")}
            </p>
            <p className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100 mt-1">
              {formatWeeklyAmount(formatVal(totalWages), weeklySuffix)}
            </p>
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
              {t("finances.budget")}:{" "}
              {formatWeeklyAmount(formatVal(weeklyWageBudget), weeklySuffix)}{" "}
              —{" "}
              {totalWages <= weeklyWageBudget ? (
                <span className="text-primary-500">
                  {t("finances.underBudget")}
                </span>
              ) : (
                <span className="text-red-500">{t("finances.overBudget")}</span>
              )}
            </p>
          </div>
          <ProgressBar
            value={Math.min(
              100,
              Math.round((totalWages / Math.max(1, weeklyWageBudget)) * 100),
            )}
            variant={totalWages <= weeklyWageBudget ? "success" : "danger"}
            size="md"
            showLabel
          />
        </CardBody>
      </Card>

      <Card className="lg:col-span-3">
        <CardHeader>{t("finances.cashFlow")}</CardHeader>
        <CardBody>
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <div className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-4 text-center">
              <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1">
                {t("finances.weeklyWageSpend")}
              </p>
              <p className="font-heading font-bold text-xl text-red-500">
                {formatWeeklyAmount(
                  formatSignedAmount(-totalWages),
                  weeklySuffix,
                )}
              </p>
            </div>
            <div className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-4 text-center">
              <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1">
                {t("finances.weeklySponsorIncome")}
              </p>
              <p className="font-heading font-bold text-xl text-primary-500">
                {formatWeeklyAmount(
                  formatSignedAmount(weeklySponsorIncome),
                  weeklySuffix,
                )}
              </p>
            </div>
            <div className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-4 text-center">
              <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1">
                {t("finances.projectedWeeklyNet")}
              </p>
              <p
                className={`font-heading font-bold text-xl ${projectedWeeklyNet >= 0 ? "text-primary-500" : "text-red-500"}`}
              >
                {formatWeeklyAmount(
                  formatSignedAmount(projectedWeeklyNet),
                  weeklySuffix,
                )}
              </p>
            </div>
            <div className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-4 text-center">
              <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1">
                {t("finances.cashRunway")}
              </p>
              <p className="font-heading font-bold text-base text-gray-800 dark:text-gray-100">
                {cashRunwayWeeks === null
                  ? t("finances.runwayStable")
                  : t("finances.runwayWeeks", { count: cashRunwayWeeks })}
              </p>
            </div>
          </div>
          {(myTeam.financial_ledger?.length ?? 0) > 0 && (
            <div className="mt-4">
              <FinanceCashFlowChart
                ledger={myTeam.financial_ledger ?? []}
                incomeLabel={t("finances.seasonIncome")}
                expensesLabel={t("finances.seasonExpenses")}
              />
            </div>
          )}
          <div className="mt-4 rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-4 space-y-3">
            <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
              <div className="space-y-1">
                <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {t("finances.boardSupport")}
                </p>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {t("finances.boardSupportDescription")}
                </p>
                {boardSupportPreviewText ? (
                  <p className="text-xs text-gray-600 dark:text-gray-400">
                    {boardSupportPreviewText}
                  </p>
                ) : null}
              </div>
              <Button
                disabled={!canRequestBoardSupport || actionLoading === "board-support"}
                onClick={() => void handleRequestBoardSupport()}
                size="sm"
              >
                {t("finances.requestBoardSupport")}
              </Button>
            </div>
            {boardSupportFeedback ? (
              <p
                className={`text-sm ${boardSupportFeedback.tone === "error" ? "text-red-500" : "text-primary-500"}`}
              >
                {boardSupportFeedback.text}
              </p>
            ) : null}
            {!canRequestBoardSupport ? (
              <p className="text-xs text-gray-500 dark:text-gray-400">
                {t("finances.boardSupportUnavailable")}
              </p>
            ) : null}
          </div>
        </CardBody>
      </Card>

      <Card className="lg:col-span-3">
        <CardHeader>{t("finances.wagePressure")}</CardHeader>
        <CardBody>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            <div className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-4 space-y-3">
              <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                {t("finances.wagePressure")}
              </p>
              <p className="font-heading font-bold text-2xl text-gray-900 dark:text-gray-100">
                {t("finances.wageBudgetUsed", {
                  percent: wageBudgetUsagePercent,
                })}
              </p>
              <ProgressBar
                value={Math.min(100, wageBudgetUsagePercent)}
                variant={
                  totalWages <= weeklyWageBudget ? "success" : "danger"
                }
                size="md"
                showLabel
              />
            </div>

            <div className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-4 space-y-3">
              <div className="flex items-center justify-between gap-3">
                <div className="space-y-1">
                  <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("finances.contractRisk")}
                  </p>
                  {delegatedRenewalsSummary ? (
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      {delegatedRenewalsSummary}
                    </p>
                  ) : null}
                </div>
                <div className="flex items-center gap-2">
                  <p className="text-sm font-semibold text-gray-700 dark:text-gray-300">
                    {t("finances.atRiskWages", {
                      amount: formatExactMoney(atRiskWages),
                    })}
                  </p>
                  {contractRiskPlayers.length > 0 ? (
                    <div className="flex items-center gap-2">
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={handleToggleAllRiskPlayers}
                      >
                        {t("finances.selectAllAtRisk")}
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => void handleDelegateRenewals()}
                        disabled={
                          actionLoading === "delegate-renewals" ||
                          selectedRiskPlayers.length === 0
                        }
                      >
                        {t("finances.delegateSelectedRenewals")}
                      </Button>
                    </div>
                  ) : null}
                </div>
              </div>

              {contractRiskPlayers.length > 0 ? (
                <div className="space-y-3">
                  {contractRiskPlayers.map(({ player, riskLevel }) => (
                    <div
                      key={player.id}
                      className="rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 p-3 flex items-start justify-between gap-3"
                    >
                      <div className="flex items-start gap-3">
                        <Checkbox
                          checked={selectedRiskPlayerIds.includes(player.id)}
                          onChange={() => handleToggleRiskPlayer(player.id)}
                          aria-label={t("finances.selectRiskPlayer", {
                            player: player.full_name,
                          })}
                          className="mt-1"
                        />
                        <div className="space-y-1">
                          <p className="font-semibold text-sm text-gray-900 dark:text-gray-100">
                            {player.full_name}
                          </p>
                          <p className="text-xs text-gray-600 dark:text-gray-400">
                            {t("finances.contractExpiresOn", {
                              date: player.contract_end,
                            })}
                          </p>
                          <p className="text-xs text-gray-600 dark:text-gray-400">
                            {t("playerProfile.yearsRemaining")}:{" "}
                            {getContractYearsRemaining(
                              player.contract_end,
                              gameState.clock.current_date,
                            )}
                          </p>
                        </div>
                      </div>
                      <div className="flex flex-col items-end gap-2">
                        <Badge variant={getContractRiskBadgeVariant(riskLevel)}>
                          {riskLevel === "critical"
                            ? t("finances.contractRiskCritical")
                            : t("finances.contractRiskWarning")}
                        </Badge>
                        <span className="text-xs font-semibold text-gray-700 dark:text-gray-300">
                          {formatWeeklyAmount(
                            formatExactMoney(
                              annualAmountToWeeklyCommitment(player.wage),
                            ),
                            weeklySuffix,
                          )}
                        </span>
                        {onSelectPlayer ? (
                          <Button
                            size="sm"
                            variant="outline"
                            onClick={(event) => {
                              event.stopPropagation();
                              onSelectPlayer(player.id, {
                                openRenewal: true,
                              });
                            }}
                          >
                            {t("common.renewContract")}
                          </Button>
                        ) : null}
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {t("finances.noContractRisks")}
                </p>
              )}
            </div>
          </div>
        </CardBody>
      </Card>

      <Card className="lg:col-span-3">
        <CardHeader>{t("finances.sponsors")}</CardHeader>
        <CardBody>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            <div className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-4 space-y-2">
              <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                {t("finances.activeSponsor")}
              </p>
              {activeSponsorship ? (
                <>
                  <h3 className="font-heading font-bold text-base text-gray-900 dark:text-gray-100 uppercase tracking-wide">
                    {activeSponsorship.sponsor_name}
                  </h3>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    {t("finances.sponsorWeeklyValue", {
                      amount: formatExactMoney(activeSponsorship.base_value),
                    })}
                  </p>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    {t("finances.sponsorRemainingWeeks", {
                      count: activeSponsorship.remaining_weeks,
                    })}
                  </p>
                </>
              ) : (
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {t("finances.noActiveSponsor")}
                </p>
              )}
            </div>

            <div className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-4 space-y-3">
              <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                {t("finances.pendingSponsorOffers")}
              </p>
              <div className="rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 p-4 space-y-3">
                <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                  <div className="space-y-1">
                    <h3 className="font-semibold text-sm text-gray-900 dark:text-gray-100">
                      {t("finances.pitchSponsor")}
                    </h3>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      {t("finances.sponsorPitchDescription")}
                    </p>
                    {sponsorPitchPreviewText ? (
                      <p className="text-xs text-gray-600 dark:text-gray-400">
                        {sponsorPitchPreviewText}
                      </p>
                    ) : null}
                  </div>
                  <Button
                    size="sm"
                    onClick={() => void handleRequestSponsorPitch()}
                    disabled={
                      actionLoading === "sponsor-pitch" ||
                      !canRequestSponsorPitch
                    }
                  >
                    {t("finances.pitchSponsor")}
                  </Button>
                </div>
                {sponsorPitchDisabledReason ? (
                  <p className="text-xs text-gray-600 dark:text-gray-400">
                    {sponsorPitchDisabledReason}
                  </p>
                ) : null}
                {sponsorPitchFeedback ? (
                  <p
                    className={
                      sponsorPitchFeedback.tone === "error"
                        ? "text-sm text-red-500"
                        : "text-sm text-primary-500"
                    }
                  >
                    {sponsorPitchFeedback.text}
                  </p>
                ) : null}
              </div>
              <div className="rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 p-4 space-y-3">
                <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                  <div className="space-y-1">
                    <h3 className="font-semibold text-sm text-gray-900 dark:text-gray-100">
                      {t("finances.marketingCampaign")}
                    </h3>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      {t("finances.marketingCampaignDescription")}
                    </p>
                    {marketingCampaignPreviewText ? (
                      <p className="text-xs text-gray-600 dark:text-gray-400">
                        {marketingCampaignPreviewText}
                      </p>
                    ) : null}
                  </div>
                  <Button
                    size="sm"
                    onClick={() => void handleRequestMarketingCampaign()}
                    disabled={
                      actionLoading === "marketing-campaign" ||
                      !canRequestMarketingCampaign
                    }
                  >
                    {t("finances.launchMarketingCampaign")}
                  </Button>
                </div>
                {marketingCampaignDisabledReason ? (
                  <p className="text-xs text-gray-600 dark:text-gray-400">
                    {marketingCampaignDisabledReason}
                  </p>
                ) : null}
                {marketingCampaignFeedback ? (
                  <p
                    className={
                      marketingCampaignFeedback.tone === "error"
                        ? "text-sm text-red-500"
                        : "text-sm text-primary-500"
                    }
                  >
                    {marketingCampaignFeedback.text}
                  </p>
                ) : null}
              </div>
              {sponsorOffers.length > 0 ? (
                sponsorOffers.map((message) => {
                  const sponsorAction = message.actions.find(
                    (action) =>
                      !action.resolved &&
                      isChooseOptionAction(action.action_type),
                  );

                  if (
                    !sponsorAction ||
                    !isChooseOptionAction(sponsorAction.action_type)
                  ) {
                    return null;
                  }

                  return (
                    <div
                      key={message.id}
                      className="rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 p-4 space-y-3"
                    >
                      <div className="space-y-1">
                        <h3 className="font-semibold text-sm text-gray-900 dark:text-gray-100">
                          {message.subject}
                        </h3>
                        <p className="text-sm text-gray-600 dark:text-gray-400">
                          {message.body}
                        </p>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        {sponsorAction.action_type.ChooseOption.options.map(
                          (option) => {
                            const optionLoadingKey = `sponsor:${message.id}:${option.id}`;
                            return (
                              <div
                                key={option.id}
                                className="min-w-55 flex-1 rounded-lg border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-3 space-y-2"
                              >
                                <p className="text-xs text-gray-600 dark:text-gray-400">
                                  {option.description}
                                </p>
                                <Button
                                  disabled={actionLoading === optionLoadingKey}
                                  onClick={() =>
                                    void handleSponsorOption(
                                      message.id,
                                      sponsorAction.id,
                                      option.id,
                                    )
                                  }
                                  size="sm"
                                  variant={
                                    option.id === "decline"
                                      ? "outline"
                                      : "primary"
                                  }
                                >
                                  {option.label}
                                </Button>
                              </div>
                            );
                          },
                        )}
                      </div>
                    </div>
                  );
                })
              ) : (
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {t("finances.noPendingSponsorOffers")}
                </p>
              )}
            </div>
          </div>
        </CardBody>
      </Card>

      <FinancesFacilitiesCard
        facilities={facilities}
        financeSnapshot={financeSnapshot}
        teamFinance={myTeam.finance}
        facilityUpgradeError={facilityUpgradeError}
        actionLoading={actionLoading}
        onUpgrade={(facility) => void handleUpgradeFacility(facility)}
      />

      <FinancesPayrollTable roster={roster} onSelectPlayer={onSelectPlayer} />
    </div>
  );
}
