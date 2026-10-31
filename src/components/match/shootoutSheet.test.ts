import { describe, expect, it } from "vitest";

import { buildKickSheet } from "./shootoutSheet";
import { makeSnapshot } from "./matchScreenFixtures";
import type { EnginePlayerData, MatchEvent, MatchSnapshot } from "./types";

type Kick = "ShootoutGoal" | "ShootoutMiss";

function kick(side: "Home" | "Away", eventType: Kick, playerId: string | null): MatchEvent {
  return {
    minute: 120,
    event_type: eventType,
    side,
    zone: "Box",
    player_id: playerId,
    secondary_player_id: null,
  };
}

/** A shootout snapshot with the given kicks; Alice (p1) plays for the home side, Bob (p2) away. */
function shootout(events: MatchEvent[], phase = "PenaltyShootout"): MatchSnapshot {
  return { ...makeSnapshot(), phase, events };
}

/** n kicks a side, alternating home then away, all scored. */
function levelKicks(n: number): MatchEvent[] {
  return Array.from({ length: n }).flatMap(() => [
    kick("Home", "ShootoutGoal", "p1"),
    kick("Away", "ShootoutGoal", "p2"),
  ]);
}

describe("buildKickSheet", () => {
  /**
   * Given a shootout that has just begun
   * When the sheet is built
   * Then it has the five regular rounds, every kick still to come, and round 1 is under way.
   */
  it("starts with five rounds still to come", () => {
    const sheet = buildKickSheet(shootout([]));

    expect(sheet.rows.map((row) => row.round)).toEqual([1, 2, 3, 4, 5]);
    expect(
      sheet.rows.every((row) => row.home.state === "toCome" && row.away.state === "toCome"),
    ).toBe(true);
    expect(sheet.currentRound).toBe(1);
  });

  /**
   * Given the home side missed its first kick and scored its second, and the away side scored once
   * When the sheet is built
   * Then each round holds that side's kick in the order taken, named for its kicker.
   */
  it("writes each kick in the round it was taken, with its kicker", () => {
    const sheet = buildKickSheet(
      shootout([
        kick("Home", "ShootoutMiss", "p1"),
        kick("Away", "ShootoutGoal", "p2"),
        kick("Home", "ShootoutGoal", "p1"),
      ]),
    );

    expect(sheet.rows[0]).toEqual({
      round: 1,
      home: { state: "missed", kickerName: "Alice" },
      away: { state: "scored", kickerName: "Bob" },
    });
    expect(sheet.rows[1]).toEqual({
      round: 2,
      home: { state: "scored", kickerName: "Alice" },
      away: { state: "toCome" },
    });
    expect(sheet.currentRound).toBe(2);
  });

  /**
   * Given a kicker who came off the bench, so is listed only among the substitutes
   * When the sheet is built
   * Then the kick is still named for him.
   */
  it("names a kicker who came on from the bench", () => {
    const base = makeSnapshot();
    const sub: EnginePlayerData = { ...base.home_team.players[0], id: "sub1", name: "Sam" };
    const sheet = buildKickSheet({
      ...base,
      phase: "PenaltyShootout",
      home_bench: [sub],
      events: [kick("Home", "ShootoutGoal", "sub1")],
    });

    expect(sheet.rows[0].home).toEqual({ state: "scored", kickerName: "Sam" });
  });

  /**
   * Given an in-match penalty in the event log before the shootout
   * When the sheet is built
   * Then only the shootout's kicks are written.
   */
  it("leaves out a penalty from the match itself", () => {
    const penalty: MatchEvent = {
      ...kick("Home", "ShootoutGoal", "p1"),
      event_type: "PenaltyGoal",
    };
    const sheet = buildKickSheet(shootout([penalty]));

    expect(sheet.rows[0].home).toEqual({ state: "toCome" });
  });

  /**
   * Given both sides level after five kicks each
   * When the sheet is built
   * Then a sixth round, sudden death, is added and is the one under way.
   */
  it("adds the next sudden-death round once both sides are level", () => {
    const sheet = buildKickSheet(shootout(levelKicks(5)));

    expect(sheet.rows).toHaveLength(6);
    expect(sheet.rows[5]).toEqual({
      round: 6,
      home: { state: "toCome" },
      away: { state: "toCome" },
    });
    expect(sheet.currentRound).toBe(6);
  });

  /**
   * Given a sudden-death round where only the home side has kicked
   * When the sheet is built
   * Then no further round is added until the away side kicks.
   */
  it("adds no round while one is under way", () => {
    const sheet = buildKickSheet(shootout([...levelKicks(5), kick("Home", "ShootoutGoal", "p1")]));

    expect(sheet.rows).toHaveLength(6);
    expect(sheet.rows[5].away).toEqual({ state: "toCome" });
    expect(sheet.currentRound).toBe(6);
  });

  /**
   * Given a shootout decided in round 4, before the away side's fourth kick
   * When the sheet is built after the final whistle
   * Then it ends at round 4, the away cell is marked not taken, and no round is under way.
   */
  it("ends at the deciding round once the shootout is over", () => {
    const events = [
      ...levelKicks(3).map((evt) =>
        evt.side === "Away" ? { ...evt, event_type: "ShootoutMiss" } : evt,
      ),
      kick("Home", "ShootoutGoal", "p1"),
    ];
    const sheet = buildKickSheet(shootout(events, "Finished"));

    expect(sheet.rows).toHaveLength(4);
    expect(sheet.rows[3].away).toEqual({ state: "notTaken" });
    expect(sheet.currentRound).toBeNull();
  });
});

