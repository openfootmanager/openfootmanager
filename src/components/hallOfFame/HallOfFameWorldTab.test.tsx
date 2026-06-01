import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import HallOfFameWorldTab from "./HallOfFameWorldTab";

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

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "V. Vale",
    full_name: "Victor Vale",
    date_of_birth: "1988-01-01",
    nationality: "ENG",
    football_nation: "ENG",
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
    condition: 0,
    morale: 0,
    injury: null,
    team_id: null,
    retired: true,
    contract_end: null,
    wage: 0,
    market_value: 0,
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
    career: [
      {
        season: 1,
        team_id: "team-1",
        team_name: "Alpha FC",
        appearances: 210,
        goals: 110,
        assists: 35,
      },
      {
        season: 2,
        team_id: "team-2",
        team_name: "Beta United",
        appearances: 95,
        goals: 28,
        assists: 12,
      },
    ],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ovr: 82,
    potential: 82,
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
    players: [
      createPlayer(),
      createPlayer({
        id: "player-2",
        match_name: "N. North",
        full_name: "Nico North",
        career: [
          {
            season: 2,
            team_id: "team-2",
            team_name: "Beta United",
            appearances: 80,
            goals: 12,
            assists: 18,
          },
        ],
      }),
      createPlayer({
        id: "active-player",
        match_name: "C. Current",
        full_name: "Current Pro",
        retired: false,
        team_id: "team-1",
        career: [],
      }),
    ],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("HallOfFameWorldTab", () => {
  it("renders retired legends and past champions from the existing game payload", () => {
    render(<HallOfFameWorldTab gameState={createGameState()} />);

    expect(screen.getByText("Hall of Fame")).toBeInTheDocument();
    expect(screen.getByText("Victor Vale")).toBeInTheDocument();
    expect(screen.getByText("Nico North")).toBeInTheDocument();
    expect(screen.queryByText("Current Pro")).not.toBeInTheDocument();
    expect(screen.getByText("Titles: 2")).toBeInTheDocument();
    expect(screen.getByText("Alpha FC")).toBeInTheDocument();
    expect(screen.getByText("Beta United")).toBeInTheDocument();
  });

  it("routes player and team selection from the hall of fame cards", () => {
    const onSelectPlayer = vi.fn();
    const onSelectTeam = vi.fn();

    render(
      <HallOfFameWorldTab
        gameState={createGameState()}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Victor Vale" }));
    fireEvent.click(screen.getByRole("button", { name: "Beta United" }));

    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");
    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
  });
});
