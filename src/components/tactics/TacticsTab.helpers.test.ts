import { describe, expect, it } from "vitest";
import type { PlayerData } from "../../store/gameStore";
import { createPositionTable } from "../../lib/positionTable";
import type { FamiliarityTable } from "../../hooks/useFamiliarity";
import type { Familiarity } from "../../store/generated/Familiarity";
import { cannedPositionTable } from "../../test-utils/positionTable";
import {
  buildTacticsRoster,
  countOutOfPositionPlayers,
  filterAndSortTacticsPlayers,
  resolveStartingXiIds,
} from "./TacticsTab.helpers";

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

describe("resolveStartingXiIds", () => {
  it("prefers exact slot matches when filling pending tactics slots", () => {
    const availablePlayers = [
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
    const playersById = new Map(availablePlayers.map((player) => [player.id, player]));

    const ids = resolveStartingXiIds({
      availablePlayers,
      positionTable: cannedPositionTable(),
      formation: "4-4-2",
      pendingStartingXiIds: ["gk"],
      playersById,
      savedStartingXiIds: [],
    });

    expect(ids[1]).toBe("lb");
  });
});

describe("the tactics roster and its lists, by the backend's position table", () => {
  const roster = () => [
    makePlayer("fwd", "Striker", { team_id: "t1", match_day_eligible: true }),
    makePlayer("gk", "Goalkeeper", { team_id: "t1", match_day_eligible: true }),
    makePlayer("def", "CenterBack", { team_id: "t1", match_day_eligible: true }),
  ];

  /**
   * Given a squad listed in no particular order
   * When the tactics roster is built with the loaded table
   * Then the lines run keeper, defence, attack: the order is the table's, not the input's.
   */
  it("orders the roster by the table's lines", () => {
    const ids = buildTacticsRoster(roster(), "t1", cannedPositionTable()).map((p) => p.id);

    expect(ids).toEqual(["gk", "def", "fwd"]);
  });

  /**
   * Given the same lists and a loaded table
   * When the XI list is sorted by position
   * Then it runs keeper, defence, attack too.
   */
  it("sorts a list by position along the table's lines", () => {
    const positionTable = cannedPositionTable();
    const sorted = filterAndSortTacticsPlayers(
      roster(),
      {
        playerSearch: "",
        positionFilter: "All",
        positionTable,
        section: "bench",
        xiActivePosition: new Map(),
      },
      {
        positionTable,
        section: "bench",
        sortDir: "asc",
        sortKey: "pos",
        xiActivePosition: new Map(),
      },
    );

    expect(sorted.map((p) => p.id)).toEqual(["gk", "def", "fwd"]);
  });

  /**
   * Given a defender put in the forward line, with the table loaded and not loaded
   * When the out-of-position players are counted
   * Then he is counted once loaded, and nobody is counted before the table arrives.
   */
  it("counts out-of-position starters only once the table has loaded", () => {
    const xi = [makePlayer("def", "CenterBack", { team_id: "t1" })];
    const slots = new Map([["def", "Striker"]]);

    expect(countOutOfPositionPlayers(xi, slots, cannedPositionTable())).toBe(1);
    expect(countOutOfPositionPlayers(xi, slots, createPositionTable([]))).toBe(0);
  });

  describe("when the screen reads the backend's familiarity", () => {
    // A table that says what the test says per player, whatever his own positions are: it proves
    // the helpers read the table and do not grade the player themselves.
    const byPlayer = (words: Record<string, Familiarity>): FamiliarityTable => ({
      loaded: true,
      familiarityOf: (playerId) => words[playerId] ?? null,
    });

    /**
     * Given a centre back in his own slot whom the backend calls out of position there
     * When the out-of-position players are counted with that table
     * Then he is counted: the backend's word wins over his positions.
     */
    it("counts what the backend calls out of position", () => {
      const xi = [makePlayer("cb", "CenterBack", { team_id: "t1" })];
      const slots = new Map([["cb", "CenterBack"]]);

      expect(
        countOutOfPositionPlayers(
          xi,
          slots,
          cannedPositionTable(),
          byPlayer({ cb: "out_of_position" }),
        ),
      ).toBe(1);
      expect(countOutOfPositionPlayers(xi, slots, cannedPositionTable())).toBe(0);
    });

    /**
     * Given a pool of midfielders in which only the lowest-rated, last-named one is the backend's
     *       natural goalkeeper, and no saved lineup
     * When the lineup is resolved with that table
     * Then he takes the goalkeeper slot: the backend's grade orders the auto-fill.
     */
    it("fills a slot by the backend's grade", () => {
      const weak = { ...makePlayer("zz-keeper", "Midfielder").attributes, pace: 20, passing: 20 };
      const availablePlayers = [
        ...Array.from({ length: 11 }, (_, index) => makePlayer(`m${index}`, "Midfielder")),
        makePlayer("zz-keeper", "Midfielder", { attributes: weak, condition: 50 }),
      ];
      const playersById = new Map(availablePlayers.map((player) => [player.id, player]));

      const ids = resolveStartingXiIds({
        availablePlayers,
        positionTable: cannedPositionTable(),
        familiarity: byPlayer({ "zz-keeper": "natural" }),
        formation: "4-4-2",
        pendingStartingXiIds: null,
        playersById,
        savedStartingXiIds: [],
      });

      expect(ids[0]).toBe("zz-keeper");
    });
  });
});
