import { beforeAll, beforeEach, describe, expect, it } from "vitest";

import { i18nReady } from "../i18n";
import { takeMissingKeys } from "../i18n/missingKeys";
import { useSettingsStore } from "../store/settingsStore";
import type { MessageAction, MessageData, NewsArticle } from "../store/types";
import { resolveMessage, resolveNewsArticle } from "./backendI18n";

/**
 * The drift guard between the two English resolvers.
 *
 * The MCP server shows an agent the same inbox and news the player reads, so it has to turn the
 * backend's translation keys into text too. `src-tauri/src/mcp_server/backend_text.rs` does that in
 * Rust, and `backendI18n.ts` stays the reference. This file is the one list of cases both read:
 * `backendTextGolden.json` holds each case's input and the text `backendI18n.ts` makes of it. Here
 * the frontend is held to the file; the Rust test holds its resolver to the same file, so a change
 * to either side that the other does not follow fails the job that owns the stale side.
 *
 * To change a case, or after changing how a key or a param is resolved, regenerate the file from
 * the frontend and commit it:
 *
 *   rm src/utils/backendTextGolden.json && npm exec --no -- vitest run src/utils/backendTextGolden.test.ts
 *
 * (A run with the file missing writes it; under CI a missing or different file fails.)
 *
 * Not covered, on purpose: the params the frontend expands from structured JSON (roundups,
 * standings, scorers, press quotes), a division's or competition's display name, and country names.
 * The Rust resolver leaves those unexpanded and says so in its output (`details`).
 */

type GoldenMessage = {
  id?: string;
  subject?: string;
  body?: string;
  sender?: string;
  sender_role?: string;
  subject_key?: string;
  body_key?: string;
  sender_key?: string;
  sender_role_key?: string;
  i18n_params?: Record<string, string>;
  actions?: MessageAction[];
};

type GoldenArticle = {
  id?: string;
  headline?: string;
  body?: string;
  source?: string;
  headline_key?: string;
  body_key?: string;
  source_key?: string;
  i18n_params?: Record<string, string>;
};

type GoldenCase =
  | { name: string; kind: "message"; input: GoldenMessage; expected?: unknown }
  | { name: string; kind: "article"; input: GoldenArticle; expected?: unknown };

