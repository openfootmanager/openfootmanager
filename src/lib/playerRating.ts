import type { PlayerData } from "../store/gameStore";
import {
    compatibilityPenalty,
    criticalPenalty,
    footednessPenalty,
    primaryPosition,
} from "./playerRatingPenalties";
import { weightedPositionScore } from "./playerRatingScore";
import { canonicalPosition, exactPosition } from "./playerPositions";

export { canonicalPosition } from "./playerPositions";

export function calcOvr(player: PlayerData, position?: string): number {
    const targetPosition = position ? exactPosition(position) : primaryPosition(player);
    const weightedScore = weightedPositionScore(player, targetPosition);
    const penalty = criticalPenalty(player, targetPosition);
    const fitPenalty = position ? compatibilityPenalty(player, targetPosition) : 0;
    const sidePenalty = position ? footednessPenalty(player, targetPosition) : 0;

    return Math.round(
        Math.max(1, Math.min(99, weightedScore - penalty - fitPenalty - sidePenalty)),
    );
}

export function positionBadgeVariant(pos: string): "accent" | "primary" | "success" | "danger" {
    switch (pos) {
        case "Goalkeeper":
            return "accent";
        case "Defender":
        case "RightBack":
        case "CenterBack":
        case "LeftBack":
        case "RightWingBack":
        case "LeftWingBack":
            return "primary";
        case "Midfielder":
        case "DefensiveMidfielder":
        case "CentralMidfielder":
        case "AttackingMidfielder":
        case "RightMidfielder":
        case "LeftMidfielder":
            return "success";
        case "Forward":
        case "RightWinger":
        case "LeftWinger":
        case "Striker":
            return "danger";
        default:
            return "primary";
    }
}
