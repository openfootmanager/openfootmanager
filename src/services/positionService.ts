import { invoke } from "@tauri-apps/api/core";

import type { FamiliarityProjection } from "../store/generated/FamiliarityProjection";
import type { PositionRatings } from "../store/generated/PositionRatings";

/** A position, the group (Goalkeeper, Defender, Midfielder or Forward) it belongs to and its sort rank. */
export interface PositionGroupEntry {
  position: string;
  group: string;
  /** Where the position sorts in a squad list (lower first). */
  rank: number;
}

/** Every position with its group: the backend's one rule, no game needed. */
export async function getPositionGroups(): Promise<PositionGroupEntry[]> {
  return invoke<PositionGroupEntry[]>("get_position_groups");
}

/**
 * What one player is worth at each position he plays, as the club may see it
 * (`ofm_core::position_ratings`); null when it knows nothing of his attributes.
 */
export async function getPositionRatings(playerId: string): Promise<PositionRatings | null> {
  return invoke<PositionRatings | null>("get_position_ratings", { playerId });
}

/**
 * How each of the manager's club's players stands at every position, in the backend's three words
 * (`ofm_core::familiarity_projection`): the one rule the squad and tactics screens show.
 */
export async function getFamiliarity(): Promise<FamiliarityProjection> {
  return invoke<FamiliarityProjection>("get_familiarity");
}
