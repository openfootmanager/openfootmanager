import { useTranslation } from "react-i18next";
import { User } from "lucide-react";
import { Card, CardHeader, CardBody, Badge } from "../ui";
import {
  formatExactMoney,
  formatVal,
  positionBadgeVariant,
} from "../../lib/helpers";
import { annualAmountToWeeklyCommitment } from "../../lib/finance";
import type { PlayerData, PlayerSelectionOptions } from "../../store/gameStore";
import ContextMenu from "../ContextMenu";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";

interface FinancesPayrollTableProps {
  roster: PlayerData[];
  onSelectPlayer?: (id: string, options?: PlayerSelectionOptions) => void;
}

export default function FinancesPayrollTable({
  roster,
  onSelectPlayer,
}: FinancesPayrollTableProps) {
  const { t } = useTranslation();

  return (
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
                        label: t("squad.viewProfile"),
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
                        {formatExactMoney(annualAmountToWeeklyCommitment(p.wage))}
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
  );
}
