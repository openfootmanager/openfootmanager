import { FinanceCashFlowChart } from "./FinanceCashFlowChart";

import { Card, CardHeader, CardBody, Button } from "../ui";
import { formatWeeklyAmount } from "../../lib/helpers";

import { formatSignedAmount } from "./FinancesTab.helpers";

import type { FinanceViewState } from "./useFinanceView";

export function FinanceCashFlowCard({
  state,
}: {
  state: Pick<
    FinanceViewState,
    | "t"
    | "totalWages"
    | "weeklySponsorIncome"
    | "projectedWeeklyNet"
    | "cashRunwayWeeks"
    | "financeSnapshot"
    | "boardSupportPreviewText"
    | "canRequestBoardSupport"
    | "actionLoading"
    | "handleRequestBoardSupport"
    | "boardSupportFeedback"
    | "previewsCurrent"
  >;
}) {
  const {
    t,
    totalWages,
    weeklySponsorIncome,
    projectedWeeklyNet,
    cashRunwayWeeks,
    financeSnapshot,
    boardSupportPreviewText,
    canRequestBoardSupport,
    actionLoading,
    handleRequestBoardSupport,
    boardSupportFeedback,
    previewsCurrent,
  } = state;
  return (
    <Card className="lg:col-span-3">
      <CardHeader>{t("finances.cashFlow")}</CardHeader>
      <CardBody>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <div className="rounded-xl border border-line bg-sunken p-4 text-center">
            <p className="text-caption font-heading font-bold text-fg-muted mb-1">
              {t("finances.weeklyWageSpend")}
            </p>
            <p className="font-heading font-bold text-heading text-danger">
              {formatWeeklyAmount(formatSignedAmount(-totalWages))}
            </p>
          </div>
          <div className="rounded-xl border border-line bg-sunken p-4 text-center">
            <p className="text-caption font-heading font-bold text-fg-muted mb-1">
              {t("finances.weeklySponsorIncome")}
            </p>
            <p className="font-heading font-bold text-heading text-positive">
              {formatWeeklyAmount(formatSignedAmount(weeklySponsorIncome))}
            </p>
          </div>
          <div className="rounded-xl border border-line bg-sunken p-4 text-center">
            <p className="text-caption font-heading font-bold text-fg-muted mb-1">
              {t("finances.projectedWeeklyNet")}
            </p>
            <p
              className={`font-heading font-bold text-heading ${projectedWeeklyNet >= 0 ? "text-positive" : "text-danger"}`}
            >
              {formatWeeklyAmount(formatSignedAmount(projectedWeeklyNet))}
            </p>
          </div>
          <div className="rounded-xl border border-line bg-sunken p-4 text-center">
            <p className="text-caption font-heading font-bold text-fg-muted mb-1">
              {t("finances.cashRunway")}
            </p>
            <p className="font-heading font-bold text-body text-fg">
              {cashRunwayWeeks === null
                ? t("finances.runwayStable")
                : t("finances.runwayWeeks", { count: cashRunwayWeeks })}
            </p>
          </div>
        </div>
        {financeSnapshot.cashFlow.length > 0 && (
          <div className="mt-4">
            <FinanceCashFlowChart
              weeks={financeSnapshot.cashFlow}
              // The weeks hold every movement, transfer and loan fees included, which the season
              // figures beside them leave out: so the series are money in and out, not the season's.
              incomeLabel={t("finances.moneyIn")}
              expensesLabel={t("finances.moneyOut")}
            />
          </div>
        )}
        <div className="mt-4 rounded-xl border border-line bg-sunken p-4 space-y-3">
          <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
            <div className="space-y-1">
              <p className="text-caption font-heading font-bold text-fg-muted">
                {t("finances.boardSupport")}
              </p>
              <p className="text-body text-fg-muted">{t("finances.boardSupportDescription")}</p>
              {boardSupportPreviewText ? (
                <p className="text-caption text-fg-muted">{boardSupportPreviewText}</p>
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
              className={`text-body ${boardSupportFeedback.tone === "error" ? "text-danger" : "text-positive"}`}
            >
              {boardSupportFeedback.text}
            </p>
          ) : null}
          {previewsCurrent && !canRequestBoardSupport ? (
            <p className="text-caption text-fg-muted">{t("finances.boardSupportUnavailable")}</p>
          ) : null}
        </div>
      </CardBody>
    </Card>
  );
}
