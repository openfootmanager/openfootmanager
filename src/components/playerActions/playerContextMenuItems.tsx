import {
    ArrowUp,
    Building2,
    Gavel,
    GraduationCap,
    Repeat,
    ScanSearch,
    ShoppingCart,
    User,
    UserPlus,
} from "lucide-react";

import type { ContextMenuItem } from "../ContextMenu";

type MenuTranslateFn = (
    key: string,
    options?: Record<string, string | number>,
) => string;

export type ScoutMenuState =
    | "ready"
    | "busy"
    | "already-assigned"
    | "unavailable";

export function buildDividerMenuItem(): ContextMenuItem {
    return {
        label: "",
        icon: undefined,
        onClick: () => { },
        divider: true,
    };
}

export function buildViewProfileMenuItem(
    t: MenuTranslateFn,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("squad.viewProfile"),
        icon: <User className="w-4 h-4" />,
        onClick,
    };
}

export function buildViewTeamMenuItem(
    t: MenuTranslateFn,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("common.viewTeam"),
        icon: <Building2 className="w-4 h-4" />,
        onClick,
    };
}

export function buildToggleTransferListMenuItem(
    t: MenuTranslateFn,
    transferListed: boolean,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: transferListed
            ? t("squad.removeFromTransferList")
            : t("squad.addToTransferList"),
        icon: <ShoppingCart className="w-4 h-4" />,
        onClick,
    };
}

export function buildToggleLoanListMenuItem(
    t: MenuTranslateFn,
    loanListed: boolean,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: loanListed
            ? t("squad.removeFromLoanList")
            : t("squad.addToLoanList"),
        icon: <Repeat className="w-4 h-4" />,
        onClick,
    };
}

export function buildScoutPlayerMenuItem(
    t: MenuTranslateFn,
    state: ScoutMenuState,
    onClick: () => void,
): ContextMenuItem {
    return {
        label:
            state === "already-assigned"
                ? t("scouting.scoutingInProgress")
                : state === "unavailable"
                    ? t("scouting.noScoutsFree")
                    : t("scouting.scoutBtn"),
        icon: <ScanSearch className="w-4 h-4" />,
        disabled: state !== "ready",
        onClick,
    };
}

export function buildMakeTransferBidMenuItem(
    t: MenuTranslateFn,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("transfers.makeBid"),
        icon: <Gavel className="w-4 h-4" />,
        onClick,
    };
}

export function buildOfferFreeAgentContractMenuItem(
    t: MenuTranslateFn,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("transfers.offerContract"),
        icon: <UserPlus className="w-4 h-4" />,
        onClick,
    };
}

export function buildDelegateToYouthAcademyMenuItem(
    t: MenuTranslateFn,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("youthAcademy.delegateToYouthAcademy"),
        icon: <GraduationCap className="w-4 h-4" />,
        onClick,
    };
}

export function buildPromoteToSeniorSquadMenuItem(
    t: MenuTranslateFn,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("youthAcademy.promoteToSeniorSquad"),
        icon: <ArrowUp className="w-4 h-4" />,
        onClick,
    };
}
