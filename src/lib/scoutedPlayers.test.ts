import { describe, expect, it } from "vitest";
import type { PlayerData, ScoutedPlayer } from "../store/gameStore";
import { findScoutedReport } from "./scoutedPlayers";

type Attributes = PlayerData["attributes"];

function scouted(playerId: string, scoutedOn: string, pace: number): ScoutedPlayer {
  return {
    player_id: playerId,
    scouted_on: scoutedOn,
    attributes: { pace } as Attributes,
  };
}

describe("findScoutedReport", () => {
  /**
   * Given a player who was scouted this season
   * When the report is looked up
   * Then it carries the scouted attributes and date, and is current
   */
  it("returns the snapshot of a player scouted this season", () => {
    const report = findScoutedReport([scouted("p1", "2026-09-10", 71)], "p1", "2026-08-01");

    expect(report?.scoutedOn).toBe("2026-09-10");
    expect(report?.attributes.pace).toBe(71);
    expect(report?.outOfDate).toBe(false);
  });

  /**
   * Given a player scouted before the current season began
   * When the report is looked up
   * Then it is marked out of date
   */
  it("marks a report from before the season start as out of date", () => {
    const report = findScoutedReport([scouted("p1", "2026-03-10", 71)], "p1", "2026-08-01");

    expect(report?.outOfDate).toBe(true);
  });

  /**
   * Given a report scouted exactly on the season's first day
   * When the report is looked up
   * Then it is still current
   */
  it("keeps a report from the season's first day current", () => {
    const report = findScoutedReport([scouted("p1", "2026-08-01", 71)], "p1", "2026-08-01");

    expect(report?.outOfDate).toBe(false);
  });

  /**
   * Given nobody scouted this player, or an older save without the list
   * When the report is looked up
   * Then there is none
   */
  it("returns null for an unscouted player or a missing list", () => {
    expect(findScoutedReport([scouted("p2", "2026-09-10", 71)], "p1", "2026-08-01")).toBeNull();
    expect(findScoutedReport(undefined, "p1", "2026-08-01")).toBeNull();
  });

  /**
   * Given a game with no known season start
   * When the report is looked up
   * Then it is not called out of date
   */
  it("does not call a report out of date when the season start is unknown", () => {
    expect(findScoutedReport([scouted("p1", "2020-01-01", 71)], "p1", null)?.outOfDate).toBe(false);
  });
});
