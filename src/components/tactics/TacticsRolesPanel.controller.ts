import { setTeamMatchRoles } from "../../services/tacticsService";
import type {
    GameStateData,
    PlayerData,
    TeamMatchRolesData,
} from "../../store/types";
import { buildUpdatedMatchRoles } from "./TacticsRolesPanel.helpers";

type GameUpdater = (gameState: GameStateData) => void;

export async function persistTacticsMatchRoles(
    nextRoles: TeamMatchRolesData,
    onGameUpdate: GameUpdater,
): Promise<GameStateData | null> {
    try {
        const updated = await setTeamMatchRoles(nextRoles);
        onGameUpdate(updated);
        return updated;
    } catch (error) {
        console.error("Failed to set team match roles:", error);
        return null;
    }
}

export async function applyTacticsRoleSelection(
    startingPlayers: PlayerData[],
    effectiveRoles: TeamMatchRolesData,
    role: keyof TeamMatchRolesData,
    playerId: string,
    onGameUpdate: GameUpdater,
): Promise<GameStateData | null> {
    return persistTacticsMatchRoles(
        buildUpdatedMatchRoles(startingPlayers, effectiveRoles, role, playerId),
        onGameUpdate,
    );
}

export async function autoSelectTacticsAssignments(
    effectiveRoles: TeamMatchRolesData,
    onGameUpdate: GameUpdater,
): Promise<GameStateData | null> {
    return persistTacticsMatchRoles(effectiveRoles, onGameUpdate);
}