import type { TFunction } from "i18next";
import { Eye, UserMinus, UserPlus } from "lucide-react";

import type { StaffRow } from "../../services/staffService";
import type { ContextMenuItem } from "../ContextMenu";

/** What a staff member's menu needs from the Staff page. */
export interface StaffMenuContext {
  t: TFunction;
  /** The club's own staff, which can be released, or the hiring market, which can be hired. */
  list: "mystaff" | "available";
  /** The member whose hire or release is on its way to the backend, if any. */
  pendingId: string | null;
  onHire: (id: string) => void;
  onRelease: (id: string) => void;
  /** Opens another tab; a scout's menu offers the scouting workflow through it. */
  onNavigate?: (tab: string) => void;
}

/**
 * A staff member's actions, the same on his card and on his table row: a scout's scouting
 * workflow and the release for the club's own staff, the hire for the market's.
 */
export function staffMenuItems(staff: StaffRow, context: StaffMenuContext): ContextMenuItem[] {
  const { t, onNavigate } = context;
  const pending = context.pendingId === staff.id;
  if (context.list === "available") {
    return [
      {
        label: t("staff.hireStaff"),
        icon: <UserPlus className="w-4 h-4" />,
        onClick: () => context.onHire(staff.id),
        disabled: pending,
      },
    ];
  }
  return [
    ...(staff.role === "Scout" && onNavigate
      ? [
          {
            label: t("staff.openScoutingWorkflow"),
            icon: <Eye className="w-4 h-4" />,
            onClick: () => onNavigate("Scouting"),
            disabled: false,
          } satisfies ContextMenuItem,
        ]
      : []),
    {
      label: t("staff.releaseStaff"),
      icon: <UserMinus className="w-4 h-4" />,
      onClick: () => context.onRelease(staff.id),
      danger: true,
      disabled: pending,
    },
  ];
}
