import type { GameStateData, PlayerData } from "../../store/gameStore";

export interface PlayerProfileRelationship {
  /** The manager's club, or null when they are between jobs. */
  managerTeamId: string | null;
  /** Whose contract this is — the parent club while the player is out on loan. */
  contractOwnerTeamId: string | null;
  isContractOwnerClub: boolean;
  /** May the manager renew, release, or let this contract expire? */
  isManagerOwnedProfile: boolean;
  /** The player is on loan *at* the manager's club. */
  isManagerLoanClub: boolean;
  /** The player turns out for the manager, however they got there. */
  isManagerSquadProfile: boolean;
  isFreeAgent: boolean;
  hasLetExpireIntent: boolean;
  hasAssistantManager: boolean;
}

/**
 * Work out what this player is *to this manager*, which decides which actions
 * the profile offers.
 *
 * The distinction that is easy to get wrong: `isOwnClub` is not the same as
 * "the manager can act on this contract". A player at the manager's own club is
 * theirs to renew even when the caller passes `isOwnClub: false` — the club id
 * settles it. And a player loaned *in* turns out for the manager every week
 * without their contract being the manager's to touch, which is why
 * `isManagerSquadProfile` and `isManagerOwnedProfile` are separate answers.
 */
export function buildPlayerProfileRelationship(
  player: PlayerData,
  gameState: GameStateData,
  isOwnClub: boolean,
): PlayerProfileRelationship {
  const managerTeamId = gameState.manager.team_id;
  const contractOwnerTeamId =
    player.active_loan?.parent_team_id ?? player.team_id ?? null;
  const isContractOwnerClub = Boolean(
    managerTeamId && contractOwnerTeamId === managerTeamId,
  );

  // A loaned player's contract belongs to the parent club and nobody else, so
  // the caller's `isOwnClub` cannot widen it.
  const isManagerOwnedProfile = player.active_loan
    ? isContractOwnerClub
    : isOwnClub || isContractOwnerClub;

  const isManagerLoanClub = Boolean(
    managerTeamId && player.active_loan?.loan_team_id === managerTeamId,
  );

  return {
    managerTeamId,
    contractOwnerTeamId,
    isContractOwnerClub,
    isManagerOwnedProfile,
    isManagerLoanClub,
    isManagerSquadProfile:
      isManagerOwnedProfile || isOwnClub || isManagerLoanClub,
    isFreeAgent: player.team_id === null && !player.retired,
    hasLetExpireIntent:
      player.morale_core?.renewal_state?.exit_intent?.kind === "let_expire",
    hasAssistantManager: managerTeamId
      ? gameState.staff.some(
          (staff) =>
            staff.team_id === managerTeamId &&
            staff.role === "AssistantManager",
        )
      : false,
  };
}
