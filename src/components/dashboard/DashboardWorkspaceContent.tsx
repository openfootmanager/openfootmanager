import type { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import PlayerProfile from "../playerProfile/PlayerProfile";
import TeamProfile from "../teamProfile";
import DashboardAlerts from "./DashboardAlerts";
import type { DashboardAlert } from "./dashboardHelpers";
import type { DashboardProfileNavigationState } from "./dashboardProfileNavigation";
import DashboardTabContent from "./DashboardTabContent";
import type { DashboardTabContentModel } from "./dashboardTabContentModel";
import { ShieldX } from "lucide-react";
import { useTranslation } from "react-i18next";

interface DashboardWorkspaceContentProps {
  dashboardAlerts: DashboardAlert[];
  gameState: GameStateData;
  profileNavigation: DashboardProfileNavigationState;
  dashboardTabContentModel: DashboardTabContentModel;
  onBack: () => void;
  onNavigate: (tab: string) => void;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onSelectTeam: (id: string) => void;
  onGameUpdate: (state: GameStateData) => void;
  isUnemployed: boolean;
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
  isUnemployed,
}: DashboardWorkspaceContentProps) {
  const { t } = useTranslation();
  const selectedPlayer = profileNavigation.selectedPlayerId
    ? gameState.players.find(
      (player) => player.id === profileNavigation.selectedPlayerId,
    ) ?? null
    : null;
  const selectedTeam = profileNavigation.selectedTeamId
    ? gameState.teams.find((team) => team.id === profileNavigation.selectedTeamId) ??
    null
    : null;

  return (
    <div className="flex-1 overflow-auto p-6 bg-gray-100 dark:bg-navy-900">
      {isUnemployed && (
        <div className="mx-6 mt-4 flex items-center gap-3 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 dark:border-amber-900/50 dark:bg-amber-950/30">
          <ShieldX className="h-5 w-5 shrink-0 text-amber-600 dark:text-amber-500" />
          <p className="text-sm text-amber-800 dark:text-amber-300">
            {t("dashboard.unemployedBanner")}
          </p>
        </div>
      )}

      {!selectedPlayer && !selectedTeam ? (
        <DashboardAlerts alerts={dashboardAlerts} onNavigate={onNavigate} />
      ) : null}

      {selectedPlayer && !selectedTeam ? (
        <PlayerProfile
          player={selectedPlayer}
          gameState={gameState}
          isOwnClub={selectedPlayer.team_id === gameState.manager.team_id}
          startWithRenewalModal={
            profileNavigation.selectedPlayerOptions?.openRenewal === true
          }
          startWithTerminationModal={
            profileNavigation.selectedPlayerOptions?.openTermination === true
          }
          onClose={onBack}
          onSelectTeam={onSelectTeam}
          onGameUpdate={onGameUpdate}
        />
      ) : null}

      {selectedTeam ? (
        <TeamProfile
          team={selectedTeam}
          gameState={gameState}
          isOwnTeam={selectedTeam.id === gameState.manager.team_id}
          onClose={onBack}
          onSelectPlayer={onSelectPlayer}
        />
      ) : null}

      {!selectedPlayer && !selectedTeam ? (
        <DashboardTabContent viewModel={dashboardTabContentModel} />
      ) : null}
    </div>
  );
}