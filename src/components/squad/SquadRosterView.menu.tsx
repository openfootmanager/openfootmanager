import type { TFunction } from "i18next";
import { AlertTriangle, Repeat, RotateCcw, TimerOff, Trash2, Users } from "lucide-react";

import type { ContractRiskLevel } from "../../lib/contractUtils";
import { resolveInjuryName } from "../../lib/injury";
import { canDelegateToYouthAcademy, isMatchDayEligible } from "../../lib/playerSquad";
import { setPlayerSquadRole } from "../../services/squadService";
import { toggleLoanList, toggleTransferList } from "../../services/transfersService";
import type { GameStateData, PlayerData, PlayerSelectionOptions } from "../../store/gameStore";
import type { ContextMenuItem } from "../ContextMenu";
import {
  buildDelegateToYouthAcademyMenuItem,
  buildDividerMenuItem,
  buildToggleLoanListMenuItem,
  buildToggleTransferListMenuItem,
  buildViewProfileMenuItem,
} from "../playerActions/playerContextMenuItems";

interface SquadRowMenuContext {
  t: TFunction;
  inStartingXi: boolean;
  /** Someone fit is on the bench to take a starter's place. */
  canBench: boolean;
  contractRisk: ContractRiskLevel;
  /** A contract action for this player is on its way to the backend. */
  contractActionPending: boolean;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onMutationComplete?: (game: GameStateData) => void;
  updateSquadPlanning: (playerId: string, action: "promote" | "demote") => Promise<void>;
  updateContractExitIntent: (playerId: string, shouldLetExpire: boolean) => Promise<void>;
}

/** What a roster row's menu offers: the injury, the profile, the XI, the contract, the market. */
export function squadRowMenuItems(
  player: PlayerData,
  context: SquadRowMenuContext,
): ContextMenuItem[] {
  const { t, onSelectPlayer, onMutationComplete } = context;
  const hasLetExpireIntent = player.morale_core?.renewal_state?.exit_intent?.kind === "let_expire";

  return [
    ...(player.injury
      ? [
          {
            type: "label" as const,
            label: `${resolveInjuryName(player.injury.name, t)} — ${t("playerProfile.injuryDaysShort", { count: player.injury.days_remaining })}`,
            icon: <AlertTriangle className="w-3.5 h-3.5" />,
          },
          buildDividerMenuItem(),
        ]
      : []),
    buildViewProfileMenuItem(t, () => onSelectPlayer(player.id)),
    context.inStartingXi
      ? {
          label: t("squad.sendToBench"),
          icon: <RotateCcw className="w-4 h-4" />,
          disabled: !context.canBench,
          onClick: () => {
            void context.updateSquadPlanning(player.id, "demote");
          },
        }
      : {
          label: t("squad.makeStarter"),
          icon: <Users className="w-4 h-4" />,
          disabled: !isMatchDayEligible(player),
          onClick: () => {
            void context.updateSquadPlanning(player.id, "promote");
          },
        },
    buildDividerMenuItem(),
    {
      label: t("common.renewContract"),
      icon: <Repeat className="w-4 h-4" />,
      urgent: context.contractRisk !== "stable",
      disabled: !player.contract_end,
      onClick: () => onSelectPlayer(player.id, { openRenewal: true }),
    },
    hasLetExpireIntent
      ? {
          label: t("playerProfile.reopenContractTalks"),
          icon: <RotateCcw className="w-4 h-4" />,
          disabled: !player.contract_end || context.contractActionPending,
          onClick: () => {
            void context.updateContractExitIntent(player.id, false);
          },
        }
      : {
          label: t("playerProfile.letContractExpire"),
          icon: <TimerOff className="w-4 h-4" />,
          disabled: !player.contract_end || context.contractActionPending,
          onClick: () => {
            void context.updateContractExitIntent(player.id, true);
          },
        },
    {
      label: t("playerProfile.terminateContract"),
      icon: <Trash2 className="w-4 h-4" />,
      danger: true,
      disabled: !player.contract_end,
      onClick: () => onSelectPlayer(player.id, { openTermination: true }),
    },
    buildDividerMenuItem(),
    buildToggleTransferListMenuItem(t, player.transfer_listed, async () => {
      try {
        const updated = await toggleTransferList(player.id);
        onMutationComplete?.(updated);
      } catch {
        return;
      }
    }),
    buildToggleLoanListMenuItem(t, player.loan_listed, async () => {
      try {
        const updated = await toggleLoanList(player.id);
        onMutationComplete?.(updated);
      } catch {
        return;
      }
    }),
    ...(canDelegateToYouthAcademy(player)
      ? [
          buildDelegateToYouthAcademyMenuItem(t, async () => {
            try {
              const updated = await setPlayerSquadRole(player.id, "Youth");
              onMutationComplete?.(updated);
            } catch {
              return;
            }
          }),
        ]
      : []),
  ];
}