const CASES: GoldenCase[] = [
  {
    name: "a message whose subject, body, sender and action label are keys with a plain param",
    kind: "message",
    input: {
      id: "board_expect_1",
      subject_key: "be.msg.boardExpect.subject",
      body_key: "be.msg.boardExpect.body",
      sender_key: "be.sender.boardOfDirectors",
      sender_role_key: "be.role.chairman",
      i18n_params: { team: "St. Pauli" },
      actions: [
        {
          id: "ack_objectives",
          label: "",
          label_key: "be.msg.boardExpect.actionAccept",
          action_type: "Acknowledge",
          resolved: false,
        },
      ],
    },
  },
  {
    name: "a name carrying an apostrophe, an ampersand and a dot is not mistaken for a key",
    kind: "message",
    input: {
      id: "transfer_offer_1",
      subject_key: "be.msg.transferOffer.subject",
      body_key: "be.msg.transferOffer.body",
      i18n_params: { player: "D'Arcy O'Neil & Sons", team: "St. Mirren F.C.", fee: "1250000" },
    },
  },
  {
    name: "a money param is shown compact when three digits say it all",
    kind: "message",
    input: {
      id: "transfer_offer_2",
      subject_key: "be.msg.transferOffer.subject",
      body_key: "be.msg.transferOffer.body",
      i18n_params: { player: "A", team: "B", fee: "1700" },
    },
  },
  {
    name: "a money param that compact would round is shown exactly",
    kind: "message",
    input: {
      id: "transfer_offer_3",
      subject_key: "be.msg.transferOffer.subject",
      body_key: "be.msg.transferOffer.body",
      i18n_params: { player: "A", team: "B", fee: "800001" },
    },
  },
  {
    name: "a money param in whole millions drops the decimals",
    kind: "message",
    input: {
      id: "transfer_offer_4",
      subject_key: "be.msg.transferOffer.subject",
      body_key: "be.msg.transferOffer.body",
      i18n_params: { player: "A", team: "B", fee: "25000000" },
    },
  },
  {
    name: "a money param an old message saved already shortened keeps the figure it carries",
    kind: "message",
    input: {
      id: "transfer_offer_5",
      subject_key: "be.msg.transferOffer.subject",
      body_key: "be.msg.transferOffer.body",
      i18n_params: { player: "A", team: "B", fee: "€1.25M" },
    },
  },
  {
    name: "a negative money param keeps its sign",
    kind: "message",
    input: {
      id: "wage_over_budget_1",
      subject_key: "be.msg.wageOverBudget.subject",
      body_key: "be.msg.wageOverBudget.body",
      i18n_params: { weeklyWageBill: "-1500000", wageBudget: "0" },
    },
  },
  {
    name: "start and end are rendered as dates, and a plain param beside them is left alone",
    kind: "message",
    input: {
      id: "loan_agreed_1",
      subject_key: "be.msg.loanAgreed.subject",
      body_key: "be.msg.loanAgreed.bodyWithOption",
      i18n_params: {
        player: "P. Smith",
        parent: "North",
        borrower: "South",
        start: "2026-08-01",
        end: "2027-05-31",
        contribution: "50",
        fee: "4000000",
      },
    },
  },
  {
    name: "a finishing position becomes the ordinal the template asks for",
    kind: "message",
    input: {
      id: "season_payout_1",
      subject_key: "be.msg.seasonPayout.subject",
      body_key: "be.msg.seasonPayout.body",
      i18n_params: { season: "2026", amount: "9000000", position: "21" },
    },
  },
  {
    name: "the ordinal of a position ending 11, 12 and 13 is th",
    kind: "message",
    input: {
      id: "season_payout_2",
      subject_key: "be.msg.seasonPayout.subject",
      body_key: "be.msg.seasonPayout.body",
      i18n_params: { season: "2026", amount: "9000000", position: "12" },
    },
  },
  {
    name: "a param an older message saved under its old name is read under the new one",
    kind: "message",
    input: {
      id: "wage_over_budget_2",
      subject_key: "be.msg.wageOverBudget.subject",
      body_key: "be.msg.wageOverBudget.body",
      i18n_params: { annualWages: "900000", wageBudget: "800000" },
    },
  },
  {
    name: "a param that is itself a key is resolved to its text",
    kind: "message",
    input: {
      id: "mood_report_1",
      subject_key: "be.msg.moodReport.subject",
      body_key: "be.msg.moodReport.body",
      i18n_params: {
        mood: "common.moods.excellent",
        avgMorale: "71",
        avgCondition: "88",
        affected: "3",
        count: "3",
      },
    },
  },
  {
    name: "a missing key falls back to the plain text the message carries",
    kind: "message",
    input: {
      id: "legacy_1",
      subject_key: "be.msg.thereIsNoSuchKey.subject",
      subject: "A plain subject",
      body: "A plain body.",
      sender: "The Chairman",
      sender_role: "Chairman",
    },
  },
  {
    name: "a message with no keys is its plain text",
    kind: "message",
    input: { id: "plain_1", subject: "Hello", body: "Body text.", sender: "Someone" },
  },
  {
    name: "a placeholder the params do not supply is left as the frontend leaves it",
    kind: "message",
    input: {
      id: "transfer_offer_6",
      subject_key: "be.msg.transferOffer.subject",
      body_key: "be.msg.transferOffer.body",
      i18n_params: { player: "A" },
    },
  },
  {
    name: "a choice action resolves its label and each option's label and description",
    kind: "message",
    input: {
      id: "morale_talk_1",
      subject_key: "be.msg.transferOffer.subject",
      body_key: "be.msg.transferOffer.body",
      i18n_params: { player: "A", team: "B", fee: "1000" },
      actions: [
        {
          id: "respond",
          label: "",
          label_key: "be.msg.playerEvent.respond",
          action_type: {
            ChooseOption: {
              options: [
                {
                  id: "encourage",
                  label: "",
                  description: "",
                  label_key: "be.msg.playerEvent.options.moraleCrisis.encourage.label",
                  description_key: "be.msg.playerEvent.options.moraleCrisis.encourage.description",
                },
                { id: "plain", label: "Plain label", description: "Plain description" },
              ],
            },
          },
          resolved: false,
        },
        {
          id: "view_transfers",
          label: "Open",
          action_type: { NavigateTo: { route: "/dashboard?tab=Transfers" } },
          resolved: true,
        },
      ],
    },
  },
  {
    name: "an old player-event message with no label keys gets them from its id and option ids",
    kind: "message",
    input: {
      id: "morale_talk_p1",
      subject: "Talk",
      body: "Body.",
      actions: [
        {
          id: "respond",
          label: "",
          action_type: {
            ChooseOption: {
              options: [
                { id: "encourage", label: "", description: "" },
                { id: "x", label: "", description: "" },
              ],
            },
          },
          resolved: false,
        },
      ],
    },
  },
  {
    name: "a news article's headline, body and source resolve with a money param",
    kind: "article",
    input: {
      id: "news_1",
      headline_key: "be.news.majorTransfer.headline",
      body_key: "be.news.majorTransfer.body",
      source_key: "be.source.leagueChronicle",
      i18n_params: {
        player: "J. Doe",
        fromTeam: "North",
        toTeam: "South",
        fee: "12500000",
      },
    },
  },
  {
    name: "a news article with no keys is its plain text",
    kind: "article",
    input: { id: "news_2", headline: "Plain headline", body: "Plain body.", source: "Wire" },
  },
  // The structured params the screen lays out (#1388): each article carries a `*Data` JSON param
  // the frontend expands into the lines of the body, and the backend's own plain `results` /
  // `scorers` / `quotes` params for them are empty or only English fallbacks.
  {
    name: "a roundup lays out its results and names the biggest winner",
    kind: "article",
    input: {
      id: "roundup_1",
      headline_key: "be.news.roundup.headline0",
      body_key: "be.news.roundup.body",
      source_key: "be.source.leagueWire",
      i18n_params: {
        matchday: "12",
        totalGoals: "9",
        matchCount: "3",
        results: "",
        resultsData: JSON.stringify([
          { home: "North", home_goals: 4, away: "South", away_goals: 1 },
          { home: "East", home_goals: 0, away: "West", away_goals: 0 },
          { home: "Hill", home_goals: 2, away: "Vale", away_goals: 2 },
        ]),
        biggestWinner: "North",
      },
    },
  },
  {
    name: "a roundup with no biggest winner leaves that sentence out",
    kind: "article",
    input: {
      id: "roundup_2",
      headline_key: "be.news.roundup.headline1",
      body_key: "be.news.roundup.body",
      source_key: "be.source.footballHerald",
      i18n_params: {
        matchday: "3",
        totalGoals: "0",
        matchCount: "1",
        results: "",
        resultsData: JSON.stringify([{ home: "East", home_goals: 0, away: "West", away_goals: 0 }]),
        biggestWinner: "",
      },
    },
  },
  {
    name: "a preseason digest lays out its friendlies and one unbeaten club",
    kind: "article",
    input: {
      id: "preseason_1",
      headline_key: "be.news.preseasonDigest.headline",
      body_key: "be.news.preseasonDigest.bodyWithResults",
      source_key: "be.source.leagueChronicle",
      i18n_params: {
        weekStart: "2026-07-06",
        resultCount: "2",
        totalGoals: "5",
        results: "",
        resultsData: JSON.stringify([
          { home: "North", home_goals: 3, away: "South", away_goals: 1 },
          { home: "East", home_goals: 1, away: "West", away_goals: 0 },
        ]),
        unbeatenLine: "",
        unbeatenTeamsData: JSON.stringify(["North"]),
      },
    },
  },
  {
    name: "a preseason digest with no friendlies names two unbeaten clubs",
    kind: "article",
    input: {
      id: "preseason_2",
      headline_key: "be.news.preseasonDigest.headline",
      body_key: "be.news.preseasonDigest.bodyNoResults",
      source_key: "be.source.leagueChronicle",
      i18n_params: {
        weekStart: "2026-07-13",
        resultCount: "0",
        totalGoals: "0",
        results: "",
        unbeatenLine: "",
        unbeatenTeamsData: JSON.stringify(["North", "East"]),
      },
    },
  },
  {
    name: "a preseason digest lists three or more unbeaten clubs as a sentence",
    kind: "article",
    input: {
      id: "preseason_3",
      headline_key: "be.news.preseasonDigest.headline",
      body_key: "be.news.preseasonDigest.bodyNoResults",
      source_key: "be.source.leagueChronicle",
      i18n_params: {
        weekStart: "2026-07-20",
        resultCount: "0",
        totalGoals: "0",
        results: "",
        unbeatenLine: "",
        unbeatenTeamsData: JSON.stringify(["North", "East", "Hill", "Vale"]),
      },
    },
  },
  {
    name: "a standings update lays out its table",
    kind: "article",
    input: {
      id: "standings_1",
      headline_key: "be.news.standings.headline0",
      body_key: "be.news.standings.body",
      source_key: "be.source.leagueWire",
      i18n_params: {
        matchday: "8",
        leader: "North",
        standings: "",
        standingsData: JSON.stringify([
          { rank: 1, team: "North", points: 19, goal_difference: "+9" },
          { rank: 2, team: "East", points: 17, goal_difference: "+4" },
          { rank: 3, team: "Hill", points: 14, goal_difference: "-1" },
        ]),
      },
    },
  },
  {
    name: "a match report lists its scorers",
    kind: "article",
    input: {
      id: "report_1",
      headline_key: "be.news.matchReport.headline.homeWin.0",
      body_key: "be.news.matchReport.body0",
      source_key: "be.source.sportsGazette",
      i18n_params: {
        home: "North",
        away: "South",
        homeGoals: "2",
        awayGoals: "1",
        matchday: "5",
        winner: "North",
        loser: "South",
        scorers: "",
        scorersData: JSON.stringify([
          { player: "J. Doe", minute: 12, team: "North" },
          { player: "A. Roe", minute: 55, team: "South" },
          { player: "J. Doe", minute: 88, team: "North" },
        ]),
      },
    },
  },
  {
    name: "a match report with no scorers has no goals section",
    kind: "article",
    input: {
      id: "report_2",
      headline_key: "be.news.matchReport.headline.draw.1",
      body_key: "be.news.matchReport.body1.noScorers",
      source_key: "be.source.matchDayPress",
      i18n_params: {
        home: "East",
        away: "West",
        homeGoals: "0",
        awayGoals: "0",
        matchday: "6",
        scorers: "",
        scorersData: "[]",
      },
    },
  },
  {
    name: "a friendly report lists its scorers in a goals section",
    kind: "article",
    input: {
      id: "report_3",
      headline_key: "be.news.matchReport.reportFriendly.title",
      body_key: "be.news.matchReport.reportFriendly.body",
      source_key: "be.source.footballHerald",
      i18n_params: {
        home: "North",
        away: "South",
        homeGoals: "1",
        awayGoals: "0",
        scorers: "",
        scorersSection: "",
        scorersData: JSON.stringify([{ player: "J. Doe", minute: 33, team: "North" }]),
      },
    },
  },
  {
    name: "a press conference with several quotes lists them",
    kind: "article",
    input: {
      id: "press_1",
      headline_key: "be.news.pressConference.headlinePressConf",
      body_key: "be.news.pressConference.bodyMultiple",
      source_key: "be.source.leagueWire",
      i18n_params: {
        team: "North",
        result: "2-1",
        quote: "We never stopped believing",
        quotesData: JSON.stringify([
          { fallback: "We never stopped believing" },
          { fallback: "The lads showed real character" },
        ]),
      },
    },
  },
  {
    name: "a press conference with one quote shows it",
    kind: "article",
    input: {
      id: "press_2",
      headline_key: "be.news.pressConference.headlineManagerQuote",
      body_key: "be.news.pressConference.bodySingle",
      source_key: "be.source.leagueWire",
      i18n_params: {
        team: "South",
        result: "0-3",
        quote: "A tough afternoon",
        quotesData: JSON.stringify([{ fallback: "A tough afternoon" }]),
      },
    },
  },
  {
    name: "a transfer roundup lays out its deals with the fee compact",
    kind: "article",
    input: {
      id: "roundup_deals_1",
      headline_key: "be.news.transferRoundup.headline",
      body_key: "be.news.transferRoundup.body",
      source_key: "be.source.transferIntelligence",
      i18n_params: {
        weekStart: "2026-08-03",
        transferCount: "2",
        deals: "",
        dealsData: JSON.stringify([
          { player: "J. Doe", fromTeam: "North", toTeam: "South", fee: "12500000" },
          { player: "A. Roe", fromTeam: "East", toTeam: "West", fee: "800000" },
        ]),
      },
    },
  },
];

