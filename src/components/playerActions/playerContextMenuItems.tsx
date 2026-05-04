import type { TFunction } from "i18next";
import {
    ArrowUp,
    Building2,
    Gavel,
    GraduationCap,
    Repeat,
    ScanSearch,
    ShoppingCart,
    User,
} from "lucide-react";

import type { ContextMenuItem } from "../ContextMenu";

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
    t: TFunction,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("squad.viewProfile", "View profile"),
        icon: <User className="w-4 h-4" />,
        onClick,
    };
}

export function buildViewTeamMenuItem(
    t: TFunction,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("common.viewTeam", "View team"),
        icon: <Building2 className="w-4 h-4" />,
        onClick,
    };
}

export function buildToggleTransferListMenuItem(
    t: TFunction,
    transferListed: boolean,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: transferListed
            ? t("squad.removeFromTransferList", "Remove from transfer list")
            : t("squad.addToTransferList", "Add to transfer list"),
        icon: <ShoppingCart className="w-4 h-4" />,
        onClick,
    };
}

export function buildToggleLoanListMenuItem(
    t: TFunction,
    loanListed: boolean,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: loanListed
            ? t("squad.removeFromLoanList", "Remove from loan list")
            : t("squad.addToLoanList", "Add to loan list"),
        icon: <Repeat className="w-4 h-4" />,
        onClick,
    };
}

export function buildScoutPlayerMenuItem(
    t: TFunction,
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
    t: TFunction,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("transfers.makeBid", "Make Transfer Bid"),
        icon: <Gavel className="w-4 h-4" />,
        onClick,
    };
}

export function buildDelegateToYouthAcademyMenuItem(
    t: TFunction,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("youthAcademy.delegateToYouthAcademy", "Delegate to youth academy"),
        icon: <GraduationCap className="w-4 h-4" />,
        onClick,
    };
}

export function buildPromoteToSeniorSquadMenuItem(
    t: TFunction,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("youthAcademy.promoteToSeniorSquad", "Promote to senior squad"),
        icon: <ArrowUp className="w-4 h-4" />,
        onClick,
    };
}