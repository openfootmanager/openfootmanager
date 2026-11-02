import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, ManagerData, TeamData } from "../../store/gameStore";
import { useSettingsStore } from "../../store/settingsStore";
import ManagersWorldTab from "./ManagersWorldTab";
import { createSeasonContext } from "../../test-utils/factories";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "managersWorld.title") return "Managers";
      if (key === "managersWorld.subtitle") return "League managers and open jobs";
      if (key === "managersWorld.vacancy") return "Vacancy";
      if (key === "managersWorld.employed") return "Employed";
      if (key === "managersWorld.unemployed") return "Unemployed";
      if (key === "managersWorld.reputation") return "Reputation";
      if (key === "managersWorld.winRate") return "Win rate";
      if (key === "managersWorld.matches") return "Matches";
      if (key === "managersWorld.currentClub") return "Current club";
      if (key === "managersWorld.openRole") return `Open role at ${params?.team}`;
      if (key === "managersWorld.vacancyBadge") return "Board searching";
      if (key === "managersWorld.noManagers") return "No managers available";
      if (key === "dashboard.manager") return "Manager";
      if (key === "manager.trophies") return "Trophies";
      if (key === "common.listViewTable") return "Table";
      if (key === "common.listViewCards") return "Cards";
      return key;
    },
    i18n: { language: "en" },
  }),
}));

function createManager(overrides: Partial<ManagerData> = {}): ManagerData {
  return {
    id: "manager-1",
    first_name: "Jane",
    last_name: "Doe",
    display_name: "Jane Doe",
    date_of_birth: "1980-01-01",
    nationality: "ENG",
    football_nation: "ENG",
    reputation: 72,
    satisfaction: 63,
    fan_approval: 61,
    team_id: "team-1",
    career_stats: {
      matches_managed: 120,
      wins: 64,
      draws: 22,
      losses: 34,
      trophies: 2,
      best_finish: 1,
    },
    career_history: [],
    ...overrides,
  };
}

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "ENG",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 1000000,
    manager_id: "manager-1",
    reputation: 650,
    wage_budget: 500000,
    transfer_budget: 750000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-3-3",
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

function createGameState(): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: {
      current_date: "2026-08-10T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: createManager(),
    managers: [
      createManager(),
      createManager({
        id: "manager-2",
        first_name: "Marco",
        last_name: "Rossi",
        display_name: "Marco Rossi",
        team_id: null,
        reputation: 58,
        career_stats: {
          matches_managed: 84,
          wins: 35,
          draws: 18,
          losses: 31,
          trophies: 0,
          best_finish: 4,
        },
      }),
    ],
    teams: [
      createTeam(),
      createTeam({
        id: "team-2",
        name: "Beta United",
        short_name: "BET",
        manager_id: null,
        history: [
          {
            season: 2024,
            league_position: 7,
            played: 32,
            won: 11,
            drawn: 8,
            lost: 13,
            goals_for: 40,
            goals_against: 44,
            competition_id: "",
          },
          {
            season: 2025,
            league_position: 5,
            played: 46,
            won: 18,
            drawn: 12,
            lost: 16,
            goals_for: 58,
            goals_against: 52,
            competition_id: "",
          },
        ],
      }),
    ],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

function setListView(view: "table" | "cards") {
  useSettingsStore.setState({
    settings: { ...useSettingsStore.getState().settings, list_view: view },
  });
}

describe("ManagersWorldTab", () => {
  beforeEach(() => setListView("cards"));

  it("renders employed managers, unemployed managers, and explicit vacancies", () => {
    render(<ManagersWorldTab gameState={createGameState()} />);

    expect(screen.getByText("Managers")).toBeInTheDocument();
    expect(screen.getByText("Jane Doe")).toBeInTheDocument();
    expect(screen.getByText("Marco Rossi")).toBeInTheDocument();
    expect(screen.getByText("Alpha FC")).toBeInTheDocument();
    expect(screen.getAllByText("Vacancy")).toHaveLength(2);
    expect(screen.getByText("Open role at Beta United")).toBeInTheDocument();
  });

  // Given a manager known by a short name (#557)
  // When the directory renders them
  // Then it shows the display name the backend decided, not first and last name composed here.
  it("shows the display name the backend decided", () => {
    const state = createGameState();
    state.managers = [
      createManager({
        id: "pep",
        first_name: "Josep",
        last_name: "Guardiola i Sala",
        known_as: "Pep Guardiola",
        display_name: "Pep Guardiola",
      }),
    ];

    render(<ManagersWorldTab gameState={state} />);

    expect(screen.getByText("Pep Guardiola")).toBeInTheDocument();
    expect(screen.queryByText("Josep Guardiola i Sala")).toBeNull();
  });

  it("routes team selection from manager clubs and vacancy cards", () => {
    const onSelectTeam = vi.fn();

    render(<ManagersWorldTab gameState={createGameState()} onSelectTeam={onSelectTeam} />);

    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));
    fireEvent.click(screen.getByRole("button", { name: "Open role at Beta United" }));

    expect(onSelectTeam).toHaveBeenNthCalledWith(1, "team-1");
    expect(onSelectTeam).toHaveBeenNthCalledWith(2, "team-2");
  });

  it("shows vacancy match totals from the latest recorded season", () => {
    render(<ManagersWorldTab gameState={createGameState()} />);

    expect(screen.getByText("46")).toBeInTheDocument();
  });
});

