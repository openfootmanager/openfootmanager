import {
    setFormation,
    setPlayStyle,
    setStartingXI,
} from "../../services/tacticsService";
import type { GameStateData } from "../../store/types";

type GameUpdater = (gameState: GameStateData) => void;

async function runTacticsUpdate(
    action: () => Promise<GameStateData>,
    onGameUpdate: GameUpdater,
    errorMessage: string,
): Promise<GameStateData | null> {
    try {
        const updated = await action();
        onGameUpdate(updated);
        return updated;
    } catch (error) {
        console.error(errorMessage, error);
        return null;
    }
}

export async function persistTacticsStartingXI(
    playerIds: string[],
    onGameUpdate: GameUpdater,
    setPendingStartingXiIds: (playerIds: string[] | null) => void,
): Promise<GameStateData | null> {
    setPendingStartingXiIds(playerIds);

    try {
        const updated = await setStartingXI(playerIds);
        onGameUpdate(updated);
        return updated;
    } catch (error) {
        setPendingStartingXiIds(null);
        console.error("Failed to set starting XI:", error);
        return null;
    }
}

export async function applyTacticsFormationChange(
    formation: string,
    onGameUpdate: GameUpdater,
): Promise<GameStateData | null> {
    return runTacticsUpdate(
        () => setFormation(formation),
        onGameUpdate,
        "Failed to set formation:",
    );
}

export async function applyTacticsPlayStyleChange(
    playStyle: string,
    onGameUpdate: GameUpdater,
): Promise<GameStateData | null> {
    return runTacticsUpdate(
        () => setPlayStyle(playStyle),
        onGameUpdate,
        "Failed to set play style:",
    );
}