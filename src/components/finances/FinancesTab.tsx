import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  GameStateData,
  MessageAction,
  MessageData,
  PlayerSelectionOptions,
} from "../../store/gameStore";
import { Card, CardHeader, CardBody, Badge, ProgressBar, Button } from "../ui";
import { User } from "lucide-react";
import {
  formatExactMoney,
  formatVal,
  formatWeeklyAmount,
  getContractRiskBadgeVariant,
  getContractRiskLevel,
  getContractYearsRemaining,
  positionBadgeVariant,
} from "../../lib/helpers";
import {
  annualAmountToWeeklyCommitment,
  getTeamFinanceSnapshot,
} from "../../lib/finance";
import { useTranslation } from "react-i18next";
import ContextMenu from "../ContextMenu";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import { resolveMessage } from "../../utils/backendI18n";

type FacilityId = "Training" | "Medical" | "Scouting";

interface FacilityDefinition {
  effectKey: string;
  id: FacilityId;
  levelKey: "training" | "medical" | "scouting";
  titleKey: string;
}

const DEFAULT_FACILITIES = {
  training: 1,
  medical: 1,
  scouting: 1,
};

const FACILITY_DEFINITIONS: FacilityDefinition[] = [
  {
    id: "Training",
    levelKey: "training",
    titleKey: "finances.facilityTraining",
    effectKey: "finances.facilityTrainingEffect",
  },
  {
    id: "Medical",
    levelKey: "medical",
    titleKey: "finances.facilityMedical",
    effectKey: "finances.facilityMedicalEffect",
  },
  {
    id: "Scouting",
    levelKey: "scouting",
    titleKey: "finances.facilityScouting",
    effectKey: "finances.facilityScoutingEffect",
  },
];

function getFacilityUpgradeCost(level: number): number {
  return level * 250_000;
}

function formatSignedAmount(value: number): string {
  const formatted = formatVal(Math.abs(value));
  return value < 0 ? `-${formatted}` : formatted;
}

interface ResolveMessageActionResult {
  game: GameStateData;
  effect: string | null;
  effect_i18n_key?: string | null;
  effect_i18n_params?: Record<string, string> | null;
}

interface DelegatedRenewalResponseData {
  game: GameStateData;
  report: {
    success_count: number;
    failure_count: number;
    stalled_count: number;
  };
}

function isChooseOptionAction(
  actionType: MessageAction["action_type"],
): actionType is {
  ChooseOption: {
    options: Array<{ id: string; label: string; description: string }>;
  };
} {
  return typeof actionType === "object" && "ChooseOption" in actionType;
}

