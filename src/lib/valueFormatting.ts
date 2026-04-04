import { getAgeOnDate } from "./gameDate";

export function calcAge(dob: string, asOfDate?: string): number {
    return getAgeOnDate(dob, asOfDate ?? new Date().toISOString());
}

export function formatVal(value: number): string {
    if (value >= 1_000_000) {
        return `€${(value / 1_000_000).toFixed(1)}M`;
    }
    if (value >= 1_000) {
        return `€${(value / 1_000).toFixed(0)}K`;
    }
    return `€${value}`;
}

export function formatWeeklyAmount(
    formattedAmount: string,
    weeklySuffix: string,
): string {
    return `${formattedAmount}${weeklySuffix}`;
}
