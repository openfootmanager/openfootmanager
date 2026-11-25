import { Card, CardHeader, CardBody, Badge, ProgressBar, Button, Checkbox } from "../ui";
import {
  contractExpiryLabel,
  formatExactMoney,
  formatWeeklyWage,
  getContractRiskBadgeVariant,
  getContractYearsRemaining,
} from "../../lib/helpers";
import { weeklyWageAmount } from "../../lib/finance";

import type { FinanceViewState } from "./useFinanceView";

export function FinanceContractRiskCard({
  state,
}: {
  state: Pick<
    FinanceViewState,
    | "t"
    | "wageBudgetUsagePercent"
    | "totalWages"
    | "weeklyWageBudget"
    | "delegatedRenewalsSummary"
    | "atRiskWages"
    | "contractRiskPlayers"
    | "handleToggleAllRiskPlayers"
    | "handleDelegateRenewals"
    | "actionLoading"
    | "selectedRiskPlayers"
    | "deselectedRiskPlayerIds"
    | "handleToggleRiskPlayer"
    | "i18n"
    | "gameState"
    | "onSelectPlayer"
  >;
}) {
  const {
    t,
    wageBudgetUsagePercent,
    totalWages,
    weeklyWageBudget,
    delegatedRenewalsSummary,
    atRiskWages,
    contractRiskPlayers,
    handleToggleAllRiskPlayers,
    handleDelegateRenewals,
    actionLoading,
    selectedRiskPlayers,
    deselectedRiskPlayerIds,
    handleToggleRiskPlayer,
    i18n,
    gameState,
    onSelectPlayer,
  } = state;
  return (
    <Card className="lg:col-span-3">
      <CardHeader>{t("finances.wagePressure")}</CardHeader>
      <CardBody>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div className="rounded-xl border border-line bg-sunken p-4 space-y-3">
            <p className="text-caption font-heading font-bold text-fg-muted">
              {t("finances.wagePressure")}
            </p>
            <p className="font-heading font-bold text-title text-fg">
              {t("finances.wageBudgetUsed", {
                percent: wageBudgetUsagePercent,
              })}
            </p>
            <ProgressBar
              value={Math.min(100, wageBudgetUsagePercent)}
              variant={totalWages <= weeklyWageBudget ? "success" : "danger"}
              size="md"
              showLabel
            />
          </div>

          <div className="rounded-xl border border-line bg-sunken p-4 space-y-3">
            <div className="flex items-center justify-between gap-3">
              <div className="space-y-1">
                <p className="text-caption font-heading font-bold text-fg-muted">
                  {t("finances.contractRisk")}
                </p>
                {delegatedRenewalsSummary ? (
                  <p className="text-caption text-fg-muted">{delegatedRenewalsSummary}</p>
                ) : null}
              </div>
              <div className="flex items-center gap-2">
                <p className="text-body font-semibold text-fg">
                  {t("finances.atRiskWages", {
                    amount: formatExactMoney(atRiskWages),
                  })}
                </p>
                {contractRiskPlayers.length > 0 ? (
                  <div className="flex items-center gap-2">
                    <Button size="sm" variant="outline" onClick={handleToggleAllRiskPlayers}>
                      {t("finances.selectAllAtRisk")}
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => void handleDelegateRenewals()}
                      disabled={
                        actionLoading === "delegate-renewals" || selectedRiskPlayers.length === 0
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
                    className="rounded-lg border border-line bg-panel p-3 flex items-start justify-between gap-3"
                  >
                    <div className="flex items-start gap-3">
                      <Checkbox
                        checked={!deselectedRiskPlayerIds.includes(player.id)}
                        onChange={() => handleToggleRiskPlayer(player.id)}
                        aria-label={t("finances.selectRiskPlayer", {
                          player: player.full_name,
                        })}
                        className="mt-1"
                      />
                      <div className="space-y-1">
                        <p className="font-semibold text-body text-fg">{player.full_name}</p>
                        <p className="text-caption text-fg-muted">
                          {player.contract_end
                            ? contractExpiryLabel(player.contract_end, i18n.language, t)
                            : null}
                        </p>
                        <p className="text-caption text-fg-muted">
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
                      <span className="text-caption font-semibold text-fg">
                        {formatWeeklyWage(weeklyWageAmount(player.wage))}
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
              <p className="text-body text-fg-muted">{t("finances.noContractRisks")}</p>
            )}
          </div>
        </div>
      </CardBody>
    </Card>
  );
}
