import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { GameStateData } from "../store/gameStore";
import NextMatchDisplay from "./NextMatchDisplay";

vi.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => key,
    }),
}));

function createGameState(): GameStateData {
    return {
        clock: {
            current_date: "2026-07-10T12:00:00Z",
            start_date: "2026-07-01T12:00:00Z",
        },
        manager: {
            id: "manager-1",
            first_name: "Hansi",
            last_name: "Flick",
            date_of_birth: "1965-02-24",
            nationality: "Germany",
            reputation: 80,
            satisfaction: 70,
            fan_approval: 75,
            team_id: "barcelona",
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
        teams: [
            {
                id: "barcelona",
                name: "FC Barcelona",
                short_name: "FCB",
                country: "Spain",
                city: "Barcelona",
                stadium_name: "Camp Nou",
                stadium_capacity: 99354,
                finance: 1000000,
                manager_id: "manager-1",
                reputation: 90,
                wage_budget: 500000,
                transfer_budget: 1000000,
                season_income: 0,
                season_expenses: 0,
                formation: "4-3-3",
                play_style: "Possession",
                training_focus: "General",
                training_intensity: "Balanced",
                training_schedule: "Balanced",
                founded_year: 1899,
                colors: { primary: "#A50044", secondary: "#004D98" },
                starting_xi_ids: [],
                form: [],
                history: [],
            },
            {
                id: "bayern",
                name: "Munich Bayern",
                short_name: "MB",
                country: "Germany",
                city: "Munich",
                stadium_name: "Allianz Arena",
                stadium_capacity: 75000,
                finance: 1000000,
                manager_id: null,
                reputation: 90,
                wage_budget: 500000,
                transfer_budget: 1000000,
                season_income: 0,
                season_expenses: 0,
                formation: "4-2-3-1",
                play_style: "Balanced",
                training_focus: "General",
                training_intensity: "Balanced",
                training_schedule: "Balanced",
                founded_year: 1900,
                colors: { primary: "#DC052D", secondary: "#FFFFFF" },
                starting_xi_ids: [],
                form: [],
                history: [],
            },
        ],
        players: [],
        staff: [],
        messages: [],
        news: [],
        league: {
            id: "league-1",
            name: "Test League",
            season: 2026,
            standings: [],
            fixtures: [
                {
                    id: "friendly-1",
                    matchday: 0,
                    date: "2026-07-17",
                    home_team_id: "bayern",
                    away_team_id: "barcelona",
                    competition: "Friendly",
                    status: "Scheduled",
                    result: null,
                },
            ],
        },
        scouting_assignments: [],
        board_objectives: [],
    };
}

describe("NextMatchDisplay", () => {
    it("renders the user team against the opponent when the user is away", () => {
        render(<NextMatchDisplay gameState={createGameState()} />);

        expect(screen.getByText("FC Barcelona")).toBeInTheDocument();
        expect(screen.getByText("Munich Bayern")).toBeInTheDocument();
        expect(screen.getAllByText("Munich Bayern")).toHaveLength(1);
        expect(screen.getByText("home.away")).toBeInTheDocument();
    });
});