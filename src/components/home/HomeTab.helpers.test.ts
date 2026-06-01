import { describe, expect, it, beforeEach } from "vitest";

import type {
  FixtureData,
  GameStateData,
  MessageData,
  NewsArticle,
  PlayerData,
  TeamData,
} from "../../store/gameStore";
import {
  getHomeRosterOverview,
  getLeagueDigestArticles,
  getNextOpponentWidgetData,
  getOnboardingCompletionState,
  getRecentResultsForTeam,
  loadVisitedOnboardingTabs,
  saveVisitedOnboardingTabs,
} from "./HomeTab.helpers";

beforeEach(function (): void {
  localStorage.clear();
});

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
    training_focus: "Physical",
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

function createFixture(overrides: Partial<FixtureData> = {}): FixtureData {
  return {
    id: "fixture-1",
    matchday: 1,
    date: "2025-01-10",
    home_team_id: "team-1",
    away_team_id: "team-2",
    competition: "League",
    status: "Scheduled",
    result: null,
    ...overrides,
  };
}

function createNewsArticle(overrides: Partial<NewsArticle> = {}): NewsArticle {
  return {
    id: "news-1",
    headline: "Headline",
    body: "Body",
    source: "OpenFoot Times",
    date: "2025-01-10",
    category: "LeagueRoundup",
    team_ids: [],
    player_ids: [],
    match_score: null,
    read: false,
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
      current_date: "2025-01-03T00:00:00Z",
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
    players: [createPlayer()],
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

describe("HomeTab.helpers", function (): void {
  it("derives the next opponent widget data from the next scheduled fixture", function (): void {
    const gameState = createGameState({
      teams: [
        createTeam(),
        createTeam({
          id: "team-2",
          name: "Beta FC",
          short_name: "BET",
          form: ["W", "D", "W"],
        }),
      ],
      league: {
        id: "league-1",
        name: "League",
        season: 1,
        fixtures: [
          createFixture({
            id: "fixture-completed",
            date: "2025-01-05",
            matchday: 1,
            status: "Completed",
            result: {
              home_goals: 1,
              away_goals: 0,
              home_scorers: [],
              away_scorers: [],
            },
          }),
          createFixture({
            id: "fixture-next",
            date: "2025-01-12",
            matchday: 2,
            status: "Scheduled",
          }),
        ],
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
    });

    const result = getNextOpponentWidgetData(gameState);

    expect(result).not.toBeNull();
    expect(result?.fixture.id).toBe("fixture-next");
    expect(result?.opponent.name).toBe("Beta FC");
    expect(result?.isHome).toBe(true);
    expect(result?.standingPosition).toBe(1);
    expect(result?.standingPoints).toBe(3);
    expect(result?.recentForm).toEqual(["W", "D", "W"]);
  });

  it("returns the latest league digest articles in reverse chronological order", function (): void {
    const gameState = createGameState({
      news: [
        createNewsArticle({
          id: "generic-news",
          category: "TransferRumour",
          date: "2025-01-12",
        }),
        createNewsArticle({
          id: "roundup-news",
          category: "LeagueRoundup",
          date: "2025-01-14",
        }),
        createNewsArticle({
          id: "standings-news",
          category: "StandingsUpdate",
          date: "2025-01-15",
        }),
      ],
    });

    const result = getLeagueDigestArticles(gameState);

    expect(result.map((article) => article.id)).toEqual([
      "standings-news",
      "roundup-news",
    ]);
  });

  it("builds roster overview metrics, unavailable players, and momentum groups", function (): void {
    const roster = [
      createPlayer({
        id: "player-hot",
        full_name: "Hot Player",
        morale: 90,
        condition: 88,
        ovr: 72,
      }),
      createPlayer({
        id: "player-cold",
        full_name: "Cold Player",
        morale: 35,
        condition: 32,
        ovr: 58,
      }),
      createPlayer({
        id: "player-injured",
        full_name: "Injured Player",
        morale: 75,
        condition: 64,
        ovr: 64,
        injury: {
          name: "Hamstring",
          days_remaining: 14,
        },
      }),
    ];

    const result = getHomeRosterOverview(roster);

    expect(result.avgCondition).toBe(61);
    expect(result.avgOvr).toBe(65);
    expect(result.exhaustedCount).toBe(1);
    expect(result.unavailablePlayers.map((player) => player.id)).toEqual([
      "player-injured",
    ]);
    expect(result.hotPlayers.map((player) => player.id)).toEqual(["player-hot"]);
    expect(result.coldPlayers.map((player) => player.id)).toEqual([
      "player-cold",
    ]);
  });

  it("returns the latest completed results for the managed team", function (): void {
    const gameState = createGameState({
      teams: [
        createTeam(),
        createTeam({ id: "team-2", name: "Beta FC" }),
        createTeam({ id: "team-3", name: "Gamma FC" }),
      ],
      league: {
        id: "league-1",
        name: "League",
        season: 1,
        standings: [],
        fixtures: [
          createFixture({
            id: "fixture-1",
            date: "2025-01-05",
            status: "Completed",
            result: {
              home_goals: 2,
              away_goals: 1,
              home_scorers: [],
              away_scorers: [],
            },
          }),
          createFixture({
            id: "fixture-2",
            date: "2025-01-08",
            home_team_id: "team-3",
            away_team_id: "team-1",
            status: "Completed",
            result: {
              home_goals: 0,
              away_goals: 0,
              home_scorers: [],
              away_scorers: [],
            },
          }),
          createFixture({
            id: "fixture-3",
            date: "2025-01-10",
            status: "Scheduled",
          }),
        ],
      },
    });

    const result = getRecentResultsForTeam(gameState, "team-1");

    expect(result.map((entry) => entry.fixture.id)).toEqual([
      "fixture-1",
      "fixture-2",
    ]);
    expect(result[0]).toMatchObject({
      isHome: true,
      myGoals: 2,
      opponentGoals: 1,
      opponentId: "team-2",
      resultCode: "W",
    });
    expect(result[1]).toMatchObject({
      isHome: false,
      myGoals: 0,
      opponentGoals: 0,
      opponentId: "team-3",
      resultCode: "D",
    });
  });

  it("starts with no visited onboarding pages and no read inbox step", function (): void {
    const state = getOnboardingCompletionState(createGameState(), new Set<string>());

    expect(state.hasVisitedSquadPage).toBe(false);
    expect(state.hasVisitedStaffPage).toBe(false);
    expect(state.hasVisitedTacticsPage).toBe(false);
    expect(state.hasVisitedTrainingPage).toBe(false);
    expect(state.hasReadInbox).toBe(false);
    expect(state.completedSteps).toBe(0);
  });

  it("marks visited onboarding pages as done", function (): void {
    const state = getOnboardingCompletionState(
      createGameState(),
      new Set<string>(["Squad", "Tactics"]),
    );

    expect(state.hasVisitedSquadPage).toBe(true);
    expect(state.hasVisitedTacticsPage).toBe(true);
    expect(state.hasVisitedStaffPage).toBe(false);
    expect(state.hasVisitedTrainingPage).toBe(false);
    expect(state.completedSteps).toBe(2);
  });

  it("marks inbox complete after at least one message is read", function (): void {
    const gameState = createGameState({
      messages: [
        createMessage({
          id: "message-1",
          read: true,
        }),
        createMessage({
          id: "message-2",
          category: "System",
          read: false,
        }),
      ],
    });

    const state = getOnboardingCompletionState(gameState, new Set<string>());

    expect(state.hasReadInbox).toBe(true);
  });

  it("counts page visits together with the inbox step", function (): void {
    const gameState = createGameState({
      messages: [
        createMessage({
          id: "message-1",
          read: true,
        }),
      ],
    });
    const state = getOnboardingCompletionState(
      gameState,
      new Set<string>(["Squad", "Staff", "Training"]),
    );

    expect(state.completedSteps).toBe(4);
  });

  it("hides onboarding after the first week", function (): void {
    const gameState = createGameState({
      clock: {
        current_date: "2025-01-10T00:00:00Z",
        start_date: "2025-01-01T00:00:00Z",
      },
    });

    const state = getOnboardingCompletionState(gameState, new Set<string>());

    expect(state.showOnboarding).toBe(false);
  });

  it("persists visited onboarding tabs per save", function (): void {
    const gameState = createGameState();
    const otherGameState = createGameState({
      clock: {
        current_date: "2025-02-03T00:00:00Z",
        start_date: "2025-02-01T00:00:00Z",
      },
      manager: {
        ...createGameState().manager,
        id: "manager-2",
      },
    });

    saveVisitedOnboardingTabs(
      gameState,
      new Set<string>(["Squad", "Training"]),
      localStorage,
    );

    expect(Array.from(loadVisitedOnboardingTabs(gameState, localStorage))).toEqual([
      "Squad",
      "Training",
    ]);
    expect(Array.from(loadVisitedOnboardingTabs(otherGameState, localStorage))).toEqual(
      [],
    );
  });

  it("keeps onboarding completed after reloading persisted progress", function (): void {
    const gameState = createGameState({
      messages: [
        createMessage({
          id: "message-1",
          read: true,
        }),
      ],
    });

    saveVisitedOnboardingTabs(
      gameState,
      new Set<string>(["Squad", "Staff", "Tactics", "Training"]),
      localStorage,
    );

    const reloadedVisitedTabs = loadVisitedOnboardingTabs(gameState, localStorage);
    const state = getOnboardingCompletionState(gameState, reloadedVisitedTabs);

    expect(state.hasVisitedSquadPage).toBe(true);
    expect(state.hasVisitedStaffPage).toBe(true);
    expect(state.hasVisitedTacticsPage).toBe(true);
    expect(state.hasVisitedTrainingPage).toBe(true);
    expect(state.hasReadInbox).toBe(true);
    expect(state.completedSteps).toBe(5);
  });
});
