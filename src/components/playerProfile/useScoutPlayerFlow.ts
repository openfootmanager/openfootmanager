import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";

import type { GameStateData, PlayerData } from "../../store/gameStore";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import {
  getScoutAvailability,
  type PlayerProfileScoutStatus,
  type ScoutAvailability,
} from "./PlayerProfile.scouting";

interface UseScoutPlayerFlowArgs {
  player: PlayerData;
  gameState: GameStateData;
  onGameUpdate?: (game: GameStateData) => void;
}

interface UseScoutPlayerFlowResult {
  scoutAvailability: ScoutAvailability;
  scoutStatus: PlayerProfileScoutStatus;
  scoutError: string | null;
  sendScout: () => void;
}

/**
 * Sending a scout to watch this player.
 *
 * `scoutAvailability` is derived from the current status as well as the squad,
 * so the two travel together — a scout already on their way is not available
 * to send again.
 */
export function useScoutPlayerFlow({
  player,
  gameState,
  onGameUpdate,
}: UseScoutPlayerFlowArgs): UseScoutPlayerFlowResult {
  const { t } = useTranslation();
  const [scoutStatus, setScoutStatus] =
    useState<PlayerProfileScoutStatus>("idle");
  const [scoutError, setScoutError] = useState<string | null>(null);

  const scoutAvailability = getScoutAvailability({
    staff: gameState.staff,
    scoutingAssignments: gameState.scouting_assignments || [],
    youthScoutingAssignments: gameState.youth_scouting_assignments || [],
    managerTeamId: gameState.manager.team_id,
    playerId: player.id,
    scoutStatus,
  });

  function sendScout(): void {
    const availableScout = scoutAvailability.availableScout;
    if (!availableScout || !onGameUpdate) {
      return;
    }

    void (async () => {
      setScoutStatus("sending");
      setScoutError(null);

      try {
        const updated = await invoke<GameStateData>("send_scout", {
          scoutId: availableScout.id,
          playerId: player.id,
        });
        onGameUpdate(updated);
        setScoutStatus("sent");
      } catch (err) {
        setScoutError(resolveTranslatedErrorMessage(err, t));
        setScoutStatus("error");
      }
    })();
  }

  return { scoutAvailability, scoutStatus, scoutError, sendScout };
}
