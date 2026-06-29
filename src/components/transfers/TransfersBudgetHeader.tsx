import type { ComponentProps } from "react";
import { useTranslation } from "react-i18next";
import { TrendingUp } from "lucide-react";

import type { TeamData } from "../../store/gameStore";
import { Card, Badge } from "../ui";
import { formatVal, formatAnnualAmount } from "../../lib/helpers";

interface TransfersBudgetHeaderProps {
  myTeam: TeamData;
  transferWindowVariant: ComponentProps<typeof Badge>["variant"];
  transferWindowStatus: string;
  transferWindowSummary: string;
  annualWageBudget: number;
  annualSuffix: string;
  listedCount: number;
}

export default function TransfersBudgetHeader({
  myTeam,
  transferWindowVariant,
  transferWindowStatus,
  transferWindowSummary,
  annualWageBudget,
  annualSuffix,
  listedCount,
}: TransfersBudgetHeaderProps) {
  const { t } = useTranslation();

  return (
    <Card accent="primary" className="mb-5">
      <div className="bg-gradient-to-r from-navy-700 to-navy-800 p-5 rounded-t-xl flex items-center gap-6">
        <div className="flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h2 className="text-lg font-heading font-bold text-white uppercase tracking-wide flex items-center gap-2">
              <TrendingUp className="w-5 h-5 text-accent-400" />
              {t("transfers.centre")}
            </h2>
            <Badge variant={transferWindowVariant} size="sm">
              {t(`season.transferWindowStatus.${transferWindowStatus}`)}
            </Badge>
          </div>
          <p className="text-gray-400 text-xs mt-0.5">
            {t("transfers.transferWindow", { team: myTeam.name })}
          </p>
          <p className="text-gray-500 text-xs mt-1">{transferWindowSummary}</p>
        </div>
        <div className="hidden md:flex gap-4">
          <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
            <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
              {t("finances.transferBudget")}
            </p>
            <p className="font-heading font-bold text-lg text-accent-400">
              {formatVal(myTeam.transfer_budget)}
            </p>
          </div>
          <div
            data-testid="wage-budget-card"
            className="bg-white/5 rounded-xl px-4 py-2 text-center"
          >
            <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
              {t("finances.wageBudget")}
            </p>
            <p className="font-heading font-bold text-lg text-white">
              {formatAnnualAmount(formatVal(annualWageBudget), annualSuffix)}
            </p>
          </div>
          <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
            <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
              {t("transfers.listed")}
            </p>
            <p className="font-heading font-bold text-lg text-white">
              {listedCount}
            </p>
          </div>
        </div>
      </div>
    </Card>
  );
}
