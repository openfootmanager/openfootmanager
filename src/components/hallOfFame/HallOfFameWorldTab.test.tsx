import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { fetchHallOfFameLegends } from "../../services/retireesService";
import type { GameStateData, TeamData } from "../../store/gameStore";
import type { HallOfFameLegendData } from "../../store/types";
import HallOfFameWorldTab from "./HallOfFameWorldTab";

vi.mock("../../services/retireesService", () => ({
  fetchHallOfFameLegends: vi.fn(),
}));

const mockedFetchLegends = vi.mocked(fetchHallOfFameLegends);

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "hallOfFameWorld.title") return "Hall of Fame";
      if (key === "hallOfFameWorld.subtitle") return "Retired legends and past champions";
      if (key === "hallOfFameWorld.legends") return "Retired Legends";
      if (key === "hallOfFameWorld.pastChampions") return "Past Champions";
      if (key === "hallOfFameWorld.noLegends") return "No retired legends recorded yet.";
      if (key === "hallOfFameWorld.noChampions") return "No past champions recorded yet.";
      if (key === "hallOfFameWorld.appearances") return "Appearances";
      if (key === "hallOfFameWorld.goals") return "Goals";
      if (key === "hallOfFameWorld.assists") return "Assists";
      if (key === "hallOfFameWorld.titles") return "Titles";
      if (key === "hallOfFameWorld.lastClub") return "Last club";
      if (key === "hallOfFameWorld.finalSeason") return "Final season";
      if (key === "hallOfFameWorld.played") return "Played";
      if (key === "hallOfFameWorld.record") return "Record";
      if (key === "hallOfFameWorld.goalDifference") return "Goal difference";
      if (key === "hallOfFameWorld.legendsCount") return `${params?.count} legends`;
      if (key === "hallOfFameWorld.championsCount") return `${params?.count} title-winning seasons`;
      if (key === "hallOfFameWorld.seasonLabel") return `Season ${params?.season}`;
      if (key === "hallOfFameWorld.worldCupChampions") return "World Cup Champions";
      if (key === "hallOfFameWorld.worldCupEdition") return `${params?.year} World Cup`;
      return key;
    },
    i18n: { language: "en" },
  }),
}));

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

function createLegend(overrides: Partial<HallOfFameLegendData> = {}): HallOfFameLegendData {
  return {
    player_id: "player-1",
    full_name: "Victor Vale",
    nationality: "ENG",
    football_nation: "ENG",
    position: "Forward",
    appearances: 305,
    goals: 138,
    assists: 47,
    titles: 2,
    last_club_name: "Beta United",
    final_season: 2,
    ...overrides,
  };
}

function createGameState(): GameStateData {
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
    },
    teams: [
      createTeam({
        history: [
          {
            season: 1,
            league_position: 1,
            played: 38,
            won: 25,
            drawn: 8,
            lost: 5,
            goals_for: 72,
            goals_against: 30,
          },
        ],
      }),
      createTeam({
        id: "team-2",
        name: "Beta United",
        short_name: "BET",
        country: "ESP",
        history: [
          {
            season: 2,
            league_position: 1,
            played: 38,
            won: 24,
            drawn: 9,
            lost: 5,
            goals_for: 69,
            goals_against: 28,
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

describe("HallOfFameWorldTab", () => {
  beforeEach(() => {
    mockedFetchLegends.mockReset();
    mockedFetchLegends.mockResolvedValue([]);
  });

  /**
   * Given a world whose players list holds no retirees (they live in the archive)
   * When the Hall of Fame renders
   * Then the legends are the ones the backend names, and the champions still come from the clubs
   */
  it("renders the backend's retired legends and the clubs' past champions", async () => {
    mockedFetchLegends.mockResolvedValue([
      createLegend(),
      createLegend({ player_id: "player-2", full_name: "Nico North", titles: 1 }),
    ]);

    render(<HallOfFameWorldTab gameState={createGameState()} />);

    expect(await screen.findByText("Victor Vale")).toBeInTheDocument();
    expect(screen.getByText("Hall of Fame")).toBeInTheDocument();
    expect(screen.getByText("Nico North")).toBeInTheDocument();
    expect(screen.getByText("Titles: 2")).toBeInTheDocument();
    expect(mockedFetchLegends).toHaveBeenCalledTimes(1);
    expect(screen.getByText("Alpha FC")).toBeInTheDocument();
    expect(screen.getByText("Beta United")).toBeInTheDocument();
  });

  it("says so when the world has no legends yet", async () => {
    render(<HallOfFameWorldTab gameState={createGameState()} />);

    expect(await screen.findByText("No retired legends recorded yet.")).toBeInTheDocument();
  });

  it("shows World Cup champions when the world has crowned one", () => {
    const state = createGameState();
    state.world_history = {
      world_cup_champions: [
        { year: 2026, nation_code: "BR", nation_name: "Brazil" },
        { year: 2022, nation_code: "AR", nation_name: "Argentina" },
      ],
    };

    render(<HallOfFameWorldTab gameState={state} />);

    expect(screen.getByText("World Cup Champions")).toBeInTheDocument();
    expect(screen.getByTestId("world-cup-champion-2026")).toHaveTextContent("Brazil");
    expect(screen.getByTestId("world-cup-champion-2022")).toHaveTextContent("Argentina");
    expect(screen.getByText("2026 World Cup")).toBeInTheDocument();
  });

  it("hides the World Cup section before any cup is won", () => {
    render(<HallOfFameWorldTab gameState={createGameState()} />);
    expect(screen.queryByText("World Cup Champions")).not.toBeInTheDocument();
  });

  it("routes player and team selection from the hall of fame cards", async () => {
    mockedFetchLegends.mockResolvedValue([createLegend()]);
    const onSelectPlayer = vi.fn();
    const onSelectTeam = vi.fn();

    render(
      <HallOfFameWorldTab
        gameState={createGameState()}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Victor Vale" }));
    fireEvent.click(screen.getByRole("button", { name: "Beta United" }));

    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");
    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });
});
