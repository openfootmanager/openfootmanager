import { describe, expect, it } from "vitest";

import type { PlayerData, TeamMatchRolesData } from "../../store/types";
import {
    buildUpdatedMatchRoles,
    getEffectiveMatchRoles,
    getSelectorRolePlayers,
    pickBestRoleCandidate,
} from "./TacticsRolesPanel.helpers";

function makePlayer(
    id: string,
    overrides: Partial<PlayerData> = {},
): PlayerData {
    return {
        id,
        match_name: `Match ${id}`,
        full_name: `Full ${id}`,
        date_of_birth: "1996-01-15",
        nationality: "GB",
        position: "Midfielder",
        natural_position: "Midfielder",
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
            handling: 30,
            reflexes: 30,
            aerial: 60,
        },
        condition: 100,
        morale: 75,
        injury: null,
        team_id: "team_1",
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

describe("TacticsRolesPanel.helpers", () => {
    it("filters goalkeepers out of non-leadership roles", () => {
        const goalkeeper = makePlayer("gk", {
            position: "Goalkeeper",
            full_name: "Goalkeeper",
            attributes: {
                ...makePlayer("temp").attributes,
                shooting: 99,
                composure: 99,
            },
        });
        const striker = makePlayer("st", {
            position: "Forward",
            full_name: "Striker",
            attributes: {
                ...makePlayer("temp2").attributes,
                shooting: 80,
                composure: 80,
            },
        });

        expect(pickBestRoleCandidate([goalkeeper, striker], "penalty")).toBe("st");
    });

    it("allows goalkeepers to be chosen for captaincy", () => {
        const goalkeeper = makePlayer("gk", {
            position: "Goalkeeper",
            full_name: "Goalkeeper",
            attributes: {
                ...makePlayer("temp").attributes,
                leadership: 95,
                teamwork: 95,
            },
        });
        const midfielder = makePlayer("mid", {
            full_name: "Midfielder",
            attributes: {
                ...makePlayer("temp2").attributes,
                leadership: 70,
                teamwork: 70,
            },
        });

        expect(pickBestRoleCandidate([goalkeeper, midfielder], "captain")).toBe("gk");
    });

    it("builds effective roles with fallbacks for unavailable stored assignments", () => {
        const captain = makePlayer("captain", {
            full_name: "Captain",
            attributes: {
                ...makePlayer("temp").attributes,
                leadership: 90,
                teamwork: 90,
            },
        });
        const vice = makePlayer("vice", {
            full_name: "Vice",
            attributes: {
                ...makePlayer("temp2").attributes,
                leadership: 80,
                teamwork: 80,
            },
        });
        const taker = makePlayer("taker", {
            full_name: "Taker",
            position: "Forward",
            attributes: {
                ...makePlayer("temp3").attributes,
                shooting: 92,
                composure: 88,
                passing: 86,
                vision: 84,
            },
        });

        const roles = getEffectiveMatchRoles([captain, vice, taker], {
            captain: "missing",
            vice_captain: null,
            penalty_taker: "missing",
            free_kick_taker: null,
            corner_taker: null,
        });

        expect(roles).toEqual<TeamMatchRolesData>({
            captain: "captain",
            vice_captain: "vice",
            penalty_taker: "taker",
            free_kick_taker: "taker",
            corner_taker: "taker",
        });
    });

    it("reassigns the vice captain when the selected captain was already vice captain", () => {
        const captain = makePlayer("captain", {
            full_name: "Captain",
            attributes: {
                ...makePlayer("temp").attributes,
                leadership: 95,
                teamwork: 95,
            },
        });
        const vice = makePlayer("vice", {
            full_name: "Vice",
            attributes: {
                ...makePlayer("temp2").attributes,
                leadership: 85,
                teamwork: 85,
            },
        });
        const third = makePlayer("third", {
            full_name: "Third",
            attributes: {
                ...makePlayer("temp3").attributes,
                leadership: 75,
                teamwork: 75,
            },
        });

        expect(
            buildUpdatedMatchRoles(
                [captain, vice, third],
                {
                    captain: "captain",
                    vice_captain: "vice",
                    penalty_taker: null,
                    free_kick_taker: null,
                    corner_taker: null,
                },
                "captain",
                "vice",
            ),
        ).toEqual<TeamMatchRolesData>({
            captain: "vice",
            vice_captain: "captain",
            penalty_taker: null,
            free_kick_taker: null,
            corner_taker: null,
        });
    });

    it("maps selector player data from the starting xi", () => {
        const players = [
            makePlayer("one", { match_name: "One", position: "Defender" }),
            makePlayer("two", { match_name: "Two", position: "Forward" }),
        ];

        expect(getSelectorRolePlayers(players)).toEqual([
            { id: "one", name: "One", position: "Defender" },
            { id: "two", name: "Two", position: "Forward" },
        ]);
    });
});