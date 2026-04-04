import type { PlayerData } from "../store/gameStore";
import {
    DEFAULT_ATTRIBUTE_WEIGHTS,
    POSITION_ATTRIBUTE_WEIGHTS,
    type WeightedAttribute,
} from "./playerRatingConfig";
import { exactPosition } from "./playerPositions";

function weightedAverage(
    attributes: PlayerData["attributes"],
    weights: readonly WeightedAttribute[],
): number {
    return weights.reduce(
        (sum, [attribute, weight]) => sum + attributes[attribute] * weight,
        0,
    ) / 100;
}

export function weightedPositionScore(player: PlayerData, position: string): number {
    const weights =
        POSITION_ATTRIBUTE_WEIGHTS[exactPosition(position)] || DEFAULT_ATTRIBUTE_WEIGHTS;
    return weightedAverage(player.attributes, weights);
}