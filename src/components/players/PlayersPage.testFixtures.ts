import type { PlayersPage } from "../../services/playersService";
import type { PlayerData } from "../../store/gameStore";

/**
 * The page the backend would answer `get_players_page` with, for a test that says exactly which
 * players the market shows. It does not filter: who belongs on the market is the backend's rule,
 * tested once in Rust, and a test here only decides what the screen does with the answer.
 */
export function marketPageOf(
  players: PlayerData[],
  { total = players.length, page = 1 }: { total?: number; page?: number } = {},
): PlayersPage {
  return {
    items: players.map((player) => ({
      id: player.id,
      full_name: player.full_name,
      match_name: player.match_name,
      date_of_birth: player.date_of_birth,
      age: 26,
      wage: player.wage,
      nationality: player.nationality,
      position: player.position,
      natural_position: player.natural_position ?? player.position,
      team_id: player.team_id,
      team_name: null,
      market_value: player.market_value,
      ovr: 70,
      transfer_listed: player.transfer_listed,
      loan_listed: player.loan_listed,
      injured: false,
      retired: false,
      media: { face: null },
    })),
    total,
    page,
    page_size: 30,
  };
}
