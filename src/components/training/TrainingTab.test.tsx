import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import TrainingTab from "./TrainingTab";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>, fallback?: string) => {
      if (key === "common.noTeam") return "No team";
      if (key === "training.staffAlert") return "Staff alert";
      if (key === "training.staffWarning") return "Staff warning";
      if (key === "training.staffSuggestion") return "Staff suggestion";
      if (key === "training.staffAdvice.critical") return `Critical advice ${params?.criticalCount} ${params?.scheduleAdvice}`;
      if (key === "training.staffAdvice.warn") return `Warning advice ${params?.avgCondition} ${params?.exhaustedCount} ${params?.scheduleAdvice}`;
      if (key === "training.staffAdvice.ok") return "Squad is in a good place";
      if (key === "training.weeklySchedule") return "Weekly Schedule";
      if (key === "training.trainingFocus") return "Training Focus";
      if (key === "training.intensity") return "Intensity";
      if (key === "training.trainingAppliedNote") return "Applied note";
      if (key === "training.recoveryNote") return "Recovery note";
      if (key === "training.squadFitness") return "Squad Fitness";
      if (key === "training.playerFitness") return "Player Fitness";
      if (key === "training.groups.trainingGroups") return "Training Groups";
      if (key === "training.groups.noGroups") return "No groups";
      if (key === "training.todayIs") return `${params?.day} is ${params?.type}`;
      if (key === "training.aTrainingDay") return "a training day";
      if (key === "training.aRestDay") return "a rest day";
      if (key === "training.train") return "Train";
      if (key === "training.rest") return "Rest";
      if (key.startsWith("training.schedules.")) return key.replace("training.schedules.", "");
      if (key.startsWith("training.staffAdvice.scheduleAdvice.")) {
        return key.replace("training.staffAdvice.scheduleAdvice.", "");
      }
      if (key.startsWith("training.focuses.")) return key.replace("training.focuses.", "");
      if (key.startsWith("training.intensities.")) return key.replace("training.intensities.", "");
      if (key.startsWith("training.days.")) return key.replace("training.days.", "");
      if (key.startsWith("common.attributes.")) return key.replace("common.attributes.", "");
      return fallback ?? key;
    },
  }),
}));

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "GB",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 500000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 250000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "Physical",
    training_intensity: "Medium",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000000", secondary: "#ffffff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2002-01-01",
    nationality: "GB",
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
      handling: 20,
      reflexes: 20,
      aerial: 60,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-1",
    contract_end: "2027-06-30",
    wage: 12000,
    market_value: 350000,
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

function createGameState(withTeam: boolean): GameStateData {
  return {
    clock: {
      current_date: "2026-08-11T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
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
    teams: withTeam ? [createTeam()] : [],
    players: withTeam ? [createPlayer()] : [],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("TrainingTab", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("renders the no-team state when the manager has no club", () => {
    render(<TrainingTab gameState={createGameState(false)} />);

    expect(screen.getByText("No team")).toBeInTheDocument();
  });

  it("updates the weekly schedule and forwards the refreshed state", async () => {
    const updatedState = createGameState(true);
    const onGameUpdate = vi.fn();
    invokeMock.mockResolvedValue(updatedState);

    render(
      <TrainingTab gameState={createGameState(true)} onGameUpdate={onGameUpdate} />,
    );

    fireEvent.click(screen.getByRole("button", { name: /Intense.label/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("set_training_schedule", {
        schedule: "Intense",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    });
  });

  it("renders the critical staff advice banner for an exhausted squad", () => {
    const exhaustedState = createGameState(true);
    exhaustedState.players = [
      createPlayer({ id: "p1", condition: 18 }),
      createPlayer({ id: "p2", condition: 20 }),
      createPlayer({ id: "p3", condition: 24 }),
      createPlayer({ id: "p4", condition: 55 }),
    ];

    render(<TrainingTab gameState={exhaustedState} />);

    expect(screen.getByText("Staff alert")).toBeInTheDocument();
    expect(screen.getByText(/Critical advice/)).toBeInTheDocument();
  });

  it("ignores youth academy players in first-team training summaries", () => {
    const state = createGameState(true);
    state.players = [
      createPlayer({ id: "senior-1", condition: 80 }),
      createPlayer({ id: "youth-1", condition: 10, squad_role: "Youth" }),
    ];

    render(<TrainingTab gameState={state} />);

    expect(screen.queryByText(/Critical advice/)).not.toBeInTheDocument();
  });
});