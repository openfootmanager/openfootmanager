import { invoke } from "@tauri-apps/api/core";

export type PlayerSortKey =
  | "name"
  | "position"
  | "age"
  | "ovr"
  | "value"
  | "team";

export type PlayerStatusFilter = "all" | "transfer" | "loan";

export interface PlayersPageQuery {
  search: string | null;
  position: string | null;
  team_id: string | null;
  status: PlayerStatusFilter;
  sort_key: PlayerSortKey;
  sort_asc: boolean;
  page: number;
  page_size: number;
}

export interface PlayerSummary {
  id: string;
  full_name: string;
  match_name: string;
  date_of_birth: string;
  nationality: string;
  position: string;
  natural_position: string;
  team_id: string | null;
  team_name: string | null;
  market_value: number;
  ovr: number;
  transfer_listed: boolean;
  loan_listed: boolean;
  injured: boolean;
  retired: boolean;
}

export interface PlayersPage {
  items: PlayerSummary[];
  total: number;
  page: number;
  page_size: number;
}

export function fetchPlayersPage(query: PlayersPageQuery): Promise<PlayersPage> {
  return invoke<PlayersPage>("get_players_page", { query });
}
