import type { PlayerRecentMatchEntry } from "../PlayerProfile.helpers";

/** The radar's eight axes, the same for every outfield player so two radars compare at a glance. */
export const OUTFIELD_AXES = [
  "pace",
  "shooting",
  "dribbling",
  "passing",
  "vision",
  "composure",
  "strength",
  "aerial",
] as const;

/** A goalkeeper's eight, in place of the outfield ones. */
export const GOALKEEPER_AXES = [
  "handling",
  "reflexes",
  "aerial",
  "composure",
  "vision",
  "agility",
  "stamina",
  "strength",
] as const;

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

const HAN = /[\u3400-\u9fff]/;

/**
 * Each name shortened for a radar axis: its first three letters, upper-cased (its first two
 * characters for a Chinese name). Names that would collide, as two that begin alike in some
 * languages do, are lengthened one letter at a time until every label differs.
 */
export function shortLabels(names: readonly string[]): string[] {
  // Letters only: a space in a short label would read as two words.
  const letters = names.map((name) => name.replace(/\s+/g, ""));
  const lengths = letters.map((name) => (HAN.test(name) ? 2 : 3));
  const label = (index: number) => letters[index].slice(0, lengths[index]).toUpperCase();
  for (let pass = 0; pass < 16; pass += 1) {
    const labels = letters.map((_, index) => label(index));
    const clashing = labels
      .map((value, index) => ({ value, index }))
      .filter(({ value }) => labels.filter((other) => other === value).length > 1)
      .filter(({ index }) => lengths[index] < letters[index].length);
    if (clashing.length === 0) break;
    for (const { index } of clashing) lengths[index] += 1;
  }
  return letters.map((_, index) => label(index));
}
