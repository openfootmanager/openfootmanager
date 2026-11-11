import type { PlayerRecentMatchEntry } from "../PlayerProfileRecentMatchesCard";

/** A result as the form marks and the previous-matches rows draw it. */
export type MatchResult = "W" | "D" | "L";

/** How the match ended for his club: the two scores say it, so no screen guesses at a result. */
export function resultOf(match: Pick<PlayerRecentMatchEntry, "team_goals" | "opponent_goals">) {
  if (match.team_goals > match.opponent_goals) return "W";
  if (match.team_goals < match.opponent_goals) return "L";
  return "D";
}

/** The form of the last `count` matches, oldest first, as the form dots are read. `matches` is
 *  latest first, as the backend returns them. */
export function formOf(matches: readonly PlayerRecentMatchEntry[], count = 5): MatchResult[] {
  return matches
    .slice(0, count)
    .map((match) => resultOf(match))
    .reverse();
}
