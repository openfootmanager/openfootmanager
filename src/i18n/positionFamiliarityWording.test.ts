import { describe, expect, it } from "vitest";

import en from "./locales/en.json";

/** The labels that say whether a player is in a position he knows. */
const POSITION_FAMILIARITY_KEYS = [
  ["squad", "naturalFit", "Natural position"],
  ["squad", "adaptedFit", "Can play there"],
  ["tactics", "naturalFit", "Natural position"],
  ["tactics", "adaptedFit", "Can play there"],
  ["tactics", "assignBestFit", "Assign best position"],
  ["match", "formationFit", "Formation"],
] as const;

describe("position familiarity wording", () => {
  /**
   * Given the labels for natural, adapted and out-of-position players
   * When the English copy is read
   * Then it names the position and never calls it a "fit", which read as a rating
   */
  it.each(POSITION_FAMILIARITY_KEYS)(
    "%s.%s reads as a position, not a fit",
    (section, key, expected) => {
      const copy = (en as unknown as Record<string, Record<string, string>>)[section][key];
      expect(copy).toBe(expected);
      expect(copy).not.toMatch(/\bfit\b/i);
    },
  );
});
