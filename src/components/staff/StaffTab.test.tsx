import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";

import type { GameStateData, StaffData, TeamData } from "../../store/gameStore";
import type { RatingTier } from "../../services/squadService";
import { useSettingsStore } from "../../store/settingsStore";
import StaffTab from "./StaffTab";
import { createSeasonContext } from "../../test-utils/factories";

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
      if (key === "common.overallRating") return `Overall ${params?.value}`;
      if (key === "common.overallRatingTier")
        return `Overall ${params?.value}, tier ${params?.tier}`;
      if (key.startsWith("common.ratingTier.")) return key.replace("common.ratingTier.", "");
      if (key === "common.ovr") return "OVR";
      if (key === "common.listViewTable") return "Table";
      if (key === "common.listViewCards") return "Cards";
      if (key === "staff.staffActions") return `Staff actions for ${params?.name}`;
      if (key === "staff.best") return "Best";
      if (key.startsWith("staff.roles.")) return key.replace("staff.roles.", "");
      if (key.startsWith("staff.attrs.")) return key.replace("staff.attrs.", "");
      if (key.startsWith("staff.specializations."))
        return key.replace("staff.specializations.", "");
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
    display_name: "Alex Coach",
    date_of_birth: "1980-01-01",
    nationality: "GB",
    role: "Coach",
    attributes: {
      coaching: 70,
      judgingAbility: 50,
      judgingPotential: 55,
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
    season_context: createSeasonContext(),
    clock: {
      current_date: "2026-08-10T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: {
      display_name: "Jane Doe",
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

/**
 * The backend's staff slice: each row is the member with the rating the game computes for his
 * role (ofm_core's staff_rating), 50 unless a test names one.
 */
function makeStaffSlice(
  staff: StaffData[],
  overrides: Partial<{
    scouting_assignments: unknown[];
    youth_scouting_assignments: unknown[];
    ratings: Record<string, number>;
    tiers: Record<string, RatingTier>;
  }> = {},
) {
  const row = (member: StaffData) => ({
    ...member,
    rating: overrides.ratings?.[member.id] ?? 50,
    tier: overrides.tiers?.[member.id] ?? "Rest",
  });
  return {
    team_staff: staff.filter((s) => s.team_id).map(row),
    available_staff: staff.filter((s) => !s.team_id).map(row),
    scouting_assignments: overrides.scouting_assignments ?? [],
    youth_scouting_assignments: overrides.youth_scouting_assignments ?? [],
  };
}

function setListView(view: "table" | "cards") {
  useSettingsStore.setState({
    settings: { ...useSettingsStore.getState().settings, list_view: view },
  });
}

describe("StaffTab", () => {
  beforeEach(() => {
    setListView("cards");
    invokeMock.mockReset();
    const defaultStaff = [createStaff()];
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff") return makeStaffSlice(defaultStaff);
      return createGameState(defaultStaff);
    });
  });

  /**
   * Given a physio whose attributes, mixed any way here, would not make 77, and the backend's
   *       slice rating him 77
   * When his card is drawn
   * Then it shows 77: the rating is the game's (ofm_core's staff_rating, one rule per role,
   *      tested in Rust), never a formula of the page's own.
   */
  it("shows the rating the backend sends for each member", async () => {
    const physio = createStaff({
      id: "staff-physio",
      first_name: "Pat",
      last_name: "Physio",
      role: "Physio",
      attributes: { coaching: 30, judgingAbility: 30, judgingPotential: 30, physiotherapy: 90 },
    });
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff")
        return makeStaffSlice([physio], { ratings: { "staff-physio": 77 } });
      return createGameState([physio]);
    });

    render(
      <StaffTab
        gameState={createGameState([physio])}
        onGameUpdate={() => {}}
        onNavigate={() => {}}
      />,
    );

    expect(await screen.findByText("Overall 77")).toBeInTheDocument();
  });

  /**
   * Given a coach the backend rates 88 and tiers S among coaches, and a physio it rates 80 and
   *       tiers as the rest among physios
   * When their cards are drawn
   * Then each rating is painted in the tone of the tier the backend sent, and its tier is said in
   *      words: the page holds no cut-off of its own, so the coach's 88 is S and the physio's 80
   *      is not.
   */
  it("paints each member's rating in his tier among his role, as the backend sends it", async () => {
    const coach = createStaff({ id: "staff-coach", role: "Coach" });
    const physio = createStaff({ id: "staff-physio", role: "Physio" });
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff")
        return makeStaffSlice([coach, physio], {
          ratings: { "staff-coach": 88, "staff-physio": 80 },
          tiers: { "staff-coach": "S", "staff-physio": "Rest" },
        });
      return createGameState([coach, physio]);
    });

    render(
      <StaffTab
        gameState={createGameState([coach, physio])}
        onGameUpdate={() => {}}
        onNavigate={() => {}}
      />,
    );

    const coachChip = (await screen.findByText("Overall 88, tier S")).parentElement;
    expect(coachChip).toHaveClass("bg-tone-top-tint", "text-tone-top");
    const physioChip = screen.getByText("Overall 80").parentElement;
    expect(physioChip).toHaveClass("bg-sunken", "text-fg-muted");
  });

  /**
   * Given the backend's slice not yet answered, so the page draws the game state's staff, which
   *       carry no rating
   * When a card is drawn
   * Then its rating reads "–" rather than a number worked out here.
   */
  it("shows no rating before the backend's slice arrives", () => {
    const coach = createStaff({ id: "staff-coach" });
    invokeMock.mockImplementation(() => new Promise(() => {}));

    render(
      <StaffTab
        gameState={createGameState([coach])}
        onGameUpdate={() => {}}
        onNavigate={() => {}}
      />,
    );

    expect(screen.getByText("– OVR")).toBeInTheDocument();
  });

  /**
   * Given an available coach the backend rates 64, and a hire that returns the new game state
   *       (whose staff carry no rating)
   * When he is hired
   * Then his card still shows 64: the page asks the backend for the slice again rather than
   *      rebuilding it from the game state and losing every rating.
   */
  it("keeps each member's rating after a hire", async () => {
    const free = createStaff({ id: "staff-free", team_id: null });
    const hired = { ...free, team_id: "team-1" };
    let hiredYet = false;
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "hire_staff") {
        hiredYet = true;
        return createGameState([hired]);
      }
      if (command === "get_staff") {
        return makeStaffSlice([hiredYet ? hired : free], { ratings: { "staff-free": 64 } });
      }
      return createGameState([free]);
    });

    render(
      <StaffTab
        gameState={createGameState([free])}
        onGameUpdate={() => {}}
        onNavigate={() => {}}
      />,
    );
    fireEvent.click(await screen.findByRole("button", { name: /Available 1/i }));
    fireEvent.click(screen.getByRole("button", { name: "Hire staff" }));

    await waitFor(() => expect(hiredYet).toBe(true));
    fireEvent.click(await screen.findByRole("button", { name: /My Staff 1/i }));
    expect(await screen.findByText("Overall 64")).toBeInTheDocument();
  });

  /**
   * Given two free agents hired one after the other, before the first hire's refresh answers
   * When the second refresh (both hired) answers first, and the first (one hired) answers last
   * Then the page keeps the newer list, both on the staff, rather than the older snapshot that
   *      arrived late.
   */
  /**
   * Given the page's first staff fetch still out, so the page draws the game state's staff
   * When a free agent is hired, its refresh answers (him on the staff), and only then the first
   *      fetch answers with the list from before the hire
   * Then the page keeps the hire: the first fetch is older than the hire's refresh, so it is
   *      dropped as a late refresh would be.
   */
  it("keeps a hire when the page's first fetch answers after it", async () => {
    const free = createStaff({ id: "free", display_name: "Fred Free", team_id: null });
    const hired = { ...free, team_id: "team-1" };
    let answerFirstFetch: ((slice: unknown) => void) | null = null;
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_staff") {
        if (!answerFirstFetch) {
          return new Promise((resolve) => {
            answerFirstFetch = resolve;
          });
        }
        return Promise.resolve(makeStaffSlice([hired]));
      }
      return Promise.resolve(createGameState([hired]));
    });

    render(
      <StaffTab
        gameState={createGameState([free])}
        onGameUpdate={() => {}}
        onNavigate={() => {}}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Available 1/i }));
    fireEvent.click(screen.getByRole("button", { name: "Hire staff" }));
    expect(await screen.findByRole("button", { name: /My Staff 1/i })).toBeInTheDocument();

    await act(async () => {
      answerFirstFetch?.(makeStaffSlice([free]));
    });

    expect(screen.getByRole("button", { name: /My Staff 1/i })).toBeInTheDocument();
  });

  /**
   * Given the page open on one club, its staff fetch still out, when the manager moves to another
   *       club
   * When the new club's fetch answers, and only then the old club's
   * Then the page shows the new club's staff: the old club's late answer is dropped.
   */
  it("shows the new club's staff when the old club's fetch answers after a move", async () => {
    const oldCoach = createStaff({ id: "old", display_name: "Olly Old", team_id: "team-1" });
    const newCoach = createStaff({ id: "new", display_name: "Nina New", team_id: "team-2" });
    let answerOldClub: ((slice: unknown) => void) | null = null;
    invokeMock.mockImplementation((command: string, args?: { teamId?: string }) => {
      if (command === "get_staff" && args?.teamId === "team-1") {
        return new Promise((resolve) => {
          answerOldClub = resolve;
        });
      }
      if (command === "get_staff") return Promise.resolve(makeStaffSlice([newCoach]));
      return Promise.resolve(createGameState([]));
    });
    const atClub = (teamId: string, staff: StaffData[]) => {
      const state = createGameState(staff);
      return { ...state, manager: { ...state.manager, team_id: teamId } };
    };

    const { rerender } = render(
      <StaffTab
        gameState={atClub("team-1", [oldCoach])}
        onGameUpdate={() => {}}
        onNavigate={() => {}}
      />,
    );
    rerender(
      <StaffTab
        gameState={atClub("team-2", [newCoach])}
        onGameUpdate={() => {}}
        onNavigate={() => {}}
      />,
    );
    expect(await screen.findByText("Nina New")).toBeInTheDocument();

    await act(async () => {
      if (!answerOldClub) throw new Error("the old club's staff fetch was never sent");
      answerOldClub(makeStaffSlice([oldCoach]));
    });

    expect(screen.getByText("Nina New")).toBeInTheDocument();
    expect(screen.queryByText("Olly Old")).toBeNull();
  });

  it("keeps the newest staff list when an older refresh answers last", async () => {
    const first = createStaff({ id: "first", display_name: "First Free", team_id: null });
    const second = createStaff({ id: "second", display_name: "Second Free", team_id: null });
    const hired = (member: StaffData) => ({ ...member, team_id: "team-1" });
    const refreshes: ((slice: unknown) => void)[] = [];
    let opened = false;
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_staff") {
        if (!opened) {
          opened = true;
          return Promise.resolve(makeStaffSlice([first, second]));
        }
        return new Promise((resolve) => refreshes.push(resolve));
      }
      return Promise.resolve(createGameState([first, second]));
    });

    render(
      <StaffTab
        gameState={createGameState([first, second])}
        onGameUpdate={() => {}}
        onNavigate={() => {}}
      />,
    );
    fireEvent.click(await screen.findByRole("button", { name: /Available 2/i }));
    const [hireFirst, hireSecond] = screen.getAllByRole("button", { name: "Hire staff" });
    fireEvent.click(hireFirst);
    await waitFor(() => expect(refreshes).toHaveLength(1));
    fireEvent.click(hireSecond);
    await waitFor(() => expect(refreshes).toHaveLength(2));

    await act(async () => {
      refreshes[1](makeStaffSlice([hired(first), hired(second)]));
    });
    await act(async () => {
      refreshes[0](makeStaffSlice([hired(first), second]));
    });

    expect(screen.getByRole("button", { name: /My Staff 2/i })).toBeInTheDocument();
  });

  it("switches to available staff and filters by role and search", async () => {
    const staff = [
      createStaff(),
      createStaff({
        id: "staff-2",
        first_name: "Sam",
        last_name: "Scout",
        display_name: "Sam Scout",
        role: "Scout",
        team_id: null,
      }),
      createStaff({
        id: "staff-3",
        first_name: "Pat",
        last_name: "Physio",
        display_name: "Pat Physio",
        role: "Physio",
        team_id: null,
      }),
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
    const scout = createStaff({
      id: "staff-2",
      first_name: "Sam",
      last_name: "Scout",
      role: "Scout",
      team_id: null,
    });
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
      expect(invokeMock).toHaveBeenCalledWith("hire_staff", { careerToken: 0, staffId: "staff-2" });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    });
  });

  it("offers a hire action from the staff card context menu", async () => {
    const scout = createStaff({
      id: "staff-2",
      first_name: "Sam",
      last_name: "Scout",
      role: "Scout",
      team_id: null,
    });
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
    fireEvent.click(within(screen.getByRole("menu")).getByRole("menuitem", { name: "Hire staff" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("hire_staff", { careerToken: 0, staffId: "staff-2" });
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
      within(screen.getByRole("menu")).getByRole("menuitem", { name: "Release staff" }),
    );

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("release_staff", {
        careerToken: 0,
        staffId: "staff-1",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    });
  });

  it("renders judging attribute values when backend sends camelCase keys", async () => {
    // Regression: domain::StaffAttributes serializes as camelCase
    // (judgingAbility / judgingPotential); the frontend used to read
    // snake_case → values came back undefined, OVR rendered as "NaN OVR" and
    // the AttrBar showed no number with a full-width bar. See commit 09d33244.
    const staffCamelCase = createStaff({
      attributes: { coaching: 63, judgingAbility: 52, judgingPotential: 71, physiotherapy: 44 },
    });
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff") {
        return {
          team_staff: [{ ...staffCamelCase, rating: 63 }],
          available_staff: [],
          scouting_assignments: [],
          youth_scouting_assignments: [],
        };
      }
      return createGameState([]);
    });

    render(<StaffTab gameState={createGameState([])} />);

    const card = await screen.findByTestId("staff-card-staff-1");
    // The backend rates him 63. The camelCase guard rests on the attribute
    // assertions below, not on this number.
    expect(within(card).getByText("Overall 63")).toBeInTheDocument();
    expect(within(card).getByText("52")).toBeInTheDocument();
    expect(within(card).getByText("71")).toBeInTheDocument();
    expect(within(card).getByText(/judgingPotential \(71\)/)).toBeInTheDocument();
  });

  it("shows scout workload details and opens the scouting workflow", async () => {
    const scout = createStaff({
      id: "staff-2",
      first_name: "Sam",
      last_name: "Scout",
      role: "Scout",
    });
    const scoutingAssignments = [
      { id: "sa-1", scout_id: "staff-2", player_id: "player-1", days_remaining: 2 },
    ];
    const youthAssignments = [
      {
        id: "ysa-1",
        scout_id: "staff-2",
        region: "Domestic",
        objective: "Balanced",
        target_position: "Defender",
        days_remaining: 5,
      },
    ];
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff")
        return makeStaffSlice([scout], {
          scouting_assignments: scoutingAssignments,
          youth_scouting_assignments: youthAssignments,
        });
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

  /**
   * Given a staff member known as "Cafu" whose full name is Marcos Evangelista
   * When the staff card is drawn
   * Then it shows the backend's display name, not the first and last name put together.
   */
  it("shows the staff member's display name on their card", async () => {
    const known = createStaff({
      id: "staff-cafu",
      first_name: "Marcos",
      last_name: "Evangelista",
      display_name: "Cafu",
    });
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff") return makeStaffSlice([known]);
      return createGameState([known]);
    });

    render(<StaffTab gameState={createGameState([known])} />);

    const card = await screen.findByTestId("staff-card-staff-cafu");
    expect(within(card).getByText("Cafu")).toBeInTheDocument();
    expect(within(card).queryByText("Marcos Evangelista")).not.toBeInTheDocument();
  });

  /**
   * Given an available staff member known as "Cafu" (Marcos Evangelista)
   * When the manager searches by either name
   * Then the staff member is found.
   */
  it("finds a known-as staff member by their display name or their full name", async () => {
    const known = createStaff({
      id: "staff-cafu",
      first_name: "Marcos",
      last_name: "Evangelista",
      display_name: "Cafu",
      role: "Scout",
      team_id: null,
    });
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "get_staff") return makeStaffSlice([known]);
      return createGameState([known]);
    });
    render(<StaffTab gameState={createGameState([known])} />);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Available 1/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /Available 1/i }));

    for (const query of ["cafu", "evangelista"]) {
      fireEvent.change(screen.getByPlaceholderText("Search staff"), { target: { value: query } });
      expect(screen.getByText("Cafu")).toBeInTheDocument();
    }
  });
});

