import { Trophy } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { CompetitionProgress } from "./TournamentsTab.helpers";
import { competitionDisplayName } from "../../lib/competitionName";
import { Card, Select } from "../ui";
import type { LeagueData, WorldCupChampionData } from "../../store/types";

interface TournamentsLeagueHeaderProps {
  league: LeagueData;
  activeCompetitions: LeagueData[];
  onSelectCompetition: (id: string) => void;
  participantCount: number;
  progress: CompetitionProgress;
  worldCupChampion: WorldCupChampionData | null;
}

/** The competition's name, its season so far, and the picker for switching. */
export default function TournamentsLeagueHeader({
  league,
  activeCompetitions,
  onSelectCompetition,
  participantCount,
  progress,
  worldCupChampion,
}: TournamentsLeagueHeaderProps) {
  const { t } = useTranslation();

  const stats = [
    {
      key: "progress",
      label: t("tournaments.progress"),
      value: `${progress.completedMatchdays}/${progress.totalMatchdays}`,
      className: "text-white",
    },
    {
      key: "matches",
      label: t("tournaments.matches"),
      value: progress.completedMatches,
      className: "text-white",
    },
    {
      key: "goals",
      label: t("tournaments.goals"),
      value: progress.totalGoals,
      className: "text-accent-400",
    },
  ];

  return (
    <Card accent="primary" className="mb-5">
      <div className="bg-gradient-to-r from-navy-700 to-navy-800 p-6 rounded-t-xl">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-center">
          <div className="w-14 h-14 rounded-xl bg-accent-500/20 flex items-center justify-center">
            <Trophy className="w-7 h-7 text-accent-400" />
          </div>
          <div className="flex-1">
            <h2 className="text-2xl font-heading font-bold text-white uppercase tracking-wider">
              {competitionDisplayName(league, t)}
            </h2>
            <p className="text-gray-400 text-sm mt-0.5">
              {t("schedule.season", { number: league.season })} —{" "}
              {t("tournaments.nTeams", { count: participantCount })}
            </p>
          </div>
          {activeCompetitions.length > 1 && (
            <Select
              value={league.id}
              onChange={(event) => onSelectCompetition(event.target.value)}
              variant="ghost"
              aria-label={t("common.competition")}
            >
              {activeCompetitions.map((competition) => (
                <option key={competition.id} value={competition.id}>
                  {competitionDisplayName(competition, t)}
                </option>
              ))}
            </Select>
          )}
          <div className="hidden md:flex gap-4">
            {stats.map((stat) => (
              <div
                key={stat.key}
                className="bg-white/5 rounded-xl px-4 py-2 text-center"
              >
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  {stat.label}
                </p>
                <p className={`font-heading font-bold text-lg ${stat.className}`}>
                  {stat.value}
                </p>
              </div>
            ))}
          </div>
        </div>
      </div>
      {worldCupChampion && (
        <div className="flex items-center gap-3 bg-accent-500/10 px-6 py-3 rounded-b-xl border-t border-accent-500/20">
          <Trophy className="w-5 h-5 text-accent-400 flex-shrink-0" />
          <span className="text-sm font-heading font-bold uppercase tracking-wider text-accent-300">
            {t("tournaments.worldCupChampion")}:
          </span>
          <span className="text-sm font-semibold text-white">
            {worldCupChampion.nation_name}
          </span>
        </div>
      )}
    </Card>
  );
}
