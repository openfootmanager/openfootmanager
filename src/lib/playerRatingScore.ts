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

export function scorePositionAttributes(
    attributes: PlayerData["attributes"],
    position: string,
): number {
    const weights =
        POSITION_ATTRIBUTE_WEIGHTS[exactPosition(position)] || DEFAULT_ATTRIBUTE_WEIGHTS;

    return weightedAverage(attributes, weights);
}

export function roundPositionScore(
    attributes: PlayerData["attributes"],
    position: string,
): number {
    return Math.round(Math.max(1, Math.min(99, scorePositionAttributes(attributes, position))));
}

export function weightedPositionScore(player: PlayerData, position: string): number {
    return scorePositionAttributes(player.attributes, position);
}