/**
 * The club's three staff: a physio rated 90, a coach 70 and a scout 40, coaching 80, 85 and 20, and
 * paid 3000, 1000 and 2000, so each column gives its own order.
 */
function clubOfThree(): StaffData[] {
  return [
    createStaff({
      id: "coach",
      wage: 1000,
      display_name: "Cara Coach",
      role: "Coach",
      attributes: { coaching: 85, judgingAbility: 40, judgingPotential: 40, physiotherapy: 30 },
    }),
    createStaff({
      id: "physio",
      wage: 3000,
      display_name: "Pat Physio",
      role: "Physio",
      attributes: { coaching: 80, judgingAbility: 30, judgingPotential: 30, physiotherapy: 88 },
    }),
    createStaff({
      id: "scout",
      wage: 2000,
      display_name: "Sam Scout",
      role: "Scout",
      attributes: { coaching: 20, judgingAbility: 40, judgingPotential: 40, physiotherapy: 10 },
    }),
  ];
}

const CLUB_RATINGS = { coach: 70, physio: 90, scout: 40 };

/** The staff table's rows, top down, by the member each names. */
function staffOrder(): string[] {
  const names = ["Cara Coach", "Pat Physio", "Sam Scout", "Fred Free"];
  return within(screen.getByRole("table"))
    .getAllByRole("row")
    .slice(1)
    .map((row) => names.find((name) => row.textContent?.includes(name)) ?? "?");
}

