import { DollarSign, Trophy, Users } from "lucide-react";

import { annualAmountToWeeklyCommitment } from "../../lib/finance";
import { formatVal, formatWeeklyAmount } from "../../lib/helpers";
import type { TeamData } from "../../store/gameStore";
import { Card, CardBody, CardHeader } from "../ui";
import type { TeamProfileTranslate, TeamProfileViewModel } from "./TeamProfile.types";
import { InfoRow } from "./TeamProfile.primitives";

interface TeamProfileSummaryCardProps {
  team: TeamData;
  isOwnTeam: boolean;
  viewModel: TeamProfileViewModel;
  weeklySuffix: string;
  t: TeamProfileTranslate;
}

export default function TeamProfileSummaryCard({
  team,
  isOwnTeam,
  viewModel,
  weeklySuffix,
  t,
}: TeamProfileSummaryCardProps) {
  if (isOwnTeam) {
    return (
      <Card accent="accent">
        <CardHeader>{t("dashboard.finances")}</CardHeader>
        <CardBody>
          <div className="flex flex-col gap-3">
            <InfoRow
              icon={<DollarSign className="w-4 h-4" />}
              label={t("teamProfile.balance")}
              value={formatVal(team.finance)}
            />
            <InfoRow
              icon={<DollarSign className="w-4 h-4" />}
              label={t("finances.wageBudget")}
              value={formatWeeklyAmount(
                formatVal(annualAmountToWeeklyCommitment(team.wage_budget)),
                weeklySuffix,
              )}
            />
            <InfoRow
              icon={<DollarSign className="w-4 h-4" />}
              label={t("finances.transferBudget")}
              value={formatVal(team.transfer_budget)}
            />
            <InfoRow
              icon={<DollarSign className="w-4 h-4" />}
              label={t("teamProfile.totalWages")}
              value={formatWeeklyAmount(
                formatVal(annualAmountToWeeklyCommitment(viewModel.totalWages)),
                weeklySuffix,
              )}
            />
            <InfoRow
              icon={<DollarSign className="w-4 h-4" />}
              label={t("finances.squadValue")}
              value={formatVal(viewModel.totalValue)}
            />
            <InfoRow
              icon={<DollarSign className="w-4 h-4" />}
              label={t("finances.seasonIncome")}
              value={formatVal(team.season_income)}
            />
          </div>
        </CardBody>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>{t("teamProfile.squadOverview")}</CardHeader>
      <CardBody>
        <div className="flex flex-col gap-3">
          <InfoRow
            icon={<Users className="w-4 h-4" />}
            label={t("teamProfile.squadSize")}
            value={String(viewModel.roster.length)}
          />
          <InfoRow
            icon={<DollarSign className="w-4 h-4" />}
            label={t("finances.squadValue")}
            value={formatVal(viewModel.totalValue)}
          />
          <InfoRow
            icon={<Trophy className="w-4 h-4" />}
            label={t("teams.avgOvr")}
            value={String(viewModel.avgOvr)}
          />
        </div>
      </CardBody>
    </Card>
  );
}