function isPendingSponsorOffer(message: MessageData): boolean {
  return (
    message.id.startsWith("sponsor_") &&
    message.category === "Finance" &&
    message.actions.some(
      (action) => !action.resolved && isChooseOptionAction(action.action_type),
    )
  );
}

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
  const weeklySuffix = t("finances.perWeekSuffix", "/wk");
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [delegatedRenewalsSummary, setDelegatedRenewalsSummary] = useState<
    string | null
  >(null);
  const [selectedRiskPlayerIds, setSelectedRiskPlayerIds] = useState<string[]>(
    [],
  );

  const roster = gameState.players.filter((p) => p.team_id === myTeam.id);
  const teamStaff = gameState.staff.filter(
    (staffMember) => staffMember.team_id === myTeam.id,
  );
  const financeSnapshot = getTeamFinanceSnapshot(myTeam, roster, teamStaff);
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
    setActionLoading(facility);
    try {
      const updated = await invoke<GameStateData>("upgrade_facility", {
        facility,
      });
      onGameUpdate?.(updated);
    } catch (error) {
      console.error("Failed to upgrade facility:", error);
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
    <div className="max-w-6xl mx-auto grid grid-cols-1 lg:grid-cols-3 gap-5">
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
                        <input
                          type="checkbox"
                          checked={selectedRiskPlayerIds.includes(player.id)}
                          onChange={() => handleToggleRiskPlayer(player.id)}
                          aria-label={`Select ${player.full_name}`}
                          className="mt-1 h-4 w-4 rounded border-gray-300 text-primary-500 focus:ring-primary-500/30"
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
                  {t(
                    "finances.noPendingSponsorOffers",
                    "No pending sponsor offers",
                  )}
                </p>
              )}
            </div>
          </div>
        </CardBody>
      </Card>

      <Card className="lg:col-span-3">
        <CardHeader>{t("finances.facilities")}</CardHeader>
        <CardBody>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {FACILITY_DEFINITIONS.map((facility) => {
              const level = facilities[facility.levelKey];
              const nextUpgradeCost = getFacilityUpgradeCost(level);
              const canUpgrade = myTeam.finance >= nextUpgradeCost;
              const isLoading = actionLoading === facility.id;

              return (
                <div
                  key={facility.id}
                  className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 p-4 flex flex-col gap-4"
                >
                  <div className="space-y-1">
                    <h3 className="font-heading font-bold text-base text-gray-900 dark:text-gray-100 uppercase tracking-wide">
                      {t(facility.titleKey)}
                    </h3>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      {t("finances.facilityLevel", { level })}
                    </p>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      {t(facility.effectKey)}
                    </p>
                  </div>

                  <div className="space-y-2 mt-auto">
                    <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                      {t("finances.nextUpgradeCost", {
                        amount: formatExactMoney(nextUpgradeCost),
                      })}
                    </p>
                    <Button
                      disabled={!canUpgrade || isLoading}
                      onClick={() => void handleUpgradeFacility(facility.id)}
                      size="sm"
                    >
                      {t("finances.upgradeFacility")}
                    </Button>
                    {!canUpgrade && (
                      <p className="text-xs text-red-500">
                        {t("finances.insufficientFunds")}
                      </p>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </CardBody>
      </Card>

      {/* Payroll */}
      <Card className="lg:col-span-3">
        <CardHeader>{t("finances.payroll")}</CardHeader>
        <CardBody className="p-0">
          <div className="overflow-x-auto">
            <table className="w-full text-left border-collapse">
              <thead>
                <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                  <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.player")}
                  </th>
                  <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.position")}
                  </th>
                  <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("finances.wagePerWeek")}
                  </th>
                  <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("finances.marketValue")}
                  </th>
                  <th className="py-3 px-5 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                    {t("common.contract")}
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                {[...roster]
                  .sort((a, b) => b.wage - a.wage)
                  .slice(0, 10)
                  .map((p) => {
                    const contextItems = onSelectPlayer
                      ? [
                        {
                          label: t("squad.viewProfile", "View profile"),
                          icon: <User className="w-4 h-4" />,
                          onClick: () => onSelectPlayer(p.id),
                        },
                      ]
                      : [];

                    const row = (
                      <tr
                        key={p.id}
                        onClick={() => onSelectPlayer?.(p.id)}
                        className={`hover:bg-gray-50 dark:hover:bg-navy-700/50 transition-colors ${onSelectPlayer ? "cursor-pointer group" : ""}`}
                      >
                        <td className="py-3 px-5 font-semibold text-sm text-gray-800 dark:text-gray-200">
                          <span className="group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
                            {p.full_name}
                          </span>
                        </td>
                        <td className="py-3 px-5">
                          <Badge variant={positionBadgeVariant(p.position)}>
                            {translatePositionAbbreviation(t, p.position)}
                          </Badge>
                        </td>
                        <td className="py-3 px-5 text-sm font-medium text-gray-700 dark:text-gray-300">
                          {formatExactMoney(
                            annualAmountToWeeklyCommitment(p.wage),
                          )}
                        </td>
                        <td className="py-3 px-5 text-sm text-gray-600 dark:text-gray-400">
                          {formatVal(p.market_value)}
                        </td>
                        <td className="py-3 px-5 text-sm text-gray-500 dark:text-gray-400">
                          {p.contract_end
                            ? t("finances.until", {
                              year: p.contract_end.substring(0, 4),
                            })
                            : "—"}
                        </td>
                      </tr>
                    );

                    if (!onSelectPlayer) {
                      return row;
                    }

                    return (
                      <ContextMenu items={contextItems} key={p.id}>
                        {row}
                      </ContextMenu>
                    );
                  })}
              </tbody>
            </table>
          </div>
        </CardBody>
      </Card>
    </div>
  );
}
