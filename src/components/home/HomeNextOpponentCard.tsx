import { useTranslation } from "react-i18next";

import { formatDateShort, getFixtureDisplayLabel } from "../../lib/helpers";
import { Badge, Card, CardBody, CardHeader, FormDots, TeamLogo } from "../ui";
import type { NextOpponentWidgetData } from "./HomeTab.helpers";

interface HomeNextOpponentCardProps {
  nextOpponent: NextOpponentWidgetData | null;
  lang: string;
  onNavigate?: (tab: string) => void;
}

export default function HomeNextOpponentCard({
  nextOpponent,
  lang,
  onNavigate,
}: HomeNextOpponentCardProps) {
  const { t } = useTranslation();

  return (
    <Card>
      <CardHeader
        action={
          <button
            type="button"
            onClick={() => onNavigate?.("Schedule")}
            className="text-primary-500 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-primary-600 dark:hover:text-primary-300 transition-colors"
          >
            {t("dashboard.schedule")}
          </button>
        }
      >
        {t("home.nextOpponent")}
      </CardHeader>
      <CardBody>
        {nextOpponent ? (
          <div className="flex flex-col gap-3">
            {(() => {
              const fixtureLabel = getFixtureDisplayLabel(
                t,
                nextOpponent.fixture,
                nextOpponent.competitionName,
              );

              return (
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0 flex items-center gap-3">
                    <TeamLogo
                      team={nextOpponent.opponent}
                      className="h-11 w-11 shrink-0 overflow-hidden rounded-lg bg-gray-100 dark:bg-navy-700 flex items-center justify-center text-xs font-heading font-bold text-gray-500 dark:text-gray-300"
                      imageClassName="h-9 w-9 object-contain drop-shadow"
                    />
                    <div className="min-w-0">
                      <p className="text-lg font-heading font-bold text-gray-800 dark:text-gray-100 truncate">
                        {nextOpponent.opponent.name}
                      </p>
                      <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        {fixtureLabel} - {formatDateShort(nextOpponent.fixture.date, lang)}
                      </p>
                    </div>
                  </div>
                  <Badge variant={nextOpponent.isHome ? "success" : "accent"} size="sm">
                    {nextOpponent.isHome ? t("home.home") : t("home.away")}
                  </Badge>
                </div>
              );
            })()}

            {(nextOpponent.standingPosition !== null || nextOpponent.standingPoints !== null) && (
              <div className="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
                {nextOpponent.standingPosition !== null && (
                  <Badge variant="neutral" size="sm">
                    #{nextOpponent.standingPosition}
                  </Badge>
                )}
                {nextOpponent.standingPoints !== null && (
                  <span className="font-heading font-bold text-gray-700 dark:text-gray-300">
                    {nextOpponent.standingPoints} {t("common.pts")}
                  </span>
                )}
              </div>
            )}

            <FormDots results={nextOpponent.recentForm} />
          </div>
        ) : (
          <p className="text-sm text-gray-500 dark:text-gray-400 py-4 text-center">
            {t("home.noUpcomingOpponent")}
          </p>
        )}
      </CardBody>
    </Card>
  );
}
