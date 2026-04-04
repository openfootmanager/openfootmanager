import {
    autoSelectSetPieces,
    changeMatchFormation,
    changeMatchPlayStyle,
    type SetPieceRole,
    setMatchSetPieceTaker,
    swapPreMatchPlayers,
} from "../../services/liveMatchService";
import type { EnginePlayerData, MatchSnapshot } from "./types";
import {
    applyPlannedPreMatchSwaps,
    applySetPieceAssignments,
    getAutoSelectedSetPieceAssignments,
    planAutoSelectSwaps,
} from "./preMatchSetupUtils";

type MatchSide = "Home" | "Away";
type SnapshotUpdater = (snapshot: MatchSnapshot) => void;

async function runSnapshotAction(
    action: () => Promise<MatchSnapshot>,
    onUpdateSnapshot: SnapshotUpdater,
    errorMessage: string,
): Promise<MatchSnapshot | null> {
    try {
        const nextSnapshot = await action();
        onUpdateSnapshot(nextSnapshot);
        return nextSnapshot;
    } catch (error) {
        console.error(errorMessage, error);
        return null;
    }
}

export async function applyPreMatchFormationChange(
    side: MatchSide,
    formation: string,
    onUpdateSnapshot: SnapshotUpdater,
): Promise<MatchSnapshot | null> {
    return runSnapshotAction(
        () => changeMatchFormation(side, formation),
        onUpdateSnapshot,
        "Formation change failed:",
    );
}

export async function applyPreMatchPlayStyleChange(
    side: MatchSide,
    playStyle: string,
    onUpdateSnapshot: SnapshotUpdater,
): Promise<MatchSnapshot | null> {
    return runSnapshotAction(
        () => changeMatchPlayStyle(side, playStyle),
        onUpdateSnapshot,
        "Play style change failed:",
    );
}

export async function applyPreMatchSwap(
    side: MatchSide,
    selectedStarterId: string | null,
    benchPlayerId: string,
    onUpdateSnapshot: SnapshotUpdater,
): Promise<MatchSnapshot | null> {
    if (!selectedStarterId) {
        return null;
    }

    return runSnapshotAction(
        () => swapPreMatchPlayers(side, selectedStarterId, benchPlayerId),
        onUpdateSnapshot,
        "Pre-match swap failed:",
    );
}

export async function applyPreMatchSetPieceTaker(
    side: MatchSide,
    role: SetPieceRole,
    playerId: string,
    onUpdateSnapshot: SnapshotUpdater,
): Promise<MatchSnapshot | null> {
    return runSnapshotAction(
        () => setMatchSetPieceTaker(side, role, playerId),
        onUpdateSnapshot,
        "Set piece taker change failed:",
    );
}

export async function autoAssignPreMatchSetPieces(
    side: MatchSide,
    playerIds: string[],
    onUpdateSnapshot: SnapshotUpdater,
): Promise<MatchSnapshot | null> {
    try {
        const selection = await autoSelectSetPieces(playerIds);

        return await applySetPieceAssignments(
            getAutoSelectedSetPieceAssignments(selection),
            (assignment) =>
                setMatchSetPieceTaker(side, assignment.role, assignment.playerId),
            onUpdateSnapshot,
        );
    } catch (error) {
        console.error("Auto-select set pieces failed:", error);
        return null;
    }
}

export async function autoSelectPreMatchLineup(
    side: MatchSide,
    startingPlayers: EnginePlayerData[],
    benchPlayers: EnginePlayerData[],
    formationNeeds: Record<string, number>,
    onUpdateSnapshot: SnapshotUpdater,
): Promise<MatchSnapshot | null> {
    try {
        const swaps = planAutoSelectSwaps(
            startingPlayers,
            benchPlayers,
            formationNeeds,
        );

        return await applyPlannedPreMatchSwaps(
            swaps,
            (swap) => swapPreMatchPlayers(side, swap.playerOffId, swap.playerOnId),
            onUpdateSnapshot,
        );
    } catch (error) {
        console.error("Auto-select failed:", error);
        return null;
    }
}