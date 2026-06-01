import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import TeamsListTab from "./TeamsListTab";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      if (key === "teams.yourTeam") return "Your Team";
      if (key === "common.position") return "Position";
      if (key === "teams.squad") return "Squad";
      if (key === "teams.avgOvr") return "Avg OVR";
      if (key === "teams.rep") return "Rep";
      if (key === "common.value") return "Value";
      if (key === "common.pts") return "Pts";
      if (key === "teams.est") return "Est";
      if (key === "common.playStyles.Balanced") return "Equilibrado";
      if (key === "common.playStyles.Counter") return "Contra-ataque";
      return fallback ?? key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../ui", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../ui")>();

  return {
    ...actual,
    TeamLocation: ({ city, countryCode }: { city: string; countryCode: string }) => (
      <span>{`${city}, ${countryCode}`}</span>
    ),
  };
});

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

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
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

function createGameState(): GameStateData {
  return {
    clock: {
      current_date: "2026-08-01T00:00:00Z",
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
      createTeam({
        id: "team-2",
        name: "Beta FC",
        short_name: "BET",
        manager_id: "manager-2",
        founded_year: 1910,
      }),
    ],
    players: [
      createPlayer({ team_id: "team-1", market_value: 400000 }),
      createPlayer({ id: "player-2", team_id: "team-1", market_value: 300000 }),
      createPlayer({ id: "player-3", team_id: "team-2", market_value: 250000 }),
    ],
    staff: [],
    messages: [],
    news: [],
    league: {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures: [],
      standings: [
        {
          team_id: "team-2",
          played: 1,
          won: 1,
          drawn: 0,
          lost: 0,
          goals_for: 2,
          goals_against: 0,
          points: 3,
        },
        {
          team_id: "team-1",
          played: 1,
          won: 0,
          drawn: 1,
          lost: 0,
          goals_for: 1,
          goals_against: 1,
          points: 1,
        },
      ],
    },
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("TeamsListTab", () => {
  it("orders teams by league position and marks the user team", () => {
    render(<TeamsListTab gameState={createGameState()} onSelectTeam={vi.fn()} />);

    const headings = screen.getAllByRole("heading", { level: 3 });

    expect(headings[0]).toHaveTextContent("Beta FC");
    expect(headings[1]).toHaveTextContent("Alpha FC");
    expect(screen.getByText("Your Team")).toBeInTheDocument();
  });

  it("selects a team when its card is clicked", () => {
    const onSelectTeam = vi.fn();

    render(<TeamsListTab gameState={createGameState()} onSelectTeam={onSelectTeam} />);

    fireEvent.click(screen.getByText("Beta FC"));

    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });

  it("renders translated play styles instead of raw values", () => {
    render(
      <TeamsListTab
        gameState={{
          ...createGameState(),
          teams: [
            createTeam({ play_style: "Balanced" }),
            createTeam({
              id: "team-2",
              name: "Beta FC",
              short_name: "BET",
              manager_id: "manager-2",
              founded_year: 1910,
              play_style: "Counter",
            }),
          ],
        }}
        onSelectTeam={vi.fn()}
      />,
    );

    expect(screen.getByText(/4-4-2 — Equilibrado/)).toBeInTheDocument();
    expect(screen.getByText(/4-4-2 — Contra-ataque/)).toBeInTheDocument();
    expect(screen.queryByText(/4-4-2 — Balanced/)).not.toBeInTheDocument();
    expect(screen.queryByText(/4-4-2 — Counter/)).not.toBeInTheDocument();
  });
});
