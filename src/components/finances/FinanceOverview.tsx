import { useTranslation } from "react-i18next";
import type { TeamData } from "../../store/gameStore";
import type { TeamFinanceSnapshotData } from "../../services/financeService";
import { formatVal } from "../../lib/helpers";
import { Card, CardHeader, CardBody } from "../ui";

interface FinanceOverviewProps {
  myTeam: Pick<TeamData, "finance" | "wage_budget" | "season_income" | "season_expenses">;
  snapshot: Pick<TeamFinanceSnapshotData, "committedTransferFees" | "availableTransferBudget">;
  totalValue: number;
}

export function FinanceOverview({ myTeam, snapshot, totalValue }: FinanceOverviewProps) {
  const { t } = useTranslation();
  const financeItems = [
    {
      label: t("finances.clubBalance"),
      value: myTeam.finance,
      color: myTeam.finance >= 0 ? "text-positive" : "text-danger",
    },
    {
      label: t("finances.wageBudget"),
      value: myTeam.wage_budget,
      color: "text-fg",
    },
    {
      label: t("finances.availableTransferBudget"),
      value: snapshot.availableTransferBudget,
      color: "text-fg",
    },
    {
      label: t("finances.committedTransferFees"),
      value: snapshot.committedTransferFees,
      color: "text-fg",
    },
    {
      label: t("finances.seasonIncome"),
      value: myTeam.season_income,
      color: "text-positive",
    },
    {
      label: t("finances.seasonExpenses"),
      value: myTeam.season_expenses,
      color: "text-danger",
    },
  ];

  return (
    <Card accent="accent" className="lg:col-span-2">
      <CardHeader>{t("finances.overview")}</CardHeader>
      <CardBody>
        <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
          {financeItems.map((item) => (
            <div key={item.label} className="bg-sunken rounded-xl p-4 text-center">
              <p className="text-caption text-fg-muted mb-1">{item.label}</p>
              <p className={`font-heading font-bold text-heading ${item.color}`}>
                {formatVal(item.value)}
              </p>
            </div>
          ))}
          <div className="bg-sunken rounded-xl p-4 text-center">
            <p className="text-caption text-fg-muted mb-1">{t("finances.squadValue")}</p>
            <p className="font-heading font-bold text-heading text-fg">{formatVal(totalValue)}</p>
          </div>
        </div>
      </CardBody>
    </Card>
  );
}