describe("buildKickSheet with a planned order", () => {
  /** Alpha FC with Carol (p3) and Dan (p4) beside Alice (p1); Beta FC as before. */
  function withThreeHomePlayers(): MatchSnapshot {
    const base = shootout([]);
    const alice = base.home_team.players[0];
    const named = (id: string, name: string): EnginePlayerData => ({ ...alice, id, name });
    return {
      ...base,
      home_team: { ...base.home_team, players: [alice, named("p3", "Carol")] },
      // A kicker may be listed from the bench, as the sheet's own names are.
      home_bench: [named("p4", "Dan")],
    };
  }

  /**
   * Given the home manager's order Dan, Carol, Alice, with three kickers a side, before any kick
   * When the sheet is built with that plan
   * Then the home column names who will take each kick, starting again from Dan at kick 4, and the
   * away column names nobody: the opponent's order stays hidden.
   */
  it("names each planned kicker in the planning side's column only", () => {
    const sheet = buildKickSheet(withThreeHomePlayers(), {
      side: "Home",
      order: ["p4", "p3", "p1"],
      kickers: 3,
    });

    expect(sheet.rows.map((row) => row.home)).toEqual([
      { state: "toCome", plannedKickerName: "Dan" },
      { state: "toCome", plannedKickerName: "Carol" },
      { state: "toCome", plannedKickerName: "Alice" },
      { state: "toCome", plannedKickerName: "Dan" },
      { state: "toCome", plannedKickerName: "Carol" },
    ]);
    expect(sheet.rows.every((row) => row.away.state === "toCome")).toBe(true);
    expect(sheet.rows.some((row) => "plannedKickerName" in row.away)).toBe(false);
  });

  /**
   * Given an away order of four with three kickers
   * When the sheet is built with that plan
   * Then the fourth player, who sits out, is never on the sheet.
   */
  it("leaves the players who sit out off the sheet", () => {
    const sheet = buildKickSheet(withThreeHomePlayers(), {
      side: "Away",
      order: ["p2", "p4", "p3", "p1"],
      kickers: 3,
    });

    const planned = sheet.rows.map((row) =>
      row.away.state === "toCome" ? row.away.plannedKickerName : null,
    );
    expect(planned).toEqual(["Bob", "Dan", "Carol", "Bob", "Dan"]);
  });
});
