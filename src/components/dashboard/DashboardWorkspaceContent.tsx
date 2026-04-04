import { lazy, Suspense } from "react";
import type { GameStateData } from "../../store/gameStore";
import DashboardAlerts from "./DashboardAlerts";
import type { DashboardAlert } from "./dashboardHelpers";
import type { DashboardProfileNavigationState } from "./dashboardProfileNavigation";
import type { DashboardTabContentModel } from "./dashboardTabContentModel";

const PlayerProfile = lazy(() => import("../playerProfile/PlayerProfile"));
const TeamProfile = lazy(() => import("../teamProfile"));
const DashboardTabContent = lazy(() => import("./DashboardTabContent"));

function DashboardWorkspaceFallback() {
  return (
    <div className="flex min-h-[18rem] items-center justify-center rounded-2xl border border-gray-200/70 bg-white/70 dark:border-navy-700 dark:bg-navy-800/60">
      <div className="h-8 w-8 rounded-full border-4 border-primary-500 border-t-transparent animate-spin" />
    </div>
  );
}

interface DashboardWorkspaceContentProps {
  dashboardAlerts: DashboardAlert[];
  gameState: GameStateData;
  profileNavigation: DashboardProfileNavigationState;
  dashboardTabContentModel: DashboardTabContentModel;
  onBack: () => void;
  onNavigate: (tab: string) => void;
  onSelectPlayer: (id: string) => void;
  onSelectTeam: (id: string) => void;
  onGameUpdate: (state: GameStateData) => void;
}

export default function DashboardWorkspaceContent({
  dashboardAlerts,
  gameState,
  profileNavigation,
  dashboardTabContentModel,
  onBack,
  onNavigate,
  onSelectPlayer,
  onSelectTeam,
  onGameUpdate,
}: DashboardWorkspaceContentProps) {
  const selectedPlayer = profileNavigation.selectedPlayerId
    ? gameState.players.find(
      (player) => player.id === profileNavigation.selectedPlayerId,
    ) ?? null
    : null;
  const selectedTeam = profileNavigation.selectedTeamId
    ? gameState.teams.find((team) => team.id === profileNavigation.selectedTeamId) ??
    null
    : null;

  let workspaceContent = null;

  if (selectedPlayer && !selectedTeam) {
    workspaceContent = (
      <PlayerProfile
        player={selectedPlayer}
        gameState={gameState}
        isOwnClub={selectedPlayer.team_id === gameState.manager.team_id}
        startWithRenewalModal={
          profileNavigation.selectedPlayerOptions?.openRenewal === true
        }
        onClose={onBack}
        onSelectTeam={onSelectTeam}
        onGameUpdate={onGameUpdate}
      />
    );
  } else if (selectedTeam) {
    workspaceContent = (
      <TeamProfile
        team={selectedTeam}
        gameState={gameState}
        isOwnTeam={selectedTeam.id === gameState.manager.team_id}
        onClose={onBack}
        onSelectPlayer={onSelectPlayer}
      />
    );
  } else {
    workspaceContent = <DashboardTabContent viewModel={dashboardTabContentModel} />;
  }

  return (
    <div className="flex-1 overflow-auto p-6 bg-gray-100 dark:bg-navy-900">
      {!selectedPlayer && !selectedTeam ? (
        <DashboardAlerts alerts={dashboardAlerts} onNavigate={onNavigate} />
      ) : null}

      <Suspense fallback={<DashboardWorkspaceFallback />}>
        {workspaceContent}
      </Suspense>
    </div>
  );
}