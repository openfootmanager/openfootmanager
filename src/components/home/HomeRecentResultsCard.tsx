import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import type { TeamData } from "../../store/gameStore";
import { Card, CardBody, CardHeader, TeamLogo } from "../ui";
import type { HomeRecentResult } from "./HomeTab.helpers";

interface HomeRecentResultsCardProps {
  recentResults: HomeRecentResult[];
  teams: TeamData[];
  onNavigate?: (tab: string) => void;
}

export default function HomeRecentResultsCard({
  recentResults,
  teams,
  onNavigate,
}: HomeRecentResultsCardProps) {
  const { t } = useTranslation();
  const teamsById = useMemo(
    () => new Map(teams.map((team) => [team.id, team])),
    [teams],
  );

  return (
    <Card>
      <CardHeader
        action={
          <button
            onClick={() => onNavigate?.("Schedule")}
            className="text-primary-500 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-primary-600 dark:hover:text-primary-300 transition-colors"
          >
            {t("dashboard.schedule")}
          </button>
        }
      >
        {t("home.recentResults")}
      </CardHeader>
      <CardBody className="p-0">
        {recentResults.length === 0 ? (
          <p className="text-gray-500 dark:text-gray-400 text-xs p-5">
            {t("home.noMatches")}
          </p>
        ) : (
          <div className="divide-y divide-gray-100 dark:divide-navy-600">
            {recentResults
              .slice(-5)
              .reverse()
              .map((result) => {
                const opponent = teamsById.get(result.opponentId);

                return (
                  <div
                    key={result.fixture.id}
                    className="flex items-center px-4 py-2.5 gap-3"
                  >
                    <span
                      className={`w-5 h-5 rounded flex items-center justify-center text-[9px] font-heading font-bold text-white flex-shrink-0 ${
                        result.resultCode === "W"
                          ? "bg-green-500"
                          : result.resultCode === "L"
                            ? "bg-red-500"
                            : "bg-gray-400"
                      }`}
                    >
                      {result.resultCode}
                    </span>
                    <span className="text-xs text-gray-500 dark:text-gray-400 flex-shrink-0 w-6">
                      {result.isHome ? t("home.home").charAt(0) : t("home.away").charAt(0)}
                    </span>
                    {opponent ? (
                      <TeamLogo
                        team={opponent}
                        className="h-7 w-7 shrink-0 overflow-hidden rounded bg-gray-100 dark:bg-navy-700 flex items-center justify-center text-[10px] font-heading font-bold text-gray-500 dark:text-gray-300"
                        imageClassName="h-5 w-5 object-contain drop-shadow"
                      />
                    ) : null}
                    <span className="text-sm font-medium text-gray-800 dark:text-gray-200 flex-1 truncate">
                      {opponent?.name ?? t("common.unknown")}
                    </span>
                    <span className="text-sm font-heading font-bold text-gray-700 dark:text-gray-300 tabular-nums">
                      {result.myGoals} - {result.opponentGoals}
                    </span>
                  </div>
                );
              })}
          </div>
        )}
      </CardBody>
    </Card>
  );
}