import { describe, expect, it } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import {
    getBenchPlayerButtonClassName,
    getEmptySlotClassName,
    getPitchPlayerButtonClassName,
    getPitchRatingClassName,
} from "./TacticsPitch.helpers";

function makePlayer(overrides: Partial<PlayerData> = {}): PlayerData {
    return {
        id: "p1",
        match_name: "Test",
        full_name: "Test Player",
        date_of_birth: "1998-01-01",
        nationality: "GB",
        position: "Defender",
        natural_position: "Defender",
        alternate_positions: [],
        training_focus: null,
        attributes: {
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 60,
            shooting: 60,
            tackling: 60,
            dribbling: 60,
            defending: 60,
            positioning: 60,
            vision: 60,
            decisions: 60,
            composure: 60,
            aggression: 60,
            teamwork: 60,
            leadership: 60,
            handling: 60,
            reflexes: 60,
            aerial: 60,
        },
        condition: 100,
        morale: 80,
        injury: null,
        team_id: "team1",
        contract_end: "2027-06-30",
        wage: 1000,
        market_value: 100000,
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

describe("TacticsPitch.helpers", () => {
    it("prioritizes the selected pitch player style", () => {
        expect(
            getPitchPlayerButtonClassName({
                dragState: null,
                comparePlayerId: "other",
                hoveredSlot: 2,
                player: makePlayer({ id: "p1" }),
                selectedPlayerId: "p1",
                slotIndex: 2,
                wrongPos: true,
            }),
        ).toContain("border-accent-300");
    });

    it("marks hovered empty slots with the primary hover style", () => {
        expect(getEmptySlotClassName(true)).toContain("border-primary-300");
        expect(getEmptySlotClassName(false)).toContain("border-white/20");
    });

    it("marks selected bench players with the accent style", () => {
        expect(
            getBenchPlayerButtonClassName({
                dragState: null,
                comparePlayerId: null,
                player: makePlayer({ id: "bench" }),
                selectedPlayerId: "bench",
            }),
        ).toContain("border-accent-300");
    });

    it("uses the wrong-position rating tone before condition tones", () => {
        expect(getPitchRatingClassName(makePlayer({ condition: 20 }), true)).toContain(
            "border-amber-200",
        );
    });

    it("uses the low-condition rating tone when the player is in position", () => {
        expect(
            getPitchRatingClassName(makePlayer({ condition: 20 }), false),
        ).toContain("border-red-200");
    });
});