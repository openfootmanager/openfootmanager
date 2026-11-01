import { canonicalPosition } from "./positions";

/** One position as the backend's `get_position_groups` names it. */
export interface PositionEntry {
  position: string;
  /** The line it belongs to: Goalkeeper, Defender, Midfielder or Forward. */
  group: string;
  /** Where it sorts in a squad list (lower first); a bucket takes the number its line starts at. */
  rank: number;
}

/** A position that is not in the table sorts after every position that is. */
const UNRANKED = 999;

/**
 * What the screens ask of the backend's position table: a position's group, its place in a squad
 * list, the place of its line, and a group's detailed members. The table is the backend's; this
 * only reads it. Until it arrives it is empty, so groups are unknown and every rank ties.
 */
export interface PositionTable {
  /** False until the table arrives; a screen that writes a selection back waits for it. */
  loaded: boolean;
  /** The group a position (or an alias such as "RB") belongs to; the position itself if unknown. */
  groupOf(position: string): string;
  /** Where the position sorts in a squad list. */
  rankOf(position: string): number;
  /** Where the position's line sorts: every position of a line shares the line's rank. */
  lineRankOf(position: string): number;
  /** The lines in pitch order (goalkeepers first); empty until the table arrives. */
  lines(): string[];
  /** The detailed positions inside a group, in pitch order; the group itself if it has none. */
  membersOf(group: string): string[];
}

export function createPositionTable(entries: readonly PositionEntry[]): PositionTable {
  const byPosition = new Map(entries.map((entry) => [entry.position, entry]));
  const entryOf = (position: string) => byPosition.get(canonicalPosition(position));
  const groupOf = (position: string) => {
    const canonical = canonicalPosition(position);
    return byPosition.get(canonical)?.group ?? canonical;
  };
  return {
    loaded: entries.length > 0,
    groupOf,
    rankOf: (position) => entryOf(position)?.rank ?? UNRANKED,
    lineRankOf: (position) => byPosition.get(groupOf(position))?.rank ?? UNRANKED,
    lines: () =>
      entries
        .filter((entry) => entry.position === entry.group)
        .sort((a, b) => a.rank - b.rank)
        .map((entry) => entry.position),
    membersOf: (group) => {
      const members = entries
        .filter((entry) => entry.group === group && entry.position !== group)
        .sort((a, b) => a.rank - b.rank)
        .map((entry) => entry.position);
      return members.length > 0 ? members : [group];
    },
  };
}
