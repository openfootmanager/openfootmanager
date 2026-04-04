import type { PlayerData, TeamMatchRolesData } from "../../store/types";
import {
    getSetPieceStats,
    roleAllowsGoalkeeper,
} from "../match/setPieceUtils";

type SelectorRole = "captain" | "vicecaptain" | "penalty" | "freekick" | "corner";

const ROLE_KEY_TO_SELECTOR_ROLE: Record<keyof TeamMatchRolesData, SelectorRole> = {
    captain: "captain",
    vice_captain: "vicecaptain",
    penalty_taker: "penalty",
    free_kick_taker: "freekick",
    corner_taker: "corner",
};

export const EMPTY_MATCH_ROLES: TeamMatchRolesData = {
    captain: null,
    vice_captain: null,
    penalty_taker: null,
    free_kick_taker: null,
    corner_taker: null,
};

export function pickBestRoleCandidate(
    players: PlayerData[],
    role: SelectorRole,
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

export function resolveAssignedRole(
    assignedId: string | null | undefined,
    availableIds: Set<string>,
    fallbackId: string | null,
): string | null {
    if (assignedId && availableIds.has(assignedId)) {
        return assignedId;
    }

    return fallbackId;
}

export function getEffectiveMatchRoles(
    startingPlayers: PlayerData[],
    matchRoles?: TeamMatchRolesData,
): TeamMatchRolesData {
    const availableIds = new Set(startingPlayers.map((player) => player.id));
    const storedRoles = matchRoles ?? EMPTY_MATCH_ROLES;
    const captain = resolveAssignedRole(
        storedRoles.captain,
        availableIds,
        pickBestRoleCandidate(startingPlayers, "captain"),
    );
    const viceCaptain = resolveAssignedRole(
        storedRoles.vice_captain,
        availableIds,
        pickBestRoleCandidate(
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
            pickBestRoleCandidate(startingPlayers, "penalty"),
        ),
        free_kick_taker: resolveAssignedRole(
            storedRoles.free_kick_taker,
            availableIds,
            pickBestRoleCandidate(startingPlayers, "freekick"),
        ),
        corner_taker: resolveAssignedRole(
            storedRoles.corner_taker,
            availableIds,
            pickBestRoleCandidate(startingPlayers, "corner"),
        ),
    };
}

export function buildUpdatedMatchRoles(
    startingPlayers: PlayerData[],
    effectiveRoles: TeamMatchRolesData,
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
            vice_captain: pickBestRoleCandidate(
                startingPlayers,
                "vicecaptain",
                [playerId],
            ),
        };
    }

    if (role === "vice_captain" && nextRoles.captain === playerId) {
        nextRoles = {
            ...nextRoles,
            captain: pickBestRoleCandidate(startingPlayers, "captain", [playerId]),
        };
    }

    return nextRoles;
}

export function getSelectorRolePlayers(players: PlayerData[]): {
    id: string;
    name: string;
    position: string;
}[] {
    return players.map((player) => ({
        id: player.id,
        name: player.match_name,
        position: player.position,
    }));
}