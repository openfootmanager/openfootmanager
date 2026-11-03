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

export function resolveInjuryName(injuryName: string, translate: TranslateFn): string {
  if (injuryName.includes(".")) {
    return translate(injuryName, { defaultValue: injuryName });
  }

  return translate(`common.injuries.${injuryName}`, {
    defaultValue: injuryName,
  });
}
