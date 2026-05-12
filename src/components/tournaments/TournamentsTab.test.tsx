import { fireEvent, render, screen } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { FixtureData, GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import TournamentsTab from "./TournamentsTab";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "tournaments.noActive") return "No active tournament";
      if (key === "schedule.standings") return "Standings";
      if (key === "schedule.fixtures") return "Fixtures";
      if (key === "common.viewTeam") return "View team";
      if (key === "squad.viewProfile") return "View profile";
      if (key === "tournaments.overview") return "Overview";
      if (key === "tournaments.leagueTable") return "League Table";
      if (key === "tournaments.nTeams") return `${params?.count} teams`;
      if (key === "tournaments.progress") return "Progress";
      if (key === "tournaments.matches") return "Matches";
      if (key === "tournaments.goals") return "Goals";
      if (key === "tournaments.topScorers") return "Top Scorers";
      if (key === "tournaments.noGoals") return "No goals yet";
      if (key === "schedule.season") return `Season ${params?.number}`;
      if (key === "common.team") return "Team";
      if (key === "common.played") return "P";
      if (key === "common.won") return "W";
      if (key === "common.drawn") return "D";
      if (key === "common.lost") return "L";
      if (key === "common.gd") return "GD";
      if (key === "common.pts") return "Pts";
      if (key === "common.position") return "Position";
      if (key.startsWith("season.phases.")) return key.replace("season.phases.", "");
      return key;
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

function createFixture(overrides: Partial<FixtureData> = {}): FixtureData {
  return {
    id: "fixture-1",
    matchday: 1,
    date: "2026-08-01",
    home_team_id: "team-1",
    away_team_id: "team-2",
    competition: "League",
    status: "Completed",
    result: {
      home_goals: 1,
      away_goals: 0,
      home_scorers: [{ player_id: "player-1", minute: 14 }],
      away_scorers: [],
    },
    ...overrides,
  };
}

function createGameState(withLeague = true): GameStateData {
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
    players: [
      createPlayer(),
      createPlayer({ id: "player-2", team_id: "team-2", full_name: "Alex Beta" }),
    ],
    staff: [],
    messages: [],
    news: [],
    league: withLeague
      ? {
        id: "league-1",
        name: "Premier League",
        season: 1,
        fixtures: [createFixture()],
        standings: [
          {
            team_id: "team-1",
            played: 1,
            won: 1,
            drawn: 0,
            lost: 0,
            goals_for: 1,
            goals_against: 0,
            points: 3,
          },
          {
            team_id: "team-2",
            played: 1,
            won: 0,
            drawn: 0,
            lost: 1,
            goals_for: 0,
            goals_against: 1,
            points: 0,
          },
        ],
      }
      : null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("TournamentsTab", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("renders the empty state when there is no active tournament", () => {
    render(<TournamentsTab gameState={createGameState(false)} onSelectTeam={vi.fn()} />);

    expect(screen.getByText("No active tournament")).toBeInTheDocument();
  });

  it("switches to standings and lets the user select a team", () => {
    const onSelectTeam = vi.fn();

    render(<TournamentsTab gameState={createGameState(true)} onSelectTeam={onSelectTeam} />);

    fireEvent.click(screen.getByRole("button", { name: /Standings/i }));
    fireEvent.click(screen.getAllByText("Beta FC")[0]);

    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });

  it("offers fixture context menu actions to open a team", () => {
    const onSelectTeam = vi.fn();

    render(<TournamentsTab gameState={createGameState(true)} onSelectTeam={onSelectTeam} />);

    fireEvent.click(screen.getByRole("button", { name: /Fixtures/i }));
    fireEvent.contextMenu(screen.getByTestId("tournaments-fixture-fixture-1"));
    fireEvent.click(screen.getByRole("button", { name: "View team: Beta FC" }));

    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });

  it("offers a top-scorer context menu action to view the player profile", () => {
    const onSelectPlayer = vi.fn();

    render(
      <TournamentsTab
        gameState={createGameState(true)}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={vi.fn()}
      />,
    );

    fireEvent.contextMenu(screen.getByTestId("tournaments-top-scorer-player-1"));
    fireEvent.click(screen.getByRole("button", { name: "View profile" }));

    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");
  });
});
