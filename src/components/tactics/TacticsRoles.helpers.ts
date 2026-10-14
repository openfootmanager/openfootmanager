import type { AutoSetPieces } from "../../services/squadService";
import type { PlayerData, TeamMatchRolesData } from "../../store/gameStore";

export const EMPTY_MATCH_ROLES: TeamMatchRolesData = {
  captain: null,
  vice_captain: null,
  penalty_taker: null,
  free_kick_taker: null,
  corner_taker: null,
};

/** The manager's assignment if that player is in the eleven, else the backend's pick (if he is). */
function resolveAssignedRole(
  assignedId: string | null | undefined,
  availableIds: Set<string>,
  backendId: string | null | undefined,
): string | null {
  if (assignedId && availableIds.has(assignedId)) {
    return assignedId;
  }

  return backendId && availableIds.has(backendId) ? backendId : null;
}

/**
 * The takers the match will use. Where the manager has assigned none, that is the backend's own
 * choice (`auto_select_set_pieces`), not a ranking of ours; the backend has no vice-captain rule,
 * so that slot is only ever the manager's.
 */
export function resolveEffectiveMatchRoles(
  startingPlayers: PlayerData[],
  matchRoles: TeamMatchRolesData | undefined,
  backendPicks: AutoSetPieces | null,
): TeamMatchRolesData {
  const availableIds = new Set(startingPlayers.map((player) => player.id));
  const storedRoles = matchRoles ?? EMPTY_MATCH_ROLES;

  return {
    captain: resolveAssignedRole(storedRoles.captain, availableIds, backendPicks?.captain),
    vice_captain: resolveAssignedRole(storedRoles.vice_captain, availableIds, null),
    penalty_taker: resolveAssignedRole(
      storedRoles.penalty_taker,
      availableIds,
      backendPicks?.penalty_taker,
    ),
    free_kick_taker: resolveAssignedRole(
      storedRoles.free_kick_taker,
      availableIds,
      backendPicks?.free_kick_taker,
    ),
    corner_taker: resolveAssignedRole(
      storedRoles.corner_taker,
      availableIds,
      backendPicks?.corner_taker,
    ),
  };
}

export function buildUpdatedMatchRolesForAssignment(
  effectiveRoles: TeamMatchRolesData,
  role: keyof TeamMatchRolesData,
  playerId: string,
  backendPicks: AutoSetPieces | null,
): TeamMatchRolesData {
  const nextRoles: TeamMatchRolesData = { ...effectiveRoles, [role]: playerId };

  // Nobody holds both armbands: the one displaced gives way. A vice-captain made captain is
  // cleared; a captain made vice falls back to the backend's captain unless that is him again.
  if (role === "captain" && nextRoles.vice_captain === playerId) {
    return { ...nextRoles, vice_captain: null };
  }

  if (role === "vice_captain" && nextRoles.captain === playerId) {
    const fallback = backendPicks?.captain;
    return { ...nextRoles, captain: fallback && fallback !== playerId ? fallback : null };
  }

  return nextRoles;
}
