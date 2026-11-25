import { getDeployedPosition } from "../squad/SquadTab.helpers";
import { getRoleOptions } from "../../lib/playerRoles";
import { naturalPositionOf } from "../../lib/positions";
import { setPlayerRole as setPlayerRoleService } from "../../services/squadService";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import type { PlayerRole } from "../../store/types";
import { useFreeAgentContractFlow } from "../transfers/useFreeAgentContractFlow";
import { useTransferBidFlow } from "../transfers/useTransferBidFlow";
import { useContractActionsFlow } from "./useContractActionsFlow";
import { useContractRenewalFlow } from "./useContractRenewalFlow";
import { useInitialModalIntent } from "./useInitialModalIntent";
import { useScoutPlayerFlow } from "./useScoutPlayerFlow";

interface UsePlayerProfileActionsArgs {
  player: PlayerData;
  gameState: GameStateData;
  isManagerOwnedProfile: boolean;
  hasAssistantManager: boolean;
  startWithRenewalModal: boolean;
  startWithTerminationModal: boolean;
  onGameUpdate?: (game: GameStateData) => void;
}

/**
 * What the manager can do from a player's profile, and the state of each: scouting him, signing a
 * free agent, bidding for him, renewing, letting his contract run down or terminating it, and
 * setting his tactical role. The renewal and termination dialogs can be asked for on arrival, when
 * the player is reached from an inbox action.
 */
export function usePlayerProfileActions({
  player,
  gameState,
  isManagerOwnedProfile,
  hasAssistantManager,
  startWithRenewalModal,
  startWithTerminationModal,
  onGameUpdate,
}: UsePlayerProfileActionsArgs) {
  const scout = useScoutPlayerFlow({ player, gameState, onGameUpdate });
  const freeAgent = useFreeAgentContractFlow({ gameState, onGameUpdate });
  const bid = useTransferBidFlow({ gameState, onGameUpdate });
  const renewal = useContractRenewalFlow({ player, gameState, hasAssistantManager, onGameUpdate });
  const contract = useContractActionsFlow({ player, onGameUpdate });

  // Both modals can be asked for on arrival, when the player reaches this
  // profile from an inbox action. One shared reset key, so switching player
  // re-arms both.
  const modalIntentResetKey = `${player.id}|${startWithRenewalModal}|${startWithTerminationModal}`;
  useInitialModalIntent({
    requested: startWithRenewalModal,
    allowed: isManagerOwnedProfile,
    isOpen: renewal.showRenewalModal,
    open: renewal.openRenewalModal,
    resetKey: modalIntentResetKey,
  });
  useInitialModalIntent({
    requested: startWithTerminationModal,
    allowed: isManagerOwnedProfile,
    isOpen: contract.showTerminationModal,
    open: () => void contract.openTerminationModal(),
    resetKey: modalIntentResetKey,
  });

  const playerTeam = gameState.teams.find((team) => team.id === player.team_id);
  const currentTacticalRole: PlayerRole = playerTeam?.player_roles?.[player.id] ?? "Standard";
  // Offer roles for the deployed slot when the player is in the starting XI —
  // the backend validates set_player_role against it, so natural-position
  // roles would be rejected for an out-of-position starter (issue #272).
  const roleValidationPosition =
    (playerTeam && getDeployedPosition(playerTeam, player.id)) || naturalPositionOf(player);
  const tacticalRole = {
    value: currentTacticalRole,
    options: getRoleOptions(roleValidationPosition, currentTacticalRole),
    onChange: async (role: PlayerRole): Promise<void> => {
      if (!onGameUpdate) return;
      try {
        onGameUpdate(await setPlayerRoleService(player.id, role));
      } catch (error) {
        console.error("Failed to set player role:", error);
      }
    },
  };

  return { scout, freeAgent, bid, renewal, contract, tacticalRole, playerTeam };
}

export type PlayerProfileActions = ReturnType<typeof usePlayerProfileActions>;
