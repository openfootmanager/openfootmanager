import { describe, expect, it } from "vitest";

import {
    addGameDays,
    diffGameDays,
    formatGameDate,
    getAgeOnDate,
    parseGameDate,
    positiveGameDayDiff,
} from "./gameDate";

describe("gameDate", () => {
    it("normalizes plain and ISO date strings to UTC calendar dates", () => {
        expect(formatGameDate(parseGameDate("2026-08-10"))).toBe("2026-08-10");
        expect(formatGameDate(parseGameDate("2026-08-10T14:30:00Z"))).toBe(
            "2026-08-10",
        );
    });

    it("calculates day deltas and additions on normalized game dates", () => {
        const startDate = parseGameDate("2026-08-10")!;
        const endDate = addGameDays(startDate, 5);

        expect(diffGameDays(startDate, endDate)).toBe(5);
        expect(positiveGameDayDiff(startDate, endDate)).toBe(5);
        expect(positiveGameDayDiff(endDate, startDate)).toBeNull();
        expect(formatGameDate(endDate)).toBe("2026-08-15");
    });

    it("calculates age against an explicit game date", () => {
        expect(getAgeOnDate("2000-07-02", "2026-07-01")).toBe(25);
        expect(getAgeOnDate("2000-07-01", "2026-07-01")).toBe(26);
    });
});