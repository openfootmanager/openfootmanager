import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { TeamData } from "../store/gameStore";
import { formatVal } from "../lib/helpers";
import { Badge, Card, CardBody, TeamLocation, TeamLogo } from "../components/ui";
import { Landmark, Star, Trophy, Users } from "lucide-react";

interface TeamGroup {
  id: string;
  name: string;
  order: number;
  teams: TeamData[];
}

interface TeamSelectionGridProps {
  clubSearch: string;
  onClubSearchChange: (value: string) => void;
  filteredTeamsCount: number;
  teamGroups: TeamGroup[];
  selectedTeamId: string | null;
  onSelectTeam: (teamId: string) => void;
  getTeamAvgOvr: (teamId: string) => number;
  getTeamPlayerCount: (teamId: string) => number;
}

export default function TeamSelectionGrid({
  clubSearch,
  onClubSearchChange,
  filteredTeamsCount,
  teamGroups,
  selectedTeamId,
  onSelectTeam,
  getTeamAvgOvr,
  getTeamPlayerCount,
}: TeamSelectionGridProps) {
  const { t, i18n } = useTranslation();

  const getReputationLabel = (
    rep: number,
  ): {
    label: string;
    variant: "primary" | "accent" | "success" | "danger" | "neutral";
  } => {
    if (rep >= 750) return { label: t("teamSelect.repWorldClass"), variant: "accent" };
    if (rep >= 600) return { label: t("teamSelect.repStrong"), variant: "success" };
    if (rep >= 400) return { label: t("teamSelect.repAverage"), variant: "neutral" };
    return { label: t("teamSelect.repDeveloping"), variant: "danger" };
  };

  return (
    <div>
      <div className="mb-3 flex flex-wrap items-center gap-3">
        <input
          type="text"
          value={clubSearch}
          onChange={(event) => onClubSearchChange(event.target.value)}
          placeholder={t("teamSelect.searchClubs")}
          className="min-w-0 flex-1 rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm text-gray-700 placeholder:text-gray-400 dark:border-navy-600 dark:bg-navy-800 dark:text-gray-200"
        />
        <span className="shrink-0 text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
          {t("teamSelect.clubCount", { n: filteredTeamsCount })}
        </span>
      </div>
      {filteredTeamsCount === 0 ? (
        <p className="py-10 text-center text-sm text-gray-500 dark:text-gray-400">
          {t("teamSelect.noClubsMatch")}
        </p>
      ) : (
        <div className="max-h-[640px] space-y-5 overflow-y-auto pr-1">
          {teamGroups.map((group) => (
            <div key={group.id}>
              <p className="mb-2 text-xs font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
                {group.name}
              </p>
              <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                {group.teams.map((team) => {
                  const isSelected = selectedTeamId === team.id;
                  const avgOvr = getTeamAvgOvr(team.id);
                  const repInfo = getReputationLabel(team.reputation);
                  const playerCount = getTeamPlayerCount(team.id);

                  return (
                    <button
                      key={team.id}
                      type="button"
                      onClick={() => onSelectTeam(team.id)}
                      className={`rounded-xl text-left transition-all duration-200 ${
                        isSelected
                          ? "scale-[1.01] ring-2 ring-primary-500 ring-offset-2 dark:ring-offset-navy-900"
                          : "hover:scale-[1.01]"
                      }`}
                    >
                      <Card accent={isSelected ? "primary" : "none"} className="h-full">
                        <div
                          className={`rounded-t-xl p-4 ${
                            isSelected
                              ? "bg-gradient-to-r from-primary-600 to-primary-700"
                              : "bg-gradient-to-r from-navy-700 to-navy-800"
                          }`}
                        >
                          <div className="flex items-center justify-between">
                            <div className="flex items-center gap-3">
                              <TeamLogo
                                team={team}
                                className={`flex h-12 w-12 items-center justify-center overflow-hidden rounded-lg font-heading text-lg font-bold ${
                                  isSelected ? "bg-white/20 text-white" : "bg-white/10 text-gray-300"
                                }`}
                              />
                              <div>
                                <h3 className="font-heading text-sm font-bold uppercase tracking-wide text-white">
                                  {team.name}
                                </h3>
                                <TeamLocation
                                  city={team.city}
                                  countryCode={team.country}
                                  locale={i18n.language}
                                  className="mt-0.5 text-xs text-gray-300"
                                  iconClassName="w-3 h-3"
                                  flagClassName="text-xs leading-none"
                                />
                              </div>
                            </div>
                            {isSelected && <Star className="h-5 w-5 fill-current text-accent-400" />}
                          </div>
                        </div>

                        <CardBody className="p-4">
                          <div className="grid grid-cols-2 gap-3">
                            <InfoStat
                              icon={<Trophy className="h-3.5 w-3.5" />}
                              label={t("teamSelect.reputation")}
                              value={
                                <Badge variant={repInfo.variant} size="sm">
                                  {repInfo.label}
                                </Badge>
                              }
                            />
                            <InfoStat
                              icon={<Users className="h-3.5 w-3.5" />}
                              label={t("teamSelect.squad")}
                              value={
                                <span className="font-heading font-bold text-gray-800 dark:text-gray-200">
                                  {playerCount}
                                </span>
                              }
                            />
                            <InfoStat
                              icon={<Landmark className="h-3.5 w-3.5" />}
                              label={t("teamSelect.finances")}
                              value={
                                <span className="font-heading font-bold text-gray-800 dark:text-gray-200">
                                  {formatVal(team.finance)}
                                </span>
                              }
                            />
                            <InfoStat
                              icon={<Star className="h-3.5 w-3.5" />}
                              label={t("teamSelect.avgOvr")}
                              value={
                                <span className="font-heading text-lg font-bold text-primary-500">
                                  {avgOvr}
                                </span>
                              }
                            />
                          </div>
                        </CardBody>
                      </Card>
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function InfoStat({
  icon,
  label,
  value,
}: {
  icon: ReactNode;
  label: string;
  value: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1">
      <span className="flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500">
        {icon} {label}
      </span>
      {value}
    </div>
  );
}
