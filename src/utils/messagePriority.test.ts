import { describe, expect, it } from "vitest";

import { needsAttention } from "./messagePriority";

describe("needsAttention", () => {
  /**
   * Given each of the four message priorities
   * When asked whether it needs attention
   * Then High and Urgent do, Normal and Low do not (the backend's `needs_attention`).
   */
  it.each([
    ["Urgent", true],
    ["High", true],
    ["Normal", false],
    ["Low", false],
  ] as const)("%s needs attention: %s", (priority, expected) => {
    expect(needsAttention(priority)).toBe(expected);
  });
});
