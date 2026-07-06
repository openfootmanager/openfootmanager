export type SquadListSortKey =
  | "jersey"
  | "name"
  | "pos"
  | "fit"
  | "style"
  | "age"
  | "condition"
  | "morale"
  | "ovr"
  | "contract";

export interface SquadListSortState {
  sortKey: SquadListSortKey;
  sortDir: "asc" | "desc";
}

/**
 * Default: sort by position — starters first (XI members top), then by
 * natural_position in football-canonical order (GK → back line → midfield →
 * forwards), then by OVR desc so the strongest at each slot rises within its
 * cluster. This gives a "who plays where" view at a glance without the user
 * having to touch the sort headers.
 */
export const DEFAULT_SQUAD_LIST_SORT_STATE: SquadListSortState = {
  sortKey: "pos",
  sortDir: "asc",
};
