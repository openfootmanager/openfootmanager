export type SquadListSortKey =
  | "jersey"
  | "name"
  | "pos"
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

/** The squad roster's status filter. */
export type SquadListStatusFilter =
  | "all"
  | "xi"
  | "bench"
  | "naturalFit"
  | "needsCover"
  | "outOfPosition"
  | "injured";

/** The roster filters kept while the manager visits a profile and comes back. */
export interface SquadListFilterState {
  search: string;
  position: string;
  status: SquadListStatusFilter;
}

export const DEFAULT_SQUAD_LIST_FILTER_STATE: SquadListFilterState = {
  search: "",
  position: "All",
  status: "all",
};
