import type { FamiliarityTable } from "../hooks/useFamiliarity";
import { canonicalPosition } from "../lib/positions";
import type { PlayerData } from "../store/gameStore";
import type { Familiarity } from "../store/generated/Familiarity";
import { cannedPositionTable, CANNED_POSITION_ENTRIES } from "./positionTable";

/**
 * Stands in for the backend's familiarity in component tests, which mock `invoke` and so never
 * reach Rust. It grades a player as `player_rating::familiarity` does (his natural position,
 * an alternate, a position in his own line, or another line), and `familiarityParity.test.ts` pins
 * it to the backend's own answer for every position, so a screen test cannot drift from the rule.
 * The rule itself is tested in Rust (`ofm_core::player_rating`); the hook that reads the real
 * answer in `useFamiliarity.test.ts`.
 */

/** A legacy bucket is graded as the granular position the backend reads it as. */
const BUCKET_AS_GRANULAR: Record<string, string> = {
  Defender: "CenterBack",
  Midfielder: "CentralMidfielder",
  Forward: "Striker",
};

/** The positions `Position::is_legacy_bucket` names: the goalkeeper is among them, on purpose. */
const LEGACY_BUCKETS = new Set(["Goalkeeper", ...Object.keys(BUCKET_AS_GRANULAR)]);

/**
 * A winger and the wide midfielder on his flank step between each other as within a line, though
 * they are in different groups: `position_step`'s `is_flank_partner`, keyed "primary:slot".
 */
const FLANK_PARTNERS: ReadonlySet<string> = new Set([
  "RightWinger:RightMidfielder",
  "RightMidfielder:RightWinger",
  "LeftWinger:LeftMidfielder",
  "LeftMidfielder:LeftWinger",
]);

const POSITION_NAMES = new Set(CANNED_POSITION_ENTRIES.map((entry) => entry.position));

function graded(position: string): string {
  const canonical = canonicalPosition(position);
  return BUCKET_AS_GRANULAR[canonical] ?? canonical;
}

/** `player_rating::primary_position`: the natural position, or `position` while that is a bucket. */
function primaryOf(player: PlayerData): string {
  const natural = canonicalPosition(player.natural_position || "");
  const naturalIsBucket = natural === "" || LEGACY_BUCKETS.has(natural);
  return graded(naturalIsBucket ? player.position : natural);
}

/** One player's familiarity at one position, as the backend would answer. */
export function cannedGrade(player: PlayerData, position: string): Familiarity {
  const slot = graded(position);
  const primary = primaryOf(player);
  const table = cannedPositionTable();
  if (slot === primary) return "natural";
  if ((player.alternate_positions ?? []).some((alternate) => graded(alternate) === slot)) {
    return "can_play";
  }
  if (FLANK_PARTNERS.has(`${primary}:${slot}`)) return "can_play";
  return table.groupOf(primary) === table.groupOf(slot) ? "can_play" : "out_of_position";
}

/** A loaded table that grades whichever player `find` returns, as the backend does. */
function gradingTable(find: (playerId: string) => PlayerData | undefined): FamiliarityTable {
  return {
    loaded: true,
    familiarityOf: (playerId, position) => {
      const player = find(playerId);
      return player && POSITION_NAMES.has(position) ? cannedGrade(player, position) : null;
    },
  };
}

/** The table a screen test hands the helpers that take one: loaded, graded as the backend does. */
export function cannedFamiliarityTable(players: PlayerData[]): FamiliarityTable {
  const byId = new Map(players.map((player) => [player.id, player]));
  return gradingTable((playerId) => byId.get(playerId));
}

/** A table that has not arrived, for a test of what a screen does before it does. */
export function pendingFamiliarityTable(): FamiliarityTable {
  return { loaded: false, familiarityOf: () => null };
}

/**
 * The module a test passes to `vi.mock("…/hooks/useFamiliarity", …)`. `find` is read when a
 * screen asks about a player, so a test registers the squad it renders however it builds it; one
 * table serves every render, and `arrived` lets a test say the answer has not come yet.
 */
export function cannedFamiliarityModule(
  find: (playerId: string) => PlayerData | undefined,
  arrived: () => boolean = () => true,
): {
  useFamiliarity: () => FamiliarityTable;
  useFamiliarityRevision: () => string;
  prefetchFamiliarity: () => () => void;
  resetFamiliarityCache: () => void;
} {
  const table = gradingTable(find);
  const pending = pendingFamiliarityTable();
  return {
    useFamiliarity: () => (arrived() ? table : pending),
    useFamiliarityRevision: () => "canned",
    prefetchFamiliarity: () => () => {},
    resetFamiliarityCache: () => {},
  };
}
