import { useState } from "react";
import type { GameStateData } from "../../store/gameStore";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import { toggleLoanList, toggleTransferList } from "../../services/transfersService";
import { sendScout } from "../../services/scoutingService";
import { calculateAvailableScouts } from "../scouting/ScoutingTab.helpers";
import { buildAlreadyScoutingIds } from "../scouting/ScoutingTab.model";
import type { TFunction } from "i18next";
interface Input {
  gameState: GameStateData;
  userTeamId: string | null;
  onGameUpdate: ((game: GameStateData) => void) | undefined;
  t: TFunction<"translation", undefined>;
}
export function useTransferActions({ gameState, userTeamId, onGameUpdate, t }: Input) {
  const [scoutingPlayerId, setScoutingPlayerId] = useState<string | null>(null);
  const [scoutError, setScoutError] = useState<string | null>(null);
  const [listingError, setListingError] = useState<string | null>(null);
  const scouts = gameState.staff.filter(
    (staffMember) => staffMember.role === "Scout" && staffMember.team_id === userTeamId,
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
    setScoutingPlayerId(playerId);

    try {
      const updated = await sendScout(scout.id, playerId);
      setScoutError(null);
      onGameUpdate?.(updated);
    } catch (error) {
      console.error("Failed to send scout:", error);
      setScoutError(resolveTranslatedErrorMessage(getErrorMessage(error), t));
    } finally {
      setScoutingPlayerId(null);
    }
  };
  const handleToggleTransferListing = async (playerId: string): Promise<void> => {
    setListingError(null);

    try {
      const updated = await toggleTransferList(playerId);
      setListingError(null);
      onGameUpdate?.(updated);
    } catch (error) {
      setListingError(resolveTranslatedErrorMessage(getErrorMessage(error), t));
    }
  };
  const handleToggleLoanListing = async (playerId: string): Promise<void> => {
    setListingError(null);

    try {
      const updated = await toggleLoanList(playerId);
      setListingError(null);
      onGameUpdate?.(updated);
    } catch (error) {
      setListingError(resolveTranslatedErrorMessage(getErrorMessage(error), t));
    }
  };
  return {
    scoutingPlayerId,
    scoutError,
    listingError,
    availableScouts,
    alreadyScoutingIds,
    handleScoutPlayer,
    handleToggleTransferListing,
    handleToggleLoanListing,
  };
}
