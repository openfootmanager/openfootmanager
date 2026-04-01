import { useTranslation } from "react-i18next";
import { GameStateData } from "../store/gameStore";
import { Badge } from "./ui";
import { getTeamName, getTeamShort, findNextFixture, formatMatchDate } from "../lib/helpers";

export default function NextMatchDisplay({ gameState }: { gameState: GameStateData }) {
  const { t, i18n } = useTranslation();
  const userTeamId = gameState.manager.team_id;
  const league = gameState.league;

  if (!userTeamId || !league) {
    return <p className="text-gray-500 dark:text-gray-400 text-sm text-center py-4">{t('home.noLeagueSchedule')}</p>;
  }

  const nextFixture = findNextFixture(league.fixtures, userTeamId);
  if (!nextFixture) {
    return <p className="text-gray-500 dark:text-gray-400 text-sm text-center py-4">{t('home.seasonComplete')}</p>;
  }

  const isHome = nextFixture.home_team_id === userTeamId;
  const opponentId = isHome ? nextFixture.away_team_id : nextFixture.home_team_id;

  return (
    <div className="flex items-center justify-between py-6 px-4 bg-gray-50 dark:bg-navy-800 rounded-lg border border-gray-100 dark:border-navy-600 transition-colors">
      <div className="text-center flex-1">
        <div className="w-16 h-16 bg-gradient-to-br from-primary-500/20 to-primary-600/20 dark:from-primary-500/10 dark:to-primary-600/10 rounded-full mx-auto mb-2 flex items-center justify-center font-heading font-bold text-primary-600 dark:text-primary-400 text-lg border-2 border-primary-200 dark:border-primary-800 transition-colors">
          {getTeamShort(gameState.teams, nextFixture.home_team_id)}
        </div>
        <p className={`font-heading font-bold uppercase tracking-wide text-sm ${isHome ? "text-primary-600 dark:text-primary-400" : "text-gray-500 dark:text-gray-400"}`}>
          {getTeamName(gameState.teams, nextFixture.home_team_id)}
        </p>
      </div>

      <div className="text-center px-4 flex flex-col items-center gap-1.5">
        <span className="font-heading font-bold text-2xl text-gray-300 dark:text-navy-600">VS</span>
        <Badge variant="neutral">{formatMatchDate(nextFixture.date, i18n.language)}</Badge>
        <span className="text-xs text-gray-400 dark:text-gray-500">{t('home.matchdayN', { n: nextFixture.matchday })}</span>
        <Badge variant={isHome ? "success" : "accent"} size="sm">{isHome ? t('home.home') : t('home.away')}</Badge>
      </div>

      <div className="text-center flex-1">
        <div className="w-16 h-16 bg-gray-200 dark:bg-navy-600 rounded-full mx-auto mb-2 flex items-center justify-center font-heading font-bold text-gray-500 dark:text-gray-400 text-lg border-2 border-gray-300 dark:border-navy-600 transition-colors">
          {getTeamShort(gameState.teams, opponentId)}
        </div>
        <p className={`font-heading font-bold uppercase tracking-wide text-sm ${!isHome ? "text-primary-600 dark:text-primary-400" : "text-gray-500 dark:text-gray-400"}`}>
          {getTeamName(gameState.teams, opponentId)}
        </p>
      </div>
    </div>
  );
}
