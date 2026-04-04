import { describe, expect, it } from "vitest";

import type { PlayerData, ScoutingAssignment, TeamData } from "../../store/gameStore";
import {
  buildAlreadyScoutingIds,
  filterScoutablePlayers,
  paginateScoutablePlayers,
} from "./ScoutingTab.model";

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
    team_id: "team-2",
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

function createAssignment(
  overrides: Partial<ScoutingAssignment> = {},
): ScoutingAssignment {
  return {
    id: "assignment-1",
    scout_id: "staff-1",
    player_id: "player-1",
    days_remaining: 3,
    ...overrides,
  };
}

describe("ScoutingTab.model", () => {
  it("filters scoutable players by team, position, and search query then sorts by ovr", () => {
    const teams = [
      createTeam(),
      createTeam({ id: "team-2", name: "Beta FC", short_name: "BET", manager_id: "manager-2" }),
      createTeam({ id: "team-3", name: "Gamma FC", short_name: "GAM", manager_id: "manager-3" }),
    ];
    const players = [
      createPlayer({ id: "self", team_id: "team-1", full_name: "Own Player" }),
      createPlayer({ id: "defender", team_id: "team-2", full_name: "Alan Back", natural_position: "Defender", position: "Defender", attributes: { pace: 50, stamina: 60, strength: 65, agility: 55, passing: 58, shooting: 30, tackling: 72, dribbling: 40, defending: 74, positioning: 70, vision: 50, decisions: 63, composure: 60, aggression: 62, teamwork: 64, leadership: 55, handling: 20, reflexes: 20, aerial: 66 } }),
      createPlayer({ id: "forward", team_id: "team-3", full_name: "Carlos Goal", nationality: "BR", natural_position: "Forward", position: "Forward", attributes: { pace: 78, stamina: 70, strength: 65, agility: 75, passing: 63, shooting: 80, tackling: 30, dribbling: 79, defending: 28, positioning: 76, vision: 64, decisions: 68, composure: 72, aggression: 45, teamwork: 58, leadership: 49, handling: 20, reflexes: 20, aerial: 62 } }),
    ];

    expect(
      filterScoutablePlayers({
        players,
        teams,
        myTeamId: "team-1",
        posFilter: "All",
        searchQuery: "",
      }).map((player) => player.id),
    ).toEqual(["forward", "defender"]);

    expect(
      filterScoutablePlayers({
        players,
        teams,
        myTeamId: "team-1",
        posFilter: "Defender",
        searchQuery: "",
      }).map((player) => player.id),
    ).toEqual(["defender"]);

    expect(
      filterScoutablePlayers({
        players,
        teams,
        myTeamId: "team-1",
        posFilter: "All",
        searchQuery: "gamma",
      }).map((player) => player.id),
    ).toEqual(["forward"]);
  });

  it("paginates scoutable players safely", () => {
    const players = Array.from({ length: 25 }, (_, index) =>
      createPlayer({ id: `player-${index + 1}`, full_name: `Player ${index + 1}` }),
    );

    const firstPage = paginateScoutablePlayers(players, 0, 20);
    const lastPage = paginateScoutablePlayers(players, 5, 20);

    expect(firstPage.totalPages).toBe(2);
    expect(firstPage.safePage).toBe(0);
    expect(firstPage.players).toHaveLength(20);
    expect(lastPage.safePage).toBe(1);
    expect(lastPage.players).toHaveLength(5);
  });

  it("builds the set of already scouted player ids", () => {
    const ids = buildAlreadyScoutingIds([
      createAssignment({ player_id: "player-1" }),
      createAssignment({ id: "assignment-2", player_id: "player-2" }),
    ]);

    expect(ids.has("player-1")).toBe(true);
    expect(ids.has("player-2")).toBe(true);
    expect(ids.has("player-3")).toBe(false);
  });
});