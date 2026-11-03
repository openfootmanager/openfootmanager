import type { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import { useRetiredPlayer } from "../playerProfile/useRetiredPlayer";
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
  onNavigate,
  onSelectPlayer,
  onSelectTeam,
  onGameUpdate,
  isUnemployed,
}: DashboardWorkspaceContentProps) {
  const { t } = useTranslation();
  const rosterPlayer = profileNavigation.selectedPlayerId
    ? (gameState.players.find((player) => player.id === profileNavigation.selectedPlayerId) ?? null)
    : null;
  const retiredPlayer = useRetiredPlayer(
    rosterPlayer === null ? profileNavigation.selectedPlayerId : null,
  );
  const selectedPlayer = rosterPlayer ?? retiredPlayer;
  const selectedTeam = profileNavigation.selectedTeamId
    ? (gameState.teams.find((team) => team.id === profileNavigation.selectedTeamId) ?? null)
    : null;

  return (
    // The workspace owns the page's scroll (layout rule L1); min-h-0 lets it shrink inside the column
    // so it scrolls rather than growing past the window.
    <div className="min-h-0 flex-1 overflow-auto bg-page p-6">
      {isUnemployed && (
        <div className="mb-4 flex items-center gap-3 rounded-lg border border-line bg-warning-tint px-4 py-3">
          <ShieldX className="h-5 w-5 shrink-0 text-warning" />
          <p className="text-body text-warning">{t("dashboard.unemployedBanner")}</p>
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
          startWithRenewalModal={profileNavigation.selectedPlayerOptions?.openRenewal === true}
          startWithTerminationModal={
            profileNavigation.selectedPlayerOptions?.openTermination === true
          }
          onSelectTeam={onSelectTeam}
          onGameUpdate={onGameUpdate}
        />
      ) : null}

      {selectedTeam ? (
        <TeamProfile
          team={selectedTeam}
          gameState={gameState}
          isOwnTeam={selectedTeam.id === gameState.manager.team_id}
          onSelectPlayer={onSelectPlayer}
        />
      ) : null}

      {!selectedPlayer && !selectedTeam ? (
        <DashboardTabContent viewModel={dashboardTabContentModel} />
      ) : null}
    </div>
  );
}
