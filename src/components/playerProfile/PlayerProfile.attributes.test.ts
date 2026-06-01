import { describe, expect, it } from "vitest";
import type { PlayerData } from "../../store/gameStore";
import { buildPlayerAttributeGroups } from "./PlayerProfile.attributes";

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
    return {
        id: "player-1",
        match_name: "J. Smith",
        full_name: "John Smith",
        date_of_birth: "2000-01-01",
        nationality: "GB",
        position: "Forward",
        natural_position: "Forward",
        alternate_positions: [],
        training_focus: null,
        attributes: {
            pace: 60,
            stamina: 61,
            strength: 62,
            agility: 63,
            passing: 64,
            shooting: 65,
            tackling: 66,
            dribbling: 67,
            defending: 68,
            positioning: 69,
            vision: 70,
            decisions: 71,
            composure: 72,
            aggression: 73,
            teamwork: 74,
            leadership: 75,
            handling: 76,
            reflexes: 77,
            aerial: 78,
        },
        condition: 80,
        morale: 75,
        injury: null,
        team_id: "team-1",
        retired: false,
        contract_end: "2026-10-15",
        wage: 12000,
        market_value: 350000,
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

describe("PlayerProfile.attributes", () => {
    it("builds the standard outfield attribute groups with averages", () => {
        const groups = buildPlayerAttributeGroups(createPlayer(), (key) => key);

        expect(groups.map((group) => group.label)).toEqual([
            "common.attrGroups.physical",
            "common.attrGroups.technical",
            "common.attrGroups.mental",
        ]);
        expect(groups[0]?.attrs).toHaveLength(4);
        expect(groups[0]?.average).toBe(62);
        expect(groups[1]?.attrs).toHaveLength(5);
        expect(groups[1]?.average).toBe(66);
        expect(groups[2]?.attrs).toHaveLength(7);
        expect(groups[2]?.average).toBe(72);
    });

    it("adds the goalkeeper-specific group for goalkeepers", () => {
        const groups = buildPlayerAttributeGroups(
            createPlayer({ position: "Goalkeeper", natural_position: "Goalkeeper" }),
            (key) => key,
        );

        expect(groups).toHaveLength(4);
        expect(groups[3]).toMatchObject({
            label: "common.attrGroups.goalkeeper",
            average: 77,
        });
        expect(groups[3]?.attrs.map((attr) => attr.name)).toEqual([
            "common.attributes.handling",
            "common.attributes.reflexes",
            "common.attributes.aerial",
        ]);
    });
});