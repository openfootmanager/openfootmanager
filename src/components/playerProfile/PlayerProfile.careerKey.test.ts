import { describe, expect, it } from "vitest";

import type { PlayerData } from "../../store/gameStore";
import { createPlayer } from "../../test-utils/factories";
import { careerRevision } from "./PlayerProfile.careerKey";

const DAY = "2031-08-01T00:00:00Z";
const player = (patch: Partial<PlayerData> = {}) =>
  createPlayer({ id: "p", team_id: "north", ...patch });
const key = (patch: Partial<PlayerData> = {}, token = 1, day = DAY) =>
  careerRevision(token, player(patch), day);
const statsWith = (patch: Partial<PlayerData["stats"]>) => ({
  ...createPlayer().stats,
  ...patch,
});

describe("careerRevision", () => {
  /** Given the same player, game and day, then the revision is the same. */
  it("is stable for the same state", () => {
    expect(key()).toBe(key());
  });

  /**
   * Given a player whose season stats change on the same day (a fixture played over MCP, so the
   *       date has not moved), or whose club, seasons or moves change
   * Then the revision changes each time: the season in progress is read again, not left stale
   */
  it.each([
    ["an appearance", { stats: statsWith({ appearances: 3 }) }],
    ["a goal", { stats: statsWith({ goals: 2 }) }],
    ["an assist", { stats: statsWith({ assists: 1 }) }],
    ["minutes played", { stats: statsWith({ minutes_played: 90 }) }],
    ["a new club", { team_id: "south" }],
    [
      "a closed season",
      {
        career: [
          { season: 2030, team_id: "north", team_name: "N", appearances: 1, goals: 0, assists: 0 },
        ],
      },
    ],
  ])("changes with %s", (_what, patch) => {
    expect(key(patch)).not.toBe(key());
  });

  /** Given the next day, then the revision changes. */
  it("changes with the day", () => {
    expect(key({}, 1, "2031-08-02T00:00:00Z")).not.toBe(key());
  });

  /**
   * Given another career loaded over the same player ids and dates (a saved game loaded)
   * Then the revision differs by the career token, so an answer for the old career is never shown
   */
  it("changes with the career token", () => {
    expect(key({}, 2)).not.toBe(key({}, 1));
  });
});
