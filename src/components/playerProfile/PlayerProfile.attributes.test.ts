import { describe, expect, it } from "vitest";
import type { PlayerData } from "../../store/gameStore";
import {
    buildPlayerAttributeGroups,
    getPlayerAttributeEntry,
} from "./PlayerProfile.attributes";

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
    it("builds the standard outfield groups with aerial in Physical", () => {
        const groups = buildPlayerAttributeGroups(createPlayer(), (key) => key);

        expect(groups.map((group) => group.label)).toEqual([
            "common.attrGroups.physical",
            "common.attrGroups.technical",
            "common.attrGroups.mental",
        ]);

        // Physical includes aerial for every player — the engine uses it for
        // every header duel, not just GK claims.
        expect(groups[0]?.attrs.map((attr) => attr.name)).toEqual([
            "common.attributes.pace",
            "common.attributes.stamina",
            "common.attributes.strength",
            "common.attributes.agility",
            "common.attributes.aerial",
        ]);
        expect(groups[0]?.average).toBe(65);

        expect(groups[1]?.attrs.map((attr) => attr.name)).toEqual([
            "common.attributes.passing",
            "common.attributes.shooting",
            "common.attributes.tackling",
            "common.attributes.dribbling",
            "common.attributes.defending",
        ]);
        expect(groups[1]?.average).toBe(66);

        expect(groups[2]?.attrs.map((attr) => attr.name)).toEqual([
            "common.attributes.positioning",
            "common.attributes.vision",
            "common.attributes.decisions",
            "common.attributes.composure",
            "common.attributes.aggression",
            "common.attributes.teamwork",
            "common.attributes.leadership",
        ]);
        expect(groups[2]?.average).toBe(72);
    });

    it("adds a goalkeeper-only group with handling and reflexes for goalkeepers", () => {
        const groups = buildPlayerAttributeGroups(
            createPlayer({ position: "Goalkeeper", natural_position: "Goalkeeper" }),
            (key) => key,
        );

        expect(groups).toHaveLength(4);
        // Aerial still lives in Physical for goalkeepers — not duplicated.
        expect(groups[0]?.attrs.map((attr) => attr.name)).toContain(
            "common.attributes.aerial",
        );
        expect(groups[3]).toMatchObject({
            label: "common.attrGroups.goalkeeper",
            average: 77,
        });
        expect(groups[3]?.attrs.map((attr) => attr.name)).toEqual([
            "common.attributes.handling",
            "common.attributes.reflexes",
        ]);
    });

    // Guards against the class of bug this file was refactored to fix: an
    // attribute added to the engine PlayerAttributes struct but forgotten in
    // the profile display. If a new key appears in `player.attributes`, this
    // test fails until it's grouped into the meta table above.
    it("surfaces every engine attribute in exactly one group for outfielders and one-or-two groups for goalkeepers", () => {
        const outfielderGroups = buildPlayerAttributeGroups(
            createPlayer(),
            (key) => key,
        );
        const gkGroups = buildPlayerAttributeGroups(
            createPlayer({ position: "Goalkeeper", natural_position: "Goalkeeper" }),
            (key) => key,
        );

        const player = createPlayer();
        const allAttributeKeys = Object.keys(
            player.attributes,
        ) as (keyof PlayerData["attributes"])[];

        for (const key of allAttributeKeys) {
            const expectedName = `common.attributes.${key}`;

            const gkAppearances = gkGroups
                .flatMap((group) => group.attrs)
                .filter((attr) => attr.name === expectedName).length;
            expect(gkAppearances, `${key} on GK profile`).toBe(1);

            // Handling and reflexes are gated to the goalkeeper group; every
            // other attribute must also appear for outfielders.
            const isGoalkeeperOnly = key === "handling" || key === "reflexes";
            const outfielderAppearances = outfielderGroups
                .flatMap((group) => group.attrs)
                .filter((attr) => attr.name === expectedName).length;
            expect(
                outfielderAppearances,
                `${key} on outfielder profile`,
            ).toBe(isGoalkeeperOnly ? 0 : 1);
        }
    });

    it("looks up a single attribute by key without positional indexing", () => {
        const player = createPlayer();

        expect(getPlayerAttributeEntry(player, "aerial", (key) => key)).toEqual({
            name: "common.attributes.aerial",
            value: 78,
        });
        expect(getPlayerAttributeEntry(player, "pace", (key) => key)).toEqual({
            name: "common.attributes.pace",
            value: 60,
        });
    });
});
