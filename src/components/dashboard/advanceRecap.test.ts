import { describe, expect, it } from "vitest";

import type { GameStateData } from "../../store/gameStore";
import type { AdvanceMatchResultData } from "../../services/advanceTimeService";
import {
  buildAdvanceRecap,
  buildDigestEntries,
  detectAttentionEvents,
  nextDay,
  toDatePart,
} from "./advanceRecap";

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
          id: "loan-news-player-1-team-2-team-1-2026-07-01",
          headline: "",
          body: "",
          // Dated on the processed day: the backend stamps a day's events
          // before the clock moves on to 2026-07-02.
          date: "2026-07-01",
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
          date: "2026-07-01",
          category: "TransferRumour",
          team_ids: ["team-2"],
          player_ids: ["player-1"],
          read: false,
        },
      ],
    } as unknown as Partial<GameStateData>);

    const recap = buildAdvanceRecap(game, "2026-07-01", []);

    expect(recap.news.map((article) => article.id)).toEqual([
      "loan-news-player-1-team-2-team-1-2026-07-01",
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

  // Regression: the World Cup kickoff article is dated at the tournament's
  // kickoff, months after the rollover that creates it. Without an upper bound
  // on the recap window it matched `date >= sinceDate` on every advance and
  // repeated in every digest day until June.
  function kickoffArticle(date: string) {
    return {
      id: "world_cup_kickoff_2026",
      headline: "",
      body: "",
      date,
      category: "Editorial",
      team_ids: [],
      player_ids: [],
      read: false,
      headline_key: "be.news.worldCupKickoff.headline",
      i18n_params: { year: "2026", nations: "48" },
    };
  }

  it("keeps future-dated news out of days before the event", function (): void {
    // Advancing Jan 1 → Jan 2 with the kickoff article dated June 11. A
    // same-day article proves the window is the non-empty [Jan 1, Jan 2) and
    // only the future-dated item is excluded.
    const game = createGame({
      clock: { current_date: "2026-01-02T00:00:00Z", start_date: "2026-01-01T00:00:00Z" },
      news: [
        kickoffArticle("2026-06-11"),
        {
          id: "same-day-editorial",
          headline: "Local story",
          body: "",
          date: "2026-01-01",
          category: "Editorial",
          team_ids: [],
          player_ids: [],
          read: false,
        },
      ],
    } as unknown as Partial<GameStateData>);

    const recap = buildAdvanceRecap(game, "2026-01-01", []);
    expect(recap.news.map((article) => article.id)).toEqual(["same-day-editorial"]);
  });

  it("surfaces future-dated news exactly on the day it is dated", function (): void {
    // The digest builds each entry with sinceDate == the day just processed
    // and the clock already moved to the next day, so the window is
    // [sinceDate, clock): the article appears in the June 11 entry only.
    const entryFor = (processedDay: string, clockAfter: string) =>
      buildAdvanceRecap(
        createGame({
          clock: {
            current_date: `${clockAfter}T00:00:00Z`,
            start_date: "2026-01-01T00:00:00Z",
          },
          news: [kickoffArticle("2026-06-11")],
        } as unknown as Partial<GameStateData>),
        processedDay,
        [],
      );

    const dayBefore = entryFor("2026-06-10", "2026-06-11");
    expect(dayBefore.news).toEqual([]);

    const onTheDay = entryFor("2026-06-11", "2026-06-12");
    expect(onTheDay.news).toHaveLength(1);
    expect(onTheDay.news[0].textKey).toBe("be.news.worldCupKickoff.headline");

    const dayAfter = entryFor("2026-06-12", "2026-06-13");
    expect(dayAfter.news).toEqual([]);
  });

  it("keeps future-dated inbox messages and transfers out of earlier days", function (): void {
    const game = createGame({
      clock: { current_date: "2026-01-02T00:00:00Z", start_date: "2026-01-01T00:00:00Z" },
      messages: [
        {
          id: "future-msg",
          subject: "Scheduled briefing",
          body: "",
          date: "2026-06-11",
          read: false,
          priority: "High",
          category: "LeagueInfo",
        },
        {
          id: "same-day-msg",
          subject: "Board note",
          body: "",
          date: "2026-01-01",
          read: false,
          priority: "High",
          category: "LeagueInfo",
        },
      ],
      league: {
        id: "league-1",
        name: "League",
        season: 1,
        participant_ids: ["team-1", "team-2"],
        fixtures: [],
        standings: [],
        transfer_log: [
          {
            date: "2026-06-11",
            from_team_id: "team-2",
            to_team_id: "team-3",
            player_id: "player-1",
            fee: 1_000_000,
          },
          {
            date: "2026-01-01",
            from_team_id: "team-2",
            to_team_id: "team-3",
            player_id: "player-1",
            fee: 500_000,
          },
        ],
      },
    } as unknown as Partial<GameStateData>);

    const recap = buildAdvanceRecap(game, "2026-01-01", []);
    expect(recap.inbox.map((item) => item.id)).toEqual(["same-day-msg"]);
    expect(recap.transfers.map((transfer) => transfer.fee)).toEqual([500_000]);
  });

  it("accepts a full timestamp as sinceDate, not just a bare day", function (): void {
    // Defence-in-depth: current callers pass YYYY-MM-DD, but the lower bound
    // is day-normalized so an rfc3339 timestamp compares correctly too
    // ("2026-07-01T00:00:00Z" > "2026-07-01" would wrongly exclude same-day items).
    const game = createGame({
      news: [
        {
          id: "same-day-editorial",
          headline: "Local story",
          body: "",
          date: "2026-07-01",
          category: "Editorial",
          team_ids: [],
          player_ids: [],
          read: false,
        },
      ],
    } as unknown as Partial<GameStateData>);

    const recap = buildAdvanceRecap(game, "2026-07-01T00:00:00Z", []);
    expect(recap.news.map((article) => article.id)).toEqual(["same-day-editorial"]);
  });

  it("still includes same-window items when the clock is missing", function (): void {
    // A defensive path: with no clock the window has no upper bound, so
    // behavior falls back to the old `>= sinceDate` filter.
    const game = createGame({
      clock: null,
      news: [kickoffArticle("2026-06-11")],
    } as unknown as Partial<GameStateData>);

    const recap = buildAdvanceRecap(game, "2026-01-01", []);
    expect(recap.news).toHaveLength(1);
  });
});

