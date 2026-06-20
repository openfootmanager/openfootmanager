import type { TOptions } from "i18next";

export interface InjuryDisplayData {
    name: string;
    days_remaining: number;
}

export type InjurySeverity = "minor" | "moderate" | "serious" | "major";

type TranslateFn = (key: string, options?: TOptions) => string;

export function getInjurySeverity(daysRemaining: number): InjurySeverity {
    if (daysRemaining <= 3) {
        return "minor";
    }

    if (daysRemaining <= 7) {
        return "moderate";
    }

    if (daysRemaining <= 14) {
        return "serious";
    }

    return "major";
}

export function getInjuryBadgeClassName(daysRemaining: number): string {
    switch (getInjurySeverity(daysRemaining)) {
        case "minor":
            return "border-yellow-200 bg-yellow-50 text-yellow-700 dark:border-yellow-800/70 dark:bg-yellow-950/30 dark:text-yellow-300";
        case "moderate":
            return "border-amber-300 bg-amber-100 text-amber-800 dark:border-amber-700/70 dark:bg-amber-950/40 dark:text-amber-300";
        case "serious":
            return "border-orange-300 bg-orange-100 text-orange-800 dark:border-orange-700/70 dark:bg-orange-950/45 dark:text-orange-300";
        case "major":
        default:
            return "border-red-300 bg-red-100 text-red-800 dark:border-red-700/80 dark:bg-red-950/50 dark:text-red-300";
    }
}

export function resolveInjuryName(
    injuryName: string,
    translate: TranslateFn,
): string {
    if (injuryName.includes(".")) {
        return translate(injuryName, { defaultValue: injuryName });
    }

    return translate(`common.injuries.${injuryName}`, {
        defaultValue: injuryName,
    });
}
