import type {
  GameStateData,
  PlayerData,
  TransferOfferData,
  LoanOfferData,
} from "../../store/gameStore";

export type TransferTabView = "my_list" | "players" | "offers";

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

export function getMyListedPlayers(collections: TransferCollections): PlayerData[] {
  return uniquePlayersById([...collections.myTransferList, ...collections.myLoanList]);
}

/**
 * True when a player is committed to a move that is agreed and only awaiting
 * registration (a `PendingRegistration` transfer or loan offer). Such a player
 * is off the market — no club, including the one that agreed the deal, can open
 * a new bid — so they are hidden from the approachable lists and the bid flow
 * refuses further offers.
 */
export function playerHasPendingRegistration(player: PlayerData): boolean {
  return (
    (player.transfer_offers ?? []).some((offer) => offer.status === "PendingRegistration") ||
    (player.loan_offers ?? []).some((offer) => offer.status === "PendingRegistration")
  );
}

export function deriveTransferCollections(
  gameState: GameStateData,
  userTeamId: string | null,
): TransferCollections {
  const myTransferList = gameState.players.filter(
    (player) =>
      player.team_id === userTeamId &&
      player.transfer_listed &&
      !player.active_loan &&
      !playerHasPendingRegistration(player),
  );
  const myLoanList = gameState.players.filter(
    (player) =>
      player.team_id === userTeamId &&
      player.loan_listed &&
      !player.active_loan &&
      !playerHasPendingRegistration(player),
  );
  const marketPlayers = gameState.players.filter(
    (player) =>
      player.transfer_listed &&
      player.team_id !== userTeamId &&
      !player.active_loan &&
      !playerHasPendingRegistration(player),
  );
  const freeAgentPlayers = gameState.players.filter(
    (player) => player.team_id === null && !player.retired && !playerHasPendingRegistration(player),
  );
  const loanPlayers = gameState.players.filter(
    (player) =>
      player.loan_listed &&
      player.team_id !== userTeamId &&
      !player.active_loan &&
      !playerHasPendingRegistration(player),
  );

  return {
    myTransferList,
    myLoanList,
    marketPlayers,
    freeAgentPlayers,
    loanPlayers,
    availablePlayers: uniquePlayersById([...marketPlayers, ...loanPlayers, ...freeAgentPlayers]),
    playersWithOffers: gameState.players.filter(
      (player) =>
        getRelevantTransferOffers(player, userTeamId).length > 0 ||
        getRelevantLoanOffers(player, userTeamId).length > 0,
    ),
  };
}

/** Saved offer records involving the current club; historical parent IDs keep loans reachable after registration. */
export function getRelevantTransferOffers(
  player: PlayerData,
  userTeamId: string | null,
): TransferOfferData[] {
  if (!userTeamId) return [];
  return (player.transfer_offers ?? []).filter(
    (offer) => player.team_id === userTeamId || offer.from_team_id === userTeamId,
  );
}

export function getRelevantLoanOffers(
  player: PlayerData,
  userTeamId: string | null,
): LoanOfferData[] {
  if (!userTeamId) return [];
  return (player.loan_offers ?? []).filter(
    (offer) => offer.parent_team_id === userTeamId || offer.from_team_id === userTeamId,
  );
}

/** Availability presentation uses saved facts; backend validation remains authoritative when a deal is submitted. */
export function getPlayerDealBlocker(player: PlayerData, userTeamId: string | null): string | null {
  if (player.retired) return "transfers.dealRetired";
  if (playerHasPendingRegistration(player)) return "transfers.dealAlreadyAgreed";
  if (player.active_loan) return "transfers.dealActiveLoan";
  if (player.team_id !== null && player.team_id === userTeamId) return "transfers.dealOwnPlayer";
  return null;
}
