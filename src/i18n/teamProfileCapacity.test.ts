import { describe, expect, it } from "vitest";

import cs from "./locales/cs.json";
import en from "./locales/en.json";
import fr from "./locales/fr.json";
import ru from "./locales/ru.json";

/**
 * A key named `capacity` exists twice (the stadium label and a report-upload error), and an edit
 * aimed at one overwrote the other in three locales once. Pin the stadium label's words.
 */
describe("teamProfile.capacity", () => {
  /**
   * Given the stadium capacity label in locales where a report-upload key shares its name
   * When the locale files are read
   * Then each still says "capacity" of a stadium, not a report-server message
   */
  it("stays the stadium label in the locales that once lost it", () => {
    expect(en.teamProfile.capacity).toBe("Capacity");
    expect(cs.teamProfile.capacity).toBe("Kapacita");
    expect(fr.teamProfile.capacity).toBe("Capacité");
    expect(ru.teamProfile.capacity).toBe("Вместимость");
  });
});
