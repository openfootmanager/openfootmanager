import type { AdvanceMatchResultData } from "../../services/advanceTimeService";
import type { GameStateData } from "../../store/gameStore";
import { getUserCompetition } from "../../lib/fixtures";
import { resolveSeasonContext } from "../../lib/seasonContext";

/** Match result enriched with the user's outcome (win/draw/loss) when they were involved. */
export interface RecapMatch extends AdvanceMatchResultData {
  userResult?: "win" | "draw" | "loss";
}

/** A completed transfer surfaced in the post-advance recap. */
export interface RecapTransfer {
  player: string;
  from: string;
  to: string;
  fee: number;
  involvesUser: boolean;
  date: string;
}

/** A news/inbox line, carrying its i18n key so the modal resolves it via t(). */
export interface RecapHeadline {
  id: string;
  date: string;
  text: string;
  textKey?: string;
  params?: Record<string, string>;
  /** Message category (inbox items only) — used to resolve the icon. */
  category?: string;
}

export interface AdvanceRecap {
  /** New current date (YYYY-MM-DD) the game advanced to. */
  advancedTo: string;
  matches: RecapMatch[];
  transfers: RecapTransfer[];
  news: RecapHeadline[];
  inbox: RecapHeadline[];
  /** True when anything happened during the advance (drives the empty state). */
  hasEvents: boolean;
}

/** One day of the digest feed: the day simulated and what it produced. */
export interface DigestEntry {
  date: string;
  recap: AdvanceRecap;
}

/**
 * Something the "Continue skips to next event" advance should pause on, beyond
 * match days and blockers (which stop the loop before the day is processed).
 */
export type AttentionEventKind =
  | "highPriorityInbox"
  | "userTransfer"
  | "userNews"
  | "transferWindow";

const MAX_PER_SECTION = 8;

// Routine or duplicate categories are kept out of the "key news" section:
// match reports/roundups repeat the results list and transfer news repeats the
// transfers section.
const ROUTINE_NEWS_CATEGORIES = new Set([
  "MatchReport",
  "LeagueRoundup",
  "StandingsUpdate",
  "TransferRumour",
  "TransferRoundup",
]);

const NOTABLE_ROUTINE_NEWS_KEYS = new Set([
  "be.news.loanMove.headline",
]);

/** Date portion (YYYY-MM-DD) of an ISO timestamp. */
export function toDatePart(iso: string | null | undefined): string {
  return iso ? iso.slice(0, 10) : "";
}

function isRoutineRecapNews(article: GameStateData["news"][number]): boolean {
  return (
    ROUTINE_NEWS_CATEGORIES.has(article.category) &&
    !NOTABLE_ROUTINE_NEWS_KEYS.has(article.headline_key ?? "")
  );
}

/** Day after a YYYY-MM-DD day, UTC-safe. */
export function nextDay(day: string): string {
  const date = new Date(`${day}T00:00:00Z`);
  date.setUTCDate(date.getUTCDate() + 1);
  return date.toISOString().slice(0, 10);
}

/**
 * Assemble the post-advance recap from the new game state and the clock date
 * captured *before* the advance. Match results come from the backend (already
 * scoped to the user's competitions); transfers, news, and inbox items are
 * derived here from entries dated on or after the advance.
 */
export function buildAdvanceRecap(
  game: GameStateData,
  sinceDate: string,
  matches: AdvanceMatchResultData[],
): AdvanceRecap {
  return buildRecapWindow(
    game,
    sinceDate,
    toDatePart(game.clock?.current_date),
    matches,
  );
}

/**
 * Recap of the half-open day window [sinceDate, untilDate). Some news is dated
 * at the event it announces (e.g. the World Cup kickoff article created months
 * ahead at a season rollover; the news feed hides those until their day via
 * utils/newsVisibility). Without an upper bound such future-dated items match
 * `date >= sinceDate` on every subsequent advance and repeat in every digest
 * day until their date arrives — so the recap only surfaces items dated inside
 * the window actually simulated. The backend dates a day's events on the day
 * it processes and then moves the clock, so a whole advance is
 * [sinceDate, clock) and a single digest day is [day, nextDay(day)): exclusive
 * above, leaving items dated on the landing day to the advance that will
 * actually play that day. An empty untilDate (defensive, no clock) means no
 * upper bound.
 */
