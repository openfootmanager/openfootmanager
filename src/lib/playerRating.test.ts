import { describe, expect, it } from "vitest";
import type { PlayerData } from "../store/gameStore";
import { calcOvr, canonicalPosition } from "./playerRating";
import {
    CORE_POSITIONS,
    getPreferredPositions,
    normalisePosition,
    positionCode,
} from "./playerPositions";

function makePlayer(overrides: Partial<PlayerData> = {}): PlayerData {
    return {
        id: "player-1",
        match_name: "Player One",
        full_name: "Player One",
        date_of_birth: "2000-01-01",
        nationality: "GB",
        position: "CentralMidfielder",
        natural_position: "CentralMidfielder",
        alternate_positions: [],
        footedness: "Right",
        weak_foot: 2,
        training_focus: null,
        attributes: {
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 70,
            shooting: 55,
            tackling: 60,
            dribbling: 64,
            defending: 58,
            positioning: 62,
            vision: 68,
            decisions: 66,
            composure: 60,
            aggression: 50,
            teamwork: 64,
            leadership: 50,
            handling: 20,
            reflexes: 20,
            aerial: 45,
        },
        condition: 100,
        morale: 80,
        injury: null,
        team_id: "team-1",
        contract_end: "2028-06-30",
        wage: 1000,
        market_value: 1000000,
        stats: {
            appearances: 0,
            goals: 0,
            assists: 0,
            clean_sheets: 0,
            yellow_cards: 0,
            red_cards: 0,
            avg_rating: 0,
            minutes_played: 0,
        },
        career: [],
        transfer_listed: false,
        loan_listed: false,
        transfer_offers: [],
        traits: [],
        ...overrides,
    };
}

describe("playerPositions", () => {
    it("normalises aliases and exposes core position constants", () => {
        expect(canonicalPosition("Center Back")).toBe("CenterBack");
        expect(normalisePosition("Winger")).toBe("Forward");
        expect(positionCode("Attacking Midfielder")).toBe("AM");
        expect(CORE_POSITIONS).toEqual([
            "Goalkeeper",
            "Defender",
            "Midfielder",
            "Forward",
        ]);
    });

    it("builds preferred positions from natural and alternate roles", () => {
        const player = makePlayer({
            position: "Center Back",
            natural_position: "Center Back",
            alternate_positions: ["Right Wing Back", "Defensive Midfielder"],
        });

        expect(getPreferredPositions(player)).toEqual([
            "CenterBack",
            "RightWingBack",
            "DefensiveMidfielder",
        ]);
    });
});

describe("calcOvr", () => {
    it("applies smaller penalty to alternate positions than unrelated roles", () => {
        const player = makePlayer({
            position: "Striker",
            natural_position: "Striker",
            alternate_positions: ["RightWinger"],
            attributes: {
                ...makePlayer().attributes,
                shooting: 74,
                positioning: 72,
                decisions: 70,
                pace: 72,
                dribbling: 69,
            },
        });

        expect(calcOvr(player, "RightWinger")).toBeGreaterThan(calcOvr(player, "CenterBack"));
    });

    it("penalises wrong-footed wide players", () => {
        const player = makePlayer({
            position: "LeftBack",
            natural_position: "LeftBack",
            footedness: "Right",
            weak_foot: 1,
            attributes: {
                ...makePlayer().attributes,
                pace: 74,
                stamina: 73,
                tackling: 71,
                defending: 70,
                positioning: 69,
            },
        });

        expect(calcOvr(player, "LeftBack")).toBeLessThan(calcOvr({ ...player, footedness: "Left" }, "LeftBack"));
    });
});