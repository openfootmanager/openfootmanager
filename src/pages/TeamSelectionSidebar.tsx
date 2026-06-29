import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { LeagueData, PlayerData, TeamData } from "../store/gameStore";
import { getPlayerOvr } from "../lib/helpers";
import { competitionDisplayName } from "../lib/competitionName";
import { Badge, Card, CardBody, TeamLocation } from "../components/ui";
import { Globe, Shield, Target, Users } from "lucide-react";

interface TeamSelectionSidebarProps {
  selectedTeam: TeamData | null;
  selectedTeamXi: PlayerData[];
  selectedTeamCompetitions: LeagueData[];
  getTeamAvgOvr: (teamId: string) => number;
}

export default function TeamSelectionSidebar({
  selectedTeam,
  selectedTeamXi,
  selectedTeamCompetitions,
  getTeamAvgOvr,
}: TeamSelectionSidebarProps) {
  const { t, i18n } = useTranslation();
  const compName = (c: LeagueData) => competitionDisplayName(c, t);

  return (
    <Card accent="accent" className="h-fit">
      <CardBody className="space-y-5 p-5">
        {selectedTeam ? (
          <>
            <div>
              <p className="text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                {t("teamSelect.selectedClub")}
              </p>
              <h2 className="mt-1 font-heading text-2xl font-bold text-gray-900 dark:text-white">
                {selectedTeam.name}
              </h2>
              <TeamLocation
                city={selectedTeam.city}
                countryCode={selectedTeam.country}
                locale={i18n.language}
                className="mt-2 text-sm text-gray-500 dark:text-gray-400"
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <DetailTile
                icon={<Target className="h-4 w-4" />}
                label={t("teamSelect.formation")}
                value={selectedTeam.formation}
              />
              <DetailTile
                icon={<Shield className="h-4 w-4" />}
                label={t("teamSelect.overall")}
                value={String(getTeamAvgOvr(selectedTeam.id))}
              />
              <DetailTile
                icon={<Users className="h-4 w-4" />}
                label={t("teamSelect.likelyXi")}
                value={t("teamSelect.playersCount", { count: selectedTeamXi.length })}
              />
              <DetailTile
                icon={<Globe className="h-4 w-4" />}
                label={t("teamSelect.competitions")}
                value={String(selectedTeamCompetitions.length)}
              />
            </div>

            <div>
              <p className="mb-3 text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                {t("teamSelect.keyPlayers")}
              </p>
              <div className="space-y-2">
                {selectedTeamXi.slice(0, 5).map((player) => (
                  <div
                    key={player.id}
                    className="flex items-center justify-between rounded-xl bg-gray-50 px-3 py-2 dark:bg-navy-800"
                  >
                    <div>
                      <p className="font-semibold text-gray-900 dark:text-white">
                        {player.match_name}
                      </p>
                      <p className="text-xs text-gray-500 dark:text-gray-400">
                        {player.position}
                      </p>
                    </div>
                    <Badge variant="accent">{getPlayerOvr(player)}</Badge>
                  </div>
                ))}
              </div>
            </div>

            <div>
              <p className="mb-3 text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                {t("teamSelect.activeCompetitions")}
              </p>
              <div className="flex flex-wrap gap-2">
                {selectedTeamCompetitions.map((competition) => (
                  <Badge key={competition.id} variant="primary">
                    {compName(competition)}
                  </Badge>
                ))}
              </div>
              <p className="mt-3 text-xs text-gray-500 dark:text-gray-400">
                {t("teamSelect.clubCompetitionsAlwaysSimulated")}
              </p>
            </div>
          </>
        ) : (
          <p className="text-sm text-gray-500 dark:text-gray-400">
            {t("teamSelect.selectClubPrompt")}
          </p>
        )}
      </CardBody>
    </Card>
  );
}

function DetailTile({
  icon,
  label,
  value,
}: {
  icon: ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="rounded-xl bg-gray-50 px-3 py-3 dark:bg-navy-800">
      <p className="flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500">
        {icon} {label}
      </p>
      <p className="mt-1 font-heading font-bold text-gray-900 dark:text-white">{value}</p>
    </div>
  );
}
