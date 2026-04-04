import type { PlayerData } from "../../store/gameStore";
import { normalisePosition } from "../squad/SquadTab.helpers";

export const ATTRIBUTE_GROUPS: {
    labelKey: string;
    attrs: Array<keyof PlayerData["attributes"]>;
}[] = [
        {
            labelKey: "common.attrGroups.physical",
            attrs: ["pace", "stamina", "strength", "agility"],
        },
        {
            labelKey: "common.attrGroups.technical",
            attrs: ["passing", "shooting", "tackling", "dribbling", "defending"],
        },
        {
            labelKey: "common.attrGroups.mental",
            attrs: [
                "positioning",
                "vision",
                "decisions",
                "composure",
                "aggression",
                "teamwork",
                "leadership",
            ],
        },
        {
            labelKey: "common.attrGroups.goalkeeper",
            attrs: ["handling", "reflexes", "aerial"],
        },
    ];

export function valueTone(value: number): string {
    if (value >= 80) {
        return "text-success-500 dark:text-success-400";
    }

    if (value >= 65) {
        return "text-primary-500 dark:text-primary-400";
    }

    if (value >= 50) {
        return "text-accent-500 dark:text-accent-400";
    }

    return "text-gray-500 dark:text-gray-400";
}

export function valueBarTone(value: number): string {
    if (value >= 80) {
        return "bg-success-500";
    }

    if (value >= 65) {
        return "bg-primary-500";
    }

    if (value >= 50) {
        return "bg-accent-500";
    }

    return "bg-gray-300 dark:bg-navy-600";
}

export function getNormalizedPlayerPosition(player: PlayerData): string {
    return normalisePosition(player.natural_position || player.position);
}

export function getVisibleAttributeGroups(players: PlayerData[]): typeof ATTRIBUTE_GROUPS {
    const showGoalkeeperAttrs = players.some(
        (player) => getNormalizedPlayerPosition(player) === "Goalkeeper",
    );

    return ATTRIBUTE_GROUPS.filter(
        (group) =>
            group.labelKey !== "common.attrGroups.goalkeeper" || showGoalkeeperAttrs,
    );
}

export function getAttributeComparisonState(left: number, right: number): {
    leftWins: boolean;
    rightWins: boolean;
} {
    return {
        leftWins: left > right,
        rightWins: right > left,
    };
}