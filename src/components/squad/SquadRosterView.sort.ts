import { calcAge, getPlayerOvr } from "../../lib/helpers";
import { naturalPositionOf } from "../../lib/positions";
import { type SortValue, sortRows } from "../../lib/sortRows";
import type { PlayerData } from "../../store/gameStore";
import type { SquadListSortKey, SquadListSortState } from "./SquadRosterView.state";

/** What the position sort needs to know about the squad beyond each player. */
export interface SquadSortContext {
  isStarter: (player: PlayerData) => boolean;
  /** A position's place in the pitch's order, keeper first (the backend's position table). */
  positionRankOf: (position: string) => number;
}

/** What a player sorts by in each column; a missing shirt number or contract end is none. */
function squadSortValue(
  player: PlayerData,
  key: SquadListSortKey,
  context: SquadSortContext,
): SortValue {
  switch (key) {
    case "jersey":
      return player.jersey_number ?? null;
    case "name":
      return player.full_name;
    case "pos":
      // The default view: the XI first, in the pitch's order of their natural positions, the
      // strongest at each position first.
      return [
        context.isStarter(player) ? 0 : 1,
        context.positionRankOf(naturalPositionOf(player)),
        -getPlayerOvr(player),
      ];
    case "age":
      return calcAge(player.date_of_birth);
    case "condition":
      return player.condition;
    case "morale":
      return player.morale;
    case "ovr":
      return getPlayerOvr(player);
    case "contract":
      return player.contract_end;
  }
}

/**
 * The roster in the order its table (and its cards) show. Through `sortRows`, so a tie keeps the
 * roster's order and a player with no value sorts last in both directions.
 */
export function sortSquadRoster(
  players: readonly PlayerData[],
  sort: SquadListSortState,
  context: SquadSortContext,
  locale: string,
): PlayerData[] {
  return sortRows(
    players,
    (player) => squadSortValue(player, sort.sortKey, context),
    sort.sortDir,
    locale,
  );
}
