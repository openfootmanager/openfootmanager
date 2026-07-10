import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { JSX } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import type { TransferOfferData } from "../../store/types";
import { negotiateTransferPersonalTerms } from "../../services/transfersService";
import { resolveBackendError } from "../../utils/backendI18n";
import { useTransferPersonalTermsFlow } from "./useTransferPersonalTermsFlow";

vi.mock("../../services/transfersService", () => ({
  negotiateTransferPersonalTerms: vi.fn(),
}));

vi.mock("../../utils/backendI18n", () => ({
  resolveBackendError: vi.fn((error: unknown) =>
    error instanceof Error ? error.message : String(error),
  ),
}));

const mockedNegotiate = vi.mocked(negotiateTransferPersonalTerms);
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
    wage_budget: 2000000,
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

function createOffer(overrides: Partial<TransferOfferData> = {}): TransferOfferData {
  return {
    id: "offer-1",
    from_team_id: "team-1",
    fee: 2000000,
    wage_offered: 0,
    contract_years_offered: null,
    last_manager_fee: 2000000,
    negotiation_round: 1,
    suggested_counter_fee: null,
    status: "PersonalTermsPending",
    date: "2026-08-01",
    registration_date: null,
    personal_terms_status: "Open",
    personal_terms_round: 1,
    suggested_wage: null,
    suggested_contract_years: null,
    personal_terms_blocked_until: null,
    ...overrides,
  };
}

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "target-1",
    match_name: "T. Arget",
    full_name: "Target Player",
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
    wage: 5000,
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
    transfer_offers: [createOffer()],
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

function response(
  overrides: Partial<
    Awaited<ReturnType<typeof negotiateTransferPersonalTerms>>
  > = {},
): Awaited<ReturnType<typeof negotiateTransferPersonalTerms>> {
  return {
    success: false,
    status: "PersonalTermsPending",
    wage_offered: null,
    contract_years: null,
    suggested_wage: null,
    suggested_contract_years: null,
    personal_terms_round: 2,
    error: null,
    feedback: null,
    game: createGameState(),
    ...overrides,
  };
}

function HookHarness({
  gameState,
  target,
  offerId = "offer-1",
  buyerTeamId = "team-1",
}: {
  gameState: GameStateData;
  target: PlayerData;
  offerId?: string;
  buyerTeamId?: string;
}): JSX.Element {
  const {
    wageOffer,
    setWageOffer,
    contractYears,
    setContractYears,
    personalTermsRound,
    personalTermsStatus,
    personalTermsError,
    submitDisabled,
    openPersonalTermsNegotiation,
    submitPersonalTerms,
  } = useTransferPersonalTermsFlow({ gameState });

  return (
    <div>
      <button
        onClick={() => openPersonalTermsNegotiation(target, offerId, buyerTeamId)}
      >
        Open
      </button>
      <label htmlFor="wage">Wage</label>
      <input
        id="wage"
        value={wageOffer}
        onChange={(event) => setWageOffer(event.target.value)}
      />
      <label htmlFor="years">Years</label>
      <input
        id="years"
        value={contractYears}
        onChange={(event) => setContractYears(event.target.value)}
      />
      <button onClick={() => void submitPersonalTerms()}>Submit</button>
      <span data-testid="round">{personalTermsRound}</span>
      <span data-testid="status">{personalTermsStatus ?? "none"}</span>
      <span data-testid="error">{personalTermsError ?? ""}</span>
      <span data-testid="disabled">{String(submitDisabled)}</span>
    </div>
  );
}

describe("useTransferPersonalTermsFlow", () => {
  beforeEach(() => {
    mockedNegotiate.mockReset();
    mockedResolveBackendError.mockClear();
  });

  it("submits the user's wage and length to the backend", async () => {
    const target = createPlayer();
    const gameState = createGameState([target]);
    mockedNegotiate.mockResolvedValue(response({ personal_terms_round: 2 }));

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.change(screen.getByLabelText("Wage"), {
      target: { value: "12000" },
    });
    fireEvent.change(screen.getByLabelText("Years"), { target: { value: "4" } });
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => {
      expect(mockedNegotiate).toHaveBeenCalledWith(
        "target-1",
        "offer-1",
        "team-1",
        12000,
        4,
      );
    });
  });

  it("does not submit an invalid wage", async () => {
    const target = createPlayer();
    const gameState = createGameState([target]);

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.change(screen.getByLabelText("Wage"), { target: { value: "abc" } });
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => {
      expect(mockedNegotiate).not.toHaveBeenCalled();
    });
  });

  it("does not submit offers longer than five years", async () => {
    const target = createPlayer();
    const gameState = createGameState([target]);

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.change(screen.getByLabelText("Years"), { target: { value: "6" } });
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => {
      expect(mockedNegotiate).not.toHaveBeenCalled();
    });
  });

  it("prefills inputs from the player's live counter proposal", async () => {
    const target = createPlayer({
      transfer_offers: [
        createOffer({
          suggested_wage: 18000,
          suggested_contract_years: 4,
          personal_terms_round: 3,
        }),
      ],
    });
    const gameState = createGameState([target]);

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => {
      expect(screen.getByLabelText("Wage")).toHaveValue("18000");
      expect(screen.getByLabelText("Years")).toHaveValue("4");
      expect(screen.getByTestId("round")).toHaveTextContent("3");
    });
  });

  it("keeps talks live on a counter and prefills the next round", async () => {
    const target = createPlayer();
    const gameState = createGameState([target]);
    mockedNegotiate.mockResolvedValue(
      response({
        success: false,
        status: "PersonalTermsPending",
        suggested_wage: 15000,
        suggested_contract_years: 3,
        personal_terms_round: 2,
      }),
    );

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.change(screen.getByLabelText("Wage"), {
      target: { value: "8000" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => {
      expect(screen.getByLabelText("Wage")).toHaveValue("15000");
      expect(screen.getByTestId("round")).toHaveTextContent("2");
      expect(screen.getByTestId("status")).toHaveTextContent(
        "PersonalTermsPending",
      );
      // Talks are still live, so submitting again is allowed.
      expect(screen.getByTestId("disabled")).toHaveTextContent("false");
    });
  });

  it("disables further submission once the deal is signed", async () => {
    const target = createPlayer();
    const gameState = createGameState([target]);
    mockedNegotiate.mockResolvedValue(
      response({ success: true, status: "Accepted" }),
    );

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.change(screen.getByLabelText("Wage"), {
      target: { value: "20000" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => {
      expect(screen.getByTestId("status")).toHaveTextContent("Accepted");
      expect(screen.getByTestId("disabled")).toHaveTextContent("true");
    });

    mockedNegotiate.mockClear();
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));
    await waitFor(() => {
      expect(mockedNegotiate).not.toHaveBeenCalled();
    });
  });

  it("resolves backend errors before storing them", async () => {
    const target = createPlayer();
    const gameState = createGameState([target]);
    mockedNegotiate.mockRejectedValue(
      new Error("be.error.transfers.wageBudgetExceeded"),
    );
    mockedResolveBackendError.mockReturnValue("Wage budget exceeded");

    render(<HookHarness gameState={gameState} target={target} />);

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.change(screen.getByLabelText("Wage"), {
      target: { value: "12000" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => {
      expect(mockedResolveBackendError).toHaveBeenCalled();
      expect(screen.getByTestId("error")).toHaveTextContent(
        "Wage budget exceeded",
      );
    });
  });
});
