import { useRef, useState } from "react";
import { ChevronDown, Repeat, RotateCcw, TimerOff, Trash2, Users } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { GameStateData, PlayerData } from "../../store/gameStore";
import type { ContractRiskLevel } from "../../lib/helpers";
import ContextMenu, { type ContextMenuHandle, type ContextMenuItem } from "../ContextMenu";
import {
    buildDelegateToYouthAcademyMenuItem,
    buildDividerMenuItem,
    buildMakeTransferBidMenuItem,
    buildOfferFreeAgentContractMenuItem,
    buildToggleLoanListMenuItem,
    buildToggleTransferListMenuItem,
} from "../playerActions/playerContextMenuItems";
import {
    buildDemoteFromStartingXi,
    buildPromoteToStartingXi,
    buildStartingXIIds,
} from "../squad/SquadTab.helpers";
import { canDelegateToYouthAcademy } from "../../lib/playerSquad";
import { setPlayerSquadRole, setStartingXi } from "../../services/squadService";
import { toggleLoanList, toggleTransferList } from "../../services/transfersService";
import { resolveTranslatedErrorMessage } from "../../utils/errorMessage";
import { Button } from "../ui";

interface PlayerProfileActionsMenuProps {
    player: PlayerData;
    gameState: GameStateData;
    isManagerOwnedProfile: boolean;
    isFreeAgent: boolean;
    hasLetExpireIntent: boolean;
    contractRiskLevel: ContractRiskLevel;
    actionSubmitting: boolean;
    onGameUpdate: (game: GameStateData) => void;
    onOpenRenewal: () => void;
    onMarkLetExpire: () => void;
    onClearLetExpire: () => void;
    onOpenTermination: () => void;
    onOpenBid: () => void;
    onOpenFreeAgentContract: () => void;
    onError: (message: string) => void;
}

/**
 * "Actions" dropdown on the player profile. Reuses the shared player
 * context-menu builders so the profile offers the same management
 * actions as the squad roster (contract, market listing, squad
 * planning, youth delegation) and the transfer surfaces (bid, free
 * agent offer) without leaving the page.
 */
