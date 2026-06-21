import type { PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import { getSetPieceStats } from "../match/SetPieceSelector";

export const EMPTY_MATCH_ROLES: TeamMatchRolesData = {
  captain: null,
  vice_captain: null,
  penalty_taker: null,
  free_kick_taker: null,
  corner_taker: null,
};

function roleAllowsGoalkeeper(role: string): boolean {
  return role === "captain" || role === "vicecaptain";
}

function pickBestCandidate(
  players: PlayerData[],
  role: string,
  excludedIds: string[] = [],
): string | null {
  const excludedIdSet = new Set(excludedIds);
  const candidates = players
    .filter((player) => {
      if (excludedIdSet.has(player.id)) {
        return false;
      }

      if (roleAllowsGoalkeeper(role)) {
        return true;
      }

      return player.position !== "Goalkeeper";
    })
    .sort((leftPlayer, rightPlayer) => {
      return (
        getSetPieceStats(role, rightPlayer).score -
          getSetPieceStats(role, leftPlayer).score ||
        leftPlayer.full_name.localeCompare(rightPlayer.full_name)
      );
    });

  return candidates[0]?.id ?? null;
}

function resolveAssignedRole(
  assignedId: string | null | undefined,
  availableIds: Set<string>,
  fallbackId: string | null,
): string | null {
  if (assignedId && availableIds.has(assignedId)) {
    return assignedId;
  }

  return fallbackId;
}

export function resolveEffectiveMatchRoles(
  startingPlayers: PlayerData[],
  matchRoles?: TeamMatchRolesData,
): TeamMatchRolesData {
  const availableIds = new Set(startingPlayers.map((player) => player.id));
  const storedRoles = matchRoles ?? EMPTY_MATCH_ROLES;
  const captain = resolveAssignedRole(
    storedRoles.captain,
    availableIds,
    pickBestCandidate(startingPlayers, "captain"),
  );
  const viceCaptain = resolveAssignedRole(
    storedRoles.vice_captain,
    availableIds,
    pickBestCandidate(
      startingPlayers,
      "vicecaptain",
      captain ? [captain] : [],
    ),
  );

  return {
    captain,
    vice_captain: viceCaptain,
    penalty_taker: resolveAssignedRole(
      storedRoles.penalty_taker,
      availableIds,
      pickBestCandidate(startingPlayers, "penalty"),
    ),
    free_kick_taker: resolveAssignedRole(
      storedRoles.free_kick_taker,
      availableIds,
      pickBestCandidate(startingPlayers, "freekick"),
    ),
    corner_taker: resolveAssignedRole(
      storedRoles.corner_taker,
      availableIds,
      pickBestCandidate(startingPlayers, "corner"),
    ),
  };
}

export function buildUpdatedMatchRolesForAssignment(
  effectiveRoles: TeamMatchRolesData,
  startingPlayers: PlayerData[],
  role: keyof TeamMatchRolesData,
  playerId: string,
): TeamMatchRolesData {
  let nextRoles: TeamMatchRolesData = {
    ...effectiveRoles,
    [role]: playerId,
  };

  if (role === "captain" && nextRoles.vice_captain === playerId) {
    nextRoles = {
      ...nextRoles,
      vice_captain: pickBestCandidate(
        startingPlayers,
        "vicecaptain",
        playerId ? [playerId] : [],
      ),
    };
  }

  if (role === "vice_captain" && nextRoles.captain === playerId) {
    nextRoles = {
      ...nextRoles,
      captain: pickBestCandidate(
        startingPlayers,
        "captain",
        playerId ? [playerId] : [],
      ),
    };
  }

  return nextRoles;
}

export { pickBestCandidate };
