import { describe, expect, it } from "vitest";

import type { GameStateData } from "../../store/gameStore";
import type { AdvanceMatchResultData } from "../../services/advanceTimeService";
import { buildAdvanceRecap, toDatePart } from "./advanceRecap";

function createGame(overrides: Partial<GameStateData> = {}): GameStateData {
  return {
    clock: {
      current_date: "2026-07-02T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: { team_id: "team-1" },
    teams: [
      { id: "team-1", name: "User FC" },
      { id: "team-2", name: "Rival FC" },
      { id: "team-3", name: "Other FC" },
    ],
    players: [{ id: "player-1", full_name: "John Star" }],
    news: [],
    messages: [],
    league: null,
    // The helper only reads the fields above; cast the partial fixture.
    ...overrides,
  } as unknown as GameStateData;
}

const matchOnDay: AdvanceMatchResultData = {
  date: "2026-07-01",
  competition: "League",
  international: false,
  home_team: "User FC",
  away_team: "Rival FC",
  home_goals: 2,
  away_goals: 1,
  involves_user: true,
};

describe("advanceRecap", function (): void {
  it("derives the advanced-to date from the new clock", function (): void {
    expect(toDatePart("2026-07-02T00:00:00Z")).toBe("2026-07-02");
    const recap = buildAdvanceRecap(createGame(), "2026-07-01", []);
    expect(recap.advancedTo).toBe("2026-07-02");
  });

  it("reports no events on a quiet advance", function (): void {
    const recap = buildAdvanceRecap(createGame(), "2026-07-01", []);
    expect(recap.hasEvents).toBe(false);
    expect(recap.matches).toEqual([]);
    expect(recap.transfers).toEqual([]);
    expect(recap.news).toEqual([]);
    expect(recap.inbox).toEqual([]);
  });

  it("collects transfers from the advance, resolving names and user involvement", function (): void {
    const game = createGame({
      league: {
        id: "league-1",
        name: "League",
        season: 1,
        fixtures: [],
        standings: [],
        transfer_log: [
          // Before the advance — excluded.
          {
            date: "2026-06-30",
            from_team_id: "team-2",
            to_team_id: "team-3",
            player_id: "player-1",
            fee: 100,
          },
          // During the advance, involving the user's club.
          {
            date: "2026-07-01",
            from_team_id: "team-1",
            to_team_id: "team-2",
            player_id: "player-1",
            fee: 2_000_000,
          },
        ],
      },
    } as unknown as Partial<GameStateData>);

    const recap = buildAdvanceRecap(game, "2026-07-01", []);

    expect(recap.transfers).toHaveLength(1);
    expect(recap.transfers[0]).toMatchObject({
      player: "John Star",
      from: "User FC",
      to: "Rival FC",
      fee: 2_000_000,
      involvesUser: true,
    });
    expect(recap.hasEvents).toBe(true);
  });

  it("keeps key news but drops routine and transfer-duplicate categories", function (): void {
    const game = createGame({
      news: [
        {
          id: "injury",
          headline: "Star out injured",
          body: "",
          date: "2026-07-01",
          category: "InjuryNews",
          team_ids: [],
          player_ids: [],
          read: false,
          headline_key: "news.injury",
          i18n_params: { player: "John Star" },
        },
        {
          id: "roundup",
          headline: "League roundup",
          body: "",
          date: "2026-07-01",
          category: "LeagueRoundup",
          team_ids: [],
          player_ids: [],
          read: false,
        },
        {
          id: "old-editorial",
          headline: "Old take",
          body: "",
          date: "2026-06-20",
          category: "Editorial",
          team_ids: [],
          player_ids: [],
          read: false,
        },
      ],
    } as unknown as Partial<GameStateData>);

    const recap = buildAdvanceRecap(game, "2026-07-01", []);

    expect(recap.news.map((article) => article.id)).toEqual(["injury"]);
    expect(recap.news[0]).toMatchObject({
      textKey: "news.injury",
      params: { player: "John Star" },
    });
  });

  it("keeps completed loan move news in the advance recap", function (): void {
    const game = createGame({
      news: [
        {
          id: "loan-news-player-1-team-2-team-1-2026-07-02",
          headline: "",
          body: "",
          date: "2026-07-02",
          category: "TransferRumour",
          team_ids: ["team-1", "team-2"],
          player_ids: ["player-1"],
          read: false,
          headline_key: "be.news.loanMove.headline",
          body_key: "be.news.loanMove.body",
          i18n_params: {
            player: "John Star",
            fromTeam: "Rival FC",
            toTeam: "User FC",
            endDate: "2027-01-01",
          },
        },
        {
          id: "ordinary-rumour",
          headline: "Rumour mill",
          body: "",
          date: "2026-07-02",
          category: "TransferRumour",
          team_ids: ["team-2"],
          player_ids: ["player-1"],
          read: false,
        },
      ],
    } as unknown as Partial<GameStateData>);

    const recap = buildAdvanceRecap(game, "2026-07-01", []);

    expect(recap.news.map((article) => article.id)).toEqual([
      "loan-news-player-1-team-2-team-1-2026-07-02",
    ]);
    expect(recap.news[0]).toMatchObject({
      textKey: "be.news.loanMove.headline",
      params: {
        player: "John Star",
        fromTeam: "Rival FC",
        toTeam: "User FC",
        endDate: "2027-01-01",
      },
    });
    expect(recap.hasEvents).toBe(true);
  });

  it("collects only high-priority inbox items from the advance", function (): void {
    const game = createGame({
      messages: [
        {
          id: "digest",
          subject: "Transfer interest",
          body: "",
          sender: "",
          sender_role: "",
          date: "2026-07-01",
          read: false,
          category: "Transfer",
          priority: "High",
          actions: [],
          subject_key: "be.msg.transferInterest.subject",
          i18n_params: { player: "John Star" },
        },
        {
          id: "low",
          subject: "Routine note",
          body: "",
          sender: "",
          sender_role: "",
          date: "2026-07-01",
          read: false,
          category: "System",
          priority: "Normal",
          actions: [],
        },
      ],
    } as unknown as Partial<GameStateData>);

    const recap = buildAdvanceRecap(game, "2026-07-01", []);

    expect(recap.inbox.map((item) => item.id)).toEqual(["digest"]);
    expect(recap.inbox[0].textKey).toBe("be.msg.transferInterest.subject");
  });

  it("reads transfer_log from competitions, not the deprecated league mirror", function (): void {
    // When competitions are present they take precedence over game.league.
    const game = createGame({
      competitions: [
        {
          id: "comp-1",
          name: "Premier League",
          season: 1,
          participant_ids: ["team-1", "team-2"],
          fixtures: [],
          standings: [],
          transfer_log: [
            {
              date: "2026-07-01",
              from_team_id: "team-1",
              to_team_id: "team-2",
              player_id: "player-1",
              fee: 5_000_000,
            },
          ],
        },
      ],
      // Deliberately absent from the deprecated mirror.
      league: null,
    } as unknown as Partial<GameStateData>);

    const recap = buildAdvanceRecap(game, "2026-07-01", []);
    expect(recap.transfers).toHaveLength(1);
    expect(recap.transfers[0].fee).toBe(5_000_000);
  });

  it("flags match results as events", function (): void {
    const recap = buildAdvanceRecap(createGame(), "2026-07-01", [matchOnDay]);
    expect(recap.matches).toHaveLength(1);
    expect(recap.hasEvents).toBe(true);
  });
});