function messageOf(input: GoldenMessage): MessageData {
  return {
    id: "m",
    subject: "",
    body: "",
    sender: "",
    sender_role: "",
    date: "2026-08-01",
    read: false,
    category: "General",
    priority: "Normal",
    actions: [],
    context: {},
    ...input,
  } as MessageData;
}

function articleOf(input: GoldenArticle): NewsArticle {
  return {
    id: "a",
    headline: "",
    body: "",
    source: "",
    date: "2026-08-01",
    category: "General",
    team_ids: [],
    player_ids: [],
    match_score: null,
    read: false,
    ...input,
  } as NewsArticle;
}

function expectedOf(testCase: GoldenCase): unknown {
  const expected = resolvedText(testCase);
  // A case asks for an absent key on purpose (its fallback is what is under test), which the test
  // setup would otherwise report as a missing translation.
  takeMissingKeys();
  return expected;
}

function resolvedText(testCase: GoldenCase): unknown {
  if (testCase.kind === "article") {
    const { headline, body, source } = resolveNewsArticle(articleOf(testCase.input));
    return { headline, body, source };
  }
  const resolved = resolveMessage(messageOf(testCase.input));
  return {
    subject: resolved.subject,
    body: resolved.body,
    sender: resolved.sender,
    sender_role: resolved.sender_role,
    actions: resolved.actions.map((action) => ({
      id: action.id,
      label: action.label,
      options:
        typeof action.action_type === "object" && "ChooseOption" in action.action_type
          ? action.action_type.ChooseOption.options.map((option) => ({
              id: option.id,
              label: option.label,
              description: option.description,
            }))
          : [],
    })),
  };
}

const originalSettings = useSettingsStore.getState().settings;

beforeAll(async () => {
  await i18nReady;
});

beforeEach(() => {
  // The currency the golden text is written in: the base one, which is what the backend's raw
  // amounts are in and what an agent is shown.
  useSettingsStore.setState({
    settings: { ...originalSettings, currency: "EUR", language: "en" },
    currency: { code: "EUR", symbol: "€", exchange_rate: 1 },
    supportedCurrencies: { EUR: { code: "EUR", symbol: "€", exchange_rate: 1 } },
  });
});

describe("the backend text golden file", () => {
  /**
   * Given the cases this file lists
   * When `backendI18n.ts` resolves each in English
   * Then `backendTextGolden.json` is the cases and that text, exactly: the Rust resolver the MCP
   *      server uses is held to the same file, so neither side can change without the other.
   *      deleting the file and running this test rewrites it after a change both resolvers have been taught.
   */
  it("is what the frontend resolver writes for the cases it lists", async () => {
    const cases = CASES.map((testCase) => ({ ...testCase, expected: expectedOf(testCase) }));

    await expect(`${JSON.stringify({ cases }, null, 2)}\n`).toMatchFileSnapshot(
      "./backendTextGolden.json",
    );
  });
});
