import { describe, expect, it } from "vitest";

import { getTrainingStaffAdvice } from "./trainingAdvice";

const t = (key: string, params?: Record<string, string | number>) => {
  if (key === "training.staffAdvice.critical") {
    return `critical:${params?.criticalCount}:${params?.scheduleAdvice}`;
  }
  if (key === "training.staffAdvice.warn") {
    return `warn:${params?.avgCondition}:${params?.exhaustedCount}:${params?.scheduleAdvice}`;
  }
  if (key === "training.staffAdvice.ok") {
    return "ok";
  }

  return key;
};

describe("trainingAdvice", () => {
  it("returns critical advice when too many players are in a critical condition", () => {
    expect(
      getTrainingStaffAdvice(t, {
        criticalCount: 3,
        avgCondition: 58,
        exhaustedCount: 2,
        currentSchedule: "Intense",
        currentFocus: "Physical",
      }),
    ).toEqual({
      level: "critical",
      message: "critical:3:training.staffAdvice.scheduleAdvice.criticalIntense",
    });
  });

  it("returns warning advice when condition is trending too low", () => {
    expect(
      getTrainingStaffAdvice(t, {
        criticalCount: 1,
        avgCondition: 47,
        exhaustedCount: 4,
        currentSchedule: "Balanced",
        currentFocus: "Technical",
      }),
    ).toEqual({
      level: "warn",
      message: "warn:47:4:training.staffAdvice.scheduleAdvice.warnBalanced",
    });
  });

  it("returns a positive suggestion for well-rested squads on a light schedule", () => {
    expect(
      getTrainingStaffAdvice(t, {
        criticalCount: 0,
        avgCondition: 84,
        exhaustedCount: 0,
        currentSchedule: "Light",
        currentFocus: "Technical",
      }),
    ).toEqual({
      level: "ok",
      message: "ok",
    });
  });

  it("returns null when no staff advice is needed", () => {
    expect(
      getTrainingStaffAdvice(t, {
        criticalCount: 0,
        avgCondition: 71,
        exhaustedCount: 1,
        currentSchedule: "Balanced",
        currentFocus: "Physical",
      }),
    ).toBeNull();
  });
});
