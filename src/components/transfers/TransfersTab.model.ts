import type { GameStateData, PlayerData } from "../../store/gameStore";
import { canonicalPosition, normalisePosition } from "../squad/SquadTab.helpers";

// Specific positions grouped by the broad category they refine. Used both by
// the position-refinement popover in TransfersTab.tsx and as default selections
// when a group is activated.
export const SPECIFIC_POSITIONS_BY_GROUP: Record<string, string[]> = {
  Goalkeeper: ["Goalkeeper"],
  Defender: ["CenterBack", "LeftBack", "RightBack", "LeftWingBack", "RightWingBack"],
  Midfielder: [
    "DefensiveMidfielder",
    "CentralMidfielder",
    "AttackingMidfielder",
    "LeftMidfielder",
    "RightMidfielder",
  ],
  Forward: ["LeftWinger", "RightWinger", "Striker"],
};

export type TransferTabView =
  | "my_list"
  | "players"
  | "offers";

export type TransferAvailabilityFilter =
  | "all"
  | "transfer"
  | "loan"
  | "free_agent";

export interface TransferCollections {
  myTransferList: PlayerData[];
  myLoanList: PlayerData[];
  marketPlayers: PlayerData[];
  freeAgentPlayers: PlayerData[];
  loanPlayers: PlayerData[];
  availablePlayers: PlayerData[];
  playersWithOffers: PlayerData[];
}

export function uniquePlayersById(players: PlayerData[]): PlayerData[] {
  const seenPlayerIds = new Set<string>();

  return players.filter((player) => {
    if (seenPlayerIds.has(player.id)) {
      return false;
    }

    seenPlayerIds.add(player.id);
    return true;
  });
}

export function getMyListedPlayers(
  collections: TransferCollections,
): PlayerData[] {
  return uniquePlayersById([
    ...collections.myTransferList,
    ...collections.myLoanList,
  ]);
}

export function deriveTransferCollections(
  gameState: GameStateData,
  userTeamId: string | null,
): TransferCollections {
  const myTransferList = gameState.players.filter(
    (player) =>
      player.team_id === userTeamId &&
      player.transfer_listed &&
      !player.active_loan,
  );
  const myLoanList = gameState.players.filter(
    (player) =>
      player.team_id === userTeamId && player.loan_listed && !player.active_loan,
  );
  const marketPlayers = gameState.players.filter(
    (player) =>
      player.transfer_listed && player.team_id !== userTeamId && !player.active_loan,
  );
  const freeAgentPlayers = gameState.players.filter(
    (player) => player.team_id === null && !player.retired,
  );
  const loanPlayers = gameState.players.filter(
    (player) =>
      player.loan_listed && player.team_id !== userTeamId && !player.active_loan,
  );

  return {
    myTransferList,
    myLoanList,
    marketPlayers,
    freeAgentPlayers,
    loanPlayers,
    availablePlayers: uniquePlayersById([
      ...marketPlayers,
      ...loanPlayers,
      ...freeAgentPlayers,
    ]),
    playersWithOffers: gameState.players.filter(
      (player) =>
        (player.transfer_offers.length > 0 ||
          (player.loan_offers?.length ?? 0) > 0) &&
        (player.team_id === userTeamId ||
          player.transfer_offers.some(
            (offer) => offer.from_team_id === userTeamId,
          ) ||
          (player.loan_offers ?? []).some(
            (offer) => offer.from_team_id === userTeamId,
          )),
    ),
  };
}

export function getCurrentTransferList(
  view: TransferTabView,
  collections: TransferCollections,
): PlayerData[] {
  switch (view) {
    case "my_list":
      return getMyListedPlayers(collections);
    case "players":
      return collections.availablePlayers;
    case "offers":
    default:
      return collections.playersWithOffers;
  }
}

export interface TransferAffordability {
  transferBudget: number;
  finance: number;
}

// Mirrors ofm_core/src/transfers.rs: a bid `fee` exceeds the buyer's budget
// when it overruns either the transfer budget or the club's cash balance.
export function bidExceedsAffordability(
  fee: number,
  affordability: TransferAffordability,
): boolean {
  return fee > affordability.transferBudget || fee > affordability.finance;
}

export function filterTransferPlayers(
  players: PlayerData[],
  search: string,
  posFilter: string | null,
  availabilityFilter: TransferAvailabilityFilter = "all",
  affordability: TransferAffordability | null = null,
  specificPositions: readonly string[] = [],
): PlayerData[] {
  const specificSet =
    specificPositions.length > 0 ? new Set(specificPositions) : null;

  return players.filter((player) => {
    if (availabilityFilter === "transfer" && !player.transfer_listed) {
      return false;
    }

    if (availabilityFilter === "loan" && !player.loan_listed) {
      return false;
    }

    if (availabilityFilter === "free_agent" && player.team_id !== null) {
      return false;
    }

    if (
      affordability &&
      player.team_id !== null &&
      player.transfer_listed &&
      bidExceedsAffordability(player.market_value, affordability)
    ) {
      return false;
    }

    const rawPosition = player.natural_position || player.position;

    if (posFilter && normalisePosition(rawPosition) !== posFilter) {
      return false;
    }

    if (specificSet && !specificSet.has(canonicalPosition(rawPosition))) {
      return false;
    }

    if (search.length >= 2) {
      const query = search.toLowerCase();

      if (
        !player.full_name.toLowerCase().includes(query) &&
        !player.nationality.toLowerCase().includes(query)
      ) {
        return false;
      }
    }

    return true;
  });
}