export default function PlayerProfileActionsMenu({
    player,
    gameState,
    isManagerOwnedProfile,
    isFreeAgent,
    hasLetExpireIntent,
    contractRiskLevel,
    actionSubmitting,
    onGameUpdate,
    onOpenRenewal,
    onMarkLetExpire,
    onClearLetExpire,
    onOpenTermination,
    onOpenBid,
    onOpenFreeAgentContract,
    onError,
}: PlayerProfileActionsMenuProps) {
    const { t } = useTranslation();
    const menuRef = useRef<ContextMenuHandle>(null);
    const [mutationPending, setMutationPending] = useState(false);

    // No management action applies to a retired player, regardless of
    // which club last held their contract.
    if (player.retired) {
        return null;
    }

    const managerTeamId = gameState.manager.team_id;
    const managerTeam = gameState.teams.find((team) => team.id === managerTeamId);
    const actionBusy = mutationPending || actionSubmitting;

    const runMutation = async (mutation: () => Promise<GameStateData>): Promise<void> => {
        setMutationPending(true);
        try {
            onGameUpdate(await mutation());
        } catch (error) {
            onError(resolveTranslatedErrorMessage(error, t));
        } finally {
            setMutationPending(false);
        }
    };

    const items: ContextMenuItem[] = [];

    // Squad planning only applies while the player is actually in the
    // manager's squad (not loaned out elsewhere).
    const isInManagerSquad =
        isManagerOwnedProfile && managerTeam && player.team_id === managerTeamId;

    if (isInManagerSquad) {
        const roster = gameState.players.filter(
            (candidate) => candidate.team_id === managerTeamId,
        );
        const available = roster.filter((candidate) => !candidate.injury);
        const formation = managerTeam.formation || "4-4-2";
        const startingXiIds = buildStartingXIIds(
            available,
            managerTeam.starting_xi_ids ?? [],
            formation,
        );
        const inXI = startingXiIds.includes(player.id);
        const playersById = new Map(available.map((candidate) => [candidate.id, candidate]));

        const persistStartingXi = (nextXiIds: string[] | null): void => {
            if (!nextXiIds || nextXiIds.join(",") === startingXiIds.join(",")) {
                return;
            }
            void runMutation(() => setStartingXi(nextXiIds));
        };

        items.push(
            inXI
                ? {
                      label: t("squad.sendToBench"),
                      icon: <RotateCcw className="w-4 h-4" />,
                      disabled:
                          actionBusy ||
                          available.filter((candidate) => !startingXiIds.includes(candidate.id))
                              .length === 0,
                      onClick: () =>
                          persistStartingXi(
                              buildDemoteFromStartingXi(
                                  startingXiIds,
                                  available,
                                  formation,
                                  player.id,
                              ),
                          ),
                  }
                : {
                      label: t("squad.makeStarter"),
                      icon: <Users className="w-4 h-4" />,
                      disabled: actionBusy || Boolean(player.injury),
                      onClick: () =>
                          persistStartingXi(
                              buildPromoteToStartingXi(
                                  startingXiIds,
                                  playersById,
                                  formation,
                                  player.id,
                              ),
                          ),
                  },
            buildDividerMenuItem(),
        );
    }

    if (isManagerOwnedProfile) {
        items.push(
            {
                label: t("common.renewContract"),
                icon: <Repeat className="w-4 h-4" />,
                urgent: contractRiskLevel !== "stable",
                disabled: !player.contract_end,
                onClick: onOpenRenewal,
            },
            hasLetExpireIntent
                ? {
                      label: t("playerProfile.reopenContractTalks"),
                      icon: <RotateCcw className="w-4 h-4" />,
                      disabled: !player.contract_end || actionSubmitting,
                      onClick: onClearLetExpire,
                  }
                : {
                      label: t("playerProfile.letContractExpire"),
                      icon: <TimerOff className="w-4 h-4" />,
                      disabled: !player.contract_end || actionSubmitting,
                      onClick: onMarkLetExpire,
                  },
            {
                label: t("playerProfile.terminateContract"),
                icon: <Trash2 className="w-4 h-4" />,
                danger: true,
                disabled: !player.contract_end,
                onClick: onOpenTermination,
            },
            buildDividerMenuItem(),
            {
                ...buildToggleTransferListMenuItem(t, player.transfer_listed, () => {
                    void runMutation(() => toggleTransferList(player.id));
                }),
                disabled: actionBusy,
            },
            {
                ...buildToggleLoanListMenuItem(t, player.loan_listed, () => {
                    void runMutation(() => toggleLoanList(player.id));
                }),
                disabled: actionBusy,
            },
        );

        if (canDelegateToYouthAcademy(player)) {
            items.push({
                ...buildDelegateToYouthAcademyMenuItem(t, () => {
                    void runMutation(() => setPlayerSquadRole(player.id, "Youth"));
                }),
                disabled: actionBusy,
            });
        }
    } else if (managerTeamId) {
        if (isFreeAgent) {
            items.push(buildOfferFreeAgentContractMenuItem(t, onOpenFreeAgentContract));
        } else {
            items.push(buildMakeTransferBidMenuItem(t, onOpenBid));
        }
    }

    if (items.length === 0) {
        return null;
    }

    return (
        <ContextMenu ref={menuRef} items={items}>
            <Button
                size="sm"
                variant="outline"
                onClick={(event) => {
                    const rect = event.currentTarget.getBoundingClientRect();
                    menuRef.current?.open(rect.left, rect.bottom + 4);
                }}
            >
                {t("common.actions")}
                <ChevronDown className="w-4 h-4 ml-1" />
            </Button>
        </ContextMenu>
    );
}
