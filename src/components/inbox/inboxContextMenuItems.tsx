import { Check, Eye, Trash2 } from "lucide-react";
import type { TFunction } from "i18next";

import type { ContextMenuItem } from "../ContextMenu";

export function buildOpenMessageMenuItem(
    t: TFunction,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("inbox.openMessage"),
        icon: <Eye className="w-4 h-4" />,
        onClick,
    };
}

export function buildMarkMessageReadMenuItem(
    t: TFunction,
    disabled: boolean,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("inbox.markAsRead"),
        icon: <Check className="w-4 h-4" />,
        disabled,
        onClick,
    };
}

export function buildDeleteMessageMenuItem(
    t: TFunction,
    onClick: () => void,
): ContextMenuItem {
    return {
        label: t("inbox.deleteMessage"),
        icon: <Trash2 className="w-4 h-4" />,
        danger: true,
        onClick,
    };
}
