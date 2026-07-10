import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlayerData, TeamData } from "../../store/gameStore";
import PersonalTermsModal, {
  type PersonalTermsFormProps,
} from "./PersonalTermsModal";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "transfers.personalTermsModalTitle")
        return `Personal terms: ${params?.player}`;
      if (key === "transfers.personalTermsWageLabel") return "Weekly Wage";
      if (key === "transfers.personalTermsLengthLabel") return "Contract Length";
      if (key === "transfers.personalTermsSubmit") return "Submit Terms";
      if (key === "transfers.personalTermsSuggestedHint")
        return `They want ${params?.wage} over ${params?.years}y`;
      if (key === "transfers.personalTermsRoundLabel")
        return `Round ${params?.round}`;
      if (key === "transfers.playerValue") return `Value ${params?.value}`;
      if (key === "transfers.submitting") return "Submitting";
      if (key === "transfers.close") return "Close";
      if (key === "common.freeAgent") return "Free Agent";
      return key;
    },
  }),
}));

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-2",
    name: "Seller FC",
    short_name: "SEL",
    country: "England",
    city: "Liverpool",
    stadium_name: "Seller Ground",
    stadium_capacity: 28000,
    finance: 5000000,
    manager_id: null,
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
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

function baseProps(
  overrides: Partial<PersonalTermsFormProps> = {},
): PersonalTermsFormProps {
  return {
    player: createPlayer(),
    teams: [createTeam()],
    wage: "12000",
    onWageChange: vi.fn(),
    contractYears: "4",
    onContractYearsChange: vi.fn(),
    round: 1,
    suggestedWage: null,
    suggestedYears: null,
    feedback: null,
    error: null,
    submitting: false,
    submitDisabled: false,
    terminal: false,
    onSubmit: vi.fn(),
    onClose: vi.fn(),
    ...overrides,
  };
}

describe("PersonalTermsModal", () => {
  it("submits the current terms and forwards input edits", () => {
    const props = baseProps();
    render(<PersonalTermsModal {...props} />);

    expect(screen.getByLabelText("Weekly Wage")).toHaveValue(12000);
    expect(screen.getByLabelText("Contract Length")).toHaveValue(4);

    fireEvent.change(screen.getByLabelText("Weekly Wage"), {
      target: { value: "15000" },
    });
    expect(props.onWageChange).toHaveBeenCalledWith("15000");

    fireEvent.click(screen.getByRole("button", { name: "Submit Terms" }));
    expect(props.onSubmit).toHaveBeenCalledTimes(1);
  });

  it("disables the submit button when submitDisabled is set", () => {
    const props = baseProps({ submitDisabled: true });
    render(<PersonalTermsModal {...props} />);

    expect(screen.getByRole("button", { name: "Submit Terms" })).toBeDisabled();
  });

  it("shows the player's counter hint while talks are live", () => {
    const props = baseProps({ suggestedWage: 18000, suggestedYears: 4 });
    render(<PersonalTermsModal {...props} />);

    expect(screen.getByText(/They want/)).toBeInTheDocument();
  });

  it("hides submit and disables inputs once the deal is terminal", () => {
    const props = baseProps({ terminal: true, suggestedWage: 18000 });
    render(<PersonalTermsModal {...props} />);

    expect(
      screen.queryByRole("button", { name: "Submit Terms" }),
    ).not.toBeInTheDocument();
    expect(screen.getByLabelText("Weekly Wage")).toBeDisabled();
    // The suggested-counter hint is not shown after the talks have ended.
    expect(screen.queryByText(/They want/)).not.toBeInTheDocument();
  });

  it("closes when the close button is clicked", () => {
    const props = baseProps();
    render(<PersonalTermsModal {...props} />);

    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(props.onClose).toHaveBeenCalledTimes(1);
  });
});
