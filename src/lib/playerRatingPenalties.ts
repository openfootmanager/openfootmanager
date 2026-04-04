import type { PlayerData } from "../store/gameStore";
import { POSITION_CRITICAL_ATTRIBUTES } from "./playerRatingConfig";
import {
    canonicalPosition,
    exactPosition,
    positionGroup,
} from "./playerPositions";

export function primaryPosition(player: PlayerData): string {
    const preferred = canonicalPosition(player.natural_position || player.position);

    if (["Defender", "Midfielder", "Forward", "Goalkeeper"].includes(preferred)) {
        return exactPosition(player.position || preferred);
    }

    return exactPosition(preferred);
}

export function compatibilityPenalty(player: PlayerData, position: string): number {
    const exact = exactPosition(position);
    const primary = primaryPosition(player);
    if (primary === exact) {
        return 0;
    }

    const alternates = (player.alternate_positions || []).map(exactPosition);
    if (alternates.includes(exact)) {
        return 4;
    }

    if (positionGroup(primary) === positionGroup(exact)) {
        return 8;
    }

    return 14;
}

function sideForPosition(position: string): "Left" | "Right" | null {
    switch (exactPosition(position)) {
        case "LeftBack":
        case "LeftWingBack":
        case "LeftMidfielder":
        case "LeftWinger":
            return "Left";
        case "RightBack":
        case "RightWingBack":
        case "RightMidfielder":
        case "RightWinger":
            return "Right";
        default:
            return null;
    }
}

export function footednessPenalty(player: PlayerData, position: string): number {
    const side = sideForPosition(position);
    if (!side) {
        return 0;
    }

    const footedness = player.footedness || "Right";
    if (footedness === "Both" || footedness === side) {
        return 0;
    }

    const weakFoot = Math.max(1, Math.min(5, player.weak_foot ?? 2));
    return Math.max(0, 10 - weakFoot * 2);
}

export function criticalPenalty(player: PlayerData, position: string): number {
    const criticalAttributes = POSITION_CRITICAL_ATTRIBUTES[exactPosition(position)];
    if (!criticalAttributes || criticalAttributes.length === 0) {
        return 0;
    }

    const criticalMinimum = Math.min(
        ...criticalAttributes.map((attribute) => player.attributes[attribute]),
    );

    return criticalMinimum >= 45 ? 0 : (45 - criticalMinimum) * 0.6;
}