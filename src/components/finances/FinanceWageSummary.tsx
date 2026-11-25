import { Card, CardHeader, CardBody, ProgressBar } from "../ui";
import { formatWeeklyWage } from "../../lib/helpers";

import type { FinanceViewState } from "./useFinanceView";

export function FinanceWageSummary({
  state,
}: {
  state: Pick<FinanceViewState, "t" | "totalWages" | "weeklyWageBudget">;
}) {
  const { t, totalWages, weeklyWageBudget } = state;
  return (
    <Card>
      <CardHeader>{t("finances.wageBill")}</CardHeader>
      <CardBody>
        <div className="text-center mb-4">
          <p className="text-caption font-heading font-bold text-fg-muted">
            {t("finances.weeklyTotal")}
          </p>
          <p className="font-heading font-bold text-title text-fg mt-1">
            {formatWeeklyWage(totalWages, "compact")}
          </p>
          <p className="text-caption text-fg-muted mt-1">
            {t("finances.budget")}: {formatWeeklyWage(weeklyWageBudget, "compact")} —{" "}
            {totalWages <= weeklyWageBudget ? (
              <span className="text-positive">{t("finances.underBudget")}</span>
            ) : (
              <span className="text-danger">{t("finances.overBudget")}</span>
            )}
          </p>
        </div>
        <ProgressBar
          value={Math.min(100, Math.round((totalWages / Math.max(1, weeklyWageBudget)) * 100))}
          variant={totalWages <= weeklyWageBudget ? "success" : "danger"}
          size="md"
          showLabel
        />
      </CardBody>
    </Card>
  );
}