export function buildRecapWindow(
  game: GameStateData,
  sinceDate: string,
  untilDate: string,
  matches: AdvanceMatchResultData[],
): AdvanceRecap {
  const advancedTo = untilDate;
  const inWindow = (date: string): boolean => {
    const day = toDatePart(date);
    return day >= sinceDate && (!advancedTo || day < advancedTo);
  };
  const userTeamId = game.manager?.team_id ?? null;

  const teamName = new Map((game.teams ?? []).map((team) => [team.id, team.name]));
  const playerName = new Map(
    (game.players ?? []).map((player) => [
      player.id,
      player.full_name ?? player.match_name ?? player.id,
    ]),
  );

  const userTeamName = userTeamId ? (teamName.get(userTeamId) ?? null) : null;
  const recapMatches: RecapMatch[] = matches.map((match) => {
    if (!match.involves_user || !userTeamName) return match;
    const isHome = match.home_team === userTeamName;
    const userGoals = isHome ? match.home_goals : match.away_goals;
    const oppGoals = isHome ? match.away_goals : match.home_goals;
    const userResult: RecapMatch["userResult"] =
      userGoals > oppGoals ? "win" : userGoals < oppGoals ? "loss" : "draw";
    return { ...match, userResult };
  });

  const userCompetition = getUserCompetition(game);
  const transfers: RecapTransfer[] = (userCompetition?.transfer_log ?? [])
    .filter((entry) => inWindow(entry.date))
    .sort((left, right) => right.date.localeCompare(left.date))
    .slice(0, MAX_PER_SECTION)
    .map((entry) => ({
      player: playerName.get(entry.player_id) ?? entry.player_id,
      from: teamName.get(entry.from_team_id) ?? entry.from_team_id,
      to: teamName.get(entry.to_team_id) ?? entry.to_team_id,
      fee: entry.fee,
      involvesUser:
        entry.from_team_id === userTeamId || entry.to_team_id === userTeamId,
      date: entry.date,
    }));

  const news: RecapHeadline[] = (game.news ?? [])
    .filter(
      (article) =>
        inWindow(article.date) &&
        !isRoutineRecapNews(article),
    )
    .sort((left, right) => right.date.localeCompare(left.date))
    .slice(0, MAX_PER_SECTION)
    .map((article) => ({
      id: article.id,
      date: article.date,
      text: article.headline,
      textKey: article.headline_key,
      params: article.i18n_params,
    }));

  const inbox: RecapHeadline[] = (game.messages ?? [])
    .filter((message) => inWindow(message.date) && message.priority === "High")
    .sort((left, right) => right.date.localeCompare(left.date))
    .slice(0, MAX_PER_SECTION)
    .map((message) => ({
      id: message.id,
      date: message.date,
      text: message.subject,
      textKey: message.subject_key,
      params: message.i18n_params,
      category: message.category,
    }));

  const hasEvents =
    recapMatches.length > 0 ||
    transfers.length > 0 ||
    news.length > 0 ||
    inbox.length > 0;

  return { advancedTo, matches: recapMatches, transfers, news, inbox, hasEvents };
}

const DAY_PATTERN = /^\d{4}-\d{2}-\d{2}$/;

// Hard cap on synthesized digest days: the backend multi-day advances stop
// after 60 processed days, so anything past that means bad input dates.
const MAX_ENTRY_DAYS = 90;

/**
 * Split a batch multi-day advance (Skip to Match Day, plain Continue) into the
 * same per-day entries the streaming digest loop produces, so every advance
 * flow renders through the one digest feed. Day i covers [day, nextDay(day))
 * with that day's match results; days outside [sinceDate, clock) fall back to
 * a single catch-all entry rather than being dropped.
 */
export function buildDigestEntries(
  game: GameStateData,
  sinceDate: string,
  results: AdvanceMatchResultData[],
): DigestEntry[] {
  const advancedTo = toDatePart(game.clock?.current_date);
  if (
    !DAY_PATTERN.test(sinceDate) ||
    !DAY_PATTERN.test(advancedTo) ||
    sinceDate >= advancedTo
  ) {
    const recap = buildAdvanceRecap(game, sinceDate, results);
    return recap.hasEvents ? [{ date: sinceDate, recap }] : [];
  }

  const entries: DigestEntry[] = [];
  for (
    let day = sinceDate;
    day < advancedTo && entries.length < MAX_ENTRY_DAYS;
    day = nextDay(day)
  ) {
    const until = nextDay(day);
    const dayMatches = results.filter((match) => toDatePart(match.date) === day);
    entries.push({
      date: day,
      recap: buildRecapWindow(game, day, until, dayMatches),
    });
  }
  return entries;
}

/**
 * What in a just-simulated day should pause the "Continue to next event"
 * advance. Match days and blockers stop the loop before the day runs; these
 * are the notable outcomes of a day that did run: a high-priority inbox item,
 * a transfer in/out of the user's club, key news tagging the club or its
 * players, and landing on a transfer-window open or deadline day (mirrors the
 * backend's continue_reached_attention_event).
 */
export function detectAttentionEvents(
  game: GameStateData,
  recap: AdvanceRecap,
): AttentionEventKind[] {
  const events: AttentionEventKind[] = [];

  if (recap.inbox.length > 0) events.push("highPriorityInbox");
  if (recap.transfers.some((transfer) => transfer.involvesUser)) {
    events.push("userTransfer");
  }

  const userTeamId = game.manager?.team_id ?? null;
  if (userTeamId && recap.news.length > 0) {
    const squadIds = new Set(
      (game.players ?? [])
        .filter((player) => player.team_id === userTeamId)
        .map((player) => player.id),
    );
    const articleById = new Map(
      (game.news ?? []).map((article) => [article.id, article]),
    );
    const tagsUser = recap.news.some((headline) => {
      const article = articleById.get(headline.id);
      if (!article) return false;
      return (
        article.team_ids.includes(userTeamId) ||
        article.player_ids.some((id) => squadIds.has(id))
      );
    });
    if (tagsUser) events.push("userNews");
  }

  const window = resolveSeasonContext(game).transfer_window;
  const landedOn = recap.advancedTo;
  if (
    landedOn &&
    (window.status === "DeadlineDay" ||
      toDatePart(window.opens_on) === landedOn ||
      toDatePart(window.closes_on) === landedOn)
  ) {
    events.push("transferWindow");
  }

  return events;
}
