import { beforeEach, describe, expect, it, vi } from "vitest";

import type {
    GameStateData,
    PlayerData,
    TeamMatchRolesData,
} from "../../store/types";
import {
    applyTacticsRoleSelection,
    autoSelectTacticsAssignments,
    persistTacticsMatchRoles,
} from "./TacticsRolesPanel.controller";

const setTeamMatchRolesMock = vi.fn();

vi.mock("../../services/tacticsService", () => ({
    setTeamMatchRoles: (...args: unknown[]) => setTeamMatchRolesMock(...args),
}));

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

function makeGameState(): GameStateData {
    return {
        userTeamId: "team_1",
        manager: null,
        teams: [],
        players: [],
        staff: [],
        fixtures: [],
        news: [],
        inbox: [],
        scouting: { reports: [], assignments: [] },
        transfer_list: [],
        watchlist: [],
        shortlists: [],
        negotiations: [],
        finance: {
            budget: 0,
            wage_budget: 0,
            weekly_wage_spend: 0,
            transfer_budget: 0,
            sponsorship_income: 0,
            prize_money: 0,
            ticket_sales: 0,
            merchandise: 0,
            total_income: 0,
            total_expenses: 0,
        },
        settings: {
            currency: "GBP",
            language: "en",
            theme: "dark",
            difficulty: "Normal",
        },
        world: {
            current_date: "2026-08-01",
            current_season: 2026,
            current_matchday: 1,
        },
        training: { schedule: [], focus: "Balanced", intensity: "Normal" },
    } as unknown as GameStateData;
}

describe("TacticsRolesPanel.controller", () => {
    beforeEach(() => {
        setTeamMatchRolesMock.mockReset();
    });

    it("persists match roles and reports the updated game state", async () => {
        const updatedGame = makeGameState();
        const onGameUpdate = vi.fn();
        const nextRoles: TeamMatchRolesData = {
            captain: "captain-1",
            vice_captain: "vice-1",
            penalty_taker: "penalty-1",
            free_kick_taker: "free-1",
            corner_taker: "corner-1",
        };
        setTeamMatchRolesMock.mockResolvedValue(updatedGame);

        const result = await persistTacticsMatchRoles(nextRoles, onGameUpdate);

        expect(setTeamMatchRolesMock).toHaveBeenCalledWith(nextRoles);
        expect(onGameUpdate).toHaveBeenCalledWith(updatedGame);
        expect(result).toBe(updatedGame);
    });

    it("builds and persists updated roles for a single selection", async () => {
        const captain = makePlayer("captain", {
            full_name: "Captain",
            attributes: { ...makePlayer("seed-1").attributes, leadership: 90, teamwork: 90 },
        });
        const vice = makePlayer("vice", {
            full_name: "Vice",
            attributes: { ...makePlayer("seed-2").attributes, leadership: 85, teamwork: 85 },
        });
        const updatedGame = makeGameState();
        const onGameUpdate = vi.fn();

        setTeamMatchRolesMock.mockResolvedValue(updatedGame);

        await applyTacticsRoleSelection(
            [captain, vice],
            {
                captain: "captain",
                vice_captain: "vice",
                penalty_taker: null,
                free_kick_taker: null,
                corner_taker: null,
            },
            "captain",
            "vice",
            onGameUpdate,
        );

        expect(setTeamMatchRolesMock).toHaveBeenCalledWith({
            captain: "vice",
            vice_captain: "captain",
            penalty_taker: null,
            free_kick_taker: null,
            corner_taker: null,
        });
        expect(onGameUpdate).toHaveBeenCalledWith(updatedGame);
    });

    it("persists the auto-selected default assignments", async () => {
        const updatedGame = makeGameState();
        const onGameUpdate = vi.fn();
        const effectiveRoles: TeamMatchRolesData = {
            captain: "captain-1",
            vice_captain: "vice-1",
            penalty_taker: "penalty-1",
            free_kick_taker: "free-1",
            corner_taker: "corner-1",
        };

        setTeamMatchRolesMock.mockResolvedValue(updatedGame);

        const result = await autoSelectTacticsAssignments(
            effectiveRoles,
            onGameUpdate,
        );

        expect(setTeamMatchRolesMock).toHaveBeenCalledWith(effectiveRoles);
        expect(onGameUpdate).toHaveBeenCalledWith(updatedGame);
        expect(result).toBe(updatedGame);
    });
});