import { describe, expect, it } from "vitest";
import type { PlayerData } from "../../store/gameStore";
import {
  applyLineupDrop,
  applyLineupSwap,
  buildActivePositionMap,
  buildDemoteFromStartingXi,
  buildRoleCoverageSummary,
  getBestRoleForFormation,
  getPlayStyleFit,
  buildPitchRows,
  buildPitchSlotRows,
  buildPromoteToStartingXi,
  buildStartingXIIds,
  getSquadTacticalFit,
  getPitchSlotWidth,
  getPreferredPositions,
  isPlayerOutOfPosition,
  normalisePosition,
  parseFormationSlots,
  positionCode,
  translatePositionAbbreviation,
} from "./SquadTab.helpers";

const makePlayer = (
  id: string,
  position: string,
  overrides: Partial<PlayerData> = {},
): PlayerData => ({
  id,
  match_name: id,
  full_name: `Player ${id}`,
  date_of_birth: "1998-01-01",
  nationality: "GB",
  position,
  natural_position: position,
  alternate_positions: [],
  training_focus: null,
  attributes: {
    pace: 60,
    stamina: 60,
    strength: 60,
    agility: 60,
    passing: 60,
    shooting: 60,
    tackling: 60,
    dribbling: 60,
    defending: 60,
    positioning: 60,
    vision: 60,
    decisions: 60,
    composure: 60,
    aggression: 60,
    teamwork: 60,
    leadership: 60,
    handling: 60,
    reflexes: 60,
    aerial: 60,
  },
  condition: 100,
  morale: 80,
  injury: null,
  team_id: "team1",
  retired: false,
  contract_end: "2027-06-30",
  wage: 1000,
  market_value: 100000,
  stats: {
    appearances: 0,
    goals: 0,
    assists: 0,
    clean_sheets: 0,
    yellow_cards: 0,
    red_cards: 0,
    avg_rating: 0,
    minutes_played: 0,
  },
  career: [],
  transfer_listed: false,
  loan_listed: false,
  transfer_offers: [],
  traits: [],
  ...overrides,
});

