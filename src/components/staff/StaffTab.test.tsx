import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";

import type { GameStateData, StaffData, TeamData } from "../../store/gameStore";
import StaffTab from "./StaffTab";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>, fallback?: string) => {
      if (key === "finances.perWeekSuffix") return "/wk";
      if (key === "staff.myStaff") return `My Staff ${params?.count}`;
      if (key === "staff.available") return `Available ${params?.count}`;
      if (key === "staff.searchStaff") return "Search staff";
      if (key === "common.all") return "All";
      if (key === "staff.noStaffMatch") return "No staff match";
      if (key === "staff.noAvailableStaff") return "No available staff";
      if (key === "staff.releaseStaff") return "Release staff";
      if (key === "staff.hireStaff") return "Hire staff";
      if (key === "staff.openScoutingWorkflow") return "Open scouting workflow";
      if (key === "staff.activeAssignment") return "active assignment";
      if (key === "staff.activeAssignments") return "active assignments";
      if (key === "staff.youthSearch") return "youth search";
      if (key === "staff.youthSearches") return "youth searches";
      if (key === "common.age") return "Age";
      if (key === "staff.best") return "Best";
      if (key.startsWith("staff.roles.")) return key.replace("staff.roles.", "");
      if (key.startsWith("staff.attrs.")) return key.replace("staff.attrs.", "");
      if (key.startsWith("staff.specializations.")) return key.replace("staff.specializations.", "");
      return fallback ?? key;
    },
    i18n: { language: "en" },
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
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000000", secondary: "#ffffff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

function createStaff(overrides: Partial<StaffData> = {}): StaffData {
  return {
    id: "staff-1",
    first_name: "Alex",
    last_name: "Coach",
    date_of_birth: "1980-01-01",
    nationality: "GB",
    role: "Coach",
    attributes: {
      coaching: 70,
      judging_ability: 50,
      judging_potential: 55,
      physiotherapy: 30,
    },
    team_id: "team-1",
    specialization: "Youth",
    wage: 1200,
    contract_end: "2027-06-30",
    ...overrides,
  };
}

function createGameState(staff: StaffData[]): GameStateData {
  return {
    clock: {
      current_date: "2026-08-10T00:00:00Z",
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
    teams: [
      createTeam(),
      createTeam({ id: "team-2", name: "Beta FC", short_name: "BET", manager_id: "manager-2" }),
    ],
    players: [],
    staff,
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

function makeStaffSlice(staff: StaffData[], overrides: Partial<{ scouting_assignments: unknown[]; youth_scouting_assignments: unknown[] }> = {}) {
  return {
    team_staff: staff.filter((s) => s.team_id),
    available_staff: staff.filter((s) => !s.team_id),
    scouting_assignments: overrides.scouting_assignments ?? [],
    youth_scouting_assignments: overrides.youth_scouting_assignments ?? [],
  };
}

describe("StaffTab", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    const defaultStaff = [createStaff()];
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff") return makeStaffSlice(defaultStaff);
      return createGameState(defaultStaff);
    });
  });

  it("switches to available staff and filters by role and search", async () => {
    const staff = [
      createStaff(),
      createStaff({ id: "staff-2", first_name: "Sam", last_name: "Scout", role: "Scout", team_id: null }),
      createStaff({ id: "staff-3", first_name: "Pat", last_name: "Physio", role: "Physio", team_id: null }),
    ];
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff") return makeStaffSlice(staff);
      return createGameState(staff);
    });

    render(<StaffTab gameState={createGameState(staff)} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Available 2/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /Available 2/i }));
    fireEvent.click(screen.getByRole("button", { name: /Scout/i }));
    fireEvent.change(screen.getByPlaceholderText("Search staff"), {
      target: { value: "sam" },
    });

    expect(screen.getByText("Sam Scout")).toBeInTheDocument();
    expect(screen.queryByText("Pat Physio")).not.toBeInTheDocument();
  });

  it("hires an available staff member and forwards the updated state", async () => {
    const scout = createStaff({ id: "staff-2", first_name: "Sam", last_name: "Scout", role: "Scout", team_id: null });
    const updatedState = createGameState([]);
    const onGameUpdate = vi.fn();
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff") return makeStaffSlice([scout]);
      return updatedState;
    });

    render(<StaffTab gameState={createGameState([scout])} onGameUpdate={onGameUpdate} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Available 1/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /Available 1/i }));
    fireEvent.click(screen.getByTitle("Hire staff"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("hire_staff", { staffId: "staff-2" });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    });
  });

  it("offers a hire action from the staff card context menu", async () => {
    const scout = createStaff({ id: "staff-2", first_name: "Sam", last_name: "Scout", role: "Scout", team_id: null });
    const updatedState = createGameState([]);
    const onGameUpdate = vi.fn();
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff") return makeStaffSlice([scout]);
      return updatedState;
    });

    render(<StaffTab gameState={createGameState([scout])} onGameUpdate={onGameUpdate} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Available 1/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /Available 1/i }));
    fireEvent.contextMenu(screen.getByTestId("staff-card-staff-2"));
    fireEvent.click(
      within(screen.getByRole("menu")).getByRole("button", { name: "Hire staff" }),
    );

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("hire_staff", { staffId: "staff-2" });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    });
  });

  it("offers a release action from the staff card context menu", async () => {
    const staff = [createStaff()];
    const updatedState = createGameState([]);
    const onGameUpdate = vi.fn();
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff") return makeStaffSlice(staff);
      return updatedState;
    });

    render(<StaffTab gameState={createGameState(staff)} onGameUpdate={onGameUpdate} />);

    await waitFor(() => {
      expect(screen.getByTestId("staff-card-staff-1")).toBeInTheDocument();
    });
    fireEvent.contextMenu(screen.getByTestId("staff-card-staff-1"));
    fireEvent.click(
      within(screen.getByRole("menu")).getByRole("button", { name: "Release staff" }),
    );

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("release_staff", { staffId: "staff-1" });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    });
  });

  it("shows scout workload details and opens the scouting workflow", async () => {
    const scout = createStaff({ id: "staff-2", first_name: "Sam", last_name: "Scout", role: "Scout" });
    const scoutingAssignments = [
      { id: "sa-1", scout_id: "staff-2", player_id: "player-1", days_remaining: 2 },
    ];
    const youthAssignments = [
      { id: "ysa-1", scout_id: "staff-2", region: "Domestic", objective: "Balanced", target_position: "Defender", days_remaining: 5 },
    ];
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff") return makeStaffSlice([scout], { scouting_assignments: scoutingAssignments, youth_scouting_assignments: youthAssignments });
      return createGameState([scout]);
    });
    const onNavigate = vi.fn();

    render(<StaffTab gameState={createGameState([scout])} onNavigate={onNavigate} />);

    await waitFor(() => {
      expect(screen.getByText("2 active assignments")).toBeInTheDocument();
    });
    expect(screen.getByText("1 youth search")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Open scouting workflow" }));

    expect(onNavigate).toHaveBeenCalledWith("Scouting");
  });
});