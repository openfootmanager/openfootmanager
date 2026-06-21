import { useTranslation } from "react-i18next";
import { GameStateData } from "../store/gameStore";
import { Badge, TeamLogo } from "./ui";
import {
  getTeamName,
  getUserCompetition,
  getUserNextFixture,
  formatMatchDate,
  isSeasonComplete,
} from "../lib/helpers";

export default function NextMatchDisplay({
  gameState,
}: {
  gameState: GameStateData;
}) {
  const { t } = useTranslation();
  const userTeamId = gameState.manager.team_id;
  const league = getUserCompetition(gameState);

  if (!userTeamId || !league) {
    return (
      <p className="text-gray-500 dark:text-gray-400 text-sm text-center py-4">
        {t("home.noLeagueSchedule")}
      </p>
    );
  }

  const nextFixture = getUserNextFixture(gameState);
  if (!nextFixture) {
    return (
      <p className="text-gray-500 dark:text-gray-400 text-sm text-center py-4">
        {t(
          isSeasonComplete(league)
            ? "home.seasonComplete"
            : "home.noUpcomingOpponent",
        )}
      </p>
    );
  }

  const isHome = nextFixture.home_team_id === userTeamId;
  const opponentId = isHome
    ? nextFixture.away_team_id
    : nextFixture.home_team_id;
  const userTeam = gameState.teams.find((team) => team.id === userTeamId);
  const opponentTeam = gameState.teams.find((team) => team.id === opponentId);
  const fixtureLabel =
    nextFixture.competition === "League"
      ? t("home.matchdayN", { n: nextFixture.matchday })
      : nextFixture.competition === "PreseasonTournament"
        ? t("season.preseasonTournament")
        : t("season.friendly");

  return (
    <div className="flex items-center justify-between py-6 px-4 bg-gray-50 dark:bg-navy-800 rounded-lg border border-gray-100 dark:border-navy-600 transition-colors">
      <div className="text-center flex-1">
        {userTeam && (
          <TeamLogo
            team={userTeam}
            className="w-16 h-16 rounded-full mx-auto mb-2 flex items-center justify-center overflow-hidden border-2 border-primary-200 dark:border-primary-800"
            imageClassName="h-12 w-12 object-contain drop-shadow"
            style={{ backgroundColor: userTeam.colors.primary }}
          />
        )}
        <p
          className="font-heading font-bold uppercase tracking-wide text-sm text-primary-600 dark:text-primary-400"
        >
          {getTeamName(gameState.teams, userTeamId)}
        </p>
      </div>

      <div className="text-center px-4 flex flex-col items-center gap-1.5">
        <span className="font-heading font-bold text-2xl text-gray-300 dark:text-navy-600">
          VS
        </span>
        <Badge variant="neutral">{formatMatchDate(nextFixture.date)}</Badge>
        <span className="text-xs text-gray-400 dark:text-gray-500">
          {fixtureLabel}
        </span>
        <Badge variant={isHome ? "success" : "accent"} size="sm">
          {isHome ? t("home.home") : t("home.away")}
        </Badge>
      </div>

      <div className="text-center flex-1">
        {opponentTeam && (
          <TeamLogo
            team={opponentTeam}
            className="w-16 h-16 rounded-full mx-auto mb-2 flex items-center justify-center overflow-hidden border-2 border-gray-300 dark:border-navy-600"
            imageClassName="h-12 w-12 object-contain drop-shadow"
            style={{ backgroundColor: opponentTeam.colors.primary }}
          />
        )}
        <p
          className="font-heading font-bold uppercase tracking-wide text-sm text-gray-500 dark:text-gray-400"
        >
          {getTeamName(gameState.teams, opponentId)}
        </p>
      </div>
    </div>
  );
}
