import type { PlayerSortKey } from "../../services/playersService";

/**
 * The fields a player list can be sorted by, as the card views' sort control offers them (the
 * backend sorts the page by `sort_key`). The Players list, the transfer market and the scouting
 * search all page through the same query, so they offer the same fields.
 */
export const PLAYER_SORT_OPTIONS: readonly { key: PlayerSortKey; labelKey: string }[] = [
  { key: "name", labelKey: "common.name" },
  { key: "position", labelKey: "common.position" },
  { key: "age", labelKey: "common.age" },
  { key: "team", labelKey: "common.team" },
  { key: "value", labelKey: "common.value" },
  { key: "ovr", labelKey: "common.ovr" },
];
