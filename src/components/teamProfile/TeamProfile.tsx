import { useTranslation } from "react-i18next";

import { usePlayerKnowledge } from "../../hooks/usePlayerKnowledge";
import { knowledgeRevision } from "../../lib/playerKnowledge";

import type { TeamProfileProps } from "./TeamProfile.types";
import TeamProfileAdvancedStatsCard from "./TeamProfileAdvancedStatsCard";
import TeamProfileClubDetailsCard from "./TeamProfileClubDetailsCard";
import TeamProfileHeroCard from "./TeamProfileHeroCard";
import TeamProfileHistoryCard from "./TeamProfileHistoryCard";
import TeamProfileLeagueStandingCard from "./TeamProfileLeagueStandingCard";
import TeamProfileRecentMatchesCard from "./TeamProfileRecentMatchesCard";
import TeamProfileRosterCard from "./TeamProfileRosterCard";
import TeamProfileSummaryCard from "./TeamProfileSummaryCard";
import { useClubLeagueStanding } from "./useClubLeagueStanding";
import { useTeamProfileStats } from "./useTeamProfileStats";
import { buildTeamProfileViewModel } from "./TeamProfile.viewModel";

export default function TeamProfile({
  team,
  gameState,
  isOwnTeam,
  onSelectPlayer,
}: TeamProfileProps) {
  const { t, i18n } = useTranslation();
  const { standing: leagueStanding } = useClubLeagueStanding(team.id, gameState.clock.current_date);
  const viewModel = buildTeamProfileViewModel(team, gameState, leagueStanding);
  const { teamStatsOverview, recentMatches } = useTeamProfileStats(team.id);
  const knowledgeOf = usePlayerKnowledge(
    viewModel.roster.map((player) => player.id),
    knowledgeRevision({
      careerToken: gameState.career_token,
      date: gameState.clock.current_date,
      playerClubId: team.id,
      managerClubId: gameState.manager.team_id,
      seasonStart: gameState.season_context?.season_start,
    }),
  );

  return (
    <div className="max-w-6xl mx-auto">
      <TeamProfileHeroCard team={team} viewModel={viewModel} locale={i18n.language} t={t} />

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
        <TeamProfileClubDetailsCard team={team} t={t} />
        <TeamProfileSummaryCard team={team} isOwnTeam={isOwnTeam} viewModel={viewModel} t={t} />
        <TeamProfileLeagueStandingCard standings={viewModel.standings} t={t} />

        {teamStatsOverview && <TeamProfileAdvancedStatsCard overview={teamStatsOverview} t={t} />}

        <TeamProfileRecentMatchesCard matches={recentMatches} t={t} />

        <TeamProfileRosterCard
          roster={viewModel.roster}
          knowledgeOf={knowledgeOf}
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
