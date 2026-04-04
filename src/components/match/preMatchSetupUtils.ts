import type {
    AutoSelectedSetPieces,
    SetPieceRole,
} from "../../services/liveMatchService";
import type { EnginePlayerData, MatchSnapshot } from "./types";
import { getPositionOvr } from "./matchLineupUtils";

export interface PlannedPreMatchSwap {
    playerOffId: string;
    playerOnId: string;
}

export interface PlannedSetPieceAssignment {
    playerId: string;
    role: SetPieceRole;
}

type SnapshotCallback = (snapshot: MatchSnapshot) => void;

const AUTO_SELECTED_SET_PIECE_ORDER: ReadonlyArray<{
    key: keyof AutoSelectedSetPieces;
    role: SetPieceRole;
}> = [
        { key: "captain", role: "captain" },
        { key: "penalty_taker", role: "penalty" },
        { key: "free_kick_taker", role: "freekick" },
        { key: "corner_taker", role: "corner" },
    ];

function getConditionWeightedOvr(player: EnginePlayerData): number {
    return getPositionOvr(player) * (player.condition / 100);
}

export function planAutoSelectSwaps(
    startingPlayers: EnginePlayerData[],
    benchPlayers: EnginePlayerData[],
    formationNeeds: Record<string, number>,
): PlannedPreMatchSwap[] {
    const pool = [...startingPlayers, ...benchPlayers];
    const idealIds: string[] = [];
    const selectedIds = new Set<string>();

    for (const position of ["Goalkeeper", "Defender", "Midfielder", "Forward"]) {
        let selectedForPosition = 0;
        const candidates = pool
            .filter((player) => player.position === position)
            .sort(
                (leftPlayer, rightPlayer) =>
                    getConditionWeightedOvr(rightPlayer) - getConditionWeightedOvr(leftPlayer),
            );

        const needed = formationNeeds[position] || 0;
        for (const candidate of candidates) {
            if (selectedForPosition >= needed) {
                break;
            }

            if (selectedIds.has(candidate.id)) {
                continue;
            }

            idealIds.push(candidate.id);
            selectedIds.add(candidate.id);
            selectedForPosition += 1;
        }
    }

    if (idealIds.length < 11) {
        const rest = pool
            .filter((player) => !selectedIds.has(player.id))
            .sort(
                (leftPlayer, rightPlayer) =>
                    getConditionWeightedOvr(rightPlayer) - getConditionWeightedOvr(leftPlayer),
            );

        for (const player of rest) {
            if (idealIds.length >= 11) {
                break;
            }

            idealIds.push(player.id);
            selectedIds.add(player.id);
        }
    }

    const currentIds = new Set(startingPlayers.map((player) => player.id));
    const toAdd = idealIds.filter((id) => !currentIds.has(id));
    const toRemove = startingPlayers
        .map((player) => player.id)
        .filter((id) => !selectedIds.has(id));

    return toAdd.slice(0, toRemove.length).map((playerOnId, index) => ({
        playerOffId: toRemove[index],
        playerOnId,
    }));
}

export function getAutoSelectedSetPieceAssignments(
    selection: AutoSelectedSetPieces,
): PlannedSetPieceAssignment[] {
    return AUTO_SELECTED_SET_PIECE_ORDER.flatMap(({ key, role }) => {
        const playerId = selection[key];
        return playerId ? [{ playerId, role }] : [];
    });
}

export async function applyPlannedPreMatchSwaps(
    swaps: PlannedPreMatchSwap[],
    applySwap: (swap: PlannedPreMatchSwap) => Promise<MatchSnapshot>,
    onSnapshot?: SnapshotCallback,
): Promise<MatchSnapshot | null> {
    let latestSnapshot: MatchSnapshot | null = null;

    for (const swap of swaps) {
        latestSnapshot = await applySwap(swap);
        onSnapshot?.(latestSnapshot);
    }

    return latestSnapshot;
}

export async function applySetPieceAssignments(
    assignments: PlannedSetPieceAssignment[],
    applyAssignment: (
        assignment: PlannedSetPieceAssignment,
    ) => Promise<MatchSnapshot>,
    onSnapshot?: SnapshotCallback,
): Promise<MatchSnapshot | null> {
    let latestSnapshot: MatchSnapshot | null = null;

    for (const assignment of assignments) {
        latestSnapshot = await applyAssignment(assignment);
        onSnapshot?.(latestSnapshot);
    }

    return latestSnapshot;
}