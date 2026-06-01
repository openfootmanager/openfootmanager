import { fireEvent, render, screen } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";

import type { GameStateData, SeasonAwardsData } from "../store/gameStore";
import EndOfSeasonScreen from "./EndOfSeasonScreen";

const setShowFiredModal = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "endOfSeason.seasonComplete") return "Season complete";
      if (key === "endOfSeason.seasonLine") return `Season ${params?.season} ${params?.league}`;
      if (key === "endOfSeason.position") return "Position";
      if (key === "endOfSeason.points") return "Points";
      if (key === "endOfSeason.finalStandings") return "Final standings";
      if (key === "endOfSeason.processing") return "Processing";
      if (key === "endOfSeason.startNextSeason") return "Start next season";
      if (key === "endOfSeason.statsArchived") return "Stats archived";
      if (key === "endOfSeason.continueDashboard") return "Continue to dashboard";
      if (key === "awardsCeremony.title") return "Awards Ceremony";
      if (key === "awardsCeremony.subtitle") return `Season ${params?.season} in ${params?.league}`;
      if (key === "awardsCeremony.managerOfSeason") return "Manager of the Season";
      if (key === "awardsCeremony.continue") return "Continue to dashboard";
      if (key === "awardsCeremony.goals") return "Goals";
      if (key === "awardsCeremony.rating") return "Rating";
      if (key === "awardsCeremony.winRate") return "Win rate";
      if (key === "common.won") return "W";
      if (key === "common.drawn") return "D";
      if (key === "common.lost") return "L";
      if (key === "common.gf") return "GF";
      if (key === "common.ga") return "GA";
      if (key === "common.place.1") return "1st";
      if (key === "common.place.2") return "2nd";
      if (key === "common.place.3") return "3rd";
      if (key === "common.place.other") return `${params?.n}th`;
      return key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../store/gameStore", async () => {
  const actual = await vi.importActual<typeof import("../store/gameStore")>("../store/gameStore");
  return {
    ...actual,
    useGameStore: (selector: (state: { setShowFiredModal: typeof setShowFiredModal }) => unknown) => selector({ setShowFiredModal }),
  };
});

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

function createGameState(): GameStateData {
  return {
    clock: {
      current_date: "2026-05-20T00:00:00Z",
      start_date: "2025-07-01T00:00:00Z",
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
        matches_managed: 38,
        wins: 25,
        draws: 8,
        losses: 5,
        trophies: 2,
        best_finish: 1,
      },
      career_history: [],
    },
    managers: [],
    teams: [
      {
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
      },
      {
        id: "team-2",
        name: "Beta FC",
        short_name: "BET",
        country: "ENG",
        city: "Manchester",
        stadium_name: "Beta Ground",
        stadium_capacity: 28000,
        finance: 1000000,
        manager_id: "manager-2",
        reputation: 620,
        wage_budget: 500000,
        transfer_budget: 750000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-3-3",
        play_style: "Balanced",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1902,
        colors: { primary: "#111111", secondary: "#eeeeee" },
        starting_xi_ids: [],
        form: [],
        history: [],
      },
    ],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: {
      id: "league-1",
      name: "Premier League",
      season: 6,
      standings: [
        {
          team_id: "team-1",
          played: 38,
          won: 25,
          drawn: 8,
          lost: 5,
          goals_for: 72,
          goals_against: 30,
          points: 83,
        },
        {
          team_id: "team-2",
          played: 38,
          won: 22,
          drawn: 9,
          lost: 7,
          goals_for: 65,
          goals_against: 35,
          points: 75,
        },
      ],
      fixtures: [],
      transfer_log: [],
    },
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("EndOfSeasonScreen", () => {
  it("shows the awards ceremony after advancing to the next season", async () => {
    vi.mocked(invoke).mockResolvedValue({
      game: createGameState(),
      summary: {
        season: 6,
        league_name: "Premier League",
        champion_id: "team-1",
        champion_name: "Alpha FC",
        user_position: 1,
        user_points: 83,
        user_won: 25,
        user_drawn: 8,
        user_lost: 5,
        user_goals_for: 72,
        user_goals_against: 30,
        golden_boot_player: "Victor Vale",
        golden_boot_goals: 24,
        poty_player: "Victor Vale",
        poty_rating: 7.8,
        total_teams: 2,
        season_awards: createAwards(),
      },
    });

    const onGameUpdate = vi.fn();

    render(
      <EndOfSeasonScreen gameState={createGameState()} onGameUpdate={onGameUpdate} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Start next season" }));

    expect(await screen.findByText("Awards Ceremony")).toBeInTheDocument();
    expect(screen.getAllByText("Manager of the Season").length).toBeGreaterThan(0);
    expect(screen.getByText("Jane Doe")).toBeInTheDocument();
    expect(onGameUpdate).toHaveBeenCalled();
  });
});
