import type { CareerStartPhase, CreateManagerFormData } from "../components/menu/CreateManagerForm";
import type { WorldDatabaseInfo } from "../components/menu/WorldSelect";

export const MANAGER_MINIMUM_AGE = 30;
export const MIN_CAREER_START_YEAR = 2020;
export const DEFAULT_GENERATED_HISTORY_DEPTH_YEARS = 12;
export const MAX_GENERATED_HISTORY_DEPTH_YEARS = 24;
export const GENERATED_HISTORY_DEPTH_STORAGE_KEY = "ofm-generated-history-depth-years";

export type StartupOptionsPayload = {
  startYear: number;
  startPhase: CareerStartPhase;
  historyDepthYears: number;
};

export function historyModeFromMetadata(
  metadata: unknown,
): WorldDatabaseInfo["history_mode"] {
  const kind =
    metadata && typeof metadata === "object" && "kind" in metadata
      ? (metadata as { kind?: unknown }).kind
      : undefined;

  if (kind === "historicalSnapshot") return "reference";
  if (kind === "rosterBaseline") return "hybrid";
  return undefined;
}

export function defaultCareerStartYear(): string {
  return String(new Date().getFullYear());
}

export function parseCareerStartYear(rawValue: string): number | null {
  const trimmed = rawValue.trim();
  if (!/^\d+$/.test(trimmed)) return null;

  const parsed = Number(trimmed);
  if (!Number.isInteger(parsed)) return null;
  return parsed;
}

export function isCareerStartPhase(value: string): value is CareerStartPhase {
  return value === "seasonStart" || value === "midSeason";
}

export function normalizeHistoryDepthYears(value: number): number | null {
  if (!Number.isInteger(value)) return null;
  if (value < 0 || value > MAX_GENERATED_HISTORY_DEPTH_YEARS) return null;
  return value;
}

export function initialHistoryDepthYears(): number {
  if (typeof window === "undefined") {
    return DEFAULT_GENERATED_HISTORY_DEPTH_YEARS;
  }

  const storedValue = window.localStorage.getItem(
    GENERATED_HISTORY_DEPTH_STORAGE_KEY,
  );
  if (storedValue === null) {
    return DEFAULT_GENERATED_HISTORY_DEPTH_YEARS;
  }

  const parsedValue = Number(storedValue);
  return (
    normalizeHistoryDepthYears(parsedValue) ??
    DEFAULT_GENERATED_HISTORY_DEPTH_YEARS
  );
}

export function buildStartupOptions(
  formData: CreateManagerFormData,
  historyDepthYears: number,
): StartupOptionsPayload | null {
  const startYear = parseCareerStartYear(formData.startYear);
  if (startYear === null || startYear < MIN_CAREER_START_YEAR) {
    return null;
  }
  if (!isCareerStartPhase(formData.startPhase)) {
    return null;
  }
  const normalizedHistoryDepthYears = normalizeHistoryDepthYears(
    historyDepthYears,
  );
  if (normalizedHistoryDepthYears === null) {
    return null;
  }

  return {
    startYear,
    startPhase: formData.startPhase,
    historyDepthYears: normalizedHistoryDepthYears,
  };
}

export type IsoDateParts = {
  year: number;
  month: number;
  day: number;
};

export function parseIsoDateParts(isoDob: string): IsoDateParts | null {
  if (!isoDob) return null;

  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(isoDob);
  if (!match) return null;

  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const birthDate = new Date(Date.UTC(year, month - 1, day));

  if (
    Number.isNaN(birthDate.getTime()) ||
    birthDate.getUTCFullYear() !== year ||
    birthDate.getUTCMonth() !== month - 1 ||
    birthDate.getUTCDate() !== day
  ) {
    return null;
  }

  return { year, month, day };
}

// World Cups fall in the summers of 2022, 2026, 2030, … (every year ≡ 2 mod 4).
// Matches the backend start_date_for_year rule: June 1 in WC years, July 1 otherwise.
function isWorldCupYear(year: number): boolean {
  return year % 4 === 2;
}

export function careerStartReferenceDate(
  startYear: number,
  startPhase: CareerStartPhase,
): Date {
  const month = isWorldCupYear(startYear) ? 5 : 6; // 0-indexed: 5=June, 6=July
  const referenceDate = new Date(Date.UTC(startYear, month, 1));
  if (startPhase === "midSeason") {
    referenceDate.setUTCDate(referenceDate.getUTCDate() + 120);
  }
  return referenceDate;
}

export function flooredAgeFromIsoDate(
  isoDob: string,
  referenceDate: Date,
): number | null {
  const parts = parseIsoDateParts(isoDob);
  if (!parts) return null;

  let age = referenceDate.getUTCFullYear() - parts.year;
  const hasHadBirthdayThisYear =
    referenceDate.getUTCMonth() > parts.month - 1 ||
    (referenceDate.getUTCMonth() === parts.month - 1 &&
      referenceDate.getUTCDate() >= parts.day);

  if (!hasHadBirthdayThisYear) {
    age -= 1;
  }
  return Number.isNaN(age) ? null : age;
}

export function dobValidationMessage(
  formData: CreateManagerFormData,
  historyDepthYears: number,
  t: (key: string, options?: Record<string, unknown>) => string,
): string | null {
  if (!formData.dob) return null;

  if (parseIsoDateParts(formData.dob) === null) {
    return t("validation.invalidDate");
  }

  const startupOptions = buildStartupOptions(formData, historyDepthYears);
  if (!startupOptions) return null;

  const age = flooredAgeFromIsoDate(
    formData.dob,
    careerStartReferenceDate(startupOptions.startYear, startupOptions.startPhase),
  );
  if (age === null) return t("validation.invalidDate");
  if (age < MANAGER_MINIMUM_AGE) {
    return t("validation.minAge", { min: MANAGER_MINIMUM_AGE });
  }
  if (age > 99) return t("validation.invalidDob");
  return null;
}

