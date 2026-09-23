import { Trophy } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge, Card, CardBody, CardHeader } from "../ui";

interface LeagueStandingSnapshot {
  team_id: string;
  played: number;
  won: number;
  drawn: number;
  lost: number;
  goals_for: number;
  goals_against: number;
  points: number;
}

interface HomeLeaguePositionCardProps {
  isPreseason: boolean;
  phase: string;
  seasonStartLabel: string | null;
  myStanding: number | null;
  myStandingData: LeagueStandingSnapshot | null;
  teamForm: string[];
  onNavigate?: (tab: string) => void;
}

export default function HomeLeaguePositionCard({
  isPreseason,
  phase,
  seasonStartLabel,
  myStanding,
  myStandingData,
  teamForm,
  onNavigate,
}: HomeLeaguePositionCardProps) {
  const { t } = useTranslation();

  const lastThreeResults = teamForm.slice(-3);
  const winStreak =
    lastThreeResults.length >= 3 && lastThreeResults.every((result) => result === "W");
  const loseStreak =
    lastThreeResults.length >= 3 && lastThreeResults.every((result) => result === "L");
  const unbeaten = teamForm.length >= 4 && teamForm.every((result) => result !== "L");

  return (
    <Card accent="accent">
      <CardHeader
        action={
          <button
            type="button"
            onClick={() => onNavigate?.("Schedule")}
            className="text-primary-500 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-primary-600 dark:hover:text-primary-300 transition-colors"
          >
            {t("home.standings")}
          </button>
        }
      >
        {t("home.leaguePosition")}
      </CardHeader>
      <CardBody>
        {isPreseason ? (
          <div className="flex flex-col items-center gap-2 py-4 text-center">
            <Badge variant="accent" size="sm">
              {t(`season.phases.${phase}`)}
            </Badge>
            <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
              {seasonStartLabel
                ? t("season.startsOn", { date: seasonStartLabel })
                : t("season.noOpener")}
            </p>
            <p className="text-xs text-gray-500 dark:text-gray-400 max-w-xs">
              {t("season.standingsLocked")}
            </p>
          </div>
        ) : myStanding && myStandingData ? (
          <div className="flex flex-col items-center gap-3">
            <div className="flex items-center gap-3">
              <div className="w-16 h-16 rounded-xl bg-accent-500/10 flex items-center justify-center">
                <span className="text-3xl font-heading font-bold text-accent-500">
                  {myStanding}
                </span>
              </div>
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider">
                  {myStanding === 1
                    ? t("common.place.1")
                    : myStanding === 2
                      ? t("common.place.2")
                      : myStanding === 3
                        ? t("common.place.3")
                        : t("common.place.other", { n: myStanding })}
                </p>
                <p className="text-lg font-heading font-bold text-gray-800 dark:text-gray-100">
                  {myStandingData.points} pts
                </p>
              </div>
            </div>
            <div className="w-full grid grid-cols-4 text-center gap-1">
              <div>
                <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase">P</p>
                <p className="text-sm font-heading font-bold text-gray-700 dark:text-gray-300">
                  {myStandingData.played}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase">W</p>
                <p className="text-sm font-heading font-bold text-green-500">
                  {myStandingData.won}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase">D</p>
                <p className="text-sm font-heading font-bold text-gray-500">
                  {myStandingData.drawn}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase">L</p>
                <p className="text-sm font-heading font-bold text-red-500">{myStandingData.lost}</p>
              </div>
            </div>
            {teamForm.length > 0 && (
              <div className="flex flex-col items-center gap-1.5 mt-1">
                <div className="flex gap-1.5">
                  {teamForm.map((result, index) => (
                    <span
                      key={`${result}-${index}`}
                      className={`w-6 h-6 rounded flex items-center justify-center text-[10px] font-heading font-bold text-white ${
                        result === "W"
                          ? "bg-green-500"
                          : result === "L"
                            ? "bg-red-500"
                            : "bg-gray-400"
                      }`}
                    >
                      {result}
                    </span>
                  ))}
                </div>
                {winStreak && (
                  <span className="text-[10px] font-heading font-bold text-green-500 uppercase tracking-wider">
                    {t("home.winningStreak")}
                  </span>
                )}
                {loseStreak && (
                  <span className="text-[10px] font-heading font-bold text-red-500 uppercase tracking-wider">
                    {t("home.losingStreak")}
                  </span>
                )}
                {!winStreak && !loseStreak && unbeaten && (
                  <span className="text-[10px] font-heading font-bold text-primary-500 uppercase tracking-wider">
                    {t("home.unbeatenRun")}
                  </span>
                )}
              </div>
            )}
          </div>
        ) : (
          <div className="flex flex-col items-center gap-2 py-4">
            <Trophy className="w-8 h-8 text-gray-300 dark:text-navy-600" />
            <p className="text-xs text-gray-500 dark:text-gray-400">{t("home.noLeague")}</p>
          </div>
        )}
      </CardBody>
    </Card>
  );
}
