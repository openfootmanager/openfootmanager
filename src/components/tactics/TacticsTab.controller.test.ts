import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData } from "../../store/types";
import {
    applyTacticsFormationChange,
    applyTacticsPlayStyleChange,
    persistTacticsStartingXI,
} from "./TacticsTab.controller";

const setFormationMock = vi.fn();
const setPlayStyleMock = vi.fn();
const setStartingXIMock = vi.fn();

vi.mock("../../services/tacticsService", () => ({
    setFormation: (...args: unknown[]) => setFormationMock(...args),
    setPlayStyle: (...args: unknown[]) => setPlayStyleMock(...args),
    setStartingXI: (...args: unknown[]) => setStartingXIMock(...args),
}));

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

describe("TacticsTab.controller", () => {
    beforeEach(() => {
        setFormationMock.mockReset();
        setPlayStyleMock.mockReset();
        setStartingXIMock.mockReset();
    });

    it("persists the starting xi and sets pending ids before the request", async () => {
        const updatedGame = makeGameState();
        const onGameUpdate = vi.fn();
        const setPendingStartingXiIds = vi.fn();

        setStartingXIMock.mockResolvedValue(updatedGame);

        const result = await persistTacticsStartingXI(
            ["p1", "p2", "p3"],
            onGameUpdate,
            setPendingStartingXiIds,
        );

        expect(setPendingStartingXiIds).toHaveBeenCalledWith(["p1", "p2", "p3"]);
        expect(setStartingXIMock).toHaveBeenCalledWith(["p1", "p2", "p3"]);
        expect(onGameUpdate).toHaveBeenCalledWith(updatedGame);
        expect(result).toBe(updatedGame);
    });

    it("clears pending xi ids when the starting xi save fails", async () => {
        const onGameUpdate = vi.fn();
        const setPendingStartingXiIds = vi.fn();
        const consoleErrorSpy = vi
            .spyOn(console, "error")
            .mockImplementation(() => undefined);

        setStartingXIMock.mockRejectedValue(new Error("network"));

        const result = await persistTacticsStartingXI(
            ["p1", "p2", "p3"],
            onGameUpdate,
            setPendingStartingXiIds,
        );

        expect(setPendingStartingXiIds).toHaveBeenNthCalledWith(1, ["p1", "p2", "p3"]);
        expect(setPendingStartingXiIds).toHaveBeenNthCalledWith(2, null);
        expect(consoleErrorSpy).toHaveBeenCalled();
        expect(onGameUpdate).not.toHaveBeenCalled();
        expect(result).toBeNull();

        consoleErrorSpy.mockRestore();
    });

    it("applies a formation change through the controller", async () => {
        const updatedGame = makeGameState();
        const onGameUpdate = vi.fn();

        setFormationMock.mockResolvedValue(updatedGame);

        const result = await applyTacticsFormationChange("4-3-3", onGameUpdate);

        expect(setFormationMock).toHaveBeenCalledWith("4-3-3");
        expect(onGameUpdate).toHaveBeenCalledWith(updatedGame);
        expect(result).toBe(updatedGame);
    });

    it("applies a play style change through the controller", async () => {
        const updatedGame = makeGameState();
        const onGameUpdate = vi.fn();

        setPlayStyleMock.mockResolvedValue(updatedGame);

        const result = await applyTacticsPlayStyleChange("Counter", onGameUpdate);

        expect(setPlayStyleMock).toHaveBeenCalledWith("Counter");
        expect(onGameUpdate).toHaveBeenCalledWith(updatedGame);
        expect(result).toBe(updatedGame);
    });
});