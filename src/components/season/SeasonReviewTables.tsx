import { Crown, Trophy } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { GameStateData } from "../../store/gameStore";
import { Card, CardBody } from "../ui";

interface Props {
  gameState: GameStateData;
  league: GameStateData["league"];
  standings: NonNullable<GameStateData["league"]>["standings"];
  isChampion: boolean;
}

export default function SeasonReviewTables({ gameState, league, standings, isChampion }: Props) {
  const { t } = useTranslation();
  const userTeamId = gameState.manager.team_id;
  const champion = standings[0];
  const championName = gameState.teams.find((team) => team.id === champion?.team_id)?.name || "";
  return (
    <>
      {/* Final top 5 standings */}
      {league && (
        <Card className="mb-6">
          <CardBody>
            <h3 className="font-heading font-bold text-sm uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3 flex items-center gap-2">
              <Trophy className="w-4 h-4 text-accent-500" /> {t("endOfSeason.finalStandings")}
            </h3>
            <div className="divide-y divide-gray-100 dark:divide-navy-600">
              {standings.slice(0, 5).map((entry, idx) => {
                const teamName = gameState.teams.find((t) => t.id === entry.team_id)?.name || "";
                const isUser = entry.team_id === userTeamId;
                const gd = entry.goals_for - entry.goals_against;
                return (
                  <div
                    key={entry.team_id}
                    className={`flex items-center py-2.5 gap-3 ${isUser ? "bg-primary-50/50 dark:bg-primary-500/5 -mx-2 px-2 rounded-lg" : ""}`}
                  >
                    <span
                      className={`font-heading font-bold text-sm w-6 text-center ${idx === 0 ? "text-accent-500" : "text-gray-400"}`}
                    >
                      {idx + 1}
                    </span>
                    <span
                      className={`flex-1 text-sm font-semibold ${isUser ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                    >
                      {teamName}
                    </span>
                    <span className="text-xs text-gray-500 dark:text-gray-400 tabular-nums w-16 text-center">
                      {entry.won}W {entry.drawn}D {entry.lost}L
                    </span>
                    <span
                      className={`text-xs font-semibold tabular-nums w-8 text-center ${gd > 0 ? "text-primary-500" : gd < 0 ? "text-red-500" : "text-gray-500"}`}
                    >
                      {gd > 0 ? `+${gd}` : gd}
                    </span>
                    <span className="font-heading font-bold text-sm text-gray-800 dark:text-gray-100 tabular-nums w-8 text-right">
                      {entry.points}
                    </span>
                  </div>
                );
              })}
            </div>
          </CardBody>
        </Card>
      )}

      {/* Champion + awards */}
      {league && champion && !isChampion && (
        <Card className="mb-6">
          <CardBody>
            <div className="flex items-center gap-3">
              <Crown className="w-6 h-6 text-accent-500" />
              <div>
                <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-200 uppercase tracking-wider">
                  {t("endOfSeason.leagueChampions")}
                </p>
                <p className="text-lg font-heading font-bold text-accent-500">{championName}</p>
              </div>
            </div>
          </CardBody>
        </Card>
      )}
    </>
  );
}
