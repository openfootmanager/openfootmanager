import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { GameStateData } from "../../store/gameStore";
import DashboardTabContent from "./DashboardTabContent";
import { createDashboardTabContentModel } from "./dashboardTabContentModel";

vi.mock("../EndOfSeasonScreen", () => ({
    default: () => <div>End Of Season Mock</div>,
}));

vi.mock("../home/HomeTab", () => ({
    default: () => <div>Home Tab Mock</div>,
}));

vi.mock("../squad/SquadTab", () => ({
    default: () => <div>Squad Tab Mock</div>,
}));

vi.mock("../manager/ManagersWorldTab", () => ({
    default: () => <div>Managers World Mock</div>,
}));

vi.mock("../transfers/TransferCentreWorldTab", () => ({
    default: () => <div>Transfer Centre World Mock</div>,
}));

vi.mock("../hallOfFame/HallOfFameWorldTab", () => ({
    default: () => <div>Hall Of Fame World Mock</div>,
}));

function createGameState(): GameStateData {
    return {
        clock: {
            current_date: "2026-07-10T12:00:00Z",
            start_date: "2026-07-01T12:00:00Z",
        },
        manager: {
            id: "manager-1",
            first_name: "Jane",
            last_name: "Doe",
            date_of_birth: "1980-01-01",
            nationality: "England",
            reputation: 50,
            satisfaction: 50,
            fan_approval: 50,
            team_id: "team-1",
            career_stats: {
                matches_managed: 0,
                wins: 0,
                draws: 0,
                losses: 0,
                trophies: 0,
                best_finish: null,
            },
            career_history: [],
        },
        teams: [],
        players: [],
        staff: [],
        messages: [],
        news: [],
        league: null,
        scouting_assignments: [],
        board_objectives: [],
    };
}

function renderTabContent(activeTab: string, seasonComplete = false) {
    const gameState = createGameState();

    render(
        <DashboardTabContent
            viewModel={createDashboardTabContentModel({
                activeTab,
                gameState,
                seasonComplete,
                visitedOnboardingTabs: new Set<string>(),
                initialMessageId: null,
                handlers: {
                    onSelectPlayer: vi.fn(),
                    onSelectTeam: vi.fn(),
                    onGameUpdate: vi.fn(),
                    onNavigate: vi.fn(),
                },
            })}
        />,
    );
}

describe("DashboardTabContent", () => {
    it("renders the home tab immediately for the default dashboard view", () => {
        renderTabContent("Home");

        expect(screen.getByText("Home Tab Mock")).toBeInTheDocument();
    });

    it("renders the end-of-season screen for a completed home season", () => {
        renderTabContent("Home", true);

        expect(screen.getByText("End Of Season Mock")).toBeInTheDocument();
    });

    it("loads a non-home tab through suspense when selected", async () => {
        renderTabContent("Squad");

        expect(await screen.findByText("Squad Tab Mock")).toBeInTheDocument();
    });

    it("loads the managers world tab when selected", async () => {
        renderTabContent("Managers");

        expect(await screen.findByText("Managers World Mock")).toBeInTheDocument();
    });

    it("loads the transfer centre world tab when selected", async () => {
        renderTabContent("TransferCentre");

        expect(await screen.findByText("Transfer Centre World Mock")).toBeInTheDocument();
    });

    it("loads the hall of fame world tab when selected", async () => {
        renderTabContent("HallOfFame");

        expect(await screen.findByText("Hall Of Fame World Mock")).toBeInTheDocument();
    });

    it("falls back to the home content when the active tab is unknown", () => {
        const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => { });

        renderTabContent("LegacyTab");

        expect(screen.getByText("Home Tab Mock")).toBeInTheDocument();
        expect(warnSpy).toHaveBeenCalledWith(
            "DashboardTabContent received unexpected activeTab",
            "LegacyTab",
        );

        warnSpy.mockRestore();
    });
});