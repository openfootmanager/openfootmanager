import { roundPositionScore } from "../../lib/playerRatingScore";
import type { EnginePlayerData } from "./types";

export const POSITION_KEY_STATS: Record<
    string,
    { label: string; key: string }[]
> = {
    Goalkeeper: [
        { label: "HAN", key: "handling" },
        { label: "REF", key: "reflexes" },
        { label: "AER", key: "aerial" },
    ],
    Defender: [
        { label: "DEF", key: "defending" },
        { label: "TAC", key: "tackling" },
        { label: "STR", key: "strength" },
    ],
    Midfielder: [
        { label: "PAS", key: "passing" },
        { label: "VIS", key: "vision" },
        { label: "STA", key: "stamina" },
    ],
    Forward: [
        { label: "SHO", key: "shooting" },
        { label: "PAC", key: "pace" },
        { label: "DRI", key: "dribbling" },
    ],
};

export function getPositionOvr(player: EnginePlayerData): number {
    return roundPositionScore(
        {
            pace: player.pace,
            stamina: player.stamina,
            strength: player.strength,
            agility: player.agility,
            passing: player.passing,
            shooting: player.shooting,
            tackling: player.tackling,
            dribbling: player.dribbling,
            defending: player.defending,
            positioning: player.positioning,
            vision: player.vision,
            decisions: player.decisions,
            composure: player.composure,
            aggression: player.aggression,
            teamwork: player.teamwork,
            leadership: player.leadership,
            handling: player.handling,
            reflexes: player.reflexes,
            aerial: player.aerial,
        },
        player.position,
    );
}

export function conditionTextColor(condition: number): string {
    if (condition >= 75) return "text-primary-400";
    if (condition >= 50) return "text-amber-400";
    return "text-red-400";
}

export function statColor(value: number): string {
    if (value >= 75) return "text-primary-400 font-bold";
    if (value >= 60) return "text-gray-200";
    return "text-gray-500";
}

export function getStatVal(player: EnginePlayerData, key: string): number {
    return (player as unknown as Record<string, number>)[key] ?? 0;
}

export function parseFormationNeeds(formation: string): Record<string, number> {
    const parts = formation
        .split("-")
        .map(Number)
        .filter((value) => !Number.isNaN(value));

    if (parts.length === 3) {
        return {
            Goalkeeper: 1,
            Defender: parts[0],
            Midfielder: parts[1],
            Forward: parts[2],
        };
    }

    if (parts.length === 4) {
        return {
            Goalkeeper: 1,
            Defender: parts[0],
            Midfielder: parts[1] + parts[2],
            Forward: parts[3],
        };
    }

    return { Goalkeeper: 1, Defender: 4, Midfielder: 4, Forward: 2 };
}