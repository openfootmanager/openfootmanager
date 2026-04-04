import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { TeamProfileProps } from "./TeamProfile.types";
import TeamProfileAdvancedStatsCard from "./TeamProfileAdvancedStatsCard";
import TeamProfileClubDetailsCard from "./TeamProfileClubDetailsCard";
import TeamProfileHeroCard from "./TeamProfileHeroCard";
import TeamProfileHistoryCard from "./TeamProfileHistoryCard";
import TeamProfileLeagueStandingCard from "./TeamProfileLeagueStandingCard";
import TeamProfileRecentMatchesCard from "./TeamProfileRecentMatchesCard";
import TeamProfileRosterCard from "./TeamProfileRosterCard";
import TeamProfileSummaryCard from "./TeamProfileSummaryCard";
import { useTeamProfileStats } from "./useTeamProfileStats";
import { buildTeamProfileViewModel } from "./TeamProfile.viewModel";

export default function TeamProfile({
  team,
  gameState,
  isOwnTeam,
  onClose,
  onSelectPlayer,
}: TeamProfileProps) {
  const { t, i18n } = useTranslation();
  const weeklySuffix = t("finances.perWeekSuffix", "/wk");
  const viewModel = buildTeamProfileViewModel(team, gameState);
  const { teamStatsOverview, recentMatches } = useTeamProfileStats(team.id);

  return (
    <div className="max-w-6xl mx-auto">
      <button
        onClick={onClose}
        className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 transition-colors mb-4"
      >
        <ArrowLeft className="w-4 h-4" />
        <span className="font-heading font-bold uppercase tracking-wider">
          {t("common.back")}
        </span>
      </button>

      <TeamProfileHeroCard
        team={team}
        viewModel={viewModel}
        locale={i18n.language}
        t={t}
      />

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
        <TeamProfileClubDetailsCard team={team} t={t} />
        <TeamProfileSummaryCard
          team={team}
          isOwnTeam={isOwnTeam}
          viewModel={viewModel}
          weeklySuffix={weeklySuffix}
          t={t}
        />
        <TeamProfileLeagueStandingCard standings={viewModel.standings} t={t} />

        {teamStatsOverview && (
          <TeamProfileAdvancedStatsCard overview={teamStatsOverview} t={t} />
        )}

        <TeamProfileRecentMatchesCard matches={recentMatches} t={t} />

        <TeamProfileRosterCard
          currentDate={gameState.clock.current_date}
          roster={viewModel.roster}
          isOwnTeam={isOwnTeam}
          locale={i18n.language}
          t={t}
          onSelectPlayer={onSelectPlayer}
        />
        <TeamProfileHistoryCard history={team.history} t={t} />
      </div>
    </div>
  );
}