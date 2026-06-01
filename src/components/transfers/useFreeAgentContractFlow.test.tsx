import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { JSX } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import {
  offerFreeAgentContract,
  previewFreeAgentContractImpact,
} from "../../services/freeAgentService";
import { resolveBackendError } from "../../utils/backendI18n";
import { useFreeAgentContractFlow } from "./useFreeAgentContractFlow";

vi.mock("../../services/freeAgentService", () => ({
  offerFreeAgentContract: vi.fn(),
  previewFreeAgentContractImpact: vi.fn(),
}));

vi.mock("../../utils/backendI18n", () => ({
  resolveBackendError: vi.fn((error: unknown) =>
    error instanceof Error ? error.message : String(error),
  ),
}));

const mockedOfferFreeAgentContract = vi.mocked(offerFreeAgentContract);
const mockedPreviewFreeAgentContractImpact = vi.mocked(
  previewFreeAgentContractImpact,
);
const mockedResolveBackendError = vi.mocked(resolveBackendError);

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
    colors: { primary: "#111111", secondary: "#ffffff" },
    facilities: { training: 1, medical: 1, scouting: 1 },
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
    id: "free-agent-1",
    match_name: "F. Agent",
    full_name: "Free Agent",
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
    team_id: null,
    retired: false,
    contract_end: null,
    wage: 0,
    market_value: 600000,
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
    teams: [createTeam()],
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
  const {
    contractWage,
    setContractWage,
    contractLength,
    setContractLength,
    openFreeAgentContract,
    submitFreeAgentContract,
  } = useFreeAgentContractFlow({ gameState });

  return (
    <div>
      <button onClick={() => openFreeAgentContract(target)}>Open</button>
      <label htmlFor="wage">Wage</label>
      <input
        id="wage"
        value={contractWage}
        onChange={(event) => setContractWage(event.target.value)}
      />
      <label htmlFor="years">Years</label>
      <input
        id="years"
        value={contractLength}
        onChange={(event) => setContractLength(event.target.value)}
      />
      <button onClick={() => void submitFreeAgentContract()}>Submit</button>
    </div>
  );
}

describe("useFreeAgentContractFlow", () => {
  beforeEach(() => {
    mockedOfferFreeAgentContract.mockReset();
    mockedPreviewFreeAgentContractImpact.mockReset();
    mockedResolveBackendError.mockClear();
    mockedPreviewFreeAgentContractImpact.mockResolvedValue({
      projection: {
        current_annual_wage_bill: 0,
        projected_annual_wage_bill: 4000,
        annual_wage_budget: 50000,
        annual_soft_cap: 55000,
        current_weekly_wage_spend: 0,
        projected_weekly_wage_spend: 4000,
        current_cash_runway_weeks: 40,
        projected_cash_runway_weeks: 30,
        currently_over_budget: false,
        policy_allows: true,
      },
    });
  });

  it("does not submit when the computed wage is invalid", async () => {
    const target = createPlayer();
    const gameState = createGameState([target]);

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => {
      expect(mockedPreviewFreeAgentContractImpact).toHaveBeenCalledWith(
        target.id,
        3000,
      );
    });

    fireEvent.change(screen.getByLabelText("Wage"), {
      target: { value: "abc" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => {
      expect(mockedOfferFreeAgentContract).not.toHaveBeenCalled();
    });
  });

  it("submits the contract offer with current values", async () => {
    const target = createPlayer();
    const gameState = createGameState([target]);
    mockedOfferFreeAgentContract.mockResolvedValue({
      outcome: "counter_offer",
      game: gameState,
      suggested_wage: 4000,
      suggested_years: 3,
      session_status: "open",
      is_terminal: false,
      feedback: null,
    });

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.change(screen.getByLabelText("Wage"), {
      target: { value: "4000" },
    });
    fireEvent.change(screen.getByLabelText("Years"), {
      target: { value: "3" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => {
      expect(mockedOfferFreeAgentContract).toHaveBeenCalledWith(
        target.id,
        4000,
        3,
      );
    });
  });

  it("does not submit offers longer than five years", async () => {
    const target = createPlayer();
    const gameState = createGameState([target]);

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.change(screen.getByLabelText("Years"), {
      target: { value: "6" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => {
      expect(mockedOfferFreeAgentContract).not.toHaveBeenCalled();
    });
  });

  it("derives default contract years from the current in-game date", async () => {
    const target = createPlayer({
      id: "veteran-free-agent",
      date_of_birth: "1994-12-15",
    });
    const gameState = createGameState([target]);

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => {
      expect(screen.getByLabelText("Years")).toHaveValue("2");
    });
  });

  it("resolves backend errors before storing them", async () => {
    const target = createPlayer();
    const gameState = createGameState([target]);
    mockedOfferFreeAgentContract.mockRejectedValue(
      new Error("be.error.contracts.boardWagePolicy?budget=50000"),
    );
    mockedResolveBackendError.mockReturnValue("Board wage policy");

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => {
      expect(mockedResolveBackendError).toHaveBeenCalledWith(
        expect.objectContaining({
          message: "be.error.contracts.boardWagePolicy?budget=50000",
        }),
      );
    });
  });
});