/**
 * Jane (72, Alpha FC, 120 matches, 2 trophies), Marco (58, between jobs, 84 matches, 3 trophies)
 * and Abe (50, between jobs, 300 matches, 5 trophies): each sort gives its own order.
 */
function gameStateWithThree(): GameStateData {
  const state = createGameState();
  const [jane, marco] = state.managers ?? [];
  state.managers = [
    jane,
    // Three trophies, so the trophy order (Abe, Marco, Jane) differs from the match order.
    { ...marco, career_stats: { ...marco.career_stats, trophies: 3 } },
    createManager({
      id: "manager-3",
      first_name: "Abe",
      last_name: "Zed",
      display_name: "Abe Zed",
      team_id: null,
      reputation: 50,
      career_stats: {
        matches_managed: 300,
        wins: 150,
        draws: 70,
        losses: 80,
        trophies: 5,
        best_finish: 1,
      },
    }),
  ];
  return state;
}

const MANAGER_NAMES = ["Jane Doe", "Marco Rossi", "Abe Zed"];

/** The managers table's rows, top down, by the manager each names. */
function managerOrder(): string[] {
  return within(screen.getByRole("table", { name: "Managers" }))
    .getAllByRole("row")
    .slice(1)
    .map((row) => {
      const text = within(row).getAllByRole("cell")[0].textContent ?? "";
      return MANAGER_NAMES.find((name) => text.includes(name)) ?? text;
    });
}

/** A manager's row, read by the header above each cell. */
function managerRow(name: string): Record<string, string> {
  const table = screen.getByRole("table", { name: "Managers" });
  const headers = within(table)
    .getAllByRole("columnheader")
    .map((header) => header.textContent?.trim() ?? "");
  const row = within(table)
    .getAllByRole("row")
    .find((candidate) => candidate.textContent?.includes(name));
  if (!row) throw new Error(`no row for ${name}`);
  const cells = within(row).getAllByRole("cell");
  return Object.fromEntries(
    headers.map((header, index) => [header, cells[index].textContent ?? ""]),
  );
}

