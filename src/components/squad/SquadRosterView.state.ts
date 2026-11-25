import { useState } from "react";

export const SQUAD_FIT_STATUS_OPTIONS = [
  ["naturalFit", "squad.naturalFit"],
  ["needsCover", "squad.needsCover"],
  ["outOfPosition", "squad.outOfPosition"],
] as const;

export type FilterScope =
  | "all"
  | "xi"
  | "bench"
  | "injured"
  | (typeof SQUAD_FIT_STATUS_OPTIONS)[number][0];

export interface SquadRosterFilters {
  playerSearch: string;
  positionFilter: string;
  statusFilter: FilterScope;
}

export const DEFAULT_SQUAD_ROSTER_FILTERS: SquadRosterFilters = {
  playerSearch: "",
  positionFilter: "All",
  statusFilter: "all",
};

export interface SquadRosterFilterControl {
  value: SquadRosterFilters;
  onChange: (filters: SquadRosterFilters) => void;
}

/** The dashboard owns filters when the roster can be replaced by a player profile. */
export function useSquadRosterFilters(control?: SquadRosterFilterControl) {
  const [localFilters, setLocalFilters] = useState(DEFAULT_SQUAD_ROSTER_FILTERS);
  const filters = control?.value ?? localFilters;
  const update = control?.onChange ?? setLocalFilters;
  return {
    ...filters,
    setPlayerSearch: (playerSearch: string) => update({ ...filters, playerSearch }),
    setPositionFilter: (positionFilter: string) => update({ ...filters, positionFilter }),
    setStatusFilter: (statusFilter: FilterScope) => update({ ...filters, statusFilter }),
    clearFilters: () => update(DEFAULT_SQUAD_ROSTER_FILTERS),
  };
}

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
