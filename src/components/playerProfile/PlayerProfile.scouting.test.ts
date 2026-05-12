import { describe, expect, it } from "vitest";
import type {
    ScoutingAssignment,
    StaffData,
    YouthScoutingAssignment,
} from "../../store/gameStore";
import { getScoutAvailability } from "./PlayerProfile.scouting";

function createScout(overrides: Partial<StaffData> = {}): StaffData {
    return {
        id: "scout-1",
        first_name: "Sam",
        last_name: "Scout",
        date_of_birth: "1985-01-01",
        nationality: "GB",
        role: "Scout",
        attributes: {
            coaching: 10,
            judging_ability: 14,
            judging_potential: 13,
            physiotherapy: 1,
        },
        team_id: "team-1",
        specialization: null,
        wage: 8000,
        contract_end: "2027-06-30",
        ...overrides,
    };
}

describe("PlayerProfile scouting helpers", () => {
    it("returns an available scout when one is free", () => {
        const availability = getScoutAvailability({
            staff: [createScout()],
            scoutingAssignments: [],
            youthScoutingAssignments: [],
            managerTeamId: "team-1",
            playerId: "player-1",
            scoutStatus: "idle",
        });

        expect(availability.scouts).toHaveLength(1);
        expect(availability.availableScout?.id).toBe("scout-1");
        expect(availability.canScout).toBe(true);
    });

    it("marks the player as already scouting when an assignment exists", () => {
        const assignments: ScoutingAssignment[] = [
            {
                id: "assignment-1",
                scout_id: "scout-1",
                player_id: "player-1",
                days_remaining: 3,
            },
        ];

        const availability = getScoutAvailability({
            staff: [createScout()],
            scoutingAssignments: assignments,
            youthScoutingAssignments: [],
            managerTeamId: "team-1",
            playerId: "player-1",
            scoutStatus: "idle",
        });

        expect(availability.alreadyScouting).toBe(true);
        expect(availability.canScout).toBe(false);
    });

    it("treats all scouts as busy when each one has an active assignment", () => {
        const availability = getScoutAvailability({
            staff: [createScout(), createScout({ id: "scout-2" })],
            scoutingAssignments: [
                {
                    id: "assignment-1",
                    scout_id: "scout-1",
                    player_id: "player-a",
                    days_remaining: 2,
                },
                {
                    id: "assignment-2",
                    scout_id: "scout-2",
                    player_id: "player-b",
                    days_remaining: 2,
                },
            ],
            youthScoutingAssignments: [],
            managerTeamId: "team-1",
            playerId: "player-1",
            scoutStatus: "idle",
        });

        expect(availability.availableScout).toBeNull();
        expect(availability.allBusy).toBe(true);
        expect(availability.canScout).toBe(false);
    });

    it("treats scouts on youth assignments as busy", () => {
        const youthAssignments: YouthScoutingAssignment[] = [
            {
                id: "youth-1",
                scout_id: "scout-1",
                region: "Domestic",
                objective: "Balanced",
                target_position: null,
                days_remaining: 4,
            },
        ];

        const availability = getScoutAvailability({
            staff: [createScout()],
            scoutingAssignments: [],
            youthScoutingAssignments: youthAssignments,
            managerTeamId: "team-1",
            playerId: "player-1",
            scoutStatus: "idle",
        });

        expect(availability.availableScout).toBeNull();
        expect(availability.allBusy).toBe(true);
        expect(availability.canScout).toBe(false);
    });
});