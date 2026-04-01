import { useMemo, useState } from "react";
import { GameStateData, FixtureData } from "../store/gameStore";
import { Card, CardBody, Badge } from "./ui";
import { Calendar as CalendarIcon, TableProperties, Trophy } from "lucide-react";
import { formatMatchDate } from "../lib/helpers";
import { useTranslation } from "react-i18next";

interface ScheduleTabProps {
  gameState: GameStateData;
  onSelectTeam: (id: string) => void;
}

export default function ScheduleTab({ gameState, onSelectTeam }: ScheduleTabProps) {
  const { t, i18n } = useTranslation();
  const [view, setView] = useState<"fixtures" | "standings">("fixtures");
  const league = gameState.league;
  const userTeamId = gameState.manager.team_id;

  if (!league) {
    return <p className="text-gray-500 dark:text-gray-400 text-center py-8">{t('schedule.noLeague')}</p>;
  }

  const teamNameById = useMemo(
    () => new Map(gameState.teams.map(team => [team.id, team.name])),
    [gameState.teams]
  );

  const sortedMatchdays = useMemo(() => {
    const matchdays = new Map<number, FixtureData[]>();
    league.fixtures.forEach(fixture => {
      const list = matchdays.get(fixture.matchday);
      if (list) {
        list.push(fixture);
      } else {
        matchdays.set(fixture.matchday, [fixture]);
      }
    });
    return Array.from(matchdays.entries()).sort((a, b) => a[0] - b[0]);
  }, [league.fixtures]);

  const standings = useMemo(() => (
    [...league.standings].sort((a, b) =>
      b.points - a.points || (b.goals_for - b.goals_against) - (a.goals_for - a.goals_against) || b.goals_for - a.goals_for
    )
  ), [league.standings]);

  return (
    <div className="max-w-6xl mx-auto">
      {/* Tab switcher */}
      <div className="flex gap-2 mb-5">
        <button
          onClick={() => setView("fixtures")}
          className={`px-4 py-2 rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-all ${
            view === "fixtures"
              ? "bg-primary-500 text-white shadow-md shadow-primary-500/20"
              : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 border border-gray-200 dark:border-navy-600"
          }`}
        >
          <CalendarIcon className="w-4 h-4 inline mr-1.5 -mt-0.5" /> {t('schedule.fixtures')}
        </button>
        <button
          onClick={() => setView("standings")}
          className={`px-4 py-2 rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-all ${
            view === "standings"
              ? "bg-primary-500 text-white shadow-md shadow-primary-500/20"
              : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 border border-gray-200 dark:border-navy-600"
          }`}
        >
          <TableProperties className="w-4 h-4 inline mr-1.5 -mt-0.5" /> {t('schedule.standings')}
        </button>
      </div>

      {view === "fixtures" && (
        <div className="flex flex-col gap-4">
          {sortedMatchdays.map(([md, fixtures]) => (
            <Card key={md}>
              <div className="px-5 py-3 border-b border-gray-100 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 rounded-t-xl">
                <h4 className="font-heading font-bold text-sm uppercase tracking-wider text-gray-600 dark:text-gray-300">
                  {t('schedule.matchday', { number: md })} — {formatMatchDate(fixtures[0].date, i18n.language)}
                </h4>
              </div>
              <CardBody className="p-0">
                <div className="divide-y divide-gray-100 dark:divide-navy-600">
                  {fixtures.map(f => {
                    const isUserMatch = f.home_team_id === userTeamId || f.away_team_id === userTeamId;
                    const completed = f.status === "Completed";
                    return (
                      <div key={f.id} className={`flex items-center px-5 py-3 transition-colors ${isUserMatch ? "bg-primary-50/50 dark:bg-primary-500/5" : ""}`}>
                        <span onClick={() => onSelectTeam(f.home_team_id)} className={`flex-1 text-right font-semibold text-sm cursor-pointer hover:underline ${f.home_team_id === userTeamId ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}>
                          {teamNameById.get(f.home_team_id) || f.home_team_id}
                        </span>
                        <div className="w-24 text-center mx-3">
                          {completed && f.result ? (
                            <span className="font-heading font-bold text-lg text-gray-800 dark:text-gray-100">
                              {f.result.home_goals} - {f.result.away_goals}
                            </span>
                          ) : (
                            <Badge variant="neutral" size="sm">vs</Badge>
                          )}
                        </div>
                        <span onClick={() => onSelectTeam(f.away_team_id)} className={`flex-1 text-left font-semibold text-sm cursor-pointer hover:underline ${f.away_team_id === userTeamId ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}>
                          {teamNameById.get(f.away_team_id) || f.away_team_id}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </CardBody>
            </Card>
          ))}
        </div>
      )}

      {view === "standings" && (
        <Card>
          <div className="p-5 border-b border-gray-100 dark:border-navy-600 bg-gradient-to-r from-navy-700 to-navy-800 rounded-t-xl">
            <h3 className="text-lg font-heading font-bold text-white flex items-center gap-2 uppercase tracking-wide">
              <Trophy className="text-accent-400 w-5 h-5" />
              {league.name} — {t('schedule.season', { number: league.season })}
            </h3>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-left border-collapse">
              <thead>
                <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 w-8">#</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t('common.team')}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t('common.played')}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t('common.won')}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t('common.drawn')}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t('common.lost')}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t('common.gf')}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t('common.ga')}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t('common.gd')}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t('common.pts')}</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                {standings.map((entry, idx) => {
                  const isUser = entry.team_id === userTeamId;
                  const gd = entry.goals_for - entry.goals_against;
                  return (
                    <tr key={entry.team_id} className={`transition-colors ${isUser ? "bg-primary-50 dark:bg-primary-500/10" : "hover:bg-gray-50 dark:hover:bg-navy-700/50"}`}>
                      <td className="py-3 px-4 font-heading font-bold text-sm text-gray-400 dark:text-gray-500">{idx + 1}</td>
                      <td onClick={() => onSelectTeam(entry.team_id)} className={`py-3 px-4 font-semibold text-sm cursor-pointer hover:underline ${isUser ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}>
                        {teamNameById.get(entry.team_id) || entry.team_id}
                      </td>
                      <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">{entry.played}</td>
                      <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">{entry.won}</td>
                      <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">{entry.drawn}</td>
                      <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">{entry.lost}</td>
                      <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">{entry.goals_for}</td>
                      <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">{entry.goals_against}</td>
                      <td className={`py-3 px-4 text-center text-sm font-semibold tabular-nums ${gd > 0 ? "text-primary-500" : gd < 0 ? "text-red-500" : "text-gray-500 dark:text-gray-400"}`}>
                        {gd > 0 ? `+${gd}` : gd}
                      </td>
                      <td className="py-3 px-4 text-center font-heading font-bold text-sm text-gray-800 dark:text-gray-100 tabular-nums">{entry.points}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </Card>
      )}
    </div>
  );
}
