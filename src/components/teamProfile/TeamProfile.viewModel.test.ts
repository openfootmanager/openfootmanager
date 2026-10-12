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
      createPlayer({
        id: "goalkeeper",
        full_name: "Goal Keeper",
        position: "Goalkeeper",
        natural_position: "Goalkeeper",
      }),
      createPlayer({
        id: "forward",
        full_name: "Front Man",
        position: "Forward",
        natural_position: "Forward",
      }),
      createPlayer({
        id: "midfielder",
        full_name: "Mid Field",
        position: "Midfielder",
        natural_position: "Midfielder",
      }),
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

  it("sorts a roster whose positions are granular, not legacy buckets", () => {
    // The sort above passes only because its fixture uses the four legacy
    // bucket names. `domain::player::Position` has thirteen granular variants
    // as well — and `is_legacy_bucket()` exists precisely because the buckets
    // are the old shape. A generated squad is granular, so this is the case
    // the team profile actually renders.
    const gameState = createGameState();
    gameState.players = [
      createPlayer({
        id: "striker",
        full_name: "S. Triker",
        position: "Striker",
        natural_position: "Striker",
      }),
      createPlayer({
        id: "keeper",
        full_name: "K. Eeper",
        position: "Goalkeeper",
        natural_position: "Goalkeeper",
      }),
      createPlayer({
        id: "centre-back",
        full_name: "C. Back",
        position: "CenterBack",
        natural_position: "CenterBack",
      }),
    ];

    const viewModel = buildTeamProfileViewModel(createTeam(), gameState);

    expect(viewModel.roster.map((player) => player.id)).toEqual([
      "keeper",
      "centre-back",
      "striker",
    ]);
  });

  it("counts loan wage commitments instead of full current-roster wages", () => {
    const team = createTeam({ id: "team-1" });
    const gameState = createGameState({
      players: [
        createPlayer({
          id: "owned-player",
          team_id: "team-1",
          wage: 10000,
        }),
        createPlayer({
          id: "loaned-out",
          team_id: "team-2",
          wage: 20000,
          active_loan: {
            parent_team_id: "team-1",
            loan_team_id: "team-2",
            start_date: "2026-08-01",
            end_date: "2027-01-28",
            wage_contribution_pct: 60,
            buy_option_fee: null,
          },
        }),
        createPlayer({
          id: "loaned-in",
          team_id: "team-1",
          wage: 30000,
          active_loan: {
            parent_team_id: "team-2",
            loan_team_id: "team-1",
            start_date: "2026-08-01",
            end_date: "2027-01-28",
            wage_contribution_pct: 50,
            buy_option_fee: null,
          },
        }),
      ],
    });

    const viewModel = buildTeamProfileViewModel(team, gameState);

    expect(viewModel.roster.map((player) => player.id)).toEqual(["owned-player", "loaned-in"]);
    expect(viewModel.totalWages).toBe(33000);
  });

  function table(
    id: string,
    teamIds: string[],
    format: "LeagueTable" | "Knockout" = "LeagueTable",
  ) {
    return {
      id,
      name: id,
      season: 1,
      rules: { format, counts_in_season_flow: format === "LeagueTable" },
      participant_ids: teamIds,
      fixtures: [],
      standings:
        format === "LeagueTable"
          ? teamIds.map((teamId, index) => ({
              team_id: teamId,
              played: 1,
              won: 0,
              drawn: 0,
              lost: 0,
              goals_for: 0,
              goals_against: 0,
              points: teamIds.length - index,
            }))
          : [],
    };
  }

  /**
   * Given the manager's club in one division and the viewed club in another
   * When the viewed club's profile is built
   * Then its position and standing come from the division that contains it
   */
  it("finds the league position of a club outside the manager's division", () => {
    const gameState = createGameState({
      league: table("division-1", ["team-1", "team-2"]),
      competitions: [
        table("division-1", ["team-1", "team-2"]),
        table("division-2", ["team-8", "team-9", "team-7"]),
      ],
    });

    const viewModel = buildTeamProfileViewModel(createTeam({ id: "team-9" }), gameState);

    expect(viewModel.leaguePos).toBe(2);
    expect(viewModel.standings?.team_id).toBe("team-9");
  });

  /**
   * Given a club that is in a knockout cup and in no league table
   * When its profile is built
   * Then it has no league position and no standing, not a cup's empty table
   */
  it("gives a club that is only in a cup no league position", () => {
    const gameState = createGameState({
      competitions: [
        table("division-1", ["team-1", "team-2"]),
        table("cup", ["team-5"], "Knockout"),
      ],
    });

    const viewModel = buildTeamProfileViewModel(createTeam({ id: "team-5" }), gameState);

    expect(viewModel.leaguePos).toBe(0);
    expect(viewModel.standings).toBeNull();
  });

  /**
   * Given a club that appears in no competition
   * When its profile is built
   * Then it has no league position and no standing
   */
  it("gives a club in no competition no league position", () => {
    const gameState = createGameState({
      competitions: [table("division-1", ["team-1", "team-2"])],
    });

    const viewModel = buildTeamProfileViewModel(createTeam({ id: "team-404" }), gameState);

    expect(viewModel.leaguePos).toBe(0);
    expect(viewModel.standings).toBeNull();
  });

  /**
   * Given a club in both a league and a cup
   * When its profile is built
   * Then the league table is used
   */
  it("uses the league table of a club that is also in a cup", () => {
    const gameState = createGameState({
      competitions: [
        table("cup", ["team-1", "team-2"], "Knockout"),
        table("division-1", ["team-2", "team-1"]),
      ],
    });

    const viewModel = buildTeamProfileViewModel(createTeam({ id: "team-1" }), gameState);

    expect(viewModel.leaguePos).toBe(2);
  });
});
