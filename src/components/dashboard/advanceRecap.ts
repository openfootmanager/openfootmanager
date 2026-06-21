import type { AdvanceMatchResultData } from "../../services/advanceTimeService";
import type { GameStateData } from "../../store/gameStore";
import { getUserCompetition } from "../../lib/fixtures";

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
  const advancedTo = toDatePart(game.clock?.current_date);
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
    .filter((entry) => entry.date >= sinceDate)
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
        article.date >= sinceDate &&
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
    .filter((message) => message.date >= sinceDate && message.priority === "High")
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
