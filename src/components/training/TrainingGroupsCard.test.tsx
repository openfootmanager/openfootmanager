import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import TrainingGroupsCard from "./TrainingGroupsCard";

const setTrainingGroupsMock = vi.fn();
const setPlayerTrainingFocusMock = vi.fn();

vi.mock("../../services/trainingService", () => ({
  setTrainingGroups: (...args: unknown[]) => setTrainingGroupsMock(...args),
  setPlayerTrainingFocus: (...args: unknown[]) =>
    setPlayerTrainingFocusMock(...args),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, _params?: Record<string, string | number>, fallback?: string) => {
      if (key === "training.groups.trainingGroups") return "Training Groups";
      if (key === "training.groups.noGroups") return "No groups";
      if (key === "training.groups.addGroup") return "Add Group";
      if (key === "training.groups.trainingGroupsDesc") return "Groups description";
      if (key === "training.groups.teamDefault") return "Team Default";
      if (key === "training.groups.group") return "Group";
      if (key === "training.effectiveFocus") return "Effective Focus";
      if (key === "training.groups.removeGroup") return "Remove Group";
      if (key.startsWith("training.groups.defaultGroupNames.")) return fallback ?? "Group";
      if (key.startsWith("training.focuses.")) return key.replace("training.focuses.", "");
      if (key === "common.player") return "Player";
      if (key === "common.position") return "Position";
      return fallback ?? key;
    },
  }),
}));

function createTeam(overrides: Partial<TeamData> & { training_groups?: unknown } = {}): TeamData {
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
    retired: false,
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

function createGameState(team: TeamData): GameStateData {
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
    teams: [team],
    players: [createPlayer()],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("TrainingGroupsCard", () => {
  beforeEach(() => {
    setTrainingGroupsMock.mockReset();
    setPlayerTrainingFocusMock.mockReset();
  });

  it("renders the empty groups state", () => {
    render(
      <TrainingGroupsCard
        gameState={createGameState(createTeam())}
        onGameUpdate={vi.fn()}
        roster={[createPlayer()]}
        isSaving={false}
        setIsSaving={vi.fn()}
        trainingFocusIds={["Physical", "Technical", "Recovery"]}
        trainingFocusIcons={{}}
      />,
    );

    expect(screen.getByText("Training Groups")).toBeInTheDocument();
    expect(screen.getByText("No groups")).toBeInTheDocument();
  });

  it("adds a training group and forwards the updated game state", async () => {
    const updatedState = createGameState(
      createTeam({
        training_groups: [
          {
            id: "grp-1",
            name: "Group 1",
            focus: "Physical",
            player_ids: [],
          },
        ],
      }),
    );
    const onGameUpdate = vi.fn();
    const setIsSaving = vi.fn();
    setTrainingGroupsMock.mockResolvedValue(updatedState);

    render(
      <TrainingGroupsCard
        gameState={createGameState(createTeam())}
        onGameUpdate={onGameUpdate}
        roster={[createPlayer()]}
        isSaving={false}
        setIsSaving={setIsSaving}
        trainingFocusIds={["Physical", "Technical", "Recovery"]}
        trainingFocusIcons={{}}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /Add Group/i }));

    await waitFor(() => {
      expect(setTrainingGroupsMock).toHaveBeenCalledTimes(1);
      expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
      expect(setIsSaving).toHaveBeenCalledWith(true);
      expect(setIsSaving).toHaveBeenCalledWith(false);
    });
  });
});