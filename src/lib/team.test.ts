import { describe, expect, it } from "vitest";

import { storedTeamLabel } from "./team";

const t = (key: string) => `[${key}]`;

describe("storedTeamLabel", () => {
  /**
   * Given a stored name that says "no club": nothing, the English literal an older save wrote,
   *      or the free-agent key a news article carries
   * When it is labelled
   * Then it is the translated "free agent".
   */
  it.each(["", "  ", "Free Agent", "common.freeAgent"])("words %j as a free agent", (stored) => {
    expect(storedTeamLabel(stored, t)).toBe("[common.freeAgent]");
  });

  it("words a missing name as a free agent", () => {
    expect(storedTeamLabel(undefined, t)).toBe("[common.freeAgent]");
    expect(storedTeamLabel(null, t)).toBe("[common.freeAgent]");
  });

  /**
   * Given a club's name (a package can author a historical club by name alone)
   * When it is labelled
   * Then the name stays.
   */
  it("keeps a club's name", () => {
    expect(storedTeamLabel("Juventus", t)).toBe("Juventus");
  });
});
