import { ArrowRightLeft, BadgeEuro, CalendarDays, Percent } from "lucide-react";
import type { TOptions } from "i18next";

import { formatDate } from "../../lib/dateFormatting";
import { formatVal } from "../../lib/helpers";
import type { ActiveLoanData, TeamData } from "../../store/gameStore";
import { Card, CardBody } from "../ui";
import { getPlayerTeamName } from "./PlayerProfile.helpers";

type TranslateFn = (key: string, options?: TOptions) => string;

interface PlayerProfileLoanStatusBannerProps {
  loan: ActiveLoanData;
  teams: TeamData[];
  managerTeamId: string | null;
  language: string;
  t: TranslateFn;
}

export default function PlayerProfileLoanStatusBanner({
  loan,
  teams,
  managerTeamId,
  language,
  t,
}: PlayerProfileLoanStatusBannerProps) {
  const labels = {
    freeAgent: t("common.freeAgent"),
    unknown: t("common.unknown"),
  };
  const parentTeamName = getPlayerTeamName(teams, loan.parent_team_id, labels);
  const loanTeamName = getPlayerTeamName(teams, loan.loan_team_id, labels);
  const relationship =
    managerTeamId === loan.parent_team_id
      ? t("playerProfile.loanedOutTo", { team: loanTeamName })
      : managerTeamId === loan.loan_team_id
        ? t("playerProfile.loanedInFrom", { team: parentTeamName })
        : t("playerProfile.loanBetweenClubs", {
          parent: parentTeamName,
          loanTeam: loanTeamName,
        });

  return (
    <Card accent="primary" className="mb-5">
      <CardBody>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary-500/10">
              <ArrowRightLeft className="h-5 w-5 text-primary-500" />
            </div>
            <div>
              <p className="font-heading text-xs font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                {t("playerProfile.loanStatus")}
              </p>
              <p className="text-sm font-semibold text-gray-800 dark:text-gray-100">
                {relationship}
              </p>
            </div>
          </div>
          <div className="grid gap-2 text-xs text-gray-500 dark:text-gray-400 sm:grid-cols-3 sm:text-right">
            <span className="inline-flex items-center gap-1 sm:justify-end">
              <CalendarDays className="h-3.5 w-3.5" />
              {t("playerProfile.loanUntil", {
                date: formatDate(loan.end_date, language),
              })}
            </span>
            <span className="inline-flex items-center gap-1 sm:justify-end">
              <Percent className="h-3.5 w-3.5" />
              {t("playerProfile.loanWageSplit", {
                percent: loan.wage_contribution_pct,
              })}
            </span>
            {loan.buy_option_fee ? (
              <span className="inline-flex items-center gap-1 sm:justify-end">
                <BadgeEuro className="h-3.5 w-3.5" />
                {t("playerProfile.loanBuyOption", {
                  fee: formatVal(loan.buy_option_fee),
                })}
              </span>
            ) : null}
          </div>
        </div>
      </CardBody>
    </Card>
  );
}
