import { describe, expect, it } from "vitest";

import { localDateIso } from "./valueFormatting";

describe("localDateIso", () => {
  /**
   * Given a moment late on the player's local evening
   * When it is written as a date
   * Then it is the player's own day, whatever day it is in UTC
   */
  it("uses the local calendar day, not the UTC one", () => {
    expect(localDateIso(new Date(2026, 9, 12, 23, 30))).toBe("2026-10-12");
    expect(localDateIso(new Date(2026, 9, 12, 0, 30))).toBe("2026-10-12");
  });

  /**
   * Given single-digit months and days
   * When written
   * Then they are zero padded
   */
  it("pads months and days", () => {
    expect(localDateIso(new Date(2026, 0, 5, 12))).toBe("2026-01-05");
  });
});
