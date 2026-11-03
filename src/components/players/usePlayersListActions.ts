import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

import { sendScout } from "../../services/scoutingService";
import { toggleLoanList, toggleTransferList } from "../../services/transfersService";
import type { PlayerSummary } from "../../services/playersService";
import type { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import type { ContextMenuItem } from "../ContextMenu";
import {
  buildDividerMenuItem,
  buildOfferFreeAgentContractMenuItem,
  buildMakeTransferBidMenuItem,
  buildScoutPlayerMenuItem,
  buildToggleLoanListMenuItem,
  buildToggleTransferListMenuItem,
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";
import { calculateAvailableScouts } from "../scouting/ScoutingTab.helpers";
import { buildAlreadyScoutingIds } from "../scouting/ScoutingTab.model";
import { useFreeAgentContractFlow } from "../transfers/useFreeAgentContractFlow";
import { useTransferBidFlow } from "../transfers/useTransferBidFlow";

interface UsePlayersListActionsArgs {
  gameState: GameStateData;
  onGameUpdate?: (game: GameStateData) => void;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onSelectTeam: (id: string) => void;
  /** Asks the list for its page again, after an action changed what it shows. */
  refetchSlice: () => void;
}

/**
 * What a player's row or card can do besides open him: the context menu's actions, and the
 * transfer-bid and free-agent-contract flows and the scouting assignment behind them. The table
 * and the cards share this, so both views offer the same actions.
 */
export function usePlayersListActions({
  gameState,
  onGameUpdate,
  onSelectPlayer,
  onSelectTeam,
  refetchSlice,
}: UsePlayersListActionsArgs) {
  const { t } = useTranslation();
  const [sendingPlayerId, setSendingPlayerId] = useState<string | null>(null);
  // The last scout that could not be sent, and the last listing that was not saved: two alerts
  // above the list, so one action succeeding never hides the other's failure.
  const [scoutError, setScoutError] = useState<string | null>(null);
  const [listingError, setListingError] = useState<string | null>(null);
  const managerTeamId = gameState.manager.team_id ?? "";

  const handleGameUpdate = useCallback(
    (game: GameStateData) => {
      onGameUpdate?.(game);
      refetchSlice();
    },
    [onGameUpdate, refetchSlice],
  );

  const findFullPlayer = useCallback(
    (id: string) => gameState.players.find((p) => p.id === id),
    [gameState.players],
  );
  const bidFlow = useTransferBidFlow({
    gameState,
    onGameUpdate: handleGameUpdate,
  });
  const contractFlow = useFreeAgentContractFlow({
    gameState,
    onGameUpdate: handleGameUpdate,
  });
  const scouts = gameState.staff.filter(
    (staffMember) => staffMember.role === "Scout" && staffMember.team_id === managerTeamId,
  );
  const scoutingAssignments = gameState.scouting_assignments || [];
  const allScoutingAssignments = [
    ...scoutingAssignments,
    ...(gameState.youth_scouting_assignments || []),
  ];
  const availableScouts = calculateAvailableScouts(scouts, allScoutingAssignments);
  const alreadyScoutingIds = buildAlreadyScoutingIds(scoutingAssignments);

  const handleScoutPlayer = async (playerId: string): Promise<void> => {
    if (availableScouts.length === 0) {
      setScoutError(null);
      return;
    }

    const scout = availableScouts[0];
    setScoutError(null);
    setSendingPlayerId(playerId);

    try {
      const updated = await sendScout(scout.id, playerId);
      setScoutError(null);
      handleGameUpdate(updated);
    } catch (error) {
      console.error("Failed to send scout:", error);
      setScoutError(resolveTranslatedErrorMessage(getErrorMessage(error), t));
    } finally {
      setSendingPlayerId(null);
    }
  };

  const toggleListing = async (toggle: () => Promise<GameStateData>): Promise<void> => {
    setListingError(null);
    try {
      handleGameUpdate(await toggle());
    } catch (error) {
      console.error("Failed to change the player's listing:", error);
      setListingError(resolveTranslatedErrorMessage(getErrorMessage(error), t));
    }
  };

  const contextItemsFor = (summary: PlayerSummary): ContextMenuItem[] => {
    const scoutState = alreadyScoutingIds.has(summary.id)
      ? "already-assigned"
      : sendingPlayerId === summary.id
        ? "busy"
        : availableScouts.length === 0
          ? "unavailable"
          : "ready";
    const contextItems = [
      buildViewProfileMenuItem(t, () => onSelectPlayer(summary.id)),
      ...(summary.team_id
        ? [
            buildViewTeamMenuItem(t, () => {
              onSelectTeam(summary.team_id!);
            }),
          ]
        : []),
    ];

    if (summary.team_id === managerTeamId) {
      contextItems.push(buildDividerMenuItem());
      contextItems.push(
        buildToggleTransferListMenuItem(t, summary.transfer_listed, () =>
          toggleListing(() => toggleTransferList(summary.id)),
        ),
      );
      contextItems.push(
        buildToggleLoanListMenuItem(t, summary.loan_listed, () =>
          toggleListing(() => toggleLoanList(summary.id)),
        ),
      );
    } else {
      const playerActions = summary.team_id
        ? [
            buildMakeTransferBidMenuItem(t, () => {
              const full = findFullPlayer(summary.id);
              if (full) bidFlow.openBidNegotiation(full);
            }),
            buildScoutPlayerMenuItem(t, scoutState, () => {
              void handleScoutPlayer(summary.id);
            }),
          ]
        : summary.retired
          ? []
          : [
              buildOfferFreeAgentContractMenuItem(t, () => {
                const full = findFullPlayer(summary.id);
                if (full) contractFlow.openFreeAgentContract(full);
              }),
              buildScoutPlayerMenuItem(t, scoutState, () => {
                void handleScoutPlayer(summary.id);
              }),
            ];

      if (playerActions.length > 0) {
        contextItems.push(buildDividerMenuItem());
        contextItems.push(...playerActions);
      }
    }
    return contextItems;
  };

  return { contextItemsFor, scoutError, listingError, bidFlow, contractFlow };
}
