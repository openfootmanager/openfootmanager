import { invoke } from "@tauri-apps/api/core";

import type { VisibleWage } from "./scoutingService";

export type PlayerSortKey = "name" | "position" | "age" | "ovr" | "value" | "team";

export type PlayerStatusFilter = "all" | "transfer" | "loan" | "free_agent" | "available";

/** One position test: an exact position, or every position in a group. */
export type PositionFilter = { group: string } | { exact: string };

/** Which players a list is about, before any other filter narrows it. */
export type PlayerScope = "all" | "market" | "scoutable" | "user_listed" | "with_offers";

export interface PlayersPageQuery {
  search: string | null;
  position: string | null;
  /** Exact positions and group shorthands; a player matching any one passes. */
  positions: PositionFilter[];
  age_min: number | null;
  age_max: number | null;
  ovr_min: number | null;
  ovr_max: number | null;
  value_min: number | null;
  value_max: number | null;
  /** Only players the user's club can pay for out of cash and transfer budget. */
  in_budget: boolean;
  scope: PlayerScope;
  team_id: string | null;
  status: PlayerStatusFilter;
  sort_key: PlayerSortKey;
  sort_asc: boolean;
  page: number;
  page_size: number;
}

export interface PlayerSummaryMedia {
  face: string | null;
}

export interface PlayerSummary {
  id: string;
  full_name: string;
  match_name: string;
  date_of_birth: string;
  /** Whole years on the game's current date. */
  age: number;
  /** Weekly wage as the club may see it: exact (own), about the figure (scouted), or hidden. */
  wage: VisibleWage;
  nationality: string;
  position: string;
  natural_position: string;
  team_id: string | null;
  team_name: string | null;
  market_value: number;
  /** His rating when the club knows it (its own player, a division rival); null otherwise. */
  ovr: number | null;
  transfer_listed: boolean;
  loan_listed: boolean;
  injured: boolean;
  retired: boolean;
  media: PlayerSummaryMedia;
}

/** How many players each status chip would show, with every other filter as the query has it. */
export type StatusCounts = Record<PlayerStatusFilter, number>;

export interface PlayersPage {
  items: PlayerSummary[];
  status_counts: StatusCounts;
  total: number;
  page: number;
  page_size: number;
}

export function fetchPlayersPage(query: PlayersPageQuery): Promise<PlayersPage> {
  return invoke<PlayersPage>("get_players_page", { query });
}
