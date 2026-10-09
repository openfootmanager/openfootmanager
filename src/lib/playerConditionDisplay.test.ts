import { describe, expect, it } from "vitest";
import { condBgColor } from "./playerConditionDisplay";

describe("condition fills in both themes", () => {
  /** Given a condition bucket, when its fill renders in either theme,
   * then the same severity has an explicit colour for each theme. */
  it.each([
    [75, "primary"],
    [50, "amber"],
    [49, "red"],
  ] as const)("keeps the %s percent condition warning in both themes", (condition, colour) => {
    expect(condBgColor(condition)).toBe(`bg-${colour}-500 dark:bg-${colour}-400`);
  });
});
