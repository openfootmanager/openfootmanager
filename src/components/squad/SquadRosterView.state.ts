export type SquadListSortKey =
  | "pos"
  | "name"
  | "age"
  | "condition"
  | "morale"
  | "ovr";

export interface SquadListSortState {
  sortKey: SquadListSortKey;
  sortDir: "asc" | "desc";
}

export const DEFAULT_SQUAD_LIST_SORT_STATE: SquadListSortState = {
  sortKey: "pos",
  sortDir: "asc",
};
