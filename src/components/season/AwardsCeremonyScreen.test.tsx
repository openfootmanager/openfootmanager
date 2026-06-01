import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { GameStateData, ManagerData, PlayerData, TeamData } from "../../store/gameStore";
import type { SeasonAwardsData } from "../../store/types";
import AwardsCeremonyScreen from "./AwardsCeremonyScreen";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "awardsCeremony.title") return "Awards Ceremony";
      if (key === "awardsCeremony.subtitle") return `Season ${params?.season} in ${params?.league}`;
      if (key === "awardsCeremony.managerOfSeason") return "Manager of the Season";
      if (key === "awardsCeremony.goldenBoot") return "Golden Boot";
      if (key === "awardsCeremony.playerOfYear") return "Player of the Year";
      if (key === "awardsCeremony.continue") return "Continue to dashboard";
      if (key === "awardsCeremony.back") return "Back to news";
      if (key === "awardsCeremony.goals") return "Goals";
      if (key === "awardsCeremony.rating") return "Rating";
      if (key === "awardsCeremony.winRate") return "Win rate";
      if (key === "awardsCeremony.record") return "Record";
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
    date_of_birth: "1994-01-01",
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
    condition: 100,
    morale: 80,
    injury: null,
    team_id: "team-1",
    retired: false,
    contract_end: "2027-06-30",
    wage: 0,
    market_value: 0,
    stats: {
      appearances: 30,
      goals: 24,
      assists: 10,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 7.8,
      minutes_played: 2500,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ovr: 82,
    potential: 84,
    ...overrides,
  };
}

function createManager(overrides: Partial<ManagerData> = {}): ManagerData {
  return {
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
      matches_managed: 38,
      wins: 25,
      draws: 8,
      losses: 5,
      trophies: 2,
      best_finish: 1,
    },
    career_history: [],
    ...overrides,
  };
}

function createGameState(): GameStateData {
  return {
    clock: {
      current_date: "2026-08-10T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: createManager(),
    managers: [createManager()],
    teams: [createTeam()],
    players: [createPlayer()],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

function createAwards(): SeasonAwardsData {
  return {
    golden_boot: [
      {
        player_id: "player-1",
        player_name: "Victor Vale",
        team_id: "team-1",
        team_name: "Alpha FC",
        value: 24,
      },
    ],
    assist_king: [],
    player_of_year: [
      {
        player_id: "player-1",
        player_name: "Victor Vale",
        team_id: "team-1",
        team_name: "Alpha FC",
        value: 7.8,
      },
    ],
    clean_sheet_king: [],
    most_appearances: [],
    young_player: [],
    manager_of_season: [
      {
        manager_id: "manager-1",
        manager_name: "Jane Doe",
        team_id: "team-1",
        team_name: "Alpha FC",
        value: 79,
        win_rate: 66,
      },
    ],
  };
}

describe("AwardsCeremonyScreen", () => {
  it("renders marquee winners including manager of the season", () => {
    render(
      <AwardsCeremonyScreen
        season={6}
        leagueName="Premier League"
        gameState={createGameState()}
        awards={createAwards()}
      />,
    );

    expect(screen.getByText("Awards Ceremony")).toBeInTheDocument();
    expect(screen.getAllByText("Manager of the Season").length).toBeGreaterThan(0);
    expect(screen.getByText("Jane Doe")).toBeInTheDocument();
    expect(screen.getAllByText("Victor Vale").length).toBeGreaterThan(0);
  });

  it("routes winner interactions and renders ceremony actions", () => {
    const onSelectPlayer = vi.fn();
    const onSelectTeam = vi.fn();
    const onContinue = vi.fn();

    render(
      <AwardsCeremonyScreen
        season={6}
        leagueName="Premier League"
        gameState={createGameState()}
        awards={createAwards()}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
        onContinue={onContinue}
      />,
    );

    fireEvent.click(screen.getAllByRole("button", { name: "Victor Vale" })[0]!);
    fireEvent.click(screen.getAllByRole("button", { name: "Alpha FC" })[0]!);
    fireEvent.click(screen.getByRole("button", { name: "Continue to dashboard" }));

    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");
    expect(onSelectTeam).toHaveBeenCalledWith("team-1");
    expect(onContinue).toHaveBeenCalled();
  });

  it("renders non-interactive winner names when selection handlers are missing", () => {
    render(
      <AwardsCeremonyScreen
        season={6}
        leagueName="Premier League"
        gameState={createGameState()}
        awards={createAwards()}
      />,
    );

    expect(screen.queryByRole("button", { name: "Victor Vale" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Alpha FC" })).not.toBeInTheDocument();
    expect(screen.getAllByText("Victor Vale").length).toBeGreaterThan(0);
  });
});