function renderClub(staff: StaffData[] = clubOfThree()) {
  invokeMock.mockImplementation(async (command: string) => {
    if (command === "get_staff") return makeStaffSlice(staff, { ratings: CLUB_RATINGS });
    return createGameState(staff);
  });
  render(
    <StaffTab gameState={createGameState(staff)} onGameUpdate={() => {}} onNavigate={() => {}} />,
  );
}

describe("StaffTab as a table", () => {
  beforeEach(() => {
    setListView("table");
    invokeMock.mockReset();
  });

  /**
   * Given the club's physio, coach and scout, rated 90, 70 and 40 by the backend
   * When the Staff page opens in the table view (the default)
   * Then they are one table, highest rating first, each row showing the backend's rating.
   */
  it("lists the club's staff as a table, highest rating first", async () => {
    renderClub();

    await waitFor(() => expect(staffOrder()).toEqual(["Pat Physio", "Cara Coach", "Sam Scout"]));
    const physioRow = within(screen.getByRole("table"))
      .getAllByRole("row")
      .find((row) => row.textContent?.includes("Pat Physio"));
    expect(physioRow?.textContent).toContain("90");
  });

  /**
   * Given the table in rating order
   * When the coaching header is pressed
   * Then the staff run from the best coach (the coach 85, the physio 80, the scout 20): an
   *      attribute starts from the best.
   */
  it("sorts by an attribute, best first", async () => {
    renderClub();
    await waitFor(() => expect(staffOrder()).toHaveLength(3));

    fireEvent.click(screen.getByRole("button", { name: "coaching" }));

    expect(staffOrder()).toEqual(["Cara Coach", "Pat Physio", "Sam Scout"]);
  });

  /**
   * Given a member of the club's staff in the table
   * When his row's actions button is pressed and Release is chosen
   * Then the menu is the card's, and the release goes to the backend for him.
   */
  it("releases a member from his row's actions menu", async () => {
    renderClub();
    await waitFor(() => expect(staffOrder()).toHaveLength(3));

    fireEvent.click(screen.getByRole("button", { name: "Staff actions for Cara Coach" }));
    fireEvent.click(
      within(screen.getByRole("menu")).getByRole("menuitem", { name: "Release staff" }),
    );

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("release_staff", {
        careerToken: 0,
        staffId: "coach",
      }),
    );
  });

  /**
   * Given a free agent on the hiring market
   * When the Available list's row actions button is pressed and Hire is chosen
   * Then the hire goes to the backend for him.
   */
  it("hires a free agent from his row's actions menu", async () => {
    const free = createStaff({ id: "free", display_name: "Fred Free", team_id: null });
    renderClub([...clubOfThree(), free]);

    fireEvent.click(await screen.findByRole("button", { name: /Available 1/i }));
    fireEvent.click(screen.getByRole("button", { name: "Staff actions for Fred Free" }));
    fireEvent.click(within(screen.getByRole("menu")).getByRole("menuitem", { name: "Hire staff" }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("hire_staff", { careerToken: 0, staffId: "free" }),
    );
  });

  /**
   * Given the Staff page as a table
   * When Cards is picked on the view switch
   * Then the staff are drawn as cards, and the choice is saved as the one list view every list
   *      opens with.
   */
  it("switches to cards from the view switch and saves the choice", async () => {
    renderClub();
    await waitFor(() => expect(staffOrder()).toHaveLength(3));

    await act(async () => {
      fireEvent.click(screen.getByRole("radio", { name: "Cards" }));
    });

    expect(useSettingsStore.getState().settings.list_view).toBe("cards");
    expect(invokeMock).toHaveBeenCalledWith("save_settings", {
      settings: expect.objectContaining({ list_view: "cards" }),
    });
    expect(screen.queryByRole("table")).toBeNull();
    expect(screen.getByTestId("staff-card-coach")).toBeInTheDocument();
  });
});
