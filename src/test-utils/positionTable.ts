import { createPositionTable, type PositionEntry, type PositionTable } from "../lib/positionTable";

/**
 * Stands in for the backend's position table in component tests, which mock `invoke` and so never
 * reach Rust. It carries the groups and sort ranks of `Position::to_group_position` and
 * `Position::sort_rank`; the rules are tested in Rust (`domain` and `ofm_core::position_group`), and
 * the hook that reads the real table in `usePositionGroups.test.ts`.
 */
export const CANNED_POSITION_ENTRIES: PositionEntry[] = [
  { position: "Goalkeeper", group: "Goalkeeper", rank: 10 },
  { position: "Defender", group: "Defender", rank: 20 },
  { position: "CenterBack", group: "Defender", rank: 21 },
  { position: "LeftBack", group: "Defender", rank: 22 },
  { position: "RightBack", group: "Defender", rank: 23 },
  { position: "LeftWingBack", group: "Defender", rank: 24 },
  { position: "RightWingBack", group: "Defender", rank: 25 },
  { position: "Midfielder", group: "Midfielder", rank: 30 },
  { position: "DefensiveMidfielder", group: "Midfielder", rank: 31 },
  { position: "CentralMidfielder", group: "Midfielder", rank: 32 },
  { position: "AttackingMidfielder", group: "Midfielder", rank: 33 },
  { position: "LeftMidfielder", group: "Midfielder", rank: 34 },
  { position: "RightMidfielder", group: "Midfielder", rank: 35 },
  { position: "Forward", group: "Forward", rank: 40 },
  { position: "LeftWinger", group: "Forward", rank: 41 },
  { position: "RightWinger", group: "Forward", rank: 42 },
  { position: "Striker", group: "Forward", rank: 43 },
];

/** The table a component test hands the pure helpers that take one. */
export function cannedPositionTable(): PositionTable {
  return createPositionTable(CANNED_POSITION_ENTRIES);
}

/** The module a test passes to `vi.mock("…/hooks/usePositionGroups", …)`. */
export function cannedPositionsModule(): {
  usePositionGroups: () => (position: string) => string | null;
  usePositionTable: () => PositionTable;
  resetPositionGroupsCache: () => void;
} {
  const table = cannedPositionTable();
  const groups = new Map(CANNED_POSITION_ENTRIES.map((entry) => [entry.position, entry.group]));
  return {
    usePositionGroups: () => (position) => groups.get(position) ?? null,
    usePositionTable: () => table,
    resetPositionGroupsCache: () => {},
  };
}
