import { describe, expect, it } from "vitest";

import type { GameStateData, MessageData, PlayerData, TeamData } from "../../store/gameStore";
import {
  getDashboardAlerts,
  getDashboardSearchResults,
  getManagerTeamName,
  getPlayerBadgeVariant,
  getTodayMatchFixture,
  getUnreadMessagesCount,
} from "./dashboardHelpers";

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "BR",
    city: "Rio",
    stadium_name: "Alpha Arena",
    stadium_capacity: 50000,
    finance: 0,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 0,
    transfer_budget: 0,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Medium",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: {
      primary: "#111111",
      secondary: "#ffffff",
    },
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
      pace: 10,
      stamina: 10,
      strength: 10,
      agility: 10,
      passing: 10,
      shooting: 10,
      tackling: 10,
      dribbling: 10,
      defending: 10,
      positioning: 10,
      vision: 10,
      decisions: 10,
      composure: 10,
      aggression: 10,
      teamwork: 10,
      leadership: 10,
      handling: 10,
      reflexes: 10,
      aerial: 10,
    },
    condition: 80,
    morale: 80,
    injury: null,
    team_id: "team-1",
    retired: false,
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
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

function createMessage(overrides: Partial<MessageData> = {}): MessageData {
  return {
    id: "message-1",
    subject: "Subject",
    body: "Body",
    sender: "Sender",
    sender_role: "Role",
    date: "2025-01-10",
    read: false,
    category: "System",
    priority: "Normal",
    actions: [],
    context: {
      team_id: null,
      player_id: null,
      fixture_id: null,
      match_result: null,
    },
    ...overrides,
  };
}

function createGameState(overrides: Partial<GameStateData> = {}): GameStateData {
  return {
    clock: {
      current_date: "2025-01-10T00:00:00Z",
      start_date: "2025-01-01T00:00:00Z",
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
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures: [],
      standings: [],
    },
    scouting_assignments: [],
    board_objectives: [],
    ...overrides,
  };
}

function translateDashboardAlert(
  key: string,
  options?: Record<string, unknown>,
): string {
  return options ? `${key}:${JSON.stringify(options)}` : key;
}

