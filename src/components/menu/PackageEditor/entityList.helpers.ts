import type { TeamDef } from "./types";

/**
 * How many rows an entity list shows before it asks to be told to show more.
 * The list column is a narrow sidebar — about a dozen rows fit on screen — so
 * fifty is several screens of scrolling, while still bounding both the DOM and
 * the one asset read each row with a photo or crest fires on mount.
 */
export const ENTITY_LIST_PAGE_SIZE = 50;

export interface CappedRows<T> {
  visible: T[];
  hiddenCount: number;
}

/**
 * Take the revealed slice of a list.
 *
 * The window stretches by up to one page to keep a selected row visible:
 * duplicating the last visible record selects the copy just below it, and
 * opening a form for a row the list cannot show is disorienting. It stops
 * there deliberately — clearing a search can put the selected record thousands
 * of rows down, and chasing it would render everything and undo the cap.
 */
export function capRows<T>(
  rows: T[],
  visibleCount: number,
  pageSize: number,
  isSelected: (row: T) => boolean,
): CappedRows<T> {
  if (rows.length <= visibleCount) {
    return { visible: rows, hiddenCount: 0 };
  }

  // Only the stretch window is worth searching. Scanning the whole list to
  // find a selection that would be rejected anyway is the O(N)-per-render work
  // the rest of this file exists to remove — and this runs on every keystroke,
  // since the edit buffer shares a tree with the lists.
  let limit = visibleCount;
  const end = Math.min(rows.length, visibleCount + pageSize);
  for (let pos = visibleCount; pos < end; pos += 1) {
    if (isSelected(rows[pos])) {
      limit = pos + 1;
      break;
    }
  }

  return { visible: rows.slice(0, limit), hiddenCount: rows.length - limit };
}

/**
 * Club id → display name, so a list row can name a player's or staff member's
 * club without scanning the team array. The lists resolved it with a `find`
 * inside the row map, which is O(rows × teams) on every render — noticeable
 * once a package holds a full pyramid rather than a sample league.
 *
 * Records with no id are skipped: a half-typed team would otherwise claim the
 * empty club id that "no club" uses.
 */
export function buildTeamNameMap(teams: TeamDef[] | undefined): Map<string, string> {
  const names = new Map<string, string>();
  teams?.forEach((team) => {
    if (team.id) {
      names.set(team.id, team.name);
    }
  });
  return names;
}
