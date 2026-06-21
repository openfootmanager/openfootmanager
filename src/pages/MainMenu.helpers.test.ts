import { describe, expect, it } from "vitest";
import {
  DEFAULT_GENERATED_HISTORY_DEPTH_YEARS,
  MAX_GENERATED_HISTORY_DEPTH_YEARS,
  MANAGER_MINIMUM_AGE,
  historyModeFromMetadata,
  parseCareerStartYear,
  isCareerStartPhase,
  normalizeHistoryDepthYears,
  buildStartupOptions,
  parseIsoDateParts,
  careerStartReferenceDate,
  flooredAgeFromIsoDate,
  dobValidationMessage,
} from "./MainMenu.helpers";
import type { CreateManagerFormData } from "../components/menu/CreateManagerForm";

const t = (key: string, options?: Record<string, unknown>): string => {
  if (key === "validation.invalidDate") return "Invalid date";
  if (key === "validation.minAge") return `Min age ${options?.min}`;
  if (key === "validation.invalidDob") return "Invalid date of birth";
  return key;
};

const baseForm = (): CreateManagerFormData => ({
  firstName: "Jane",
  lastName: "Doe",
  dob: "1980-07-01",
  startYear: "2025",
  startPhase: "seasonStart",
  nationality: "GB",
});

describe("historyModeFromMetadata", () => {
  it("returns reference for historicalSnapshot", () => {
    expect(historyModeFromMetadata({ kind: "historicalSnapshot" })).toBe("reference");
  });

  it("returns hybrid for rosterBaseline", () => {
    expect(historyModeFromMetadata({ kind: "rosterBaseline" })).toBe("hybrid");
  });

  it("returns undefined for unknown kind", () => {
    expect(historyModeFromMetadata({ kind: "other" })).toBeUndefined();
    expect(historyModeFromMetadata(null)).toBeUndefined();
    expect(historyModeFromMetadata(undefined)).toBeUndefined();
    expect(historyModeFromMetadata({})).toBeUndefined();
  });
});

describe("parseCareerStartYear", () => {
  it("parses valid integer strings", () => {
    expect(parseCareerStartYear("2025")).toBe(2025);
    expect(parseCareerStartYear("  2030  ")).toBe(2030);
  });

  it("returns null for non-numeric input", () => {
    expect(parseCareerStartYear("abc")).toBeNull();
    expect(parseCareerStartYear("")).toBeNull();
    expect(parseCareerStartYear("20.5")).toBeNull();
    expect(parseCareerStartYear("2025a")).toBeNull();
  });
});

describe("isCareerStartPhase", () => {
  it("accepts valid phases", () => {
    expect(isCareerStartPhase("seasonStart")).toBe(true);
    expect(isCareerStartPhase("midSeason")).toBe(true);
  });

  it("rejects invalid phases", () => {
    expect(isCareerStartPhase("")).toBe(false);
    expect(isCareerStartPhase("postSeason")).toBe(false);
  });
});

describe("normalizeHistoryDepthYears", () => {
  it("returns the value for valid integers in range", () => {
    expect(normalizeHistoryDepthYears(0)).toBe(0);
    expect(normalizeHistoryDepthYears(12)).toBe(12);
    expect(normalizeHistoryDepthYears(MAX_GENERATED_HISTORY_DEPTH_YEARS)).toBe(MAX_GENERATED_HISTORY_DEPTH_YEARS);
  });

  it("returns null for out-of-range values", () => {
    expect(normalizeHistoryDepthYears(-1)).toBeNull();
    expect(normalizeHistoryDepthYears(MAX_GENERATED_HISTORY_DEPTH_YEARS + 1)).toBeNull();
  });

  it("returns null for non-integers", () => {
    expect(normalizeHistoryDepthYears(1.5)).toBeNull();
  });
});