describe("dashboardHelpers", function (): void {
  it("finds today's scheduled fixture for the manager's team", function (): void {
    const fixture = {
      id: "fixture-1",
      matchday: 12,
      date: "2025-01-10",
      home_team_id: "team-1",
      away_team_id: "team-2",
      competition: "League" as const,
      status: "Scheduled" as const,
      result: null,
    };
    const gameState = createGameState({
      league: {
        id: "league-1",
        name: "League",
        season: 1,
        fixtures: [fixture],
        standings: [],
      },
    });

    expect(getTodayMatchFixture(gameState)).toEqual(fixture);
  });

  it("returns null when no fixture matches today", function (): void {
    const gameState = createGameState();

    expect(getTodayMatchFixture(gameState)).toBeNull();
  });

  it("returns manager team name and unread count", function (): void {
    const gameState = createGameState({
      messages: [
        createMessage({ id: "m1", read: false }),
        createMessage({ id: "m2", read: true }),
      ],
    });

    expect(getManagerTeamName(gameState)).toBe("Alpha FC");
    expect(getUnreadMessagesCount(gameState)).toBe(1);
  });

  it("filters dashboard search results for teams and players", function (): void {
    const gameState = createGameState({
      teams: [
        createTeam(),
        createTeam({ id: "team-2", name: "Bravo United", short_name: "BRV" }),
      ],
      players: [
        createPlayer(),
        createPlayer({
          id: "player-2",
          full_name: "Bruno Silva",
          match_name: "B. Silva",
        }),
      ],
    });

    expect(getDashboardSearchResults(gameState, "b")).toEqual({
      matchedPlayers: [],
      matchedTeams: [],
    });

    const results = getDashboardSearchResults(gameState, "br");

    expect(results.matchedPlayers).toHaveLength(1);
    expect(results.matchedPlayers[0].id).toBe("player-2");
    expect(results.matchedTeams).toHaveLength(1);
    expect(results.matchedTeams[0].id).toBe("team-2");
  });

  it("maps player positions to dashboard badge variants", function (): void {
    expect(getPlayerBadgeVariant("Goalkeeper")).toBe("accent");
    expect(getPlayerBadgeVariant("Defender")).toBe("primary");
    expect(getPlayerBadgeVariant("Midfielder")).toBe("success");
    expect(getPlayerBadgeVariant("Forward")).toBe("danger");
  });

  it("builds dashboard alerts for critical squad issues", function (): void {
    const roster = [
      createPlayer({ id: "p1", condition: 20, injury: { name: "Hamstring", days_remaining: 5 } }),
      createPlayer({ id: "p2", condition: 21, injury: { name: "Ankle", days_remaining: 3 } }),
      createPlayer({ id: "p3", condition: 22 }),
      createPlayer({ id: "p4", condition: 80 }),
      createPlayer({ id: "p5", condition: 80 }),
      createPlayer({ id: "p6", condition: 80 }),
      createPlayer({ id: "p7", condition: 80 }),
      createPlayer({ id: "p8", condition: 80 }),
      createPlayer({ id: "p9", condition: 80 }),
      createPlayer({ id: "p10", condition: 80 }),
      createPlayer({ id: "p11", condition: 80 }),
    ];
    const team = createTeam({
      starting_xi_ids: [
        "p1",
        "p2",
        "p3",
        "p4",
        "p5",
        "p6",
        "p7",
        "p8",
        "p9",
        "p10",
        "p11",
      ],
    });
    const gameState = createGameState({
      teams: [team],
      players: roster,
      messages: [createMessage({ id: "urgent-1", priority: "Urgent", read: false })],
    });

    const alerts = getDashboardAlerts(gameState, true, translateDashboardAlert);
    const alertIds = alerts.map((alert) => alert.id);

    expect(alertIds).toContain("exhausted");
    expect(alertIds).toContain("injured_xi");
    expect(alertIds).toContain("urgent");
    expect(alertIds).toContain("matchxi");
  });

  it("builds dashboard alerts for finance pressure", function (): void {
    const team = createTeam({
      finance: 25000,
      wage_budget: 500000,
    });
    const gameState = createGameState({
      teams: [team],
      players: [
        createPlayer({ id: "p1", wage: 300000 }),
        createPlayer({ id: "p2", wage: 300000 }),
      ],
    });

    const alerts = getDashboardAlerts(gameState, false, translateDashboardAlert);
    const alertIds = alerts.map((alert) => alert.id);

    expect(alertIds).toContain("finance_crisis");
    expect(alertIds).toContain("wage_pressure");
  });

  it("does not warn about an incomplete Starting XI when a healthy roster can normalize a partial saved lineup", function (): void {
    const roster = [
      createPlayer({ id: "p1", position: "Goalkeeper", natural_position: "Goalkeeper" }),
      createPlayer({ id: "p2", position: "Defender", natural_position: "Defender" }),
      createPlayer({ id: "p3", position: "Defender", natural_position: "Defender" }),
      createPlayer({ id: "p4", position: "Defender", natural_position: "Defender" }),
      createPlayer({ id: "p5", position: "Defender", natural_position: "Defender" }),
      createPlayer({ id: "p6", position: "Midfielder", natural_position: "Midfielder" }),
      createPlayer({ id: "p7", position: "Midfielder", natural_position: "Midfielder" }),
      createPlayer({ id: "p8", position: "Midfielder", natural_position: "Midfielder" }),
      createPlayer({ id: "p9", position: "Midfielder", natural_position: "Midfielder" }),
      createPlayer({ id: "p10", position: "Forward", natural_position: "Forward" }),
      createPlayer({ id: "p11", position: "Forward", natural_position: "Forward" }),
    ];
    const team = createTeam({
      starting_xi_ids: ["p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8"],
    });
    const gameState = createGameState({
      teams: [team],
      players: roster,
    });

    const alerts = getDashboardAlerts(gameState, true, translateDashboardAlert);
    const alertIds = alerts.map((alert) => alert.id);

    expect(alertIds).not.toContain("xi");
    expect(alertIds).not.toContain("matchxi");
    expect(alertIds).not.toContain("injured_xi");
  });
});
