import { diffGameDays, parseGameDate } from "./gameDate";
import { CONTRACT_RISK_DAYS } from "./domainConstants";

export type ContractRiskLevel = "critical" | "warning" | "stable";

export function getDaysUntil(targetDate: string, currentDate: string): number {
    const parsedTargetDate = parseGameDate(targetDate);
    const parsedCurrentDate = parseGameDate(currentDate);

    if (!parsedTargetDate || !parsedCurrentDate) {
        return 0;
    }

    return diffGameDays(parsedCurrentDate, parsedTargetDate);
}

export function getContractRiskLevel(
    contractEnd: string | null,
    currentDate: string,
): ContractRiskLevel {
    if (!contractEnd) {
        return "stable";
    }

    const daysUntilExpiry = getDaysUntil(contractEnd, currentDate);

    if (daysUntilExpiry <= CONTRACT_RISK_DAYS.critical) {
        return "critical";
    }
    if (daysUntilExpiry <= CONTRACT_RISK_DAYS.warning) {
        return "warning";
    }
    return "stable";
}

export function getContractRiskBadgeVariant(
    level: ContractRiskLevel,
): "accent" | "success" | "danger" {
    if (level === "critical") {
        return "danger";
    }
    if (level === "warning") {
        return "accent";
    }
    return "success";
}

export function getContractYearsRemaining(
    contractEnd: string | null,
    currentDate: string,
): string {
    if (!contractEnd) {
        return "—";
    }

    const daysUntilExpiry = Math.max(0, getDaysUntil(contractEnd, currentDate));
    return (daysUntilExpiry / 365).toFixed(1);
}
