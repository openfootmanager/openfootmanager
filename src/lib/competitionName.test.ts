import { describe, expect, it } from "vitest";
import { competitionDisplayName } from "./competitionName";

const missingTranslation = (_key: string, options?: Record<string, unknown>): string =>
  String(options?.defaultValue ?? _key);

describe("competition display name", () => {
  /** Given a missing translation and stored name, when displayed, then the stored name is used. */
  it("falls back to the stored name", () => {
    expect(
      competitionDisplayName({ name_key: "missing.name", name: "League" }, missingTranslation),
    ).toBe("League");
  });

  /** Given a missing translation and no stored name, when displayed, then no raw key is shown. */
  it("does not show a missing key when the stored name is absent", () => {
    expect(competitionDisplayName({ name_key: "missing.name" }, missingTranslation)).toBe("");
  });
});
