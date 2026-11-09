import { useTranslation } from "react-i18next";
import { formatOrdinal } from "../../lib/ordinal";
import type { GameStateData } from "../../store/gameStore";
import { Card, CardBody } from "../ui";

interface Props {
  gameState: GameStateData;
  standings: NonNullable<GameStateData["league"]>["standings"];
  isChampion: boolean;
}

export default function SeasonReviewClub({ gameState, standings, isChampion }: Props) {
  const { t, i18n } = useTranslation();
  const userTeamId = gameState.manager.team_id;
  const userTeam = gameState.teams.find((team) => team.id === userTeamId);
  const userStandingIdx = standings.findIndex((entry) => entry.team_id === userTeamId);
  const userStanding = standings[userStandingIdx];
  const userHasStanding = userStandingIdx >= 0;
  const userPosition = userStandingIdx + 1;
  const posLabel = (pos: number) =>
    formatOrdinal(pos, i18n.language, (key, options) => t(key, options), "place");
  return (
    <>
      {/* User team summary */}
      {userHasStanding && (
        <Card accent={isChampion ? "accent" : "primary"} className="mb-6">
          <CardBody>
            <div className="text-center">
              <p className="text-xs font-heading font-bold uppercase tracking-widest text-gray-400 dark:text-gray-500 mb-1">
                {userTeam?.name}
              </p>
              <div className="flex items-center justify-center gap-6 mb-4">
                <div>
                  <p className="text-4xl font-heading font-bold text-gray-900 dark:text-gray-100">
                    {posLabel(userPosition)}
                  </p>
                  <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase">
                    {t("endOfSeason.position")}
                  </p>
                </div>
                <div className="w-px h-12 bg-gray-200 dark:bg-navy-600" />
                <div>
                  <p className="text-4xl font-heading font-bold text-primary-500">
                    {userStanding?.points || 0}
                  </p>
                  <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase">
                    {t("endOfSeason.points")}
                  </p>
                </div>
              </div>
              <div className="flex items-center justify-center gap-8 text-sm">
                <span className="text-green-500 font-heading font-bold">
                  {userStanding?.won || 0}
                  {t("common.won")}
                </span>
                <span className="text-gray-500 font-heading font-bold">
                  {userStanding?.drawn || 0}
                  {t("common.drawn")}
                </span>
                <span className="text-red-500 font-heading font-bold">
                  {userStanding?.lost || 0}
                  {t("common.lost")}
                </span>
                <span className="text-gray-400">
                  {userStanding?.goals_for || 0} {t("common.gf")} —{" "}
                  {userStanding?.goals_against || 0} {t("common.ga")}
                </span>
              </div>
            </div>
          </CardBody>
        </Card>
      )}
    </>
  );
}