describe("nextDay", function (): void {
  it("advances a day, crossing month and year boundaries", function (): void {
    expect(nextDay("2026-07-01")).toBe("2026-07-02");
    expect(nextDay("2026-06-30")).toBe("2026-07-01");
    expect(nextDay("2026-12-31")).toBe("2027-01-01");
  });
});

describe("buildDigestEntries", function (): void {
  it("splits a batch advance into per-day entries scoped to each day", function (): void {
    // Three processed days (clock landed on Jul 4). Each item must appear in
    // exactly the entry of the day it is dated on.
    const game = createGame({
      clock: { current_date: "2026-07-04T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
      news: [
        {
          id: "day-2-injury",
          headline: "Star out injured",
          body: "",
          date: "2026-07-02",
          category: "InjuryNews",
          team_ids: [],
          player_ids: [],
          read: false,
        },
      ],
    } as unknown as Partial<GameStateData>);
    const results: AdvanceMatchResultData[] = [
      matchOnDay,
      { ...matchOnDay, date: "2026-07-03", home_team: "Other FC", involves_user: false },
    ];

    const entries = buildDigestEntries(game, "2026-07-01", results);

    expect(entries.map((entry) => entry.date)).toEqual([
      "2026-07-01",
      "2026-07-02",
      "2026-07-03",
    ]);
    expect(entries[0].recap.matches).toHaveLength(1);
    expect(entries[0].recap.news).toEqual([]);
    expect(entries[1].recap.matches).toEqual([]);
    expect(entries[1].recap.news.map((article) => article.id)).toEqual(["day-2-injury"]);
    expect(entries[2].recap.matches).toHaveLength(1);
    // Per-day windows match the streaming loop: advancedTo is the next day.
    expect(entries[1].recap.advancedTo).toBe("2026-07-03");
  });

  it("splits per day even when sinceDate arrives as a full timestamp", function (): void {
    const game = createGame({
      clock: { current_date: "2026-07-03T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
    });
    const entries = buildDigestEntries(game, "2026-07-01T00:00:00Z", [matchOnDay]);
    expect(entries.map((entry) => entry.date)).toEqual(["2026-07-01", "2026-07-02"]);
    expect(entries[0].recap.matches).toHaveLength(1);
  });

  it("includes quiet days so the feed mirrors the streaming digest", function (): void {
    const game = createGame({
      clock: { current_date: "2026-07-03T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
    });
    const entries = buildDigestEntries(game, "2026-07-01", []);
    expect(entries).toHaveLength(2);
    expect(entries.every((entry) => !entry.recap.hasEvents)).toBe(true);
  });

  it("falls back to a single catch-all entry when no day window can be derived", function (): void {
    const game = createGame({ clock: null } as unknown as Partial<GameStateData>);

    // Even a quiet advance renders one entry rather than an empty feed.
    const quiet = buildDigestEntries(game, "2026-07-01", []);
    expect(quiet).toHaveLength(1);
    expect(quiet[0].date).toBe("2026-07-01");
    expect(quiet[0].recap.hasEvents).toBe(false);

    expect(buildDigestEntries(game, "2026-07-01", [matchOnDay])).toHaveLength(1);

    // Invalid sinceDate: the entry is dated on the post-advance clock day.
    const clockOnly = buildDigestEntries(createGame(), "", []);
    expect(clockOnly).toHaveLength(1);
    expect(clockOnly[0].date).toBe("2026-07-02");

    // Neither bound displayable and nothing happened: no entry.
    expect(buildDigestEntries(game, "", [])).toEqual([]);
  });
});

describe("detectAttentionEvents", function (): void {
  function recapFor(game: GameStateData, sinceDate = "2026-07-01") {
    return buildAdvanceRecap(game, sinceDate, []);
  }

  it("reports nothing on a quiet day", function (): void {
    const game = createGame();
    expect(detectAttentionEvents(game, recapFor(game))).toEqual([]);
  });

  it("stops on a new high-priority inbox item", function (): void {
    const game = createGame({
      messages: [
        {
          id: "offer",
          subject: "Bid received",
          body: "",
          sender: "",
          sender_role: "",
          date: "2026-07-01",
          read: false,
          category: "Transfer",
          priority: "High",
          actions: [],
        },
      ],
    } as unknown as Partial<GameStateData>);
    expect(detectAttentionEvents(game, recapFor(game))).toEqual(["highPriorityInbox"]);
  });

  it("stops on a transfer involving the user's club, not on others", function (): void {
    const transfer = (from: string, to: string) => ({
      date: "2026-07-01",
      from_team_id: from,
      to_team_id: to,
      player_id: "player-1",
      fee: 1_000_000,
    });
    const withLog = (log: unknown[]) =>
      createGame({
        league: {
          id: "league-1",
          name: "League",
          season: 1,
          fixtures: [],
          standings: [],
          transfer_log: log,
        },
      } as unknown as Partial<GameStateData>);

    const userGame = withLog([transfer("team-1", "team-2")]);
    expect(detectAttentionEvents(userGame, recapFor(userGame))).toEqual(["userTransfer"]);

    const otherGame = withLog([transfer("team-2", "team-3")]);
    expect(detectAttentionEvents(otherGame, recapFor(otherGame))).toEqual([]);
  });

  it("stops on key news tagging the user's club or squad players", function (): void {
    const article = (id: string, teamIds: string[], playerIds: string[]) => ({
      id,
      headline: "",
      body: "",
      date: "2026-07-01",
      category: "Editorial",
      team_ids: teamIds,
      player_ids: playerIds,
      read: false,
    });

    const clubGame = createGame({
      news: [article("club-news", ["team-1"], [])],
    } as unknown as Partial<GameStateData>);
    expect(detectAttentionEvents(clubGame, recapFor(clubGame))).toEqual(["userNews"]);

    const squadGame = createGame({
      players: [{ id: "player-1", full_name: "John Star", team_id: "team-1" }],
      news: [article("player-news", [], ["player-1"])],
    } as unknown as Partial<GameStateData>);
    expect(detectAttentionEvents(squadGame, recapFor(squadGame))).toEqual(["userNews"]);

    const rivalGame = createGame({
      news: [article("rival-news", ["team-2"], [])],
    } as unknown as Partial<GameStateData>);
    expect(detectAttentionEvents(rivalGame, recapFor(rivalGame))).toEqual([]);
  });

  it("detects a user transfer pushed out of the displayed section by truncation", function (): void {
    // Nine same-day transfers: the user's carries the lexicographically
    // smallest date, so the newest-first sort + MAX_PER_SECTION slice drops
    // it from the displayed list — detection must still fire.
    const transferAt = (from: string, date: string) => ({
      date,
      from_team_id: from,
      to_team_id: "team-3",
      player_id: "player-1",
      fee: 1_000_000,
    });
    const game = createGame({
      league: {
        id: "league-1",
        name: "League",
        season: 1,
        fixtures: [],
        standings: [],
        transfer_log: [
          ...Array.from({ length: 8 }, () =>
            transferAt("team-2", "2026-07-01T12:00:00Z"),
          ),
          transferAt("team-1", "2026-07-01"),
        ],
      },
    } as unknown as Partial<GameStateData>);

    const recap = recapFor(game);
    expect(recap.transfers).toHaveLength(8);
    expect(recap.transfers.some((transfer) => transfer.involvesUser)).toBe(false);
    expect(detectAttentionEvents(game, recap)).toEqual(["userTransfer"]);
  });

  it("detects user news pushed out of the displayed section by truncation", function (): void {
    const article = (id: string, teamIds: string[], date: string) => ({
      id,
      headline: "",
      body: "",
      date,
      category: "Editorial",
      team_ids: teamIds,
      player_ids: [],
      read: false,
    });
    const game = createGame({
      news: [
        ...Array.from({ length: 8 }, (_, index) =>
          article(`rival-${index}`, ["team-2"], "2026-07-01T12:00:00Z"),
        ),
        article("club-news", ["team-1"], "2026-07-01"),
      ],
    } as unknown as Partial<GameStateData>);

    const recap = recapFor(game);
    expect(recap.news).toHaveLength(8);
    expect(recap.news.map((headline) => headline.id)).not.toContain("club-news");
    expect(detectAttentionEvents(game, recap)).toEqual(["userNews"]);
  });

  it("stops when landing on a transfer-window open or deadline day", function (): void {
    const withWindow = (window: Record<string, unknown>) =>
      createGame({
        season_context: {
          phase: "InSeason",
          season_start: null,
          season_end: null,
          days_until_season_start: null,
          transfer_window: {
            status: "Closed",
            opens_on: null,
            closes_on: null,
            days_until_opens: null,
            days_remaining: null,
            ...window,
          },
        },
      } as unknown as Partial<GameStateData>);

    // The recap window lands on 2026-07-02 (the fixture clock).
    const opensToday = withWindow({ opens_on: "2026-07-02" });
    expect(detectAttentionEvents(opensToday, recapFor(opensToday))).toEqual([
      "transferWindow",
    ]);

    const deadline = withWindow({ status: "DeadlineDay" });
    expect(detectAttentionEvents(deadline, recapFor(deadline))).toEqual([
      "transferWindow",
    ]);

    const midWindow = withWindow({ status: "Open", closes_on: "2026-08-31" });
    expect(detectAttentionEvents(midWindow, recapFor(midWindow))).toEqual([]);
  });
});