describe("buildStartupOptions", () => {
  it("returns payload for valid form data", () => {
    const result = buildStartupOptions(baseForm(), 12);
    expect(result).toEqual({ startYear: 2025, startPhase: "seasonStart", historyDepthYears: 12 });
  });

  it("returns null when start year is below minimum", () => {
    expect(buildStartupOptions({ ...baseForm(), startYear: "2019" }, 12)).toBeNull();
  });

  it("returns null for non-numeric start year", () => {
    expect(buildStartupOptions({ ...baseForm(), startYear: "abc" }, 12)).toBeNull();
  });

  it("returns null for invalid start phase", () => {
    expect(buildStartupOptions({ ...baseForm(), startPhase: "postSeason" as never }, 12)).toBeNull();
  });

  it("returns null for invalid history depth", () => {
    expect(buildStartupOptions(baseForm(), -1)).toBeNull();
  });
});

describe("parseIsoDateParts", () => {
  it("parses a valid date", () => {
    expect(parseIsoDateParts("1980-07-15")).toEqual({ year: 1980, month: 7, day: 15 });
  });

  it("returns null for empty string", () => {
    expect(parseIsoDateParts("")).toBeNull();
  });

  it("returns null for wrong format", () => {
    expect(parseIsoDateParts("15/07/1980")).toBeNull();
    expect(parseIsoDateParts("1980-7-15")).toBeNull();
  });

  it("returns null for impossible dates", () => {
    expect(parseIsoDateParts("1980-02-30")).toBeNull();
    expect(parseIsoDateParts("1980-13-01")).toBeNull();
  });
});

describe("careerStartReferenceDate", () => {
  it("seasonStart returns July 1 of the start year", () => {
    const date = careerStartReferenceDate(2025, "seasonStart");
    expect(date.getUTCFullYear()).toBe(2025);
    expect(date.getUTCMonth()).toBe(6); // July = 6 (0-indexed)
    expect(date.getUTCDate()).toBe(1);
  });

  it("midSeason adds 120 days to July 1", () => {
    const seasonStart = careerStartReferenceDate(2025, "seasonStart");
    const midSeason = careerStartReferenceDate(2025, "midSeason");
    const diffMs = midSeason.getTime() - seasonStart.getTime();
    expect(diffMs / (1000 * 60 * 60 * 24)).toBe(120);
  });
});

describe("flooredAgeFromIsoDate", () => {
  it("returns full age when birthday has already passed this year", () => {
    const ref = new Date(Date.UTC(2025, 6, 1)); // July 1, 2025
    expect(flooredAgeFromIsoDate("1990-01-15", ref)).toBe(35);
  });

  it("returns age minus one when birthday has not yet occurred this year", () => {
    const ref = new Date(Date.UTC(2025, 6, 1)); // July 1, 2025
    expect(flooredAgeFromIsoDate("1990-12-15", ref)).toBe(34);
  });

  it("counts the birthday itself as having passed", () => {
    const ref = new Date(Date.UTC(2025, 6, 1)); // July 1, 2025
    expect(flooredAgeFromIsoDate("1990-07-01", ref)).toBe(35);
  });

  it("returns null for an invalid date string", () => {
    const ref = new Date(Date.UTC(2025, 6, 1));
    expect(flooredAgeFromIsoDate("not-a-date", ref)).toBeNull();
  });
});

describe("dobValidationMessage", () => {
  it("returns null when dob is empty", () => {
    expect(dobValidationMessage({ ...baseForm(), dob: "" }, 12, t)).toBeNull();
  });

  it("returns invalid-date message for malformed dob", () => {
    expect(dobValidationMessage({ ...baseForm(), dob: "not-a-date" }, 12, t)).toBe("Invalid date");
  });

  it("returns min-age message when manager would be too young at career start", () => {
    // Manager born in 2000, starting in 2025 (age ~24) — below MANAGER_MINIMUM_AGE of 30
    const msg = dobValidationMessage({ ...baseForm(), dob: "2000-07-01", startYear: "2025" }, 12, t);
    expect(msg).toBe(`Min age ${MANAGER_MINIMUM_AGE}`);
  });

  it("returns null when age is valid", () => {
    // Born 1980, starting 2025 = age 45
    expect(dobValidationMessage(baseForm(), 12, t)).toBeNull();
  });

  it("returns null when startup options cannot be built (invalid startYear)", () => {
    // If buildStartupOptions returns null, dobValidationMessage should also return null
    expect(dobValidationMessage({ ...baseForm(), startYear: "abc" }, 12, t)).toBeNull();
  });
});
