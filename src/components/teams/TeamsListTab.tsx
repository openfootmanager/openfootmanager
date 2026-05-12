import { GameStateData } from "../../store/gameStore";
import { Card, CardBody, Badge, TeamLocation } from "../ui";
import { Users, Trophy } from "lucide-react";
import { calcOvr as calcPlayerOvr, formatVal } from "../../lib/helpers";
import { useTranslation } from "react-i18next";

interface TeamsListTabProps {
  gameState: GameStateData;
  onSelectTeam: (id: string) => void;
}

export default function TeamsListTab({ gameState, onSelectTeam }: TeamsListTabProps) {
  const { t, i18n } = useTranslation();
  const userTeamId = gameState.manager.team_id;

  const allStandings = gameState.league?.standings
    ? [...gameState.league.standings].sort((a, b) =>
        b.points - a.points || (b.goals_for - b.goals_against) - (a.goals_for - a.goals_against) || b.goals_for - a.goals_for
      )
    : [];

  const teamsData = gameState.teams.map(team => {
    const roster = gameState.players.filter(p => p.team_id === team.id);
    const avgOvr = roster.length > 0
      ? Math.round(roster.reduce((s, p) => s + (p.ovr ?? calcPlayerOvr(p)), 0) / roster.length)
      : 0;
    const totalValue = roster.reduce((s, p) => s + p.market_value, 0);
    const leaguePos = allStandings.findIndex(s => s.team_id === team.id) + 1;
    const standing = allStandings.find(s => s.team_id === team.id);

    return { team, roster, avgOvr, totalValue, leaguePos, standing };
  }).sort((a, b) => a.leaguePos - b.leaguePos);

  return (
    <div className="max-w-6xl mx-auto">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {teamsData.map(({ team, roster, avgOvr, totalValue, leaguePos, standing }) => {
          const isUser = team.id === userTeamId;
          return (
            <Card
              key={team.id}
              className={`cursor-pointer hover:shadow-lg transition-all ${isUser ? "ring-2 ring-primary-500/30" : ""}`}
            >
              <div
                onClick={() => onSelectTeam(team.id)}
                className="overflow-hidden rounded-xl"
              >
                {/* Header with team color */}
                <div
                  className="p-5 flex items-center gap-4"
                  style={{ background: `linear-gradient(135deg, ${team.colors.primary}, ${team.colors.secondary}40)` }}
                >
                  <div
                    className="w-14 h-14 rounded-xl flex items-center justify-center font-heading font-bold text-xl text-white border-2 border-white/30"
                    style={{ backgroundColor: team.colors.primary }}
                  >
                    {team.short_name}
                  </div>
                  <div className="flex-1 min-w-0">
                    <h3 className="font-heading font-bold text-lg text-white uppercase tracking-wide truncate drop-shadow">
                      {team.name}
                      {isUser && <Badge variant="accent" size="sm" className="ml-2 align-middle">{t('teams.yourTeam')}</Badge>}
                    </h3>
                    <TeamLocation
                      city={team.city}
                      countryCode={team.country}
                      locale={i18n.language}
                      className="mt-0.5 text-white/70 text-xs"
                      iconClassName="w-3 h-3"
                      flagClassName="text-xs leading-none"
                    />
                  </div>
                  {leaguePos > 0 && (
                    <div className="bg-black/20 backdrop-blur rounded-lg px-3 py-1.5 text-center">
                      <p className="text-xs text-white/60 font-heading uppercase tracking-wider">{t('common.position')}</p>
                      <p className="font-heading font-bold text-xl text-white">#{leaguePos}</p>
                    </div>
                  )}
                </div>

                {/* Stats row */}
                <div className="grid grid-cols-5 gap-px bg-gray-200 dark:bg-navy-600">
                  <StatCell label={t('teams.squad')} value={String(roster.length)} />
                  <StatCell label={t('teams.avgOvr')} value={String(avgOvr)} />
                  <StatCell label={t('teams.rep')} value={String(team.reputation)} />
                  <StatCell label={t('common.value')} value={formatVal(totalValue)} />
                  <StatCell label={t('common.pts')} value={standing ? String(standing.points) : "—"} />
                </div>

                {/* Bottom info */}
                <CardBody>
                  <div className="flex items-center justify-between text-xs text-gray-500 dark:text-gray-400">
                    <span className="flex items-center gap-1">
                      <Users className="w-3.5 h-3.5" />
                      {team.formation} — {team.play_style}
                    </span>
                    <span className="flex items-center gap-1">
                      <Trophy className="w-3.5 h-3.5" />
                      {t('teams.est')} {team.founded_year}
                    </span>
                    {standing && (
                      <span className="tabular-nums">
                        {standing.won}W {standing.drawn}D {standing.lost}L
                      </span>
                    )}
                  </div>
                </CardBody>
              </div>
            </Card>
          );
        })}
      </div>
    </div>
  );
}

function StatCell({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-white dark:bg-navy-800 px-2 py-2.5 text-center">
      <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">{label}</p>
      <p className="font-heading font-bold text-sm text-gray-800 dark:text-gray-100 mt-0.5">{value}</p>
    </div>
  );
}
