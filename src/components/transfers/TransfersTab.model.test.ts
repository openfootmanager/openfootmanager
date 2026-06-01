import { describe, expect, it } from "vitest";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import {
  deriveTransferCollections,
  filterTransferPlayers,
  getCurrentTransferList,
  type TransferTabView,
} from "./TransfersTab.model";

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "User FC",
    short_name: "USR",
    country: "England",
    city: "London",
    stadium_name: "User Ground",
    stadium_capacity: 25000,
    finance: 5000000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 2000000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "Physical",
    training_intensity: "Medium",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: {
      primary: "#111111",
      secondary: "#ffffff",
    },
    facilities: {
      training: 1,
      medical: 1,
      scouting: 1,
    },
    starting_xi_ids: [],
    match_roles: {
      captain: null,
      vice_captain: null,
      penalty_taker: null,
      free_kick_taker: null,
      corner_taker: null,
    },
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
    nationality: "England",
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
      handling: 30,
      reflexes: 30,
      aerial: 60,
    },
    condition: 90,
    morale: 70,
    injury: null,
    team_id: "team-1",
    retired: false,
    contract_end: "2028-06-30",
    wage: 1000,
    market_value: 1000000,
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

function createGameState(players: PlayerData[]): GameStateData {
  return {
    clock: {
      current_date: "2026-08-01T12:00:00Z",
      start_date: "2026-07-01T12:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "England",
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
      createTeam({ id: "team-2", manager_id: null, name: "Buyer FC" }),
    ],
    players,
    staff: [],
    messages: [],
    news: [],
    league: {
      id: "league-1",
      name: "Premier Division",
      season: 1,
      fixtures: [],
      standings: [],
    },
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("TransfersTab.model", () => {
  it("derives the transfer collections for the current user team", () => {
    const userListed = createPlayer({
      id: "user-listed",
      transfer_listed: true,
    });
    const userLoanListed = createPlayer({
      id: "user-loan",
      loan_listed: true,
    });
    const marketPlayer = createPlayer({
      id: "market-player",
      team_id: "team-2",
      transfer_listed: true,
    });
    const freeAgent = createPlayer({
      id: "free-agent",
      team_id: null,
      contract_end: null,
    });
    const retiredFreeAgent = createPlayer({
      id: "retired-free-agent",
      team_id: null,
      contract_end: null,
      retired: true,
    });
    const loanPlayer = createPlayer({
      id: "loan-player",
      team_id: "team-2",
      loan_listed: true,
    });
    const offeredPlayer = createPlayer({
      id: "offered-player",
      transfer_offers: [
        {
          id: "offer-1",
          from_team_id: "team-1",
          fee: 1500000,
          wage_offered: 0,
          last_manager_fee: null,
          negotiation_round: 1,
          suggested_counter_fee: null,
          status: "Pending",
          date: "2026-08-01",
        },
      ],
    });
    const gameState = createGameState([
      userListed,
      userLoanListed,
      marketPlayer,
      freeAgent,
      retiredFreeAgent,
      loanPlayer,
      offeredPlayer,
    ]);

    const collections = deriveTransferCollections(gameState, "team-1");

    expect(collections.myTransferList.map((player) => player.id)).toEqual([
      "user-listed",
    ]);
    expect(collections.myLoanList.map((player) => player.id)).toEqual([
      "user-loan",
    ]);
    expect(collections.marketPlayers.map((player) => player.id)).toEqual([
      "market-player",
    ]);
    expect(collections.freeAgentPlayers.map((player) => player.id)).toEqual([
      "free-agent",
    ]);
    expect(collections.loanPlayers.map((player) => player.id)).toEqual([
      "loan-player",
    ]);
    expect(collections.playersWithOffers.map((player) => player.id)).toEqual([
      "offered-player",
    ]);
  });

  it("returns the current list for the selected view", () => {
    const collections = {
      myTransferList: [createPlayer({ id: "transfer" })],
      myLoanList: [createPlayer({ id: "loan" })],
      marketPlayers: [createPlayer({ id: "market" })],
      freeAgentPlayers: [createPlayer({ id: "free-agent", team_id: null })],
      loanPlayers: [createPlayer({ id: "loan-market" })],
      playersWithOffers: [createPlayer({ id: "offers" })],
    };

    const getIds = (view: TransferTabView) =>
      getCurrentTransferList(view, collections).map((player) => player.id);

    expect(getIds("my_list")).toEqual(["transfer", "loan"]);
    expect(getIds("market")).toEqual(["market"]);
    expect(getIds("free_agents")).toEqual(["free-agent"]);
    expect(getIds("loans")).toEqual(["loan-market"]);
    expect(getIds("offers")).toEqual(["offers"]);
  });

  it("filters by position and search text", () => {
    const players = [
      createPlayer({
        id: "goalkeeper",
        full_name: "Alan Keeper",
        nationality: "Spain",
        natural_position: "Goalkeeper",
        position: "Goalkeeper",
      }),
      createPlayer({
        id: "forward",
        full_name: "Carlos Striker",
        nationality: "Brazil",
        natural_position: "Forward",
        position: "Forward",
      }),
    ];

    expect(filterTransferPlayers(players, "", "Goalkeeper")).toHaveLength(1);
    expect(filterTransferPlayers(players, "bra", null).map((player) => player.id)).toEqual([
      "forward",
    ]);
    expect(filterTransferPlayers(players, "ca", "Forward").map((player) => player.id)).toEqual([
      "forward",
    ]);
  });
});
