import type { ContractWarningStage } from "../services/contractService";
import { formatDate } from "./dateFormatting";

type ExpiryTranslate = (key: string, options: { date: string }) => string;

export function contractExpiryLabel(
  contractEnd: string,
  language: string,
  t: ExpiryTranslate,
): string {
  return t("finances.contractExpiresOn", { date: formatDate(contractEnd, language) });
}

export type ContractRiskLevel = "critical" | "warning" | "stable";

export function getDaysUntil(targetDate: string, currentDate: string): number {
  const millisecondsPerDay = 1000 * 60 * 60 * 24;
  return Math.ceil(
    (new Date(targetDate).getTime() - new Date(currentDate).getTime()) / millisecondsPerDay,
  );
}

/**
 * How a warning stage reads on a badge: the last six months (and nearer) are critical, the year
 * before that is a warning, anything else is stable. Which stage a contract is in is the
 * backend's (`get_contract_warning_stages`); this only picks the colour.
 */
export function contractRiskLevelOf(
  stage: ContractWarningStage | null | undefined,
): ContractRiskLevel {
  if (stage === "six_months" || stage === "three_months" || stage === "final_weeks") {
    return "critical";
  }
  return stage === "twelve_months" ? "warning" : "stable";
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

export function getContractYearsRemaining(contractEnd: string | null, currentDate: string): string {
  if (!contractEnd) {
    return "—";
  }

  const daysUntilExpiry = Math.max(0, getDaysUntil(contractEnd, currentDate));
  return (daysUntilExpiry / 365).toFixed(1);
}
