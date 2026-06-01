import { describe, expect, it } from "vitest";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import { buildTeamProfileViewModel } from "./TeamProfile.viewModel";

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "BR",
    city: "Rio",
    stadium_name: "Alpha Arena",
    stadium_capacity: 50000,
    finance: 500000,
    manager_id: "manager-1",
    reputation: 60,
    wage_budget: 100000,
    transfer_budget: 250000,
    season_income: 150000,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "Physical",
    training_intensity: "Medium",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#111111", secondary: "#ffffff" },
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
    nationality: "BR",
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
    condition: 85,
    morale: 80,
    injury: null,
    team_id: "team-1",
    retired: false,
    contract_end: null,
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
    ovr: 60,
    ...overrides,
  };
}

function createGameState(overrides: Partial<GameStateData> = {}): GameStateData {
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
      nationality: "BR",
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
    teams: [createTeam()],
    players: [
      createPlayer({ id: "goalkeeper", full_name: "Goal Keeper", position: "Goalkeeper", natural_position: "Goalkeeper" }),
      createPlayer({ id: "forward", full_name: "Front Man", position: "Forward", natural_position: "Forward" }),
      createPlayer({ id: "midfielder", full_name: "Mid Field", position: "Midfielder", natural_position: "Midfielder" }),
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
          drawn: 0,
          lost: 1,
          goals_for: 0,
          goals_against: 2,
          points: 0,
        },
      ],
    },
    scouting_assignments: [],
    board_objectives: [],
    ...overrides,
  };
}

describe("TeamProfile.viewModel", () => {
  it("builds a sorted roster and derived team summary", () => {
    const gameState = createGameState();

    const viewModel = buildTeamProfileViewModel(createTeam(), gameState);

    expect(viewModel.roster.map((player) => player.id)).toEqual([
      "goalkeeper",
      "midfielder",
      "forward",
    ]);
    expect(viewModel.avgOvr).toBeGreaterThan(0);
    expect(viewModel.totalWages).toBe(36000);
    expect(viewModel.totalValue).toBe(1050000);
    expect(viewModel.leaguePos).toBe(2);
    expect(viewModel.manager?.id).toBe("manager-1");
    expect(viewModel.standings?.points).toBe(0);
  });
});