describe("ManagersWorldTab as a table", () => {
  beforeEach(() => setListView("table"));

  /**
   * Given two managers in work or between jobs, and a third between jobs with the longest record
   * When the Managers page opens in the table view (the default)
   * Then they are one table, highest reputation first, with the columns Manager, Current club,
   *      Reputation, Matches and Trophies; a manager between jobs reads "Unemployed".
   */
  it("lists the managers as a table, highest reputation first", () => {
    render(<ManagersWorldTab gameState={gameStateWithThree()} />);

    expect(managerOrder()).toEqual(["Jane Doe", "Marco Rossi", "Abe Zed"]);
    expect(managerRow("Jane Doe")).toEqual({
      Manager: expect.stringContaining("Jane Doe"),
      "Current club": "Alpha FC",
      Reputation: "72",
      Matches: "120",
      Trophies: "2",
    });
    expect(managerRow("Marco Rossi")["Current club"]).toBe("Unemployed");
  });

  /**
   * Given the table in reputation order
   * When the Manager header is pressed, and then the Trophies header
   * Then the managers run A to Z by name, then by trophies high to low, then by matches high to
   *      low, and back to reputation high to low: each column starts in its own direction.
   */
  it("sorts by the header pressed, each from its own starting direction", () => {
    render(<ManagersWorldTab gameState={gameStateWithThree()} />);

    fireEvent.click(screen.getByRole("button", { name: "Manager" }));
    expect(managerOrder()).toEqual(["Abe Zed", "Jane Doe", "Marco Rossi"]);

    fireEvent.click(screen.getByRole("button", { name: "Trophies" }));
    expect(managerOrder()).toEqual(["Abe Zed", "Marco Rossi", "Jane Doe"]);

    fireEvent.click(screen.getByRole("button", { name: "Matches" }));
    expect(managerOrder()).toEqual(["Abe Zed", "Jane Doe", "Marco Rossi"]);

    fireEvent.click(screen.getByRole("button", { name: "Reputation" }));
    expect(managerOrder()).toEqual(["Jane Doe", "Marco Rossi", "Abe Zed"]);
  });

  /**
   * Given a manager's row and the vacancies table below it
   * When his club's name is pressed, and then the open club's
   * Then each opens that club. A manager's own row opens nothing yet: his profile page comes with
   *      the manager profile work (M3).
   */
  it("opens a club from a manager's row or a vacancy, and opens no manager yet", () => {
    const onSelectTeam = vi.fn();
    render(<ManagersWorldTab gameState={gameStateWithThree()} onSelectTeam={onSelectTeam} />);

    const managers = screen.getByRole("table", { name: "Managers" });
    fireEvent.click(within(managers).getByRole("button", { name: "Alpha FC" }));
    const vacancies = screen.getByRole("table", { name: "Vacancy" });
    fireEvent.click(within(vacancies).getByRole("button", { name: "Beta United" }));

    expect(onSelectTeam).toHaveBeenNthCalledWith(1, "team-1");
    expect(onSelectTeam).toHaveBeenNthCalledWith(2, "team-2");
    expect(within(managers).queryByRole("button", { name: "Jane Doe" })).toBeNull();
  });

  /**
   * Given a club without a manager, of reputation 650
   * When the vacancies table is drawn
   * Then it lists that club and its reputation, and no club that has a manager.
   */
  it("lists each open job with its club's reputation", () => {
    render(<ManagersWorldTab gameState={gameStateWithThree()} />);

    const vacancies = screen.getByRole("table", { name: "Vacancy" });
    const rows = within(vacancies).getAllByRole("row").slice(1);
    expect(rows.map((row) => row.textContent)).toEqual([expect.stringMatching(/Beta United.*650/)]);
  });

  /**
   * Given the Managers page as a table
   * When Cards is picked on the view switch
   * Then the managers are drawn as cards, and the choice is saved as the one list view every list
   *      opens with.
   */
  it("switches to cards from the view switch and saves the choice", async () => {
    render(<ManagersWorldTab gameState={gameStateWithThree()} />);

    await act(async () => {
      fireEvent.click(screen.getByRole("radio", { name: "Cards" }));
    });

    expect(useSettingsStore.getState().settings.list_view).toBe("cards");
    expect(screen.queryByRole("table")).toBeNull();
    expect(screen.getByText("Open role at Beta United")).toBeInTheDocument();
  });
});
