import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { GameStateData } from "../../store/gameStore";
import TransferCentreWorldTab from "./TransferCentreWorldTab";

vi.mock("react-i18next", () => {
  return {
    useTranslation: () => ({
      t: (key: string, params?: Record<string, string | number>) => {
        if (key === "transferCentreWorld.title") return "Transfer Centre";
        if (key === "transferCentreWorld.subtitle") {
          return "Track market rumours and completed deals around the league";
        }
        if (key === "transferCentreWorld.rumours") return "Rumours";
        if (key === "transferCentreWorld.completedDeals") return "Completed deals";
        if (key === "transferCentreWorld.currentClub") return "Current club";
        if (key === "transferCentreWorld.sourceClub") return "From";
        if (key === "transferCentreWorld.destinationClub") return "To";
        if (key === "transferCentreWorld.reportedOn") return "Reported";
        if (key === "transferCentreWorld.agreedOn") return "Agreed";
        if (key === "transferCentreWorld.fee") return "Fee";
        if (key === "finances.marketValue") return "Market Value";
        if (key === "transferCentreWorld.noRumours") return "No active rumours.";
        if (key === "transferCentreWorld.noCompletedDeals") {
          return "No completed deals recorded.";
        }
        if (key === "transferCentreWorld.rumourTag") return "Rumour";
        if (key === "transferCentreWorld.completedTag") return "Completed";
        if (key === "transferCentreWorld.playerButton") return `View ${params?.player}`;
        return key;
      },
      i18n: { language: "en" },
    }),
  };
});

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
        name: "Beta United",
        short_name: "BET",
        country: "ENG",
        city: "Manchester",
        stadium_name: "Beta Park",
        stadium_capacity: 28000,
        finance: 900000,
        manager_id: null,
        reputation: 620,
        wage_budget: 450000,
        transfer_budget: 700000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-3-3",
        play_style: "Balanced",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1910,
        colors: { primary: "#0044cc", secondary: "#ffffff" },
        starting_xi_ids: [],
        form: [],
        history: [],
      },
      {
        id: "team-3",
        name: "Gamma FC",
        short_name: "GAM",
        country: "ENG",
        city: "Liverpool",
        stadium_name: "Gamma Dome",
        stadium_capacity: 27000,
        finance: 950000,
        manager_id: null,
        reputation: 630,
        wage_budget: 430000,
        transfer_budget: 720000,
        season_income: 0,
        season_expenses: 0,
        formation: "4-3-3",
        play_style: "Balanced",
        training_focus: "General",
        training_intensity: "Balanced",
        training_schedule: "Balanced",
        founded_year: 1920,
        colors: { primary: "#cc0000", secondary: "#ffffff" },
        starting_xi_ids: [],
        form: [],
        history: [],
      },
    ],
    players: [
      {
        id: "player-1",
        match_name: "Alex Star",
        full_name: "Alex Star",
        date_of_birth: "1999-02-14",
        nationality: "ENG",
        football_nation: "ENG",
        position: "Forward",
        natural_position: "Forward",
        alternate_positions: [],
        overall: 74,
        potential: 80,
        market_value: 1500000,
        wage: 250000,
        team_id: "team-1",
        morale: 65,
        condition: 92,
        sharpness: 88,
        is_injured: false,
        suspended_matches: 0,
        injury: null,
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
        attributes: {
          pace: 70,
          shooting: 72,
          passing: 68,
          dribbling: 71,
          defending: 35,
          physical: 69,
          goalkeeping: 10,
        },
        career_history: [],
      },
      {
        id: "player-2",
        match_name: "Marco Flux",
        full_name: "Marco Flux",
        date_of_birth: "1998-04-01",
        nationality: "ENG",
        football_nation: "ENG",
        position: "Forward",
        natural_position: "Forward",
        alternate_positions: [],
        overall: 77,
        potential: 81,
        market_value: 2200000,
        wage: 310000,
        team_id: "team-3",
        morale: 70,
        condition: 90,
        sharpness: 86,
        is_injured: false,
        suspended_matches: 0,
        injury: null,
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
        attributes: {
          pace: 73,
          shooting: 75,
          passing: 70,
          dribbling: 74,
          defending: 33,
          physical: 71,
          goalkeeping: 10,
        },
        career_history: [],
      },
    ],
    staff: [],
    messages: [],
    news: [],
    league: {
      id: "league-1",
      name: "Premier Division",
      season: 2026,
      fixtures: [],
      standings: [],
      transfer_log: [
        {
          date: "2026-08-09T00:00:00Z",
          from_team_id: "team-2",
          to_team_id: "team-3",
          player_id: "player-2",
          fee: 2100000,
        },
      ],
      transfer_rumours: [
        {
          id: "rumour-player-1",
          date: "2026-08-10T00:00:00Z",
          player_id: "player-1",
          player_name: "Alex Star",
          team_id: "team-1",
          team_name: "Alpha FC",
        },
      ],
    },
    scouting_assignments: [],
    board_objectives: [],
  } as unknown as GameStateData;
}

describe("TransferCentreWorldTab", () => {
  it("renders structured rumours and completed transfer deals", () => {
    render(<TransferCentreWorldTab gameState={createGameState()} />);

    expect(screen.getByText("Transfer Centre")).toBeInTheDocument();
    expect(screen.getByText("Alex Star")).toBeInTheDocument();
    expect(screen.getByText("Marco Flux")).toBeInTheDocument();
    expect(screen.getByText("Alpha FC")).toBeInTheDocument();
    expect(screen.getByText("Gamma FC")).toBeInTheDocument();
    expect(screen.getByText("Rumours")).toBeInTheDocument();
    expect(screen.getByText("Completed deals")).toBeInTheDocument();
    expect(screen.getByText("Market Value")).toBeInTheDocument();
  });

  it("routes player and team selection from rumours and completed deals", () => {
    const onSelectPlayer = vi.fn();
    const onSelectTeam = vi.fn();

    render(
      <TransferCentreWorldTab
        gameState={createGameState()}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "View Alex Star" }));
    fireEvent.click(screen.getByRole("button", { name: "Alpha FC" }));
    fireEvent.click(screen.getByRole("button", { name: "Gamma FC" }));

    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");
    expect(onSelectTeam).toHaveBeenNthCalledWith(1, "team-1");
    expect(onSelectTeam).toHaveBeenNthCalledWith(2, "team-3");
  });
});
