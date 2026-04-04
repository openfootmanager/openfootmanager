import { describe, expect, it } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import {
    ATTRIBUTE_GROUPS,
    getAttributeComparisonState,
    getNormalizedPlayerPosition,
    getVisibleAttributeGroups,
    valueBarTone,
    valueTone,
} from "./TacticsPlayerFocusPanel.helpers";

function makePlayer(overrides: Partial<PlayerData> = {}): PlayerData {
    return {
        id: "p1",
        match_name: "Test",
        full_name: "Test Player",
        date_of_birth: "1998-01-01",
        nationality: "GB",
        position: "Center Back",
        natural_position: "Center Back",
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

describe("TacticsPlayerFocusPanel.helpers", () => {
    it("keeps the goalkeeper group hidden for outfield-only comparisons", () => {
        const groups = getVisibleAttributeGroups([
            makePlayer({ position: "Midfielder", natural_position: "Midfielder" }),
        ]);

        expect(groups).toEqual(
            ATTRIBUTE_GROUPS.filter(
                (group) => group.labelKey !== "common.attrGroups.goalkeeper",
            ),
        );
    });

    it("includes the goalkeeper group when any compared player is a goalkeeper", () => {
        const groups = getVisibleAttributeGroups([
            makePlayer({ position: "Midfielder", natural_position: "Midfielder" }),
            makePlayer({ position: "Goalkeeper", natural_position: "Goalkeeper" }),
        ]);

        expect(groups).toEqual(ATTRIBUTE_GROUPS);
    });

    it("normalizes detailed positions through the shared position helper", () => {
        expect(
            getNormalizedPlayerPosition(
                makePlayer({ position: "Center Back", natural_position: "Center Back" }),
            ),
        ).toBe("Defender");
    });

    it("returns symmetric comparison state for compared attributes", () => {
        expect(getAttributeComparisonState(70, 65)).toEqual({
            leftWins: true,
            rightWins: false,
        });
        expect(getAttributeComparisonState(65, 70)).toEqual({
            leftWins: false,
            rightWins: true,
        });
        expect(getAttributeComparisonState(70, 70)).toEqual({
            leftWins: false,
            rightWins: false,
        });
    });

    it("maps value tones and bar tones across threshold boundaries", () => {
        expect(valueTone(82)).toContain("text-success-500");
        expect(valueTone(68)).toContain("text-primary-500");
        expect(valueTone(55)).toContain("text-accent-500");
        expect(valueTone(40)).toContain("text-gray-500");

        expect(valueBarTone(82)).toBe("bg-success-500");
        expect(valueBarTone(68)).toBe("bg-primary-500");
        expect(valueBarTone(55)).toBe("bg-accent-500");
        expect(valueBarTone(40)).toBe("bg-gray-300 dark:bg-navy-600");
    });
});