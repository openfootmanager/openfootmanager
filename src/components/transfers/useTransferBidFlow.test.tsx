import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { JSX } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import {
    makeTransferBid,
    previewTransferBidFinancialImpact,
} from "../../services/transfersService";
import { useTransferBidFlow } from "./useTransferBidFlow";

vi.mock("../../services/transfersService", () => ({
    makeTransferBid: vi.fn(),
    previewTransferBidFinancialImpact: vi.fn(),
}));

const mockedMakeTransferBid = vi.mocked(makeTransferBid);
const mockedPreviewTransferBidFinancialImpact = vi.mocked(
    previewTransferBidFinancialImpact,
);

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
    return {
        id: "team-1",
        name: "User FC",
        short_name: "USR",
        country: "England",
        city: "London",
        stadium_name: "User Ground",
        stadium_capacity: 25000,
        finance: 5000000,
        manager_id: "manager-1",
        reputation: 50,
        wage_budget: 50000,
        transfer_budget: 2000000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-4-2",
        play_style: "Balanced",
        training_focus: "Physical",
        training_intensity: "Medium",
        training_schedule: "Balanced",
        founded_year: 1900,
        colors: {
            primary: "#111111",
            secondary: "#ffffff",
        },
        facilities: {
            training: 1,
            medical: 1,
            scouting: 1,
        },
        starting_xi_ids: [],
        match_roles: {
            captain: null,
            vice_captain: null,
            penalty_taker: null,
            free_kick_taker: null,
            corner_taker: null,
        },
        form: [],
        history: [],
        ...overrides,
    };
}

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
    return {
        id: "player-market-1",
        match_name: "J. Smith",
        full_name: "John Smith",
        date_of_birth: "2000-01-01",
        nationality: "England",
        position: "Forward",
        natural_position: "Forward",
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
        condition: 90,
        morale: 70,
        injury: null,
        team_id: "team-2",
        retired: false,
        contract_end: "2028-06-30",
        wage: 1000,
        market_value: 1500000,
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
        transfer_listed: true,
        loan_listed: false,
        transfer_offers: [],
        traits: [],
        ...overrides,
    };
}

function createGameState(players: PlayerData[] = [createPlayer()]): GameStateData {
    return {
        clock: {
            current_date: "2026-08-01T12:00:00Z",
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
        teams: [
            createTeam(),
            createTeam({
                id: "team-2",
                name: "Seller FC",
                short_name: "SEL",
                manager_id: "manager-2",
            }),
        ],
        players,
        staff: [],
        messages: [],
        news: [],
        league: {
            id: "league-1",
            name: "Premier Division",
            season: 1,
            fixtures: [],
            standings: [],
        },
        scouting_assignments: [],
        board_objectives: [],
    };
}

function HookHarness({
    gameState,
    target,
}: {
    gameState: GameStateData;
    target: PlayerData;
}): JSX.Element {
    const { bidAmount, setBidAmount, openBidNegotiation, handleMakeBid } =
        useTransferBidFlow({ gameState });

    return (
        <div>
            <button onClick={() => openBidNegotiation(target)}>Open</button>
            <label htmlFor="bid-amount">Bid amount</label>
            <input
                id="bid-amount"
                value={bidAmount}
                onChange={(event) => setBidAmount(event.target.value)}
            />
            <button onClick={() => void handleMakeBid()}>Submit</button>
        </div>
    );
}

describe("useTransferBidFlow", function (): void {
    beforeEach(function resetMocks(): void {
        mockedMakeTransferBid.mockReset();
        mockedPreviewTransferBidFinancialImpact.mockReset();
        mockedPreviewTransferBidFinancialImpact.mockResolvedValue({
            projection: {
                transfer_budget_before: 2000000,
                transfer_budget_after: 500000,
                finance_before: 5000000,
                finance_after: 3500000,
                annual_wage_bill_before: 1000,
                annual_wage_bill_after: 2000,
                annual_wage_budget: 50000,
                projected_wage_budget_usage_pct: 4,
                exceeds_transfer_budget: false,
                exceeds_finance: false,
            },
        });
    });

    it("does not submit a bid when the computed fee is invalid", async function (): Promise<void> {
        const target = createPlayer();
        const gameState = createGameState([target]);

        render(<HookHarness gameState={gameState} target={target} />);

        fireEvent.click(screen.getByRole("button", { name: "Open" }));

        await waitFor(function (): void {
            expect(mockedPreviewTransferBidFinancialImpact).toHaveBeenCalledWith(
                target.id,
                1500000,
            );
        });

        fireEvent.change(screen.getByLabelText("Bid amount"), {
            target: { value: "abc" },
        });
        fireEvent.click(screen.getByRole("button", { name: "Submit" }));

        await waitFor(function (): void {
            expect(mockedMakeTransferBid).not.toHaveBeenCalled();
        });
    });
});