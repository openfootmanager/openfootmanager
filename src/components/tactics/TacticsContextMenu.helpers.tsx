import {
  ArrowDown,
  ArrowRight,
  ArrowUp,
  Award,
  CircleDot,
  CornerDownRight,
  Crown,
  Footprints,
  ShieldAlert,
  Shuffle,
  UserRound,
} from "lucide-react";
import type { TFunction } from "i18next";

import type { ContextMenuItem } from "../ContextMenu";
import { buildDividerMenuItem } from "../playerActions/playerContextMenuItems";
import type { PlayerData, TeamMatchRolesData } from "../../store/gameStore";
import type { SquadSection } from "../squad/SquadTab.helpers";

interface TacticsContextMenuCallbacks {
  onAssignBestFit?: (playerId: string) => void;
  onAssignMatchRole?: (
    role: keyof TeamMatchRolesData,
    playerId: string,
  ) => void;
  onClearSelection?: () => void;
  onDemoteStarter?: (playerId: string) => void;
  onOpenProfile: (playerId: string) => void;
  onPromoteBench?: (playerId: string) => void;
  onTacticalSelect?: (playerId: string, section: SquadSection) => void;
}

interface BuildTacticsPlayerContextMenuItemsOptions
  extends TacticsContextMenuCallbacks {
  isSelected: boolean;
  matchRoles?: TeamMatchRolesData;
  player: PlayerData;
  section: SquadSection;
  selectedPlayerId: string | null;
  t: TFunction;
}

export function buildTacticsPlayerContextMenuItems({
  isSelected,
  matchRoles,
  onAssignBestFit,
  onAssignMatchRole,
  onClearSelection,
  onDemoteStarter,
  onOpenProfile,
  onPromoteBench,
  onTacticalSelect,
  player,
  section,
  selectedPlayerId,
  t,
}: BuildTacticsPlayerContextMenuItemsOptions): ContextMenuItem[] {
  const items: ContextMenuItem[] = [];
  const isUnavailableBenchPlayer = section === "bench" && Boolean(player.injury);
  const canPromoteBenchPlayer = section === "bench" && !player.injury;

  if (isSelected && onClearSelection) {
    items.push({
      label: t("tactics.clearSelection"),
      icon: <ShieldAlert className="h-4 w-4" />,
      onClick: onClearSelection,
    });
  } else if (onTacticalSelect && !isUnavailableBenchPlayer) {
    items.push({
      label: selectedPlayerId
        ? t("tactics.compareWithSelected")
        : t("tactics.selectForSwap"),
      icon: <Shuffle className="h-4 w-4" />,
      onClick: () => onTacticalSelect(player.id, section),
    });
  }

  if (section === "xi" && onAssignBestFit) {
    items.push(buildDividerMenuItem());
    items.push({
      label: t("tactics.assignBestFit"),
      icon: <ArrowRight className="h-4 w-4" />,
      onClick: () => onAssignBestFit(player.id),
    });
  }

  if (section === "xi" && onDemoteStarter) {
    items.push({
      label: t("tactics.moveToBench"),
      icon: <ArrowDown className="h-4 w-4" />,
      onClick: () => onDemoteStarter(player.id),
    });
  }

  if (canPromoteBenchPlayer && onPromoteBench) {
    items.push(buildDividerMenuItem());
    items.push({
      label: t("tactics.promoteToLineup"),
      icon: <ArrowUp className="h-4 w-4" />,
      onClick: () => onPromoteBench(player.id),
    });
  }

  if (section === "xi" && onAssignMatchRole && matchRoles) {
    const roleItems: ContextMenuItem[] = [];

    if (matchRoles.captain !== player.id) {
      roleItems.push({
        label: t("tactics.makeCaptain"),
        icon: <Crown className="h-4 w-4" />,
        onClick: () => onAssignMatchRole("captain", player.id),
      });
    }

    if (matchRoles.vice_captain !== player.id) {
      roleItems.push({
        label: t("tactics.makeViceCaptain"),
        icon: <Award className="h-4 w-4" />,
        onClick: () => onAssignMatchRole("vice_captain", player.id),
      });
    }

    if (matchRoles.penalty_taker !== player.id) {
      roleItems.push({
        label: t("tactics.setPenaltyTaker"),
        icon: <CircleDot className="h-4 w-4" />,
        onClick: () => onAssignMatchRole("penalty_taker", player.id),
      });
    }

    if (matchRoles.free_kick_taker !== player.id) {
      roleItems.push({
        label: t("tactics.setFreeKickTaker"),
        icon: <Footprints className="h-4 w-4" />,
        onClick: () => onAssignMatchRole("free_kick_taker", player.id),
      });
    }

    if (matchRoles.corner_taker !== player.id) {
      roleItems.push({
        label: t("tactics.setCornerTaker"),
        icon: <CornerDownRight className="h-4 w-4" />,
        onClick: () => onAssignMatchRole("corner_taker", player.id),
      });
    }

    if (roleItems.length > 0) {
      items.push(buildDividerMenuItem());
      items.push(...roleItems);
    }
  }

  items.push(buildDividerMenuItem());
  items.push({
    label: t("squad.viewProfile"),
    icon: <UserRound className="h-4 w-4" />,
    onClick: () => onOpenProfile(player.id),
  });

  return items;
}