describe("SquadTab helpers", () => {
  it("normalises detailed positions into core roles", () => {
    expect(normalisePosition("Center Back")).toBe("Defender");
    expect(normalisePosition("Winger")).toBe("Forward");
    expect(normalisePosition("Striker")).toBe("Forward");
    expect(normalisePosition("Goalkeeper")).toBe("Goalkeeper");
  });

  it("builds preferred positions using normalised natural and alternate roles", () => {
    const player = makePlayer("p1", "Center Back", {
      natural_position: "Center Back",
      alternate_positions: ["Right Wing Back", "Defensive Midfielder"],
    });

    expect(getPreferredPositions(player)).toEqual([
      "CenterBack",
      "RightWingBack",
      "DefensiveMidfielder",
    ]);
  });

  it("detects out-of-position status using normalised roles", () => {
    const defender = makePlayer("p1", "Center Back", {
      natural_position: "Center Back",
      alternate_positions: ["Right Wing Back"],
    });

    expect(isPlayerOutOfPosition(defender, "Defender")).toBe(false);
    expect(isPlayerOutOfPosition(defender, "Midfielder")).toBe(true);
  });

  it("parses 4-part formations correctly", () => {
    expect(parseFormationSlots("4-2-3-1")).toEqual({ def: 4, mid: 5, fwd: 1 });
  });

  it("builds five pitch rows for 4-part formations", () => {
    const rows = buildPitchRows("4-2-3-1");
    expect(rows.map((row) => row.label)).toEqual(["GK", "DEF", "DM", "AM", "FWD"]);
    expect(rows[1].positions).toHaveLength(4);
    expect(rows[2].positions).toHaveLength(2);
    expect(rows[3].positions).toHaveLength(3);
    expect(rows[4].positions).toHaveLength(1);
    expect(rows[1].positions).toEqual([
      "LeftBack",
      "CenterBack",
      "CenterBack",
      "RightBack",
    ]);
  });

  it("keeps wide side-specific roles in left-to-right pitch order across formations", () => {
    expect(buildPitchRows("4-4-2")[2].positions).toEqual([
      "LeftMidfielder",
      "CentralMidfielder",
      "CentralMidfielder",
      "RightMidfielder",
    ]);
    expect(buildPitchRows("5-3-2")[1].positions).toEqual([
      "LeftWingBack",
      "CenterBack",
      "CenterBack",
      "CenterBack",
      "RightWingBack",
    ]);
    expect(buildPitchRows("4-2-3-1")[3].positions).toEqual([
      "LeftMidfielder",
      "AttackingMidfielder",
      "RightMidfielder",
    ]);
    expect(buildPitchRows("4-3-3")[3].positions).toEqual([
      "LeftWinger",
      "Striker",
      "RightWinger",
    ]);
  });

  it("returns compact pitch widths for crowded rows", () => {
    expect(getPitchSlotWidth(5)).toBeLessThan(getPitchSlotWidth(3));
    expect(getPitchSlotWidth(1)).toBeGreaterThan(getPitchSlotWidth(4));
  });

  it("prefers persisted starting XI ids when enough valid players remain", () => {
    const available = [
      makePlayer("gk", "Goalkeeper"),
      makePlayer("d1", "Defender"),
      makePlayer("d2", "Defender"),
      makePlayer("d3", "Defender"),
      makePlayer("d4", "Defender"),
      makePlayer("m1", "Midfielder"),
      makePlayer("m2", "Midfielder"),
      makePlayer("m3", "Midfielder"),
      makePlayer("m4", "Midfielder"),
      makePlayer("f1", "Forward"),
      makePlayer("f2", "Forward"),
      makePlayer("b1", "Forward"),
    ];

    const ids = buildStartingXIIds(
      available,
      ["gk", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
      "4-4-2",
    );

    expect(ids).toEqual(["gk", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"]);
  });

  it("auto-selects players by formation role when persisted ids are missing", () => {
    const available = [
      makePlayer("gk", "Goalkeeper"),
      makePlayer("d1", "Defender"),
      makePlayer("d2", "Defender"),
      makePlayer("d3", "Defender"),
      makePlayer("d4", "Defender"),
      makePlayer("m1", "Midfielder"),
      makePlayer("m2", "Midfielder"),
      makePlayer("m3", "Midfielder"),
      makePlayer("m4", "Midfielder"),
      makePlayer("f1", "Forward"),
      makePlayer("f2", "Forward"),
    ];

    const ids = buildStartingXIIds(available, [], "4-4-2");

    expect(ids).toHaveLength(11);
    expect(ids[0]).toBe("gk");
    expect(ids.slice(1, 5)).toEqual(["d1", "d2", "d3", "d4"]);
  });

  it("prefers an exact slot match over a higher-ovr same-group player", () => {
    const available = [
      makePlayer("gk", "Goalkeeper"),
      makePlayer("lb", "Left Back", {
        natural_position: "Left Back",
        attributes: {
          pace: 55,
          stamina: 55,
          strength: 55,
          agility: 55,
          passing: 55,
          shooting: 40,
          tackling: 68,
          dribbling: 50,
          defending: 68,
          positioning: 62,
          vision: 52,
          decisions: 58,
          composure: 56,
          aggression: 58,
          teamwork: 60,
          leadership: 50,
          handling: 10,
          reflexes: 10,
          aerial: 10,
        },
      }),
      makePlayer("cb1", "Center Back", {
        natural_position: "Center Back",
        attributes: {
          pace: 70,
          stamina: 70,
          strength: 74,
          agility: 62,
          passing: 62,
          shooting: 42,
          tackling: 78,
          dribbling: 52,
          defending: 80,
          positioning: 74,
          vision: 58,
          decisions: 70,
          composure: 68,
          aggression: 71,
          teamwork: 68,
          leadership: 60,
          handling: 10,
          reflexes: 10,
          aerial: 10,
        },
      }),
      makePlayer("cb2", "Center Back"),
      makePlayer("rb", "Right Back", { natural_position: "Right Back" }),
      makePlayer("m1", "Midfielder"),
      makePlayer("m2", "Midfielder"),
      makePlayer("m3", "Midfielder"),
      makePlayer("m4", "Midfielder"),
      makePlayer("f1", "Forward"),
      makePlayer("f2", "Forward"),
    ];

    const ids = buildStartingXIIds(available, [], "4-4-2");

    expect(ids[1]).toBe("lb");
  });

  it("builds pitch slot rows and active position map from xi ids", () => {
    const players = [
      makePlayer("gk", "Goalkeeper"),
      makePlayer("d1", "LeftBack"),
      makePlayer("d2", "CenterBack"),
      makePlayer("d3", "CenterBack"),
      makePlayer("d4", "RightBack"),
      makePlayer("m1", "LeftMidfielder"),
      makePlayer("m2", "CentralMidfielder"),
      makePlayer("m3", "CentralMidfielder"),
      makePlayer("m4", "RightMidfielder"),
      makePlayer("f1", "Striker"),
      makePlayer("f2", "Striker"),
    ];
    const xiIds = players.map((player) => player.id);
    const rows = buildPitchRows("4-4-2");
    const slotRows = buildPitchSlotRows(rows, xiIds, new Map(players.map((p) => [p.id, p])));
    const activeMap = buildActivePositionMap(slotRows);

    expect(slotRows[0].slots[0].player?.id).toBe("gk");
    expect(activeMap.get("d1")).toBe("LeftBack");
    expect(activeMap.get("m1")).toBe("LeftMidfielder");
    expect(activeMap.get("f2")).toBe("Striker");
  });

  it("preserves saved xi order for side-specific wide roles", () => {
    const available = [
      makePlayer("gk", "Goalkeeper"),
      makePlayer("rb", "RightBack", {
        natural_position: "RightBack",
        footedness: "Right",
        weak_foot: 1,
      }),
      makePlayer("cb1", "CenterBack"),
      makePlayer("cb2", "CenterBack"),
      makePlayer("lb", "LeftBack", {
        natural_position: "LeftBack",
        footedness: "Left",
        weak_foot: 1,
      }),
      makePlayer("rm", "RightMidfielder", {
        natural_position: "RightMidfielder",
        footedness: "Right",
        weak_foot: 1,
      }),
      makePlayer("cm1", "CentralMidfielder"),
      makePlayer("cm2", "CentralMidfielder"),
      makePlayer("lm", "LeftMidfielder", {
        natural_position: "LeftMidfielder",
        footedness: "Left",
        weak_foot: 1,
      }),
      makePlayer("st1", "Striker"),
      makePlayer("st2", "Striker"),
    ];

    const ids = buildStartingXIIds(
      available,
      ["gk", "rb", "cb1", "cb2", "lb", "rm", "cm1", "cm2", "lm", "st1", "st2"],
      "4-4-2",
    );

    expect(ids).toEqual([
      "gk",
      "rb",
      "cb1",
      "cb2",
      "lb",
      "rm",
      "cm1",
      "cm2",
      "lm",
      "st1",
      "st2",
    ]);
  });

  it("swaps XI players when dragging from one slot to another", () => {
    const nextXiIds = applyLineupDrop(
      ["gk", "d1", "d2", "d3"],
      { playerId: "d1", from: "xi", slotIndex: 1 },
      3,
    );

    expect(nextXiIds).toEqual(["gk", "d3", "d2", "d1"]);
  });

  it("replaces the target slot when dropping a bench player onto the pitch", () => {
    const nextXiIds = applyLineupDrop(
      ["gk", "d1", "d2", "d3"],
      { playerId: "b1", from: "bench", slotIndex: null },
      2,
    );

    expect(nextXiIds).toEqual(["gk", "d1", "b1", "d3"]);
  });

  it("keeps order stable when a dragged bench player is already present in the xi", () => {
    const nextXiIds = applyLineupDrop(
      ["gk", "d1", "b1", "d3"],
      { playerId: "b1", from: "bench", slotIndex: null },
      1,
    );

    expect(nextXiIds).toEqual(["gk", "b1", "d1", "d3"]);
  });

  it("supports bench-to-xi and xi-to-xi swap actions", () => {
    expect(
      applyLineupSwap(["gk", "d1", "d2"], { id: "b1", from: "bench" }, "d2", "xi"),
    ).toEqual(["gk", "d1", "b1"]);

    expect(
      applyLineupSwap(["gk", "d1", "d2"], { id: "d1", from: "xi" }, "d2", "xi"),
    ).toEqual(["gk", "d2", "d1"]);
  });

  it("returns core position codes", () => {
    expect(positionCode("Center Back")).toBe("CB");
    expect(positionCode("Striker")).toBe("ST");
  });

  it("translates normalized position abbreviations with fallback codes", () => {
    const translate = (key: string): string => key;

    expect(translatePositionAbbreviation(translate, "Center Back")).toBe(
      "common.posAbbr.CenterBack",
    );
    expect(translatePositionAbbreviation(translate, "Striker")).toBe(
      "common.posAbbr.Striker",
    );
  });

  it("classifies tactical fit as natural, adapted, or out", () => {
    expect(
      getSquadTacticalFit(
        makePlayer("lb", "Left Back", { natural_position: "Left Back" }),
        "LeftBack",
      ),
    ).toBe("natural");

    expect(
      getSquadTacticalFit(
        makePlayer("def", "Defender", { natural_position: "Defender" }),
        "CenterBack",
      ),
    ).toBe("adapted");

    expect(
      getSquadTacticalFit(
        makePlayer("fwd", "Forward", { natural_position: "Forward" }),
        "CenterBack",
      ),
    ).toBe("out");
  });

  it("promotes a bench player into the slot that fits them best", () => {
    const players = [
      makePlayer("gk", "Goalkeeper"),
      makePlayer("d1", "Defender"),
      makePlayer("d2", "Defender"),
      makePlayer("d3", "Defender"),
      makePlayer("d4", "Defender"),
      makePlayer("m1", "Midfielder"),
      makePlayer("m2", "Midfielder"),
      makePlayer("m3", "Midfielder"),
      makePlayer("m4", "Midfielder"),
      makePlayer("f1", "Forward"),
      makePlayer("f2", "Forward"),
      makePlayer("lb", "Left Back", { natural_position: "Left Back" }),
    ];
    const playersById = new Map(players.map((player) => [player.id, player]));

    expect(
      buildPromoteToStartingXi(
        ["gk", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
        playersById,
        "4-4-2",
        "lb",
      ),
    ).toEqual([
      "gk",
      "lb",
      "d2",
      "d3",
      "d4",
      "m1",
      "m2",
      "m3",
      "m4",
      "f1",
      "f2",
    ]);
  });

  it("demotes a starter by replacing them with the best bench fit for the slot", () => {
    const players = [
      makePlayer("gk", "Goalkeeper"),
      makePlayer("d1", "Defender"),
      makePlayer("d2", "Defender"),
      makePlayer("d3", "Defender"),
      makePlayer("d4", "Defender"),
      makePlayer("m1", "Midfielder"),
      makePlayer("m2", "Midfielder"),
      makePlayer("m3", "Midfielder"),
      makePlayer("m4", "Midfielder"),
      makePlayer("f1", "Forward"),
      makePlayer("f2", "Forward"),
      makePlayer("lb", "Left Back", { natural_position: "Left Back" }),
    ];

    expect(
      buildDemoteFromStartingXi(
        ["gk", "d1", "d2", "d3", "d4", "m1", "m2", "m3", "m4", "f1", "f2"],
        players,
        "4-4-2",
        "d1",
      ),
    ).toEqual([
      "gk",
      "lb",
      "d2",
      "d3",
      "d4",
      "m1",
      "m2",
      "m3",
      "m4",
      "f1",
      "f2",
    ]);
  });

  it("suggests a best role within the current formation shape", () => {
    expect(
      getBestRoleForFormation(
        makePlayer("lb", "Left Back", { natural_position: "Left Back" }),
        "4-4-2",
      ),
    ).toBe("LeftBack");

    expect(
      getBestRoleForFormation(
        makePlayer("mid", "Midfielder", { natural_position: "Midfielder" }),
        "4-2-3-1",
      ),
    ).toBe("CentralMidfielder");
  });

  it("prefers the natural position group over a tangential alternate", () => {
    // A DM with RB among alternates in a DM-less formation must resolve to a
    // midfield role, not jump across groups to RightBack.
    expect(
      getBestRoleForFormation(
        makePlayer("dm", "Defensive Midfielder", {
          natural_position: "Defensive Midfielder",
          alternate_positions: ["Right Back"],
        }),
        "4-4-2",
      ),
    ).toBe("CentralMidfielder");

    // The alternate is still used when no role in the player's own group fits.
    expect(
      getBestRoleForFormation(
        makePlayer("st", "Striker", {
          natural_position: "Striker",
          alternate_positions: ["Right Back"],
        }),
        "4-0-0",
      ),
    ).toBe("RightBack");
  });

  it("scores play-style fit using relevant attribute mixes", () => {
    expect(
      getPlayStyleFit(
        makePlayer("press", "Midfielder", {
          attributes: {
            pace: 78,
            stamina: 80,
            strength: 60,
            agility: 65,
            passing: 62,
            shooting: 58,
            tackling: 77,
            dribbling: 61,
            defending: 63,
            positioning: 66,
            vision: 60,
            decisions: 68,
            composure: 62,
            aggression: 79,
            teamwork: 81,
            leadership: 60,
            handling: 10,
            reflexes: 10,
            aerial: 60,
          },
        }),
        "HighPress",
      ),
    ).toBe("strong");

    expect(
      getPlayStyleFit(
        makePlayer("low", "Forward", {
          attributes: {
            pace: 45,
            stamina: 48,
            strength: 50,
            agility: 52,
            passing: 46,
            shooting: 49,
            tackling: 44,
            dribbling: 47,
            defending: 42,
            positioning: 48,
            vision: 46,
            decisions: 45,
            composure: 44,
            aggression: 40,
            teamwork: 43,
            leadership: 39,
            handling: 10,
            reflexes: 10,
            aerial: 55,
          },
        }),
        "Possession",
      ),
    ).toBe("risky");
  });

  it("downgrades play-style fit when the player is out of position", () => {
    const striker = makePlayer("st", "Striker", { natural_position: "Striker" });

    // Attribute-only fit (no position) is unchanged.
    expect(getPlayStyleFit(striker, "Attacking")).toBe("good");
    // In a natural slot the position imposes no penalty.
    expect(getPlayStyleFit(striker, "Attacking", "Striker")).toBe("good");
    // Played out of position the same attributes yield a worse fit.
    expect(getPlayStyleFit(striker, "Attacking", "CenterBack")).toBe("risky");
  });

  it("summarises role coverage for the active shape", () => {
    const players = [
      makePlayer("gk", "Goalkeeper"),
      makePlayer("lb", "Left Back", { natural_position: "Left Back" }),
      makePlayer("cb1", "Center Back", { natural_position: "Center Back" }),
      makePlayer("cb2", "Center Back", { natural_position: "Center Back" }),
      makePlayer("rb", "Right Back", { natural_position: "Right Back" }),
      makePlayer("lm", "Left Midfielder", { natural_position: "Left Midfielder" }),
      makePlayer("cm1", "Central Midfielder", {
        natural_position: "Central Midfielder",
      }),
      makePlayer("cm2", "Central Midfielder", {
        natural_position: "Central Midfielder",
      }),
      makePlayer("rm", "Right Midfielder", { natural_position: "Right Midfielder" }),
      makePlayer("st1", "Striker", { natural_position: "Striker" }),
      makePlayer("st2", "Striker", { natural_position: "Striker" }),
      makePlayer("gk2", "Goalkeeper", { natural_position: "Goalkeeper" }),
      makePlayer("cb3", "Center Back", { natural_position: "Center Back" }),
    ];

    const coverage = buildRoleCoverageSummary(
      players,
      ["gk", "lb", "cb1", "cb2", "rb", "lm", "cm1", "cm2", "rm", "st1", "st2"],
      "4-4-2",
    );

    expect(coverage.find((role) => role.role === "Goalkeeper")).toMatchObject({
      requiredSlots: 1,
      naturalStarters: 1,
      benchOptions: 1,
      status: "covered",
    });

    expect(coverage.find((role) => role.role === "LeftBack")).toMatchObject({
      requiredSlots: 1,
      naturalStarters: 1,
      benchOptions: 0,
      status: "thin",
    });

    expect(coverage.find((role) => role.role === "RightMidfielder")).toMatchObject({
      requiredSlots: 1,
      naturalStarters: 1,
      benchOptions: 0,
      status: "thin",
    });
  });
});
