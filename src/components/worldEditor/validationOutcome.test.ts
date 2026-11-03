import { describe, expect, it } from "vitest";

import { findingsToShow } from "./validationOutcome";

const finding = { code: "c", file: "f", params: {} };

describe("findingsToShow", () => {
  // Given a validate that found errors, only warnings, or nothing
  // When the editor decides whether to open the Issues panel
  // Then it opens for errors and for warnings alone, and stays put for a clean result.
  it("opens the Issues panel for errors and for warnings alone", () => {
    expect(findingsToShow({ issues: [finding], warnings: [] })).toBe(true);
    expect(findingsToShow({ issues: [], warnings: [finding] })).toBe(true);
    expect(findingsToShow({ issues: [], warnings: [] })).toBe(false);
  });

  // Given an older reply without a warnings list
  // When the editor decides
  // Then it does not fail.
  it("tolerates a reply with no warnings", () => {
    expect(findingsToShow({ issues: [] } as never)).toBe(false);
  });
});
