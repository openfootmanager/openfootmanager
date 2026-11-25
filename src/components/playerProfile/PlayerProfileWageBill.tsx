import { Card, CardBody, CardHeader, ProgressBar } from "../ui";
import { formatPlayerWage } from "./PlayerProfile.helpers";
import { Pending, percentOf, Row, SECTION, type TranslateFn } from "./contractSheetParts";
import type { ContractSheetData } from "./useContractSheetData";

interface PlayerProfileWageBillProps {
  data: ContractSheetData;
  /** His weekly wage when the club may see it exactly; `null` otherwise. */
  exactWeeklyWage: number | null;
  /** On loan to another club: the bill counts only the parent club's part of his wage. */
  isOnLoan: boolean;
  language: string;
  t: TranslateFn;
}

/**
 * The club's wage bill: the weekly bill, the budget in use as a bar, the runway and his share of
 * the bill, read from the projection the backend sent. The projection's "annual" fields carry weekly
 * amounts (#300). The bar and the share are ratios of its figures, and whether the club is over
 * budget is the backend's flag, not a comparison here. The share is not shown for a player on loan:
 * the bill counts only the parent club's part of his wage, and the split is the backend's.
 */
export default function PlayerProfileWageBill({
  data,
  exactWeeklyWage,
  isOnLoan,
  language,
  t,
}: PlayerProfileWageBillProps) {
  const projection = data.projection.status === "answered" ? data.projection.value : null;
  const spend = projection?.current_weekly_wage_spend ?? 0;
  const budget = projection?.annual_wage_budget ?? 0;
  const runway = projection
    ? projection.current_cash_runway_weeks === null
      ? t("finances.runwayStable")
      : t("finances.runwayWeeks", { count: projection.current_cash_runway_weeks })
    : null;
  const share =
    projection && exactWeeklyWage !== null && !isOnLoan && spend > 0
      ? percentOf(exactWeeklyWage, spend, language, 1)
      : null;

  return (
    <Card>
      <CardHeader>{t(`${SECTION}.clubBill`)}</CardHeader>
      <CardBody>
        <Pending answer={data.projection} retry={data.retry} t={t} />
        {projection ? (
          <dl className="flex flex-col gap-2">
            <Row label={t(`${SECTION}.billLabel`)}>{formatPlayerWage(spend)}</Row>
            {budget > 0 ? (
              <div className="flex flex-col gap-1">
                <Row label={t(`${SECTION}.budgetLabel`)}>
                  {percentOf(spend, budget, language, 0)}
                </Row>
                <ProgressBar
                  value={(spend / budget) * 100}
                  variant={projection.currently_over_budget ? "danger" : "primary"}
                  label={t(`${SECTION}.budgetLabel`)}
                />
              </div>
            ) : null}
            {runway ? <Row label={t(`${SECTION}.runwayLabel`)}>{runway}</Row> : null}
            {share ? <Row label={t(`${SECTION}.shareLabel`)}>{share}</Row> : null}
          </dl>
        ) : null}
      </CardBody>
    </Card>
  );
}
