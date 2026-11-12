import type { PlayerData } from "../../store/gameStore";
import type { Familiarity } from "../../store/generated/Familiarity";
import type { TacticsLineup } from "../../store/generated/TacticsLineup";

/** The table's groups, in the order they are drawn. */
export type TableGroup = "xi" | "substitutes" | "reserves";

export interface TableRow {
  player: PlayerData;
  group: TableGroup;
  /** The pitch slot a starter plays in; null on the bench. */
  slotIndex: number | null;
  /** The slot's position (a starter's), from the backend's lineup or the formation's layout. */
  position: string | null;
  /** How the starter stands at his slot, the backend's word; null when it hasn't said. */
  familiarity: Familiarity | null;
  /** The role he plays there, the backend's (the one the match will play). */
  role: string | null;
}

/** The rows in table order: the eleven by slot, the substitutes as the match ranks them, then the reserves. */
export function buildTableRows(
  starters: PlayerData[],
  others: PlayerData[],
  slotPositions: readonly string[],
  lineup: TacticsLineup | null,
  playerRoles: Record<string, string> | undefined,
): TableRow[] {
  const xi = starters.map((player, index): TableRow => {
    // The backend's word for the slot, only while its lineup still names this player there.
    const slot = lineup?.eleven[index]?.player_id === player.id ? lineup.eleven[index] : null;
    return {
      player,
      group: "xi",
      slotIndex: index,
      position: slot?.position ?? slotPositions[index] ?? null,
      familiarity: slot?.familiarity ?? null,
      role: slot?.role ?? playerRoles?.[player.id] ?? null,
    };
  });
  const reserves = new Set(lineup?.reserves ?? []);
  const benchOrder = new Map((lineup?.substitutes ?? []).map((id, index) => [id, index]));
  const rank = (player: PlayerData) => benchOrder.get(player.id) ?? Number.MAX_SAFE_INTEGER;
  const bench = others.map(
    (player): TableRow => ({
      player,
      group: reserves.has(player.id) ? "reserves" : "substitutes",
      slotIndex: null,
      position: null,
      familiarity: null,
      role: null,
    }),
  );
  const substitutes = bench
    .filter((row) => row.group === "substitutes")
    .sort((left, right) => rank(left.player) - rank(right.player));
  return [...xi, ...substitutes, ...bench.filter((row) => row.group === "reserves")];
}
